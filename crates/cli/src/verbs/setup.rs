use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use std::path::PathBuf;
use std::process::Command;

use crate::{assets, env_detect, pkg, ui, verbs::profile};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES — strict mode (errors bail, no log)
          8sync setup                          interactive y/N per personal profile
          8sync setup --no-profile             harness only (skip profile stage)
          8sync setup --dry-run                print the full plan, change nothing

        EXAMPLES — unattended mode (auto-yes, preflight, log file, skip-on-error)
          8sync setup --caelestia              ONLY Caelestia: Hyprland + Quickshell + SDDM (auto fresh|coexist)
          8sync setup --full                   ALL profiles: caelestia + vietnamese + hardware-* + displaylink + apps-personal + warp
          8sync setup --profile warp           just one named profile
          8sync setup --caelestia=rollback     restore ~/.config/hypr from backup (optional --purge to remove pkgs)

          (`--yall` and `--yes` / `-y` are kept as aliases of `--full` for muscle memory)

        EXAMPLES — profile management
          8sync setup profile list             every available profile (✓ = applied)
          8sync setup profile show alexdev     resolved packages + services + post-install
          8sync setup profile apply warp       (re-)apply one profile idempotently

        STAGE A — HARNESS (always run, idempotent)
          · pacman -S --needed github-cli       (gh — required by `8sync ship`)
          · omp AI CLI                          (curl installer from omp.sh, only if missing)
          · paru                                (AUR helper, only if missing)
          · codegraph                           (semantic code index for AI)
          · write configs + skills under ~/.config/8sync/ and ~/.omp/skills/

        STAGE B — PROFILES (opt-in personal customization)
          vietnamese        fcitx5 + Unikey input method
          hardware-cooling  CoolerControl + OpenRGB + liquidctl
          hardware-lianli   lianli-linux-git from AUR
          displaylink       evdi-dkms (DisplayLink USB monitor driver)
          apps-personal     Bitwarden
          warp              Cloudflare WARP + DoH + MASQUE  (toggle daily via `8sync sec`)
          nvidia            auto-detect driver (Blackwell→Turing: open-dkms; Maxwell/Pascal: dkms)
          caelestia         Hyprland + Quickshell + caelestia-shell + SDDM  (extends `nvidia`)
          alexdev           BUNDLE — caelestia + all personal profiles

        UNATTENDED MODE BEHAVIOUR (auto-on when --caelestia / --full / --profile is given)
          1. Preflight: print OS, display manager, registered sessions, tool presence, GPU
          2. Log every step to ~/.cache/8sync/setup-<unix_ts>.log (UI level, timestamped)
          3. On any step failure: log + track + CONTINUE (no bail) — re-run to retry
          4. Auto-yes (--noconfirm) for every pacman / AUR install

        CAELESTIA MODES (auto-detected from system state)
          fresh    no display manager + no ~/.config/hypr → installs hyprland+sddm+caelestia,
                   enables sddm.service. Reboot → SDDM → Hyprland session → Caelestia.
          coexist  existing DE (Plasma/GNOME/HyDE/etc.) detected → installs Caelestia as a
                   parallel Hyprland session. If ~/.config/hypr exists (HyDE/dotfiles),
                   it's backed up to ~/.config/hypr.bak.caelestia.<ts>/ before Caelestia
                   takes over. Current DE session stays untouched in SDDM.

        SAFETY
          · Every install is transactional: if pacman/AUR fails halfway, packages installed in
            that batch are rolled back automatically (pacman -Rns).
          · Re-running setup is idempotent — already-installed packages are skipped.
          · `--dry-run` is always safe to inspect what would change.
    "}
)]
pub struct Args {
    /// Sub-command: `profile [list|show|apply <name>]`
    pub action: Option<String>,
    /// Arguments for the sub-command.
    pub rest: Vec<String>,

    /// Install EVERYTHING unattended (alexdev bundle = caelestia + all personal profiles).
    /// Equivalent to applying the `alexdev` bundle. Aliases: `--yall`, `--yes`, `-y`.
    /// Implies preflight + log + skip-on-error.
    #[arg(long = "full", alias = "yall", alias = "yes", short = 'y')]
    pub full: bool,

    /// Skip Stage B entirely (harness only — no profile prompts).
    #[arg(long)]
    pub no_profile: bool,

