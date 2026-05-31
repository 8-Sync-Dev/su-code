// `8sync clean` — reclaim disk/RAM + report CPU/GPU on CachyOS.
//
// Philosophy (Karpathy: no cargo-cult): only do things that genuinely help and
// are safe to re-run. We do NOT force the CPU governor (amd-pstate `powersave`
// IS the efficient dynamic mode — forcing `performance` just burns watts/heat),
// we do NOT kill processes, and "free RAM" is opt-in + honest (the kernel
// reclaims pagecache on demand; dropping it is mostly cosmetic).
//
//   8sync clean              safe reclaim: pacman/AUR cache, journal, tmpfiles,
//                            thumbnails  + CPU/GPU/RAM report
//   8sync clean --deep       + orphan pkgs, regenerable dev caches, tighter journal
//   8sync clean --ram        + drop pagecache (light, cosmetic) + report
//   8sync clean --gpu        + NVIDIA persistence mode on + GPU report
//   8sync clean --dry-run    print the plan, change nothing
//   8sync clean --watch [S]  run forever, cleaning every S seconds (default 3600)
//   8sync clean --timer 1h   install a systemd USER timer (1h | 30min | …)
//   8sync clean --timer off  remove the timer

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync clean                safe reclaim + CPU/GPU/RAM report
          8sync clean --deep         + orphan pkgs + build caches (go-build/tsc/node-gyp)
          8sync clean --ram          also drop pagecache (light RAM reclaim)
          8sync clean --gpu          NVIDIA persistence mode + GPU report
          8sync clean --dry-run      preview, change nothing
          8sync clean --watch        loop forever, clean every 1h (Ctrl-C stops)
          8sync clean --watch 1800   loop, clean every 30 min
          8sync clean --timer 1h     install systemd user timer (every 1h)
          8sync clean --timer off    remove the timer

        NEVER auto-deleted (only reported + manual reclaim hint): AI models
        (huggingface/torch), Playwright/Puppeteer/Cypress/Electron browser binaries,
        and package download caches (uv/pip/yarn/pnpm/deno). Routine clean won't
        break a project or force a big re-download.

        NOTE: the CPU governor is reported, never changed — on amd-pstate the
        `powersave` governor IS the efficient dynamic mode; forcing `performance`
        only raises power/heat with no real-world win for bursty desktop work.
    "}
)]
pub struct Args {
    /// Deep clean: orphan packages + pure build caches (go-build/tsc/node-gyp) +
    /// tighter journal vacuum. Does NOT touch models, browser binaries, or
    /// package download caches (those are only reported).
    #[arg(long)]
    pub deep: bool,

    /// Also drop pagecache (light, cosmetic — kernel reclaims on demand anyway).
    #[arg(long)]
    pub ram: bool,

    /// Enable NVIDIA persistence mode + print a GPU summary.
    #[arg(long)]
    pub gpu: bool,

    /// Print the plan without changing anything.
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Run forever, cleaning every N seconds (default 3600 when flag given alone).
    #[arg(long, value_name = "SECS", num_args = 0..=1, default_missing_value = "3600")]
    pub watch: Option<u64>,

    /// Install a systemd user timer (`1h`, `30min`, …) or remove it with `off`.
    #[arg(long, value_name = "DUR")]
    pub timer: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    if let Some(spec) = a.timer.as_deref() {
        return manage_timer(spec);
    }
    if let Some(secs) = a.watch {
        let secs = secs.max(60); // never busy-loop
        ui::header(&format!("8sync clean --watch ({}s interval)", secs));
        ui::info("Ctrl-C to stop. Each pass runs a safe clean.");
        loop {
            clean_once(&a);
            ui::step(&format!("sleeping {}s …", secs));
            std::thread::sleep(std::time::Duration::from_secs(secs));
        }
    }
    clean_once(&a);
    Ok(())
}

