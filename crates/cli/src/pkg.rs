// Idempotent package management (pacman + paru + cargo + curl-installer)
use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallState {
    Missing,
    UpToDate,
    Outdated,
}

/// Check pacman state for a single package
pub fn pacman_state(pkg: &str) -> InstallState {
    let installed = Command::new("pacman")
        .args(["-Q", pkg])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !installed {
        return InstallState::Missing;
    }

    // Compare against repo version
    let local = run_capture(&["pacman", "-Q", pkg]).unwrap_or_default();
    let remote = run_capture(&["pacman", "-Si", pkg]).unwrap_or_default();
    let local_ver = local.split_whitespace().nth(1).unwrap_or("").to_string();
    let remote_ver = remote
        .lines()
        .find_map(|l| l.strip_prefix("Version").map(|s| s.trim_start_matches(" :").trim().to_string()))
        .unwrap_or_default();

    if local_ver.is_empty() || remote_ver.is_empty() {
        return InstallState::UpToDate; // be conservative
    }
    if local_ver == remote_ver {
        InstallState::UpToDate
    } else {
        InstallState::Outdated
    }
}


/// Transactional install: snapshot which pkgs are NEW, install with --needed,
/// on failure roll back any package that was successfully installed in this batch.
///
/// `noconfirm` controls whether we pass `--noconfirm` (auto-yes for unattended runs).
pub fn pacman_install_safe(pkgs: &[&str], noconfirm: bool) -> Result<()> {
    if pkgs.is_empty() { return Ok(()); }

    // 1. Snapshot pre-install state
    let new_pkgs: Vec<&str> = pkgs.iter().copied()
        .filter(|p| matches!(pacman_state(p), InstallState::Missing))
        .collect();
    let already: Vec<&str> = pkgs.iter().copied()
        .filter(|p| !matches!(pacman_state(p), InstallState::Missing))
        .collect();

    for p in &already {
        ui::skip(p, "already installed");
    }
    if new_pkgs.is_empty() {
        return Ok(());
    }

    ui::step(&format!("pacman install: {}", new_pkgs.join(" ")));
    let mut cmd = Command::new("sudo");
    cmd.arg("pacman").arg("-S").arg("--needed");
    if noconfirm { cmd.arg("--noconfirm"); }
    cmd.args(&new_pkgs);
    let status = cmd.status()?;

    if !status.success() {
        // Rollback any that DID get installed in this batch
        let installed_now: Vec<&str> = new_pkgs.iter().copied()
            .filter(|p| !matches!(pacman_state(p), InstallState::Missing))
            .collect();
        if !installed_now.is_empty() {
            ui::warn(&format!("install failed — rolling back: {}", installed_now.join(" ")));
            let mut roll = Command::new("sudo");
            roll.arg("pacman").arg("-Rns");
            if noconfirm { roll.arg("--noconfirm"); }
            roll.args(&installed_now);
            let _ = roll.status();
        }
        return Err(anyhow!("pacman install failed (rolled back)"));
    }
    Ok(())
}

/// Transactional AUR install via `helper` (paru/yay) with rollback on failure.
pub fn aur_install_safe(helper: &str, pkgs: &[&str], noconfirm: bool) -> Result<()> {
    if pkgs.is_empty() { return Ok(()); }

    let new_pkgs: Vec<&str> = pkgs.iter().copied()
        .filter(|p| matches!(pacman_state(p), InstallState::Missing))
        .collect();
    let already: Vec<&str> = pkgs.iter().copied()
        .filter(|p| !matches!(pacman_state(p), InstallState::Missing))
        .collect();

    for p in &already {
        ui::skip(p, "already installed");
    }
    if new_pkgs.is_empty() {
        return Ok(());
    }

    ui::step(&format!("{} install: {}", helper, new_pkgs.join(" ")));
    let mut cmd = Command::new(helper);
    cmd.arg("-S").arg("--needed");
    if noconfirm {
        cmd.arg("--noconfirm");
        // paru/yay still prompt for provider choice (e.g. "caelestia-shell vs
        // caelestia-shell-git") and for PKGBUILD review. Suppress both.
        // Flag set differs per helper: paru has `--skipreview`; yay uses the
        // `--answer*=None` family. Passing the wrong flag aborts the run.
        match helper {
            "paru" => { cmd.arg("--skipreview"); }
            "yay"  => {
                cmd.arg("--answerdiff=None")
                   .arg("--answeredit=None")
                   .arg("--answerclean=None");
            }
            _ => {}
        }
        cmd.arg("--mflags=--noconfirm");
        cmd.args(&new_pkgs);
        // Pipe a stream of newlines to stdin so the provider-choice prompt
        // accepts its default (1) without blocking. `yes ""` runs forever;
        // paru only reads what it needs.
        use std::process::Stdio;
        cmd.stdin(Stdio::piped());
        let mut child = cmd.spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            let _ = stdin.write_all(b"\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n");
        }
        let status = child.wait()?;
        return aur_install_finish(helper, &new_pkgs, status, noconfirm);
    }
    cmd.args(&new_pkgs);
    let status = cmd.status()?;
    aur_install_finish(helper, &new_pkgs, status, noconfirm)
}

fn aur_install_finish(helper: &str, new_pkgs: &[&str], status: std::process::ExitStatus, noconfirm: bool) -> Result<()> {

    if !status.success() {
        let installed_now: Vec<&str> = new_pkgs.iter().copied()
            .filter(|p| !matches!(pacman_state(p), InstallState::Missing))
            .collect();
        if !installed_now.is_empty() {
            ui::warn(&format!("install failed — rolling back: {}", installed_now.join(" ")));
            let mut roll = Command::new("sudo");
            roll.arg("pacman").arg("-Rns");
            if noconfirm { roll.arg("--noconfirm"); }
            roll.args(&installed_now);
            let _ = roll.status();
        }
        return Err(anyhow!("{} install failed (rolled back)", helper));
    }
    Ok(())
}


fn run_capture(cmd: &[&str]) -> Option<String> {
    let out = Command::new(cmd[0]).args(&cmd[1..]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Run a shell command, printing it first
pub fn run_loud(cmd: &str, args: &[&str]) -> Result<()> {
    ui::info(&format!("$ {} {}", cmd, args.join(" ")));
    let status = Command::new(cmd).args(args).status()?;
    if !status.success() {
        return Err(anyhow!("command failed: {}", cmd));
    }
    Ok(())
}

/// Ensure `yay` is installed (idempotent). Bootstraps from AUR via makepkg if
/// missing. Distinct from the general `aur_helper()` discovery in env_detect:
/// some profiles need yay *specifically*, even if paru is already present.
pub fn ensure_yay() -> Result<()> {
    if which::which("yay").is_ok() {
        ui::skip("yay", "present");
        return Ok(());
    }
    ui::step("yay (AUR helper required for this profile)");
    pacman_install_safe(&["git", "base-devel"], true)?;
    let cmd = "cd /tmp && rm -rf yay-bootstrap && \
        git clone https://aur.archlinux.org/yay-bin.git yay-bootstrap && \
        cd yay-bootstrap && makepkg -si --noconfirm && \
        cd .. && rm -rf yay-bootstrap";
    run_loud("sh", &["-c", cmd])?;
    if which::which("yay").is_err() {
        return Err(anyhow!("yay bootstrap finished but `yay` is not on PATH"));
    }
    ui::ok("yay installed");
    Ok(())
}