    /// Apply a specific profile non-interactively (use after Stage A).
    #[arg(long)]
    pub profile: Option<String>,

    /// Install Caelestia (auto|rollback). Plain `--caelestia` = auto-detect fresh vs coexist.
    #[arg(
        long,
        value_name = "MODE",
        num_args = 0..=1,
        default_missing_value = "auto",
    )]
    pub caelestia: Option<String>,

    /// With --caelestia=rollback: also `pacman -Rns` the Caelestia packages
    /// (caelestia-shell, quickshell, aubio). Off by default — packages stay.
    #[arg(long)]
    pub purge: bool,

    /// Auto-reboot after install completes (10s countdown — Ctrl-C cancels).
    /// Needed when a new kernel module landed (NVIDIA driver upgrade, etc.).
    /// Otherwise a logout is enough to reach the new Hyprland session.
    #[arg(long)]
    pub reboot: bool,

    /// Print the plan without making any changes.
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(a: Args) -> Result<()> {
    // Sub-command: `8sync setup profile <action>`
    if a.action.as_deref() == Some("profile") {
        return profile_sub(a.rest, a.full, a.dry_run);
    }

    // Special: `--caelestia=rollback` — restore backup, optionally purge pkgs.
    if a.caelestia.as_deref() == Some("rollback") {
        return rollback_caelestia(a.purge, a.dry_run);
    }

    ui::header("8sync setup");
    let env = env_detect::Env::detect()?;
    if !env.is_cachyos_or_arch() {
        ui::warn(&format!(
            "OS `{}` is not CachyOS/Arch — some steps may fail.",
            env.os_id
        ));
    }

    // ── YOLO mode setup: auto-on for any unattended path ─────────
    // Triggers when user requests an unambiguous install path:
    //   --full          → alexdev bundle
    //   --caelestia     → caelestia profile only
    //   --profile <n>   → just one profile
    // Strict mode (default `8sync setup` with no flags) keeps existing
    // behaviour: interactive prompts, errors bail, no log file.
    let yolo = a.full || a.caelestia.is_some() || a.profile.is_some();
    let log_path = if yolo && !a.dry_run {
        init_yolo_log().ok()
    } else {
        None
    };
    if yolo {
        preflight(&env);
    }
    let mut failures: Vec<String> = Vec::new();

    // ── Stage A: Harness (always run) ────────────────────────────
    ui::step("Stage A — coding harness");
    if a.dry_run {
        ui::info("would install: github-cli");
        ui::info("would install omp (curl) if missing");
        ui::info("would install paru (AUR helper) if missing");
        ui::info("would install codegraph (curl) if missing");
        ui::info("would write: configs + skills");
        ui::info("would register codegraph as a global+local skill");
    } else {
        try_step("github-cli", yolo, &mut failures, || {
            pkg::pacman_install_safe(&["github-cli"], true)
        })?;
        try_step("omp",        yolo, &mut failures, install_omp)?;
        try_step("paru",       yolo, &mut failures, install_aur_helper)?;
        try_step("codegraph",  yolo, &mut failures, install_codegraph)?;
        try_step("configs",    yolo, &mut failures, || install_configs(&env))?;
        try_step("skills",     yolo, &mut failures, || install_skills(&env))?;
        try_step("codegraph-skill", yolo, &mut failures, || register_codegraph_skill(&env))?;
    }

    // ── Caelestia shortcut ───────────────────────────────────────
    if let Some(mode) = a.caelestia.as_deref() {
        let resolved_mode = match mode {
            "auto" => detect_caelestia_mode(),
            "fresh" => CaelestiaMode::Fresh,
            "coexist" => CaelestiaMode::Coexist,
            other => bail!("--caelestia accepts: auto|fresh|coexist|rollback (got `{}`)", other),
        };
        let r = apply_caelestia(resolved_mode, a.dry_run);
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return r;
    }

    // ── Stage B: Profiles (optional) ─────────────────────────────
    if a.no_profile {
        ui::info("--no-profile → skipping personal profiles");
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }

    let all = profile::load_all()?;

    // explicit --profile <name>
    if let Some(name) = a.profile.as_ref() {
        ui::step(&format!("Stage B — applying profile `{}`", name));
        try_step(&format!("profile:{}", name), yolo, &mut failures, || {
            let resolved = profile::resolve(name, &all)?;
            profile::apply(&resolved, true, a.dry_run)?;
            if !a.dry_run {
                profile::mark_applied(name)?;
            }
            Ok(())
        })?;
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }

    // --full: apply alexdev bundle (caelestia + all personal profiles)
    if a.full {
        let bundle = if all.contains_key("alexdev") { "alexdev" } else { "" };
        if !bundle.is_empty() {
            ui::step(&format!("Stage B — --full: applying `{}` bundle", bundle));
            try_step(&format!("profile:{}", bundle), yolo, &mut failures, || {
                let resolved = profile::resolve(bundle, &all)?;
                profile::apply(&resolved, true, a.dry_run)?;
                if !a.dry_run {
                    profile::mark_applied(bundle)?;
                }
                Ok(())
            })?;
        } else {
            ui::warn("no `alexdev` bundle — applying every non-bundle profile individually");
            let mut names: Vec<&String> = all.keys().collect();
            names.sort();
            for n in &names {
                let p = match all.get(*n) { Some(p) => p, None => continue };
                if !p.extends.is_empty() { continue; } // skip bundles
                try_step(&format!("profile:{}", n), yolo, &mut failures, || {
                    let resolved = profile::resolve(n, &all)?;
                    profile::apply(&resolved, true, a.dry_run)?;
                    if !a.dry_run { profile::mark_applied(n)?; }
                    Ok(())
                })?;
            }
        }
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }

    // Interactive y/N per profile (skip bundle profiles)
    if !env_detect::has_tty() {
        ui::info("no TTY — skipping interactive profile prompt (use --full / --caelestia / --profile)");
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }


    ui::step("Stage B — personal profiles (y/N each)");
    let mut names: Vec<&String> = all.keys().collect();
    names.sort();
    for name in &names {
        let p = match all.get(*name) {
            Some(p) => p,
            None => continue,
        };
        if !p.extends.is_empty() {
            continue;
        }
        let desc = if p.description.is_empty() {
            name.as_str()
        } else {
            p.description.as_str()
        };
        let q = format!("Apply `{}` — {}", name, desc);
        if ui::prompt_yes_no(&q, false) {
            let resolved = profile::resolve(name, &all)?;
            if let Err(e) = profile::apply(&resolved, false, a.dry_run) {
                ui::err(&format!("profile {} failed: {}", name, e));
            } else if !a.dry_run {
                let _ = profile::mark_applied(name);
            }
        }
    }

    finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
    Ok(())
}

