// Self-update: pull latest source from GitHub, rebuild, install to ~/.local/bin
// Also exposes a rate-limited auto-check used from main() before dispatch.

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::ui;

const REPO_URL: &str = "https://github.com/8-Sync-Dev/su-code.git";
const REPO_REF: &str = "main";
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 3600); // 6h

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("8sync")
}

fn last_check_file() -> PathBuf { cache_dir().join("last_check") }
fn remote_commit_file() -> PathBuf { cache_dir().join("remote_commit") }
fn src_dir() -> PathBuf { cache_dir().join("src") }

fn build_commit() -> &'static str { env!("GIT_COMMIT_HASH") }

fn should_check() -> bool {
    let p = last_check_file();
    if !p.exists() { return true; }
    let modified = match p.metadata().and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return true,
    };
    SystemTime::now().duration_since(modified).map(|d| d > CHECK_INTERVAL).unwrap_or(true)
}

fn touch_check() {
    let _ = std::fs::create_dir_all(cache_dir());
    let _ = std::fs::write(last_check_file(), "");
}

fn fetch_remote_commit() -> Option<String> {
    // Short timeout so it never blocks. `timeout 3 git ls-remote ...`
    let out = Command::new("timeout")
        .args(["3", "git", "ls-remote", REPO_URL, REPO_REF])
        .output()
        .ok()?;
    if !out.status.success() { return None; }
    let s = String::from_utf8_lossy(&out.stdout);
    s.split_whitespace().next().map(|s| s.to_string())
}

/// Cheap auto-check called from main(). Prints a 1-line notice if newer
/// upstream commit exists. Never blocks for long (3s timeout via `timeout`).
/// Silently fails for offline/no-git users.
pub fn auto_check_notice() {
    if std::env::var("SUSYNC_NO_AUTO_CHECK").is_ok() { return; }
    if !should_check() { return; }
    touch_check();
    let local = build_commit();
    if local.is_empty() { return; }
    let Some(remote) = fetch_remote_commit() else { return; };
    if remote.starts_with(local) || local.starts_with(&remote) { return; }
    let _ = std::fs::write(remote_commit_file(), &remote);
    eprintln!(
        "\x1b[33m! 8sync update available: {} → {} — run `8sync up` to install\x1b[0m",
        &local[..local.len().min(7)],
        &remote[..remote.len().min(7)]
    );
}

/// Force self-update: git clone/pull, cargo build --release, install binary.
pub fn run_self_update(force: bool) -> Result<bool> {
    ui::step("Self-update (8sync binary from GitHub)");
    let local = build_commit();
    let remote = fetch_remote_commit().unwrap_or_default();
    if !force && !local.is_empty() && !remote.is_empty()
        && (remote.starts_with(local) || local.starts_with(&remote))
    {
        ui::skip("8sync", &format!("up to date ({})", &local[..7.min(local.len())]));
        return Ok(false);
    }

    let dir = src_dir();
    std::fs::create_dir_all(dir.parent().unwrap_or(&dir))?;
    if dir.join(".git").exists() {
        Command::new("git")
            .args(["-C", dir.to_str().unwrap(), "fetch", "--depth=1", "origin", REPO_REF])
            .status()?;
        Command::new("git")
            .args(["-C", dir.to_str().unwrap(), "reset", "--hard", &format!("origin/{}", REPO_REF)])
            .status()?;
    } else {
        let _ = std::fs::remove_dir_all(&dir);
        Command::new("git")
            .args(["clone", "--depth=1", "--branch", REPO_REF, REPO_URL, dir.to_str().unwrap()])
            .status()?;
    }

    // Build
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&dir)
        .status()?;
    if !status.success() {
        return Err(anyhow!("cargo build failed in {}", dir.display()));
    }

    // Install
    let bin_src = dir.join("target/release/8sync");
    let bin_dst = dirs::home_dir()
        .ok_or_else(|| anyhow!("no home dir"))?
        .join(".local/bin/8sync");
    std::fs::create_dir_all(bin_dst.parent().unwrap())?;
    std::fs::copy(&bin_src, &bin_dst)?;
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&bin_dst, std::fs::Permissions::from_mode(0o755));
    if !remote.is_empty() {
        let _ = std::fs::write(remote_commit_file(), &remote);
    }
    ui::ok(&format!("installed → {}", bin_dst.display()));
    Ok(true)
}
