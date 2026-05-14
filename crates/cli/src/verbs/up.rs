use anyhow::Result;
use crate::{pkg, ui};

pub fn run() -> Result<()> {
    ui::header("8sync up");
    // System packages
    pkg::run_loud("sudo", &["pacman", "-Syu", "--noconfirm"])?;
    // Paru AUR
    if which::which("paru").is_ok() {
        pkg::run_loud("paru", &["-Syu", "--noconfirm", "--aur"])?;
    }
    // Forge
    if which::which("forge").is_ok() {
        let _ = pkg::run_loud("sh", &["-c", "curl -fsSL https://forgecode.dev/cli | sh"]);
    }
    // gh + global npm tools maintained by user — skip
    ui::ok("up complete");
    Ok(())
}