fn finish_msg() {
    ui::header("Done — next steps");
    println!("  · 8sync doctor               — verify");
    println!("  · cd <project> && 8sync .    — seed agents/ + start omp --continue");
}

/// Print final summary: log path (if any) + list of failures (if any).
/// Always prints `finish_msg` next-steps at the end.
/// If `reboot=true` and no failures, triggers a 10s-cancellable reboot.
fn finish_summary(failures: &[String], log_path: Option<&PathBuf>, reboot: bool, dry_run: bool) {
    if let Some(p) = log_path {
        ui::info(&format!("full log: {}", p.display()));
    }
    if failures.is_empty() {
        ui::ok("all steps succeeded (no failures recorded)");
    } else {
        ui::warn(&format!(
            "{} step(s) failed but were skipped (unattended mode): {}",
            failures.len(),
            failures.join(", ")
        ));
        ui::info("re-run the same command to retry — every step is idempotent");
    }
    ui::close_log_file();
    finish_msg();

    if reboot && !dry_run {
        if !failures.is_empty() {
            ui::warn("--reboot requested but some steps failed — aborting reboot. Fix or re-run, then reboot manually.");
            return;
        }
        do_reboot_with_countdown(10);
    }
}

/// Print a `secs`-second countdown then `systemctl reboot`. Ctrl-C cancels.
fn do_reboot_with_countdown(secs: u32) {
    use std::io::Write;
    println!();
    ui::warn(&format!("rebooting in {}s — press Ctrl-C to cancel", secs));
    for i in (1..=secs).rev() {
        print!("\r  ⏱  {}s remaining... ", i);
        std::io::stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("\r  ⏱  rebooting now              ");
    let _ = Command::new("systemctl").arg("reboot").status();
}

// ─────────────────────────────────────────────────────────────────
// Stage A helpers
// ─────────────────────────────────────────────────────────────────

fn install_omp() -> Result<()> {
    ui::step("omp AI CLI");
    if which::which("omp").is_ok() {
        let v = env_detect::cmd_version("omp", &["--version"]).unwrap_or_default();
        ui::skip("omp", &format!("present ({})", v));
        return Ok(());
    }
    pkg::run_loud("sh", &["-c", "curl -fsSL https://omp.sh/install | sh"])?;
    Ok(())
}

fn install_aur_helper() -> Result<()> {
    ui::step("AUR helper (paru)");
    if let Some(h) = env_detect::aur_helper() {
        ui::skip(h, "present");
        return Ok(());
    }
    pkg::pacman_install_safe(&["git", "base-devel"], true)?;
    let cmd = "cd /tmp && rm -rf paru-bootstrap && \
        git clone https://aur.archlinux.org/paru.git paru-bootstrap && \
        cd paru-bootstrap && makepkg -si --noconfirm && \
        cd .. && rm -rf paru-bootstrap";
    pkg::run_loud("sh", &["-c", cmd])?;
    ui::ok("paru installed");
    Ok(())
}

fn install_codegraph() -> Result<()> {
    ui::step("codegraph (semantic code index for omp / claude / cursor)");
    if which::which("codegraph").is_ok() {
        let v = env_detect::cmd_version("codegraph", &["--version"]).unwrap_or_default();
        ui::skip("codegraph", &format!("present ({})", v));
        return Ok(());
    }
    pkg::run_loud(
        "sh",
        &[
            "-c",
            "curl -fsSL https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.sh | sh",
        ],
    )?;
    ensure_local_bin_on_path();
    Ok(())
}

/// Ensure `~/.local/bin` is on PATH in ~/.zshrc and ~/.bashrc (idempotent).
fn ensure_local_bin_on_path() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let local_bin = home.join(".local/bin");
    let marker = "# 8sync: ensure ~/.local/bin on PATH (for codegraph + 8sync)";
    let snippet = format!(
        "\n{marker}\ncase \":$PATH:\" in *\":{lb}:\"*) ;; *) export PATH=\"{lb}:$PATH\" ;; esac\n",
        lb = local_bin.display(),
    );
    for rc in [home.join(".zshrc"), home.join(".bashrc")] {
        if !rc.exists() {
            continue;
        }
        let existing = std::fs::read_to_string(&rc).unwrap_or_default();
        if existing.contains(marker) {
            continue;
        }
        if let Err(e) = std::fs::OpenOptions::new()
            .append(true)
            .open(&rc)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(snippet.as_bytes())
            })
        {
            ui::warn(&format!("could not patch {}: {}", rc.display(), e));
            continue;
        }
        ui::ok(&format!(
            "patched {} (added ~/.local/bin to PATH)",
            rc.display()
        ));
    }
}

