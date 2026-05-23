use anyhow::Result;
use crate::{env_detect, ui, verbs::{profile, sec}};

pub fn run() -> Result<()> {
    ui::header("8sync doctor");
    let env = env_detect::Env::detect()?;

    // OS / desktop stack
    check("OS", &env.os_id);
    if env_detect::is_hyde() {
        ui::ok("HyDE detected (Hyprland + wallbash theme engine)");
    }

    // AUR helper
    match env_detect::aur_helper() {
        Some(h) => ui::ok(&format!("AUR helper: {}", h)),
        None    => ui::info("AUR helper: none (paru or yay needed for AUR profiles: caelestia, hardware-lianli, warp, ...)"),
    }

    // Core harness
    check_cmd("git",     &["--version"]);
    check_cmd("omp",     &["--version"]);

    // gh is REQUIRED for `8sync ship`
    match env_detect::cmd_version("gh", &["--version"]) {
        Some(v) => ui::ok(&format!("gh: {}", v)),
        None    => ui::err("gh: MISSING — `8sync ship` needs github-cli (run `8sync setup`)"),
    }
    if let Some(out) = env_detect::cmd_version("gh", &["auth", "status"]) {
        ui::info(&format!("gh auth: {}", out));
    }

    // Configs present?
    for path in [
        env.xdg_config.join("8sync/global.toml"),
        env.xdg_config.join("8sync/skills.toml"),
        env.home.join(".omp/skills/00-force-load.md"),
    ] {
        if path.exists() {
            ui::ok(&format!("{}", path.display()));
        } else {
            ui::warn(&format!("missing: {}", path.display()));
        }
    }

    // Caelestia overlay marker
    let userprefs = env.home.join(".config/hypr/userprefs.conf");
    if let Ok(c) = std::fs::read_to_string(&userprefs) {
        if c.contains("CAELESTIA-SHELL-OVERRIDE") {
            ui::ok("caelestia-hyde overlay: installed (remove via `8sync setup --caelestia=rollback`)");
        }
    }

    // Security (warp + ufw) — compact one-liners
    sec::status_quiet();

    // Profiles applied
    let st = profile::load_state();
    if st.applied.is_empty() {
        ui::info("profiles: none applied (run `8sync setup`)");
    } else {
        ui::ok(&format!("profiles applied: {}", st.applied.join(", ")));
    }

    Ok(())
}

fn check(label: &str, value: &str) {
    ui::ok(&format!("{}: {}", label, value));
}

fn check_cmd(name: &str, args: &[&str]) {
    match env_detect::cmd_version(name, args) {
        Some(v) => ui::ok(&format!("{}: {}", name, v)),
        None => ui::warn(&format!("{}: missing", name)),
    }
}
