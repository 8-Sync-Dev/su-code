use anyhow::Result;
use crate::{pkg, ui, verbs::selfup};

pub fn run() -> Result<()> {
    ui::header("8sync up");
    // 1. Self-update 8sync binary first
    let _ = selfup::run_self_update(false);
    // 2. omp CLI: prefer its own self-update if available, else re-run installer.
    if which::which("omp").is_ok() {
        let st = std::process::Command::new("omp").arg("update").status();
        if !matches!(st, Ok(s) if s.success()) {
            let _ = pkg::run_loud("sh", &["-c", "curl -fsSL https://omp.sh/install | sh"]);
        }
    }
    ui::ok("up complete (system pkgs untouched — run `paru -Syu` manually if needed)");
    Ok(())
}