fn register_codegraph_skill(env: &env_detect::Env) -> Result<()> {
    ui::step("Register codegraph skill (force-load)");
    let skills_toml = env.xdg_config.join("8sync/skills.toml");
    if let Err(e) = crate::verbs::skill::add_spec(env, &skills_toml, "gh:colbymchenry/codegraph") {
        ui::warn(&format!(
            "could not auto-register codegraph: {} (skill will still work but missing frontmatter)",
            e
        ));
    }
    Ok(())
}

fn install_configs(env: &env_detect::Env) -> Result<()> {
    ui::step("Configs (8sync/{global,skills}.toml)");
    let pairs = [
        ("configs/global.toml", env.xdg_config.join("8sync/global.toml")),
        ("configs/skills.toml", env.xdg_config.join("8sync/skills.toml")),
    ];
    for (asset, target) in &pairs {
        let changed = assets::install(asset, target, false)?;
        if changed {
            ui::ok(&format!("wrote {}", target.display()));
        } else {
            ui::skip(&target.display().to_string(), "unchanged");
        }
    }
    Ok(())
}

fn install_skills(env: &env_detect::Env) -> Result<()> {
    ui::step("Skills (~/.omp/skills/)");
    let skills_dir = env.home.join(".omp/skills");
    std::fs::create_dir_all(&skills_dir)?;
    let trio = [
        ("skills/karpathy/SKILL.md", "karpathy-guidelines/SKILL.md"),
        ("skills/image-routing/SKILL.md", "image-routing/SKILL.md"),
        ("skills/8sync-cli/SKILL.md", "8sync-cli/SKILL.md"),
    ];
    for (src, rel) in &trio {
        let target = skills_dir.join(rel);
        let changed = assets::install(src, &target, false)?;
        if changed {
            ui::ok(&format!("wrote {}", target.display()));
        } else {
            ui::skip(&target.display().to_string(), "unchanged");
        }
    }
    let master = skills_dir.join("00-force-load.md");
    assets::install("skills/00-force-load.md", &master, true)?;
    ui::ok(&format!("wrote {}", master.display()));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// `--caelestia` — fresh / coexist auto-detection + apply + rollback
// ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaelestiaMode {
    /// No DE/DM detected, no ~/.config/hypr → bring up the full stack.
    Fresh,
    /// Existing DE detected → add Caelestia as a parallel Hyprland session.
    /// Backs up ~/.config/hypr if present.
    Coexist,
}