fn clean_once(a: &Args) {
    let dry = a.dry_run;
    ui::header(if a.deep { "8sync clean --deep" } else { "8sync clean" });

    let ram0 = mem_available_mb();
    let disk0 = root_avail_mb();

    // ── disk: pacman package cache ──────────────────────────────────
    ui::step("pacman package cache");
    if which::which("paccache").is_ok() {
        sudo_run(dry, &["paccache", "-rk2"]); // keep 2 newest of installed pkgs
        sudo_run(dry, &["paccache", "-ruk0"]); // drop ALL cached uninstalled pkgs
    } else {
        ui::skip("paccache", "pacman-contrib not installed");
    }

    // ── disk: AUR helper build/clone cache ──────────────────────────
    if which::which("paru").is_ok() {
        ui::step("paru cache");
        run_cmd(dry, &["paru", "-Sc", "--noconfirm"]);
    } else if which::which("yay").is_ok() {
        ui::step("yay cache");
        run_cmd(dry, &["yay", "-Sc", "--noconfirm"]);
    }

    // ── disk: journal ───────────────────────────────────────────────
    ui::step("systemd journal vacuum");
    let vac = if a.deep { "--vacuum-size=100M" } else { "--vacuum-size=200M" };
    sudo_run(dry, &["journalctl", vac]);

    // ── disk: tmpfiles (ages out /tmp, /var/tmp per system rules) ───
    ui::step("tmpfiles clean (/tmp, /var/tmp)");
    sudo_run(dry, &["systemd-tmpfiles", "--clean"]);

    // ── disk: user caches ───────────────────────────────────────────
    ui::step("user caches (~/.cache)");
    // ONLY pure junk / build artifacts with ZERO downloads, binaries, or models.
    // Expensive caches (AI models, Playwright/Puppeteer browser binaries, package
    // download caches) are NEVER auto-deleted — they are reported with a manual
    // reclaim command instead. See report_caches(). This is deliberate: deleting
    // them doesn't corrupt anything but forces slow re-downloads / breaks test
    // runs until re-fetched, which is not what a routine `clean` should do.
    let mut safe = vec!["thumbnails"]; // image thumbnails — regenerated on view
    if a.deep {
        // Pure compiler/build artifacts: recompiled on next build, nothing fetched.
        safe.extend(["go-build", "typescript", "node-gyp"]);
    }
    clean_cache_subdirs(dry, &safe);
    report_caches();

    // ── disk: orphan packages (deep only) ───────────────────────────
    if a.deep {
        ui::step("orphan packages");
        remove_orphans(dry);
    }

    // ── RAM (opt-in, honest) ────────────────────────────────────────
    if a.ram {
        ui::step("RAM: drop pagecache (light)");
        if dry {
            ui::info("would: sync && echo 1 > /proc/sys/vm/drop_caches");
        } else {
            let _ = Command::new("sync").status();
            let st = Command::new("sudo")
                .args(["sh", "-c", "echo 1 > /proc/sys/vm/drop_caches"])
                .status();
            match st {
                Ok(s) if s.success() => ui::ok("pagecache dropped (kernel will refill on demand)"),
                _ => ui::warn("could not drop pagecache (sudo?)"),
            }
        }
    }

    // ── GPU (opt-in) ────────────────────────────────────────────────
    if a.gpu {
        gpu_optimize(dry);
    }

    // ── report ──────────────────────────────────────────────────────
    ui::step("system status");
    report_cpu();
    report_gpu_brief();
    report_mem();

    let ram1 = mem_available_mb();
    let disk1 = root_avail_mb();
    if !dry {
        if let (Some(d0), Some(d1)) = (disk0, disk1) {
            let freed = d1 as i64 - d0 as i64;
            ui::ok(&format!("disk freed on /: {}", human_mb_delta(freed)));
        }
        if let (Some(r0), Some(r1)) = (ram0, ram1) {
            let freed = r1 as i64 - r0 as i64;
            ui::info(&format!("RAM available change: {}", human_mb_delta(freed)));
        }
    }
}

// ── cache helpers ───────────────────────────────────────────────────

fn clean_cache_subdirs(dry: bool, names: &[&str]) {
    let Some(home) = dirs::home_dir() else { return };
    let cache = home.join(".cache");
    for n in names {
        let p = cache.join(n);
        if !p.exists() {
            continue;
        }
        if dry {
            ui::info(&format!("would: rm -rf {}", p.display()));
        } else if std::fs::remove_dir_all(&p).is_ok() {
            ui::ok(&format!("cleared ~/.cache/{}", n));
        }
    }
}

