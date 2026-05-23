// Self-update: pull the prebuilt binary from the latest GitHub Release.
// Also exposes a rate-limited auto-check used from main() before dispatch.

use anyhow::{anyhow, bail, Result};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::ui;

const REPO_OWNER: &str = "8-Sync-Dev";
const REPO_NAME: &str = "su-code";
const ASSET_SUFFIX: &str = "-linux-x86_64"; // architecture suffix for the release asset
const ASSET_PREFIX: &str = "8sync-";
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 3600); // 6h

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("8sync")
}

fn last_check_file() -> PathBuf { cache_dir().join("last_check") }
fn last_seen_tag_file() -> PathBuf { cache_dir().join("last_seen_tag") }

fn build_version() -> &'static str { env!("CARGO_PKG_VERSION") }

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

/// Strip leading `v` from a release tag so it can be compared to CARGO_PKG_VERSION.
fn strip_v(s: &str) -> &str { s.strip_prefix('v').unwrap_or(s) }

/// Query GitHub for the latest release. Returns `(tag_name, asset_browser_download_url)`.
/// Short timeout so it never blocks. Silent on offline / no-curl.
fn fetch_latest_release() -> Option<(String, String)> {
    if which::which("curl").is_err() { return None; }
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time", "5",
            "-H", "Accept: application/vnd.github+json",
            "-H", "User-Agent: 8sync-selfup",
            &url,
        ])
        .output()
        .ok()?;
    if !out.status.success() { return None; }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let tag = v.get("tag_name")?.as_str()?.to_string();
    let assets = v.get("assets")?.as_array()?;
    let want_name = format!("{}{}{}", ASSET_PREFIX, tag, ASSET_SUFFIX);
    for a in assets {
        let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if name == want_name {
            let dl = a.get("browser_download_url").and_then(|u| u.as_str())?;
            return Some((tag, dl.to_string()));
        }
    }
    None
}

/// Cheap auto-check called from main(). Prints a 1-line notice if a newer
/// release exists. Never blocks for long (5s timeout). Silent on offline.
pub fn auto_check_notice() {
    if std::env::var("SUSYNC_NO_AUTO_CHECK").is_ok() { return; }
    if !should_check() { return; }
    touch_check();
    let local = build_version();
    let Some((tag, _url)) = fetch_latest_release() else { return; };
    let remote = strip_v(&tag);
    if remote == local { return; }
    let _ = std::fs::write(last_seen_tag_file(), &tag);
    eprintln!(
        "\x1b[33m! 8sync update available: v{} → {} — run `8sync up` to install\x1b[0m",
        local, tag
    );
}

/// Force self-update: download the latest release asset and install it.
/// Returns Ok(true) when a new binary was written, Ok(false) when already up-to-date.
pub fn run_self_update(force: bool) -> Result<bool> {
    ui::step("Self-update — GitHub Releases");
    let local = build_version();

    let (tag, asset_url) = fetch_latest_release()
        .ok_or_else(|| anyhow!("could not query latest release from github.com/{}/{}", REPO_OWNER, REPO_NAME))?;
    let remote = strip_v(&tag);
    if !force && remote == local {
        ui::skip("8sync", &format!("up to date (v{})", local));
        return Ok(false);
    }

    ui::info(&format!("local v{} → {} ({})", local, tag, asset_url));
    let bin_dst = dirs::home_dir()
        .ok_or_else(|| anyhow!("no home dir"))?
        .join(".local/bin/8sync");
    std::fs::create_dir_all(bin_dst.parent().unwrap())?;

    // Atomic-ish replace: download to a sibling temp file then rename.
    let tmp = bin_dst.with_extension(format!("new.{}", std::process::id()));
    let status = Command::new("curl")
        .args(["-fsSL", "--max-time", "120", "-o", tmp.to_str().unwrap(), &asset_url])
        .status()?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        bail!("download failed: {}", asset_url);
    }
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&tmp, &bin_dst)?;

    let _ = std::fs::write(last_seen_tag_file(), &tag);
    ui::ok(&format!("installed {} → {}", tag, bin_dst.display()));
    Ok(true)
}

/// Install a specific tag (e.g. `v0.6.10`). Used by `8sync up --to <tag>`
/// for reproducibility / explicit downgrade.
pub fn install_tag(tag: &str) -> Result<bool> {
    ui::step(&format!("Self-update → pinned tag {}", tag));
    let tag = tag.strip_prefix('v').map(|t| format!("v{}", t)).unwrap_or_else(|| format!("v{}", tag));
    let asset_url = format!(
        "https://github.com/{}/{}/releases/download/{}/{}{}{}",
        REPO_OWNER, REPO_NAME, tag, ASSET_PREFIX, tag, ASSET_SUFFIX
    );
    let bin_dst = dirs::home_dir()
        .ok_or_else(|| anyhow!("no home dir"))?
        .join(".local/bin/8sync");
    std::fs::create_dir_all(bin_dst.parent().unwrap())?;
    let tmp = bin_dst.with_extension(format!("new.{}", std::process::id()));
    ui::info(&format!("$ curl -fsSL --max-time 120 -o {} {}", tmp.display(), asset_url));
    let status = Command::new("curl")
        .args(["-fsSL", "--max-time", "120", "-o", tmp.to_str().unwrap(), &asset_url])
        .status()?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        bail!("download failed: {}", asset_url);
    }
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&tmp, &bin_dst)?;
    let _ = std::fs::write(last_seen_tag_file(), &tag);
    ui::ok(&format!("installed {} → {}", tag, bin_dst.display()));
    Ok(true)
}
