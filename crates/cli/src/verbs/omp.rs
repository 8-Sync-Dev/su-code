// `8sync omp update` — update omp, auto-repairing a failed/stalled update.
//
// omp self-updates by downloading the standalone release binary
// (`omp-linux-x64`, ~185 MB) to `<bin>.<pid-ish>.new` and swapping it in. On
// slow links that download can take minutes with zero progress output, and the
// updater has no timeout of its own — an interrupted run leaves multi-hundred-MB
// `omp.*.new` partials behind and the user staring at a dead terminal. (An older
// failure mode: a squatter real file where npm/bun wanted a symlink, reported as
// `EEXIST` / `Fail extracting tarball`.)
//
// This verb runs the normal update with a timeout + heartbeat so a slow
// download is visible and a stalled one gets killed; on any failure it
// auto-repairs: back up the current binary, sweep the `.new` partials, and
// reinstall via the official installer (`omp.sh/install --binary` — the same
// channel `8sync setup` uses; it curl-stall-protects and smoke-tests the
// binary). Unlike an in-omp `/command`, this runs from the shell — so it works
// even when omp itself is broken, which is exactly when you need it.
//
// `8sync up` stays decoupled (it only updates the 8sync binary); this is the
// omp-side counterpart.

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::ui;

/// Official installer (same one `8sync setup` uses). `--binary` forces the
/// prebuilt standalone → lands at `~/.local/bin/omp`, matching the self-update
/// layout instead of diverging into a bun/npm-managed install.
const INSTALLER: &str = "curl -fsSL https://omp.sh/install | sh -s -- --binary";
/// Runnable backup of the current omp binary, kept across the repair.
const BACKUP: &str = "/tmp/omp-selfheal.bak";
/// The updater downloads ~185 MB with no progress output; slow links need
/// minutes. Anything past this is a genuine stall, not slowness.
const UPDATE_TIMEOUT_SECS: u64 = 600;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync omp update            update omp; time-box + auto-repair on stall/failure
          8sync omp update --force    skip the normal try, go straight to clean reinstall
          8sync omp                   alias of `omp update`

        The updater downloads a ~185 MB standalone binary with no progress
        output — on slow links that looks like a hang, and an interrupted run
        leaves `~/.local/bin/omp.*.new` partials. This verb heartbeats while
        downloading, kills a genuine stall after 10 min, sweeps the partials,
        and on failure reinstalls via the official `omp.sh/install --binary`
        (the same channel `8sync setup` uses). Touches ONLY the omp install —
        never sudo, system pkgs, or git.
    "}
)]
pub struct Args {
    /// Sub-action. Only `update` is supported (the default).
    #[arg(value_name = "ACTION", default_value = "update")]
    pub action: String,

    /// Skip the normal `omp update` attempt and go straight to the clean reinstall.
    #[arg(long)]
    pub force: bool,
}

pub fn run(a: Args) -> Result<()> {
    if a.action != "update" {
        bail!("unknown action `{}` — only `8sync omp update` is supported", a.action);
    }
    ui::header("8sync omp update");

    // 1. Locate the live omp + record the starting version / bin type.
    let bin = which::which("omp")
        .map_err(|_| anyhow::anyhow!("omp not on PATH — run `8sync setup` first"))?;
    let current = omp_version().unwrap_or_else(|| "unknown".into());
    let symlink = std::fs::symlink_metadata(&bin)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    ui::info(&format!(
        "current: omp {current} at {} ({})",
        bin.display(),
        if symlink { "symlink — package-manager-owned" } else { "standalone file" }
    ));
    sweep_partials(bin.parent());

    // 2. Normal update (skipped with --force), time-boxed so a stalled
    //    download can't hang this command forever.
    if !a.force {
        ui::step("running `omp update`");
        match run_stream("omp", &["update"], UPDATE_TIMEOUT_SECS, 60) {
            Ok(r) => {
                for line in r.text.lines() {
                    ui::info(&format!("  {}", line.trim_end()));
                }
                if r.timed_out {
                    ui::warn(&format!(
                        "`omp update` stalled >{}s (its downloader has no timeout) — killing it and reinstalling",
                        UPDATE_TIMEOUT_SECS
                    ));
                } else if r.ok() && !is_blocked(&r.text) {
                    return report(&current);
                } else if is_blocked(&r.text) {
                    ui::warn("update blocked by a squatter on the bin path (EEXIST / tarball) — auto-repairing");
                } else {
                    ui::warn("`omp update` failed — attempting a clean reinstall");
                }
            }
            Err(e) => ui::warn(&format!("could not run `omp update` ({e}) — attempting a clean reinstall")),
        }
    }

    // 3. Repair: back up, clear the bin, reinstall via the official installer.
    repair(&bin)?;
    report(&current)
}

/// The three ways the installer reports "a real file is where my symlink goes".
fn is_blocked(text: &str) -> bool {
    text.contains("EEXIST")
        || text.contains("file already exists")
        || text.contains("Fail extracting tarball")
        || text.contains("install failed")
}

/// Result of a time-boxed child run.
struct Streamed {
    text: String,
    code: Option<i32>,
    timed_out: bool,
}

impl Streamed {
    fn ok(&self) -> bool {
        !self.timed_out && self.code == Some(0)
    }
}

