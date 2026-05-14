// 8sync bg — wallpaper all-in-one
//
// Smart-parse pattern:
//   8sync bg                       → show status
//   8sync bg <keywords...>         → search & auto-set top result (Wallhaven default)
//   8sync bg /path                 → set from file
//   8sync bg https://...           → download & set
//   8sync bg 0.7                   → set opacity
//   8sync bg + | -                 → nudge opacity ±0.05
//   8sync bg off                   → clear image
//   8sync bg pick                  → fzf picker from local cache
//   8sync bg tint 0.5              → kitty background_tint
//   8sync bg rotate on [N]         → autochange every N minutes (default 15)
//   8sync bg rotate off
//   8sync bg --source yandere|safebooru <kw>
//
// HTTP: shell out to `curl` (avoid reqwest+TLS bloat).
// State: ~/.config/8sync/state.toml  (opacity, tint, last_source)
// Cache: ~/.cache/8sync/bg/{search-cache,*.jpg}
// Library: ~/.local/share/8sync/wallpapers/*.jpg  (downloads land here)

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync bg                       show current
      8sync bg cyberpunk city        Wallhaven search & set top result
      8sync bg -s yandere scenery    use yande.re
      8sync bg -s safebooru cat      use safebooru
      8sync bg /path/to/img.jpg      set from file
      8sync bg https://x/img.jpg     download & set
      8sync bg 0.7                   opacity
      8sync bg +                     opacity +0.05
      8sync bg -                     opacity -0.05
      8sync bg tint 0.4              kitty background_tint
      8sync bg off                   clear image
      8sync bg pick                  fzf from cache+library
      8sync bg rotate on 10          rotate every 10 min (systemd-user timer)
      8sync bg rotate off
"})]
pub struct Args {
    /// Free-form args (smart-parse). See EXAMPLES.
    pub rest: Vec<String>,

    /// Source khi search: wallhaven (default) | yandere | safebooru
    #[arg(long, short = 's', default_value = "wallhaven")]
    pub source: String,

    /// Min width khi search
    #[arg(long, default_value_t = 2560)]
    pub min_width: u32,

    /// Số lượng result lấy về
    #[arg(long, short = 'n', default_value_t = 12)]
    pub limit: u32,
}

pub fn run(a: Args) -> Result<()> {
    let rest: Vec<&str> = a.rest.iter().map(|s| s.as_str()).collect();
    let joined = a.rest.join(" ");
    let trimmed = joined.trim();

    // No args → status
    if trimmed.is_empty() {
        return show_status();
    }

    // Pure opacity number
    if let Ok(v) = trimmed.parse::<f32>() {
        return set_opacity(v);
    }
    if trimmed == "+" { return nudge_opacity(0.05); }
    if trimmed == "-" { return nudge_opacity(-0.05); }
    if trimmed == "off" { return clear_bg(); }
    if trimmed == "pick" { return pick_local(); }

    // tint
    if rest.first().copied() == Some("tint") {
        return handle_tint(rest.get(1).copied());
    }
    // rotate
    if rest.first().copied() == Some("rotate") {
        return handle_rotate(rest.get(1).copied(), rest.get(2).copied());
    }
    // URL
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return set_from_url(trimmed);
    }
    // Local path
    let p = Path::new(trimmed);
    if p.exists() {
        return set_bg_file(p);
    }

    // Otherwise: treat as search keywords
    search_and_set(&a.source, trimmed, a.min_width, a.limit)
}

// ─────────────────────────────────────────────────────────────────
// State
// ─────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Default)]
struct State {
    opacity: Option<f32>,
    tint: Option<f32>,
    last_source: Option<String>,
    last_path: Option<String>,
    rotate_every_min: Option<u32>,
}

fn state_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("8sync/state.toml")
}

fn load_state() -> State {
    let p = state_path();
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| toml::from_str::<State>(&s).ok())
        .unwrap_or_default()
}

fn save_state(s: &State) -> Result<()> {
    let p = state_path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, toml::to_string_pretty(s)?)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// Opacity / tint
