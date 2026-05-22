use anyhow::Result;
use clap::Args as ClapArgs;

use crate::{pkg, ui, verbs::selfup};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync up                    full update: 8sync + omp + system (pacman+AUR) + rustup + flatpak
          8sync up --self-only        only 8sync binary + omp CLI (no sudo, no system pkgs)
          8sync up --no-system        skip pacman / AUR (everything else still runs)
          8sync up --no-tools         skip rustup + flatpak (only harness + system)

        WHAT IT DOES (each step is best-effort; failures are warned not fatal)
          1. self-update 8sync binary  (from GitHub releases)
          2. omp CLI                   (omp update, fallback to installer)
          3. system packages           (paru -Syu / yay -Syu / sudo pacman -Syu)
          4. rustup toolchains         (rustup update, if rustup present)
          5. flatpak apps              (flatpak update -y, if flatpak present)

        SUDO
          Step 3 needs root. We invoke `sudo` only when no AUR helper is found;
          paru/yay handle privilege escalation themselves.
    "}
)]
pub struct Args {
    /// Only update 8sync binary and omp CLI (skip system + tools)
    #[arg(long)]
    pub self_only: bool,

    /// Skip system package update (pacman / AUR)
    #[arg(long)]
    pub no_system: bool,

    /// Skip rustup + flatpak updates
    #[arg(long)]
    pub no_tools: bool,
}

pub fn run(a: Args) -> Result<()> {
    ui::header("8sync up");

    // 1. self-update 8sync binary first so subsequent runs use newest logic.
    ui::step("8sync binary");
    let _ = selfup::run_self_update(false);

    // 2. omp CLI
    update_omp();

    if a.self_only {
        ui::ok("self-only mode — system + tools skipped");
        return Ok(());
    }

    // 3. system packages (pacman + AUR)
    if !a.no_system {
        update_system();
    } else {
        ui::skip("system packages", "--no-system");
    }

    // 4 & 5. dev tool managers
    if !a.no_tools {
        update_rustup();
        update_flatpak();
    } else {
        ui::skip("rustup + flatpak", "--no-tools");
    }

    ui::ok("up complete");
    Ok(())
}

fn update_omp() {
    ui::step("omp CLI");
    if which::which("omp").is_err() {
        ui::skip("omp", "not installed");
        return;
    }
    let st = std::process::Command::new("omp").arg("update").status();
    if matches!(st, Ok(s) if s.success()) {
        return;
    }
    ui::warn("`omp update` failed — falling back to installer");
    let _ = pkg::run_loud("sh", &["-c", "curl -fsSL https://omp.sh/install | sh"]);
}

fn update_system() {
    ui::step("System packages (pacman + AUR)");
    // Prefer an AUR helper that already wraps pacman; it handles privilege
    // escalation and updates AUR pkgs in the same pass.
    for helper in ["paru", "yay"] {
        if which::which(helper).is_ok() {
            if let Err(e) = pkg::run_loud(helper, &["-Syu"]) {
                ui::warn(&format!("{} -Syu failed: {}", helper, e));
            }
            return;
        }
    }
    if which::which("pacman").is_err() {
        ui::skip("pacman", "not installed");
        return;
    }
    if which::which("sudo").is_err() {
        ui::warn("no sudo found — skipping pacman update");
        return;
    }
    if let Err(e) = pkg::run_loud("sudo", &["pacman", "-Syu"]) {
        ui::warn(&format!("sudo pacman -Syu failed: {}", e));
    }
}

fn update_rustup() {
    ui::step("rustup");
    if which::which("rustup").is_err() {
        ui::skip("rustup", "not installed");
        return;
    }
    if let Err(e) = pkg::run_loud("rustup", &["update"]) {
        ui::warn(&format!("rustup update failed: {}", e));
    }
}

fn update_flatpak() {
    ui::step("flatpak");
    if which::which("flatpak").is_err() {
        ui::skip("flatpak", "not installed");
        return;
    }
    if let Err(e) = pkg::run_loud("flatpak", &["update", "-y"]) {
        ui::warn(&format!("flatpak update failed: {}", e));
    }
}