fn detect_caelestia_mode() -> CaelestiaMode {
    // Coexist ONLY if a working desktop is already there. "Working" means a
    // display manager is enabled OR a non-Hyprland session entry is registered.
    // A lone ~/.config/hypr is NOT enough (user may have tinkered manually then
    // wiped) — forcing coexist on that case would skip sddm.service enable and
    // drop the user at a TTY after reboot.
    // `display-manager.service` is the canonical alias every DM symlinks to —
    // it catches plasmalogin (CachyOS KDE), ly, entrance, and anything else
    // we'd otherwise miss in the per-DM list.
    let has_dm = ["display-manager", "sddm", "plasmalogin", "gdm", "lightdm", "greetd"]
        .iter()
        .any(|d| systemctl_is_enabled(d));
    let has_other_sessions = has_non_hyprland_session();
    if has_dm || has_other_sessions {
        CaelestiaMode::Coexist
    } else {
        CaelestiaMode::Fresh
    }
}

fn systemctl_is_enabled(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["is-enabled", &format!("{}.service", unit)])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_non_hyprland_session() -> bool {
    for dir in [
        "/usr/share/wayland-sessions",
        "/usr/share/xsessions",
    ] {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            if name.ends_with(".desktop") && !name.contains("hyprland") {
                return true;
            }
        }
    }
    false
}

fn apply_caelestia(mode: CaelestiaMode, dry_run: bool) -> Result<()> {
    ui::header(&format!("8sync setup --caelestia ({})", match mode {
        CaelestiaMode::Fresh => "fresh — first DE on this machine",
        CaelestiaMode::Coexist => "coexist — adding parallel Hyprland session next to existing DE",
    }));

    if mode == CaelestiaMode::Coexist {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
        let hypr = home.join(".config/hypr");
        if hypr.exists() {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let backup = home.join(format!(".config/hypr.bak.caelestia.{}", ts));
            if dry_run {
                ui::info(&format!("would back up {} → {}", hypr.display(), backup.display()));
            } else {
                ui::step(&format!("backing up {} → {}", hypr.display(), backup.display()));
                std::fs::rename(&hypr, &backup)?;
                ui::ok("backup done");
            }
        }
    }

    // Apply the `caelestia` profile (resolves to nvidia + caelestia stack).
    let all = profile::load_all()?;
    let mut resolved = profile::resolve("caelestia", &all)?;

    // In coexist mode: do NOT touch the existing display manager — user already
    // has one (gdm/sddm/lightdm/greetd). Strip sddm out of system services so
    // we don't yank their working setup. Caelestia adds itself as a parallel
    // Hyprland session via /usr/share/wayland-sessions/hyprland.desktop, which
    // any DM will pick up on its own.
    if mode == CaelestiaMode::Coexist {
        resolved.services.system_enable.retain(|s| s != "sddm");
        ui::info("coexist: skipping sddm.service enable — your existing DM stays in charge");
        ui::info("       at next login, pick `Hyprland` (Caelestia) from the session dropdown");
    }

    profile::apply(&resolved, true, dry_run)?;
    if !dry_run {
        profile::mark_applied("caelestia")?;
    }

    if !dry_run {
        ui::header("Done — Caelestia installed");
        match mode {
            CaelestiaMode::Fresh => {
                println!("  · reboot now — SDDM will appear, pick `Hyprland`");
                println!("  · Caelestia auto-launches inside that session");
            }
            CaelestiaMode::Coexist => {
                println!("  · log out (or reboot) — your DM shows a `Hyprland` session");
                println!("  · pick it to use Caelestia; pick your old DE to stay on it");
                println!("  · to revert: `8sync setup --caelestia=rollback` (restores ~/.config/hypr)");
            }
        }
    }
    Ok(())
}