/// Report ~/.cache usage. Explicitly lists EXPENSIVE caches we never auto-delete
/// (models, browser binaries, package downloads) with the manual command to
/// reclaim each — so the user decides, and a routine clean never breaks a project.
fn report_caches() {
    let Some(home) = dirs::home_dir() else { return };
    let cache = home.join(".cache");

    // (subdir, what it is, manual reclaim command) — kept, NEVER auto-deleted.
    let kept: &[(&str, &str, &str)] = &[
        ("huggingface",   "AI models (HF)",        "huggingface-cli delete-cache  # or rm -rf ~/.cache/huggingface"),
        ("torch",         "PyTorch model cache",   "rm -rf ~/.cache/torch"),
        ("ms-playwright", "Playwright browsers",   "npx playwright uninstall --all"),
        ("puppeteer",     "Puppeteer Chromium",    "rm -rf ~/.cache/puppeteer"),
        ("Cypress",       "Cypress binary",        "cypress cache clear"),
        ("electron",      "Electron binaries",     "rm -rf ~/.cache/electron"),
        ("uv",            "uv wheel/download cache","uv cache clean"),
        ("pip",           "pip download cache",    "pip cache purge"),
        ("yarn",          "yarn package cache",    "yarn cache clean"),
        ("pnpm",          "pnpm store",            "pnpm store prune"),
        ("deno",          "deno dep cache",        "deno clean"),
    ];

    if let Ok(o) = Command::new("du").args(["-sh", "--"]).arg(&cache).output() {
        if let Some(sz) = String::from_utf8_lossy(&o.stdout).split_whitespace().next() {
            ui::info(&format!("~/.cache total: {}", sz));
        }
    }

    let mut any = false;
    for (sub, what, cmd) in kept {
        let p = cache.join(sub);
        if !p.exists() {
            continue;
        }
        let sz = Command::new("du").args(["-sh", "--"]).arg(&p).output().ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).split_whitespace().next().unwrap_or("?").to_string())
            .unwrap_or_else(|| "?".into());
        if !any {
            ui::info("kept (NOT deleted — models/binaries/downloads; reclaim yourself if needed):");
            any = true;
        }
        ui::info(&format!("  ~/.cache/{:<13} {:>6}  {} → {}", sub, sz, what, cmd));
    }
}

fn remove_orphans(dry: bool) {
    let out = Command::new("pacman").args(["-Qtdq"]).output();
    let orphans: Vec<String> = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    };
    if orphans.is_empty() {
        ui::skip("orphans", "none");
        return;
    }
    ui::info(&format!("orphans ({}): {}", orphans.len(), orphans.join(" ")));
    if dry {
        ui::info("would: sudo pacman -Rns <orphans>");
        return;
    }
    // Interactive confirm — pacman -Rns is removal; let pacman prompt.
    let mut args = vec!["pacman", "-Rns"];
    for o in &orphans {
        args.push(o);
    }
    let st = Command::new("sudo").args(&args).status();
    match st {
        Ok(s) if s.success() => ui::ok(&format!("removed {} orphan(s)", orphans.len())),
        _ => ui::warn("orphan removal skipped/failed"),
    }
}

// ── CPU / GPU / RAM reporting ───────────────────────────────────────

fn report_cpu() {
    let gov = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        .ok()
        .map(|s| s.trim().to_string());
    match gov {
        Some(g) => {
            let note = if g == "powersave" {
                " (amd-pstate dynamic — efficient; not changed)"
            } else {
                ""
            };
            ui::info(&format!("cpu governor: {}{}", g, note));
        }
        None => {}
    }
    // Load average.
    if let Ok(la) = std::fs::read_to_string("/proc/loadavg") {
        if let Some(part) = la.split_whitespace().take(3).collect::<Vec<_>>().get(..3) {
            ui::info(&format!("load avg: {}", part.join(" ")));
        }
    }
    // Top 3 CPU hogs.
    if let Ok(o) = Command::new("sh")
        .arg("-c")
        .arg("ps -eo pid,comm,%cpu --sort=-%cpu | head -4 | tail -3")
        .output()
    {
        let s = String::from_utf8_lossy(&o.stdout);
        for line in s.lines() {
            ui::info(&format!("  cpu: {}", line.trim()));
        }
    }
}

fn report_mem() {
    if let Ok(o) = Command::new("free").args(["-h"]).output() {
        let s = String::from_utf8_lossy(&o.stdout);
        for line in s.lines().take(3) {
            ui::info(&format!("  {}", line.trim_end()));
        }
    }
}