// ─────────────────────────────────────────────────────────────────
fn set_opacity(v: f32) -> Result<()> {
    let clamped = v.clamp(0.0, 1.0);
    let s = Command::new("kitty")
        .args(["@", "set-background-opacity", &format!("{:.2}", clamped)])
        .status();
    if s.is_err() || !s.unwrap().success() {
        ui::warn("kitty @ failed (no remote control? open new kitty after `8sync setup`)");
    } else {
        ui::ok(&format!("kitty opacity = {:.2}", clamped));
    }
    let mut st = load_state();
    st.opacity = Some(clamped);
    save_state(&st)?;
    Ok(())
}

fn nudge_opacity(d: f32) -> Result<()> {
    let cur = load_state().opacity.unwrap_or(0.85);
    set_opacity(cur + d)
}

fn handle_tint(v: Option<&str>) -> Result<()> {
    let v = v.ok_or_else(|| anyhow::anyhow!("usage: 8sync bg tint <0..1>"))?;
    let parsed: f32 = v.parse().context("tint must be 0..1")?;
    let clamped = parsed.clamp(0.0, 1.0);
    // kitty doesn't expose `background_tint` to @ set-... in older versions.
    // Edit kitty.conf line + reload.
    let conf = dirs::config_dir().unwrap_or_default().join("kitty/kitty.conf");
    if conf.exists() {
        let mut content = std::fs::read_to_string(&conf)?;
        let mut found = false;
        content = content
            .lines()
            .map(|l| {
                if l.trim_start().starts_with("background_tint ") {
                    found = true;
                    format!("background_tint {:.2}", clamped)
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        if !found {
            content.push_str(&format!("\nbackground_tint {:.2}\n", clamped));
        }
        std::fs::write(&conf, content)?;
        let _ = Command::new("kill").args(["-SIGUSR1"]).arg(kitty_pid()).status();
        ui::ok(&format!("kitty background_tint = {:.2} (SIGUSR1 sent)", clamped));
    } else {
        ui::warn("kitty.conf missing — run `8sync setup`");
    }
    let mut st = load_state();
    st.tint = Some(clamped);
    save_state(&st)?;
    Ok(())
}

fn kitty_pid() -> String {
    std::env::var("KITTY_PID").unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────
// Set bg
// ─────────────────────────────────────────────────────────────────
fn set_bg_file(path: &Path) -> Result<()> {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = Command::new("kitty")
        .args(["@", "set-background-image", abs.to_str().unwrap()])
        .status();
    if s.is_err() || !s.unwrap().success() {
        ui::warn("kitty @ failed (need allow_remote_control + new kitty session)");
    } else {
        ui::ok(&format!("kitty bg ← {}", abs.display()));
    }
    let mut st = load_state();
    st.last_path = Some(abs.to_string_lossy().to_string());
    save_state(&st)?;
    Ok(())
}

fn clear_bg() -> Result<()> {
    Command::new("kitty")
        .args(["@", "set-background-image", "none"])
        .status()?;
    ui::ok("kitty bg cleared");
    let mut st = load_state();
    st.last_path = None;
    save_state(&st)?;
    Ok(())
}

fn set_from_url(url: &str) -> Result<()> {
    let lib = library_dir();
    std::fs::create_dir_all(&lib)?;
    let fname = url
        .split('/')
        .last()
        .unwrap_or("bg.jpg")
        .split('?')
        .next()
        .unwrap_or("bg.jpg")
        .to_string();
    let dst = lib.join(format!("{}-{}", short_ts(), fname));
    ui::info(&format!("downloading → {}", dst.display()));
    let st = Command::new("curl")
        .args(["-fL", "-o", dst.to_str().unwrap(), url])
        .status()?;
    if !st.success() {
        anyhow::bail!("curl failed for {}", url);
    }
    set_bg_file(&dst)
}

// ─────────────────────────────────────────────────────────────────
// Status / pick / show
// ─────────────────────────────────────────────────────────────────
fn show_status() -> Result<()> {
    let st = load_state();
    println!("8sync bg status:");
    println!("  opacity:     {}", st.opacity.map(|v| format!("{:.2}", v)).unwrap_or("(unset)".into()));
    println!("  tint:        {}", st.tint.map(|v| format!("{:.2}", v)).unwrap_or("(unset)".into()));
    println!("  current bg:  {}", st.last_path.unwrap_or_else(|| "(none)".into()));
    println!("  source last: {}", st.last_source.unwrap_or_else(|| "(none)".into()));
    println!("  rotate:      {}", st.rotate_every_min.map(|n| format!("every {}m", n)).unwrap_or("off".into()));
    println!();
    println!("Library: {}", library_dir().display());
    let count = std::fs::read_dir(library_dir()).map(|i| i.count()).unwrap_or(0);
    println!("  {} image(s) cached", count);
    println!();
    println!("Try: 8sync bg <kw> | <path|url> | <0..1> | + | - | off | pick | rotate on N");
    Ok(())
}

fn pick_local() -> Result<()> {
    let lib = library_dir();
    let entries: Vec<PathBuf> = std::fs::read_dir(&lib)
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp")
                })
                .collect()
        })
        .unwrap_or_default();
    if entries.is_empty() {
        ui::warn(&format!("no images in {}", lib.display()));
        ui::info("Use: 8sync bg <keywords>  to search & download");
        return Ok(());
    }
    if which::which("fzf").is_err() {
        ui::warn("fzf not installed");
        return Ok(());
    }
    use std::io::Write;
    let list = entries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut child = Command::new("fzf")
        .args([
            "--prompt=bg > ",
            "--height=60%",
            "--reverse",
            "--preview", "kitten icat --clear --transfer-mode=memory --stdin=no --place=80x25@0x0 {} >/dev/tty 2>/dev/null; true",
            "--preview-window", "right:50%",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(list.as_bytes());
    }
    let out = child.wait_with_output()?;
    let pick = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if pick.is_empty() {
        return Ok(());
    }
    set_bg_file(Path::new(&pick))
}

// ─────────────────────────────────────────────────────────────────
// Search providers
// ─────────────────────────────────────────────────────────────────
fn search_and_set(source: &str, query: &str, min_width: u32, limit: u32) -> Result<()> {
    ui::info(&format!("searching {} for `{}` ...", source, query));
    let urls = match source {
        "wallhaven" => wallhaven_search(query, min_width, limit)?,
        "yandere" | "yande.re" | "yandere" => yandere_search(query, min_width, limit)?,
        "safebooru" => safebooru_search(query, limit)?,
        other => anyhow::bail!("unknown source: {} (use wallhaven|yandere|safebooru)", other),
    };
    if urls.is_empty() {
        ui::warn("no results");
        return Ok(());
    }
    ui::ok(&format!("got {} result(s)", urls.len()));
    let first = &urls[0];
    let mut st = load_state();
    st.last_source = Some(source.to_string());
    save_state(&st)?;
    // Download all into library for `pick`/rotate later
    let lib = library_dir();
    std::fs::create_dir_all(&lib)?;
    let q_safe: String = query
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    for (i, u) in urls.iter().enumerate() {
        let fname = format!("{}-{}-{}.jpg", source, q_safe, i);
        let dst = lib.join(&fname);
        if !dst.exists() {
            let _ = Command::new("curl")
                .args(["-fLs", "-o", dst.to_str().unwrap(), u])
                .status();
        }
    }
    // Set the first
    let fname = format!("{}-{}-0.jpg", source, q_safe);
    let target = lib.join(&fname);
    if target.exists() {
        set_bg_file(&target)?;
    } else {
        // fallback: set from URL direct download
        set_from_url(first)?;
    }
    Ok(())
}

fn wallhaven_search(query: &str, min_width: u32, limit: u32) -> Result<Vec<String>> {
    let q = urlencoding::encode(query);
    let url = format!(
        "https://wallhaven.cc/api/v1/search?q={}&atleast={}x1440&categories=111&purity=100&sorting=relevance&page=1",
        q, min_width
    );
    let body = curl_json(&url)?;
    let mut out = Vec::new();
    if let Some(arr) = body.get("data").and_then(|v| v.as_array()) {
        for item in arr.iter().take(limit as usize) {
            if let Some(p) = item.get("path").and_then(|v| v.as_str()) {
                out.push(p.to_string());
            }
        }
    }
    Ok(out)
}

fn yandere_search(query: &str, min_width: u32, limit: u32) -> Result<Vec<String>> {
    let tags = format!("{}+rating:safe+width:{}..", query.replace(' ', "+"), min_width);
    let url = format!(
        "https://yande.re/post.json?tags={}&limit={}",
        tags, limit
    );
    let body = curl_json(&url)?;
    let mut out = Vec::new();
    if let Some(arr) = body.as_array() {
        for item in arr {
            if let Some(p) = item.get("file_url").and_then(|v| v.as_str()) {
                out.push(p.to_string());
            }
        }
    }
    Ok(out)
}

fn safebooru_search(query: &str, limit: u32) -> Result<Vec<String>> {
    let tags = query.replace(' ', "+");
    let url = format!(
        "https://safebooru.org/index.php?page=dapi&s=post&q=index&json=1&tags={}&limit={}",
        tags, limit
    );
    let body = curl_json(&url)?;
    let mut out = Vec::new();
    if let Some(arr) = body.as_array() {
        for item in arr {
            let dir = item.get("directory").and_then(|v| v.as_str()).unwrap_or("");
            let img = item.get("image").and_then(|v| v.as_str()).unwrap_or("");
            if !dir.is_empty() && !img.is_empty() {
                out.push(format!("https://safebooru.org/images/{}/{}", dir, img));
            }
        }
    }
    Ok(out)
}

fn curl_json(url: &str) -> Result<serde_json::Value> {
    let out = Command::new("curl")
        .args(["-fLs", "-A", "8sync/0.1", url])
        .output()?;
    if !out.status.success() {
        anyhow::bail!("curl failed: {}", url);
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .with_context(|| format!("parse JSON from {}", url))?;
    Ok(v)
}

// ─────────────────────────────────────────────────────────────────
// Rotate daemon (systemd-user timer)
// ─────────────────────────────────────────────────────────────────
fn handle_rotate(action: Option<&str>, arg: Option<&str>) -> Result<()> {
    match action {
        Some("on") => {
            let mins: u32 = arg.and_then(|s| s.parse().ok()).unwrap_or(15);
            install_rotate_units(mins)?;
            let mut st = load_state();
            st.rotate_every_min = Some(mins);
            save_state(&st)?;
            ui::ok(&format!("rotate enabled, every {} min", mins));
            Ok(())
        }
        Some("off") => {
            disable_rotate_units()?;
            let mut st = load_state();
            st.rotate_every_min = None;
            save_state(&st)?;
            ui::ok("rotate disabled");
            Ok(())
        }
        Some("now") => rotate_now(),
        None => {
            let st = load_state();
            match st.rotate_every_min {
                Some(n) => ui::info(&format!("rotate: every {}m", n)),
                None => ui::info("rotate: off"),
            }
            Ok(())
        }
        Some(other) => {
            ui::warn(&format!("unknown rotate action `{}` (use: on N | off | now)", other));
            Ok(())
        }
    }
}

fn install_rotate_units(mins: u32) -> Result<()> {
    let unit_dir = dirs::config_dir().unwrap_or_default().join("systemd/user");
    std::fs::create_dir_all(&unit_dir)?;

    // 8sync binary path (resolve "8sync" via PATH at install time)
    let bin = which::which("8sync")
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "8sync".to_string());

    let service = format!(
        "[Unit]\nDescription=8sync bg rotate (one-shot)\n\n[Service]\nType=oneshot\nExecStart={} bg rotate now\n",
        bin
    );
    let timer = format!(
        "[Unit]\nDescription=8sync bg rotate timer\n\n[Timer]\nOnBootSec=1min\nOnUnitActiveSec={}min\nUnit=8sync-bg-rotate.service\n\n[Install]\nWantedBy=timers.target\n",
        mins
    );
    std::fs::write(unit_dir.join("8sync-bg-rotate.service"), service)?;
    std::fs::write(unit_dir.join("8sync-bg-rotate.timer"), timer)?;

    let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
    Command::new("systemctl")
        .args(["--user", "enable", "--now", "8sync-bg-rotate.timer"])
        .status()?;
    Ok(())
}

fn disable_rotate_units() -> Result<()> {
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "--now", "8sync-bg-rotate.timer"])
        .status();
    Ok(())
}

fn rotate_now() -> Result<()> {
    let lib = library_dir();
    let entries: Vec<PathBuf> = std::fs::read_dir(&lib)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    if entries.is_empty() {
        ui::warn("library empty; nothing to rotate");
        return Ok(());
    }
    let st = load_state();
    let cur = st.last_path.clone().unwrap_or_default();
    let next = entries
        .iter()
        .find(|p| p.to_string_lossy() != cur)
        .or_else(|| entries.first())
        .cloned()
        .unwrap();
    set_bg_file(&next)
}

// ─────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────
fn library_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("8sync/wallpapers")
}

fn short_ts() -> String {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", s)
}
