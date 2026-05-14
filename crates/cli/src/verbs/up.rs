use anyhow::Result;
use crate::{pkg, ui, verbs::selfup};

pub fn run() -> Result<()> {
    ui::header("8sync up");
    // 1. Self-update first (binary from GitHub)
    let _ = selfup::run_self_update(false);
    // 2. System packages
    let _ = pkg::run_loud("sudo", &["pacman", "-Syu", "--noconfirm"]);
    // 3. Paru AUR
    if which::which("paru").is_ok() {
        let _ = pkg::run_loud("paru", &["-Syu", "--noconfirm", "--aur"]);
    }
    // 4. Forge
    if which::which("forge").is_ok() {
        let _ = pkg::run_loud("sh", &["-c", "curl -fsSL https://forgecode.dev/cli | sh"]);
    }
    ui::ok("up complete");
    Ok(())
}
