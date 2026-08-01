// `8sync omp update` — update omp, auto-repairing a blocked install.
//
// omp self-updates via `omp update` (shells out to npm/bun `install -g
// @oh-my-pi/pi-coding-agent`). That breaks with `npm error EEXIST` (or bun
// `Fail extracting tarball`) when a **standalone binary** squats the bin path
// where the package manager wants its symlink — a real file at
// `~/.local/bin/omp` instead of a `node_modules` symlink. This verb runs the
// normal update, and on that failure auto-repairs: back up the current binary,
// clear the squatter, reinstall via npm (path now free → proper symlink), then
// verify. Unlike an in-omp `/command`, this runs from the shell — so it works
// even when omp itself is broken, which is exactly when you need it.
//
// `8sync up` stays decoupled (it only updates the 8sync binary); this is the
// omp-side counterpart.

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use std::path::Path;
use std::process::Command;

use crate::ui;

/// npm/bun package that provides the `omp` binary.
const PKG: &str = "@oh-my-pi/pi-coding-agent";
/// Runnable backup of the current omp binary, kept across the repair.
const BACKUP: &str = "/tmp/omp-selfheal.bak";

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync omp update            update omp to latest; auto-fix a blocked install
          8sync omp update --force    skip the normal try, go straight to clean reinstall
          8sync omp                   alias of `omp update`

        Fixes the recurring `omp update` failure `npm error EEXIST … ~/.local/bin/omp`
        (and the bun `Fail extracting tarball` variant): a real binary squatting the
        path where the package manager wants a symlink. Touches ONLY the omp install
        (its bin + the npm global) — never sudo, system pkgs, or git.
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
        if symlink { "symlink — npm-managed" } else { "real file — squats the symlink path" }
    ));

    // 2. Normal update (skipped with --force).
    if !a.force {
        ui::step("running `omp update`");
        match Command::new("omp").arg("update").output() {
            Ok(o) => {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                );
                for line in text.lines() {
                    ui::info(&format!("  {}", line.trim_end()));
                }
                if o.status.success() && !is_blocked(&text) {
                    return report(&current);
                }
                if is_blocked(&text) {
                    ui::warn("update blocked by a squatting binary (EEXIST / tarball) — auto-repairing");
                } else {
                    ui::warn("`omp update` failed — attempting a clean reinstall");
                }
            }
            Err(e) => ui::warn(&format!("could not run `omp update` ({e}) — attempting a clean reinstall")),
        }
    }

    // 3. Repair: back up, clear the squatter, reinstall via npm.
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

/// Clear the bin path and reinstall omp so the package manager writes a proper
/// symlink. `8sync` (this process) is not omp, so removing omp's file is safe.
fn repair(bin: &Path) -> Result<()> {
    if which::which("npm").is_err() {
        bail!(
            "npm not found — omp's global install needs it. Fix `~/.local/bin/{{npm,npx}}` \
             (pnpm shim, see su-code/KNOWLEDGE.md) or install node/npm, then re-run."
        );
    }

    // Insurance: keep a copy so a failed reinstall can't brick omp.
    let backed_up = std::fs::copy(bin, BACKUP).is_ok();
    if backed_up {
        ui::step(&format!("backed up {} → {BACKUP}", bin.display()));
    }
    // Clear the squatter (real file) or stale symlink so npm can write its link.
    if let Err(e) = std::fs::remove_file(bin) {
        if e.kind() != std::io::ErrorKind::NotFound {
            bail!("could not clear {} ({e})", bin.display());
        }
    }
    ui::step(&format!("reinstalling {PKG}@latest via npm"));

    let out = Command::new("npm")
        .args(["install", "-g", &format!("{PKG}@latest")])
        .output();
    let ok = match &out {
        Ok(o) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            for line in text.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev() {
                ui::info(&format!("  {}", line.trim_end()));
            }
            o.status.success()
        }
        Err(e) => {
            ui::warn(&format!("npm failed to launch ({e})"));
            false
        }
    };

    if !ok {
        // Restore the backup so the user is not left without omp.
        if backed_up && !bin.exists() {
            let _ = std::fs::copy(BACKUP, bin);
            ui::warn(&format!("reinstall failed — restored the previous binary at {}", bin.display()));
        }
        bail!("npm reinstall of {PKG} failed — see the npm output above");
    }
    let _ = std::fs::remove_file(BACKUP);
    Ok(())
}

/// Re-resolve omp and report `CURRENT → NEW` + whether the bin is now a symlink.
fn report(current: &str) -> Result<()> {
    // A removed PATH-first bin can shift `which omp`; re-resolve fresh.
    let new = omp_version().unwrap_or_else(|| "unknown".into());
    if let Ok(bin) = which::which("omp") {
        let symlink = std::fs::symlink_metadata(&bin)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if new == current {
            ui::ok(&format!("omp already current: {new} ({})", bin.display()));
        } else {
            ui::ok(&format!("omp {current} → {new} ({})", bin.display()));
        }
        if symlink {
            ui::info("bin is now a node_modules symlink — the EEXIST can't recur");
        } else {
            ui::warn("bin is still a standalone file — `8sync omp update` will be needed again next time");
        }
    } else {
        ui::warn(&format!("omp not on PATH after update (was {current}) — check your shell PATH"));
    }
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
