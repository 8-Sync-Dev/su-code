use anyhow::Result;
use crate::{pkg, ui, verbs::selfup};

pub fn run() -> Result<()> {
    ui::header("8sync up");
    // 1. Self-update 8sync binary first
    let _ = selfup::run_self_update(false);
    // 2. Forge AI CLI (curl re-installer if present)
    if which::which("forge").is_ok() {
        let _ = pkg::run_loud("sh", &["-c", "curl -fsSL https://forgecode.dev/cli | sh"]);
    }
    // Note: we DO NOT run `pacman -Syu` here. System updates are the user's choice
    // (CachyOS rolling — run `paru -Syu` yourself when ready).
    ui::ok("up complete (system pkgs untouched — run `paru -Syu` manually if needed)");
    Ok(())
}
