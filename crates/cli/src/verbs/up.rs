// `8sync up` — update the 8sync binary from the latest GitHub Release.
//
// Decoupled from omp on purpose: omp self-updates via `omp update` (or its
// own installer). Touching it here would silently change a runtime the user
// is actively in the middle of a chat with. System pkgs (pacman/AUR) are
// untouched too — user runs `paru -Syu` on their own schedule.

use anyhow::Result;
use crate::{ui, verbs::selfup};

pub fn run() -> Result<()> {
    ui::header("8sync up");
    let updated = selfup::run_self_update(true)?;
    if updated {
        ui::info("done — re-run any 8sync command to pick up the new binary");
    }
    ui::info("note: `8sync up` only updates 8sync. For omp run `omp update`; for system pkgs run `paru -Syu`.");
    Ok(())
}
