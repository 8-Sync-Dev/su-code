// Embedded asset bundle: configs, skills, wallpaper URL list
use rust_embed::RustEmbed;

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
    Ok(true)
}