fn rollback_caelestia(purge: bool, dry_run: bool) -> Result<()> {
    ui::header("8sync setup --caelestia=rollback");
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let config_dir = home.join(".config");

    // Find the most recent ~/.config/hypr.bak.caelestia.* backup.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&config_dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("hypr.bak.caelestia.") {
                candidates.push(e.path());
            }
        }
    }
    candidates.sort();
    let latest = candidates.into_iter().last();

    let hypr = home.join(".config/hypr");
    match latest {
        Some(backup) => {
            if dry_run {
                ui::info(&format!("would: rm -rf {}", hypr.display()));
                ui::info(&format!("would: mv {} {}", backup.display(), hypr.display()));
            } else {
                if hypr.exists() {
                    // Move current hypr (Caelestia's) aside as a fresh backup.
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let stash = home.join(format!(".config/hypr.caelestia-stash.{}", ts));
                    std::fs::rename(&hypr, &stash)?;
                    ui::info(&format!("Caelestia config stashed → {}", stash.display()));
                }
                std::fs::rename(&backup, &hypr)?;
                ui::ok(&format!("restored {} from {}", hypr.display(), backup.file_name().unwrap().to_string_lossy()));
            }
        }
        None => {
            ui::info("no ~/.config/hypr.bak.caelestia.* backup found — nothing to restore");
            ui::info("  (fresh installs have no pre-Caelestia config to restore)");
        }
    }

    // Remove Caelestia's cloned dots repo + symlinks it created.
    let dots = home.join(".local/share/caelestia");
    if dots.exists() {
        if dry_run {
            ui::info(&format!("would: rm -rf {}", dots.display()));
        } else {
            std::fs::remove_dir_all(&dots).ok();
            ui::ok(&format!("removed {}", dots.display()));
        }
    }
    // Unlink (only) any Caelestia symlinks the upstream installer dropped.
    for d in ["foot", "fish", "fastfetch", "uwsm", "btop", "starship.toml"] {
        let p = home.join(".config").join(d);
        if p.is_symlink() {
            if dry_run {
                ui::info(&format!("would: unlink {}", p.display()));
            } else {
                let _ = std::fs::remove_file(&p);
            }
        }
    }

    // Optional: purge packages.
    if purge {
        let cmd = "sudo pacman -Rns --noconfirm caelestia-shell quickshell caelestia-meta aubio 2>/dev/null || true";
        if dry_run {
            ui::info(&format!("would run: {}", cmd));
        } else {
            ui::info(&format!("$ {}", cmd));
            let _ = Command::new("sh").arg("-c").arg(cmd).status();
        }
    } else if !dry_run {
        ui::info("packages NOT removed (rerun with --purge to also `pacman -Rns caelestia-shell quickshell aubio`)");
    }

    ui::ok("rollback complete — reboot or restart Hyprland to apply");
    Ok(())
}

// ─── `8sync setup profile <sub>` ────────────────────────────────

