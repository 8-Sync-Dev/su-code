// Self-update: pull the prebuilt binary from the latest GitHub Release.
// Also exposes a rate-limited auto-check used from main() before dispatch.

use anyhow::{anyhow, bail, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::{platform, ui};

const REPO_OWNER: &str = "8-Sync-Dev";
const REPO_NAME: &str = "su-code";
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

/// The release-asset OS/arch label, matching the names produced by
/// `.github/workflows/release.yml` (`8sync-<tag>-<os>-<arch>[.exe]`):
/// `linux-x86_64`, `linux-aarch64`, `darwin-x86_64`, `darwin-arm64`,
/// `windows-x86_64`. macOS names its ARM slice `arm64`, not `aarch64`.
fn asset_label() -> String {
    let os = match platform::os() {
        platform::Os::Macos => "darwin",
        platform::Os::Windows => "windows",
        _ => "linux",
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" if os == "darwin" => "arm64",
        other => other,
    };
    format!("{}-{}", os, arch)
}

/// Release asset file name for `tag`, e.g. `8sync-v0.52.0-windows-x86_64.exe`.
fn asset_filename(tag: &str) -> String {
    let ext = if platform::os() == platform::Os::Windows { ".exe" } else { "" };
    format!("{}{}-{}{}", ASSET_PREFIX, tag, asset_label(), ext)
}

/// Install target: the running executable itself, so `8sync up` replaces 8sync
/// wherever it actually lives — the Unix `~/.local/bin/8sync` OR the Windows
/// `%LOCALAPPDATA%\Programs\8sync\8sync.exe`. This is what fixes the Windows
/// bug where a hard-coded extension-less `~/.local/bin/8sync` left an
/// un-runnable file (Windows then popped an "open with" dialog). Falls back to
/// the legacy Unix path only if the current exe cannot be resolved.
fn dest_path() -> Result<PathBuf> {
    if let Ok(p) = std::env::current_exe() {
        return Ok(std::fs::canonicalize(&p).unwrap_or(p));
    }
    let name = if platform::os() == platform::Os::Windows { "8sync.exe" } else { "8sync" };
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow!("no home dir"))?
        .join(".local/bin")
        .join(name))
}

/// Best-effort removal of `.8sync.old.*` leftovers from a prior Windows update
/// (a running .exe can't be deleted, so its predecessor is swept on a later run).
fn sweep_old_siblings(dst: &Path) {
    let Some(dir) = dst.parent() else { return };
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if e.file_name().to_string_lossy().starts_with(".8sync.old.") {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Download `asset_url` into a temp sibling of `dst` and move it into place with
/// platform-correct replace semantics. Unix swaps the inode with a same-dir
/// rename (fine even while running). Windows cannot overwrite a live .exe, but
/// CAN rename it aside, so the live binary is moved to `.8sync.old.<pid>` first
/// and restored if the install fails.
fn download_and_replace(asset_url: &str, dst: &Path) -> Result<()> {
    let dir = dst
        .parent()
        .ok_or_else(|| anyhow!("bad destination path: {}", dst.display()))?;
    std::fs::create_dir_all(dir)?;
    sweep_old_siblings(dst);

    let tmp = dir.join(format!(".8sync.new.{}", std::process::id()));
    let status = Command::new("curl")
        .args(["-fsSL", "--max-time", "120", "-o", tmp.to_str().unwrap(), asset_url])
        .status()?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        bail!("download failed: {}", asset_url);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }

    #[cfg(windows)]
    {
        if dst.exists() {
            let old = dir.join(format!(".8sync.old.{}", std::process::id()));
            std::fs::rename(dst, &old)
                .map_err(|e| anyhow!("could not move current binary aside ({}): {}", dst.display(), e))?;
            if let Err(e) = std::fs::rename(&tmp, dst) {
                let _ = std::fs::rename(&old, dst); // restore, don't leave headless
                let _ = std::fs::remove_file(&tmp);
                bail!("could not install new binary to {}: {}", dst.display(), e);
            }
            return Ok(());
        }
    }

    std::fs::rename(&tmp, dst)
        .map_err(|e| anyhow!("could not install {}: {}", dst.display(), e))?;
    Ok(())
}

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
    let want_name = asset_filename(&tag);
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

    let dst = dest_path()?;
    ui::info(&format!("local v{} → {} ({})", local, tag, asset_url));
    download_and_replace(&asset_url, &dst)?;

    let _ = std::fs::write(last_seen_tag_file(), &tag);
    ui::ok(&format!("installed {} → {}", tag, dst.display()));
    Ok(true)
}

/// Install a specific tag (e.g. `v0.6.10`). Used by `8sync up --to <tag>`
/// for reproducibility / explicit downgrade.
pub fn install_tag(tag: &str) -> Result<bool> {
    ui::step(&format!("Self-update → pinned tag {}", tag));
    let tag = tag.strip_prefix('v').map(|t| format!("v{}", t)).unwrap_or_else(|| format!("v{}", tag));
    let asset_url = format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        REPO_OWNER, REPO_NAME, tag, asset_filename(&tag)
    );
    let dst = dest_path()?;
    ui::info(&format!("local v{} → {} ({})", build_version(), tag, asset_url));
    download_and_replace(&asset_url, &dst)?;

    let _ = std::fs::write(last_seen_tag_file(), &tag);
    ui::ok(&format!("installed {} → {}", tag, dst.display()));
    Ok(true)
}