fn report_gpu_brief() {
    if which::which("nvidia-smi").is_err() {
        return;
    }
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
            "--format=csv,noheader",
        ])
        .output();
    if let Ok(o) = out {
        let s = String::from_utf8_lossy(&o.stdout);
        let line = s.trim();
        if !line.is_empty() {
            ui::info(&format!("gpu: {}", line));
        }
    }
}

fn gpu_optimize(dry: bool) {
    if which::which("nvidia-smi").is_err() {
        ui::skip("gpu", "no nvidia-smi");
        return;
    }
    ui::step("NVIDIA persistence mode");
    if dry {
        ui::info("would: sudo nvidia-smi -pm 1");
        return;
    }
    let st = Command::new("sudo").args(["nvidia-smi", "-pm", "1"]).status();
    match st {
        Ok(s) if s.success() => ui::ok("persistence mode ON (lower driver re-init latency)"),
        _ => ui::warn("could not set persistence mode (sudo?)"),
    }
}

// ── shell helpers ───────────────────────────────────────────────────

fn run_cmd(dry: bool, args: &[&str]) { if dry {
    ui::info(&format!("would: {}", args.join(" ")));
    return;
}
ui::info(&format!("$ {}", args.join(" ")));
let _ = Command::new(args[0]).args(&args[1..]).status(); }

fn sudo_run(dry: bool, args: &[&str]) {
    if dry {
        ui::info(&format!("would: sudo {}", args.join(" ")));
        return;
    }
    ui::info(&format!("$ sudo {}", args.join(" ")));
    let _ = Command::new("sudo").args(args).status();
}

fn mem_available_mb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

fn root_avail_mb() -> Option<u64> {
    let o = Command::new("df").args(["-BM", "--output=avail", "/"]).output().ok()?;
    let s = String::from_utf8_lossy(&o.stdout);
    let last = s.lines().nth(1)?.trim().trim_end_matches('M');
    last.parse().ok()
}

fn human_mb_delta(mb: i64) -> String {
    let sign = if mb >= 0 { "+" } else { "-" };
    let v = mb.unsigned_abs();
    if v >= 1024 {
        format!("{}{:.1} GB", sign, v as f64 / 1024.0)
    } else {
        format!("{}{} MB", sign, v)
    }
}

// ── systemd user timer ──────────────────────────────────────────────

fn manage_timer(spec: &str) -> Result<()> {
    let home = dirs::home_dir().context("no HOME")?;
    let unit_dir = home.join(".config/systemd/user");
    let svc = unit_dir.join("8sync-clean.service");
    let timer = unit_dir.join("8sync-clean.timer");

    if spec.eq_ignore_ascii_case("off") {
        ui::header("8sync clean --timer off");
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "--now", "8sync-clean.timer"])
            .status();
        let _ = std::fs::remove_file(&svc);
        let _ = std::fs::remove_file(&timer);
        let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
        ui::ok("timer removed");
        return Ok(());
    }

    ui::header(&format!("8sync clean --timer {}", spec));
    let exe = std::env::current_exe().context("current_exe")?;
    std::fs::create_dir_all(&unit_dir)?;

    let svc_body = format!(
        "[Unit]\nDescription=8sync periodic clean\n\n[Service]\nType=oneshot\nTimeoutStartSec=300\nExecStart={} clean\n",
        exe.display()
    );
    let timer_body = format!(
        "[Unit]\nDescription=8sync clean timer (every {dur})\n\n[Timer]\nOnBootSec=10min\nOnUnitActiveSec={dur}\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n",
        dur = spec
    );
    std::fs::write(&svc, svc_body)?;
    std::fs::write(&timer, timer_body)?;
    ui::ok(&format!("wrote {} + .timer", svc.display()));

    let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
    let st = Command::new("systemctl")
        .args(["--user", "enable", "--now", "8sync-clean.timer"])
        .status();
    match st {
        Ok(s) if s.success() => {
            ui::ok(&format!("timer enabled — runs `8sync clean` every {}", spec));
            ui::info("status: systemctl --user list-timers 8sync-clean.timer");
            ui::info("note: needs lingering for boot-time runs → `loginctl enable-linger $USER`");
            ui::info("note: headless timer can't sudo — auto-runs only do user-cache/paru cleanup;");
            ui::info("      run `8sync clean` in a terminal for pacman-cache + journal + tmpfiles too.");
        }
        _ => ui::warn("could not enable timer (is `systemctl --user` available?)"),
    }
    Ok(())
}
