// Embedded asset bundle: configs, skills, wallpaper URL list
use rust_embed::RustEmbed;
use std::path::Path;

#[derive(RustEmbed)]
#[folder = "../../assets/"]
pub struct Assets;

pub fn read(path: &str) -> Option<String> {
    let f = Assets::get(path)?;
    String::from_utf8(f.data.into_owned()).ok()
}

/// Write embedded asset to target path. If target exists and differs, back it up.
pub fn install(path: &str, target: &std::path::Path, force: bool) -> anyhow::Result<bool> {
    use std::fs;
    let content = read(path).ok_or_else(|| anyhow::anyhow!("asset missing: {}", path))?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    if target.exists() && !force {
        let existing = fs::read_to_string(target).unwrap_or_default();
        if existing == content {
            return Ok(false); // unchanged
        }
        let bak = target.with_extension(format!(
            "{}.bak",
            target.extension().and_then(|s| s.to_str()).unwrap_or("orig")
        ));
        fs::rename(target, &bak)?;
    }
    fs::write(target, content)?;
    #[cfg(unix)]
    if target.extension().and_then(|s| s.to_str()) == Some("sh") {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(target)?.permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(target, perms);
    }
    Ok(true)
}

/// Iterate every embedded asset path that begins with `prefix`. Returns full
/// asset paths (suitable for `read`/`install`), not paths relative to `prefix`.
pub fn iter_under(prefix: &str) -> Vec<String> {
    Assets::iter()
        .filter(|p| p.starts_with(prefix))
        .map(|p| p.into_owned())
        .collect()
}

/// Deploy every asset under `asset_prefix` into `target_dir`, preserving the
/// relative subtree. Returns (written, unchanged) counts. Files ending in `.sh`
/// get mode 0755 on unix. Backups are NOT created (this is intended for skill
/// trees that are managed entirely by 8sync; users edit local copies, not
/// global ones).
pub fn install_tree(asset_prefix: &str, target_dir: &Path) -> anyhow::Result<(usize, usize)> {
    use std::fs;
    let mut written = 0usize;
    let mut unchanged = 0usize;
    let prefix = if asset_prefix.ends_with('/') {
        asset_prefix.to_string()
    } else {
        format!("{}/", asset_prefix)
    };
    for asset_path in iter_under(&prefix) {
        let rel = &asset_path[prefix.len()..];
        if rel.is_empty() {
            continue;
        }
        let body = match read(&asset_path) {
            Some(b) => b,
            None => continue,
        };
        let target = target_dir.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let prev = fs::read_to_string(&target).unwrap_or_default();
        if prev == body && target.exists() {
            unchanged += 1;
        } else {
            fs::write(&target, &body)?;
            written += 1;
        }
        #[cfg(unix)]
        if target.extension().and_then(|s| s.to_str()) == Some("sh") {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&target)?.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&target, perms);
        }
    }
    Ok((written, unchanged))
}
