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

/// Is `remote` strictly newer than `local`? Compares dotted numeric components
/// left to right, so `0.54.0 > 0.53.9` and a LOCAL build ahead of the published
/// release (a dev build) never reports an update. String equality — the previous
/// test — called every non-identical string an upgrade, including downgrades.
/// Non-numeric suffixes are ignored rather than pulling in a semver crate.
fn is_newer(remote: &str, local: &str) -> bool {
    let part = |s: &str| -> Vec<u64> {
        s.split(['.', '-', '+'])
            .map(|c| {
                let digits: String = c.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse().unwrap_or(0)
            })
            .collect()
    };
    let (r, l) = (part(remote), part(local));
    for i in 0..r.len().max(l.len()) {
        let (a, b) = (r.get(i).copied().unwrap_or(0), l.get(i).copied().unwrap_or(0));
        if a != b {
            return a > b;
        }
    }
    false
}

/// Auto-check called from main(). Runs the network probe on a DETACHED thread and
/// never joins it, so the user's command is not delayed — previously this blocked
/// dispatch for up to 5s against a rate-limited api.github.com, on every command
/// including a bare `8sync`.
///
/// Silent unless stderr is a TTY and this is not CI: a notice in piped output or a
/// CI log is noise nobody asked for. `last_seen_tag` suppresses re-notifying about
/// a version the user has already been told about (it was written and never read).
pub fn auto_check_notice() {
    if std::env::var("SUSYNC_NO_AUTO_CHECK").is_ok() {
        return;
    }
    if std::env::var("CI").is_ok() {
        return;
    }
    if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        return;
    }

    // Print from CACHE — instant, no network on the command's critical path.
    //
    // The probe deliberately does not run inline. A previous revision spawned a
    // detached `std::thread`, which the runtime kills the moment `main` returns:
    // a `curl --max-time 5` never outlived an `8sync doctor`, and `touch_check()`
    // had already burned the 6-hour window, so the notice could never appear.
    // Refreshing a cache in a child process and reading it on the NEXT run gets a
    // reliable notice with zero added latency.
    if let Ok(tag) = std::fs::read_to_string(latest_tag_file()) {
        let tag = tag.trim();
        let local = build_version();
        if !tag.is_empty()
            && is_newer(strip_v(tag), local)
            && !std::fs::read_to_string(last_seen_tag_file()).is_ok_and(|s| s.trim() == tag)
        {
            let _ = std::fs::write(last_seen_tag_file(), tag);
            let msg =
                format!("! 8sync update available: v{local} → {tag} — run `8sync up` to install");
            if std::env::var("NO_COLOR").is_ok() {
                eprintln!("{msg}");
            } else {
                eprintln!("\x1b[33m{msg}\x1b[0m");
            }
        }
    }

    if !should_check() {
        return;
    }
    touch_check();
    spawn_detached_probe();
}

fn latest_tag_file() -> PathBuf {
    cache_dir().join("latest_tag")
}

/// Re-exec ourselves as a fire-and-forget child that refreshes the cache.
/// Detached on purpose: it must survive this process exiting, which is exactly
/// what a thread could not do.
fn spawn_detached_probe() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = std::process::Command::new(exe)
        .arg(PROBE_ARG)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Hidden argv marker for the refresh child. Matched in `main` before clap runs,
/// so it never appears in help and can never collide with a real verb.
pub const PROBE_ARG: &str = "__update-probe";

/// Child-process entry point: fetch the latest tag and cache it. Never prints.
pub fn run_probe() {
    let Some((tag, _url)) = fetch_latest_release() else {
        return;
    };
    let _ = std::fs::create_dir_all(cache_dir());
    let _ = std::fs::write(latest_tag_file(), tag);
}

/// Force self-update: download the latest release asset and install it.
/// Returns Ok(true) when a new binary was written, Ok(false) when already up-to-date.
pub fn run_self_update(force: bool) -> Result<bool> {
    ui::step("Self-update — GitHub Releases");
    let local = build_version();

    let (tag, asset_url) = fetch_latest_release()
        .ok_or_else(|| anyhow!("could not query latest release from github.com/{}/{}", REPO_OWNER, REPO_NAME))?;
    let remote = strip_v(&tag);
    // Skip unless the release is strictly newer. `up.rs` used to pass force=true
    // unconditionally, making this branch unreachable and re-downloading ~5 MB on
    // every single `8sync up`.
    if !force && !is_newer(&remote, &local) {
        ui::skip("8sync", &format!("up to date (v{local}) — `8sync up --force` to reinstall"));
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

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn only_strictly_newer_releases_notify() {
        // upgrade
        assert!(is_newer("0.54.0", "0.53.0"));
        assert!(is_newer("0.53.10", "0.53.9")); // numeric, not lexicographic
        assert!(is_newer("1.0.0", "0.99.99"));
        // equal -> never
        assert!(!is_newer("0.53.0", "0.53.0"));
        // downgrade -> never (string equality used to call this an "update")
        assert!(!is_newer("0.52.0", "0.53.0"));
        // a local dev build ahead of the published release must stay quiet
        assert!(!is_newer("0.53.0", "0.54.0-dev"));
        // missing components are zero, not an error
        assert!(is_newer("0.53.1", "0.53"));
        assert!(!is_newer("0.53", "0.53.0"));
    }
}
