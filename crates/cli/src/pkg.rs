// Idempotent package management (pacman + paru + cargo + curl-installer)
use anyhow::{anyhow, Result};
use semver::Version;
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

/// Install one or many pacman packages (idempotent)
pub fn pacman_ensure(pkgs: &[&str], update_outdated: bool) -> Result<()> {
    let mut to_install: Vec<&str> = Vec::new();
    let mut to_update: Vec<&str> = Vec::new();
    for &p in pkgs {
        match pacman_state(p) {
            InstallState::Missing => to_install.push(p),
            InstallState::Outdated => to_update.push(p),
            InstallState::UpToDate => ui::skip(p, "up to date"),
        }
    }

    if !to_install.is_empty() {
        ui::step(&format!("pacman install: {}", to_install.join(" ")));
        let status = Command::new("sudo")
            .arg("pacman")
            .arg("-S")
            .arg("--needed")
            .arg("--noconfirm")
            .args(&to_install)
            .status()?;
        if !status.success() {
            return Err(anyhow!("pacman install failed"));
        }
    }

    if !to_update.is_empty() && update_outdated {
        ui::step(&format!("pacman update: {}", to_update.join(" ")));
        let status = Command::new("sudo")
            .arg("pacman")
            .arg("-S")
            .arg("--noconfirm")
            .args(&to_update)
            .status()?;
        if !status.success() {
            return Err(anyhow!("pacman update failed"));
        }
    } else if !to_update.is_empty() {
        for p in &to_update {
            ui::skip(p, "newer available — run `8sync up` to update");
        }
    }

    Ok(())
}

/// Ensure paru is installed (idempotent). Returns false if user declines build.
pub fn ensure_paru() -> Result<bool> {
    if which::which("paru").is_ok() {
        ui::skip("paru", "already installed");
        return Ok(true);
    }

    ui::step("Building paru from AUR (needs base-devel + git)");
    pacman_ensure(&["base-devel", "git"], false)?;

    let tmp = std::env::temp_dir().join("paru-build");
    let _ = std::fs::remove_dir_all(&tmp);
    let status = Command::new("git")
        .args(["clone", "https://aur.archlinux.org/paru.git"])
        .arg(&tmp)
        .status()?;
    if !status.success() {
        return Err(anyhow!("git clone paru failed"));
    }

    let status = Command::new("makepkg")
        .arg("-si")
        .arg("--noconfirm")
        .current_dir(&tmp)
        .status()?;
    if !status.success() {
        return Err(anyhow!("makepkg paru failed"));
    }
    Ok(true)
}

/// Install AUR packages via paru (idempotent)
pub fn paru_ensure(pkgs: &[&str]) -> Result<()> {
    let mut missing: Vec<&str> = Vec::new();
    for &p in pkgs {
        if matches!(pacman_state(p), InstallState::Missing) {
            missing.push(p);
        } else {
            ui::skip(p, "already installed");
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    ui::step(&format!("paru install: {}", missing.join(" ")));
    let status = Command::new("paru")
        .arg("-S")
        .arg("--needed")
        .arg("--noconfirm")
        .args(&missing)
        .status()?;
    if !status.success() {
        return Err(anyhow!("paru install failed"));
    }
    Ok(())
}

/// Compare two semver-ish strings, returning true if `local >= want`
pub fn ver_at_least(local: &str, want: &str) -> bool {
    let l = Version::parse(strip_v(local)).ok();
    let w = Version::parse(strip_v(want)).ok();
    match (l, w) {
        (Some(a), Some(b)) => a >= b,
        _ => false,
    }
}

fn strip_v(s: &str) -> &str {
    let t = s.trim();
    t.strip_prefix('v').unwrap_or(t)
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

/// Run a shell command, suppressing output unless it fails
pub fn run_quiet(cmd: &str, args: &[&str]) -> Result<()> {
    let out = Command::new(cmd).args(args).output()?;
    if !out.status.success() {
        return Err(anyhow!(
            "{} {} failed: {}",
            cmd,
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}