fn profile_sub(rest: Vec<String>, yes_to_all: bool, dry_run: bool) -> Result<()> {
    let action = rest.first().map(|s| s.as_str()).unwrap_or("list");
    let all = profile::load_all()?;
    let state = profile::load_state();

    match action {
        "list" => {
            ui::header("Profiles");
            let mut names: Vec<&String> = all.keys().collect();
            names.sort();
            for n in names {
                let p = &all[n];
                let marker = if state.applied.iter().any(|x| x == n) {
                    "✓"
                } else {
                    " "
                };
                let kind = if !p.extends.is_empty() { "(bundle)" } else { "" };
                println!("  {} {:20} {} {}", marker, n, kind, p.description);
            }
            Ok(())
        }
        "show" => {
            let name = rest
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("usage: 8sync setup profile show <name>"))?;
            let r = profile::resolve(name, &all)?;
            println!("name         = {}", r.name);
            println!("description  = {}", r.description);
            println!("needs AUR    = {}", r.requires.aur_helper);
            println!("pacman       = {:?}", r.packages.pacman);
            println!("aur          = {:?}", r.packages.aur);
            println!("sys services = {:?}", r.services.system_enable);
            println!("user services= {:?}", r.services.user_enable);
            println!("commands     = {:?}", r.post_install.commands);
            if !r.post_install.hint.is_empty() {
                println!("\nhints:\n{}", r.post_install.hint);
            }
            Ok(())
        }
        "apply" => {
            let name = rest
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("usage: 8sync setup profile apply <name>"))?;
            let resolved = profile::resolve(name, &all)?;
            profile::apply(&resolved, yes_to_all, dry_run)?;
            if !dry_run {
                profile::mark_applied(name)?;
            }
            Ok(())
        }
        other => {
            ui::warn(&format!(
                "unknown sub-action `{}` — try list | show | apply",
                other
            ));
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// YOLO mode helpers (auto-on for --full / --caelestia / --profile)
// ─────────────────────────────────────────────────────────────────

/// Open `~/.cache/8sync/setup-<unix_ts>.log` and wire `ui::*` to tee into it.
/// Idempotent across runs (timestamped filename).
fn init_yolo_log() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = home.join(format!(".cache/8sync/setup-{}.log", ts));
    let final_path = ui::set_log_file(path)?;
    ui::ok(&format!("logging to {}", final_path.display()));
    Ok(final_path)
}

/// Quick read-only probe of system state. Prints what's already installed so
/// the install plan below is predictable. No side effects.
fn preflight(env: &env_detect::Env) {
    ui::step("Preflight — detecting current system state");

    // OS + DM
    ui::info(&format!("OS: {} ({})", env.os_id, if env.is_cachyos_or_arch() { "supported" } else { "best-effort" }));
    let dm = ["display-manager", "sddm", "plasmalogin", "gdm", "lightdm", "greetd"]
        .iter()
        .find(|d| systemctl_is_enabled(d));
    match dm {
        Some(d) => ui::info(&format!("display manager: {}.service enabled", d)),
        None => ui::info("display manager: none enabled (fresh install path)"),
    }

    // Wayland / X sessions
    let sessions = enumerate_sessions();
    if sessions.is_empty() {
        ui::info("desktop sessions: none registered");
    } else {
        ui::info(&format!("desktop sessions: {}", sessions.join(", ")));
    }

    // Core tools
    for (label, bin) in [
        ("omp", "omp"),
        ("paru", "paru"),
        ("yay", "yay"),
        ("codegraph", "codegraph"),
        ("gh", "gh"),
        ("hyprland", "Hyprland"),
        ("quickshell", "qs"),
        ("caelestia-shell", "caelestia-shell"),
    ] {
        let present = which::which(bin).is_ok();
        if present {
            let v = env_detect::cmd_version(bin, &["--version"]).unwrap_or_default();
            ui::skip(label, if v.is_empty() { "present" } else { &v });
        } else {
            ui::info(&format!("{}: missing — will be installed", label));
        }
    }

    // GPU
    if let Ok(out) = std::process::Command::new("sh")
        .arg("-c")
        .arg("lspci -nn 2>/dev/null | grep -iE 'vga|3d' | head -3")
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            for line in s.lines() {
                ui::info(&format!("gpu: {}", line.trim()));
            }
        }
    }
}

fn enumerate_sessions() -> Vec<String> {
    let mut out = Vec::new();
    for dir in ["/usr/share/wayland-sessions", "/usr/share/xsessions"] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if let Some(stripped) = n.strip_suffix(".desktop") {
                        out.push(stripped.to_string());
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// In YOLO mode: log the error and continue. In strict mode: propagate.
/// `failures` tracks step labels that errored, surfaced in the summary.
fn try_step<F>(label: &str, yolo: bool, failures: &mut Vec<String>, f: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    match f() {
        Ok(()) => Ok(()),
        Err(e) if yolo => {
            ui::err(&format!("[{}] failed: {} — continuing (unattended mode)", label, e));
            failures.push(label.to_string());
            Ok(())
        }
        Err(e) => Err(e),
    }
}