/// Spawn `cmd args`, capture combined stdout+stderr, and hard-kill the child
/// once `timeout_secs` elapse. Prints a heartbeat every `beat_secs` so a slow
/// (but healthy) download is distinguishable from a dead one. Readers run on
//  their own threads, so a kill unblocks them via the closed pipe.
fn run_stream(cmd: &str, args: &[&str], timeout_secs: u64, beat_secs: u64) -> std::io::Result<Streamed> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut out = child.stdout.take().expect("piped stdout");
    let mut err = child.stderr.take().expect("piped stderr");
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });

    let started = Instant::now();
    let deadline = started + Duration::from_secs(timeout_secs);
    let mut next_beat = started + Duration::from_secs(beat_secs);
    let mut timed_out = false;
    let code = loop {
        if let Some(st) = child.try_wait()? {
            break st.code();
        }
        if Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        if Instant::now() >= next_beat {
            ui::info(&format!(
                "  … still running ({}s elapsed) — the updater downloads ~185 MB with no progress line",
                started.elapsed().as_secs()
            ));
            next_beat += Duration::from_secs(beat_secs);
        }
        std::thread::sleep(Duration::from_millis(250));
    };
    let text = format!("{}{}", t_out.join().unwrap_or_default(), t_err.join().unwrap_or_default());
    Ok(Streamed { text, code, timed_out })
}

/// Delete leftover `omp.*.new` partial downloads (one per interrupted
/// `omp update`, each up to ~185 MB) sitting next to the live binary.
fn sweep_partials(dir: Option<&Path>) {
    let Some(dir) = dir else { return };
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut freed: u64 = 0;
    let mut n = 0u32;
    for entry in rd.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("omp.") && name.ends_with(".new") {
            if let Ok(md) = entry.metadata() {
                freed += md.len();
            }
            if std::fs::remove_file(entry.path()).is_ok() {
                n += 1;
            }
        }
    }
    if n > 0 {
        ui::step(&format!(
            "swept {n} interrupted-update partial(s) in {} ({:.0} MB freed)",
            dir.display(),
            freed as f64 / 1e6
        ));
    }
}

/// Clear the bin path and reinstall omp via the official installer so the
/// standalone lands cleanly at `~/.local/bin/omp`. `8sync` (this process) is
/// not omp, so removing omp's file is safe; the backup covers a failed
/// download mid-`curl -o`.
fn repair(bin: &Path) -> Result<()> {
    // Insurance: keep a copy so a failed reinstall can't brick omp.
    let backed_up = std::fs::copy(bin, BACKUP).is_ok();
    if backed_up {
        ui::step(&format!("backed up {} → {BACKUP}", bin.display()));
    }
    // Clear the squatter (real file) or stale symlink so nothing half-written
    // survives a failed installer run.
    if let Err(e) = std::fs::remove_file(bin) {
        if e.kind() != std::io::ErrorKind::NotFound {
            bail!("could not clear {} ({e})", bin.display());
        }
    }
    sweep_partials(bin.parent());
    ui::step("reinstalling via the official installer (omp.sh/install --binary)");

    let r = run_stream("sh", &["-c", INSTALLER], UPDATE_TIMEOUT_SECS, 60);
    let ok = match &r {
        Ok(o) => {
            for line in o.text.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev() {
                ui::info(&format!("  {}", line.trim_end()));
            }
            o.ok()
        }
        Err(e) => {
            ui::warn(&format!("installer failed to launch ({e})"));
            false
        }
    };

    if !ok || omp_version().is_none() {
        // Restore the backup so the user is not left without omp.
        if backed_up && !bin.exists() {
            let _ = std::fs::copy(BACKUP, bin);
            ui::warn(&format!("reinstall failed — restored the previous binary at {}", bin.display()));
        }
        bail!("official installer reinstall of omp failed — see the installer output above");
    }
    let _ = std::fs::remove_file(BACKUP);
    Ok(())
}

/// Re-resolve omp and report `CURRENT → NEW` + how the bin now sits on disk.
fn report(current: &str) -> Result<()> {
    // A removed PATH-first bin can shift `which omp`; re-resolve fresh.
    let new = omp_version().unwrap_or_else(|| "unknown".into());
    let bin: Option<PathBuf> = which::which("omp").ok();
    if let Some(bin) = &bin {
        let symlink = std::fs::symlink_metadata(bin)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if new == current {
            ui::ok(&format!("omp already current: {new} ({})", bin.display()));
        } else {
            ui::ok(&format!("omp {current} → {new} ({})", bin.display()));
        }
        if symlink {
            ui::info("bin is a symlink — package-manager-managed");
        } else {
            ui::info("bin is a standalone file (omp.sh layout) — matches the self-updater");
        }
    } else {
        ui::warn(&format!("omp not on PATH after update (was {current}) — check your shell PATH"));
    }
    sweep_partials(bin.as_deref().and_then(|b| b.parent()));
    ui::info("takes effect on the next omp launch");
    Ok(())
}

/// `omp --version` → e.g. `17.0.7` (strips the `omp/` prefix).
fn omp_version() -> Option<String> {
    let out = Command::new("omp").arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let first = s.lines().next()?.trim();
    Some(first.trim_start_matches("omp/").trim_start_matches("omp ").trim().to_string())
}
