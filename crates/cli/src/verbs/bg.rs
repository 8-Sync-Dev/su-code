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
      8sync bg fit scaled            fill window exactly (default — recommended)
      8sync bg fit cscaled           keep aspect ratio, crop edges
      8sync bg fit centered          no scale, center
      8sync bg overlay 0.6           dark overlay 0..1 (higher = darker, easier to read code)
      8sync bg off                   clear image (desktop shows through)
      8sync bg through               see-through mode (image=none, tint=0, opacity=0.55)
      8sync bg blend                 keep image + low opacity (image AND desktop visible)
      8sync bg blend 0.4             blend with custom opacity
      8sync bg blend /img.jpg 0.5    blend a specific image
      8sync bg apply-now             spawn a new kitty window to see changes (opacity needs new window)
      8sync bg verify                full diagnostic: kitty ver, compositor, conf, KDE rules + test window
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
    if trimmed == "apply-now" || trimmed == "apply" {
        return apply_now();
    }
    if trimmed == "verify" || trimmed == "diag" || trimmed == "test" {
        return verify_transparency();
    }
    if trimmed == "through" || trimmed == "see" || trimmed == "glass" {
        return see_through(0.55);
    }
    // `bg blend [path|opacity]` = image + low opacity + tint=0 → image hiện
    // mờ và desktop vẫn ló qua (vibe glass có ảnh).
    if rest.first().copied() == Some("blend") {
        return blend_mode(rest.get(1).copied(), rest.get(2).copied());
    }
    if trimmed == "pick" { return pick_local(); }

    if rest.first().copied() == Some("fit") {
        return handle_fit(rest.get(1).copied());
    }
    // tint / overlay (semantic alias: overlay = how dark a layer goes ON TOP of image)
    if matches!(rest.first().copied(), Some("tint") | Some("overlay") | Some("dim")) {
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
    original_path: Option<String>,
    rotate_every_min: Option<u32>,
}

fn which(name: &str) -> Option<PathBuf> {
    ::which::which(name).ok()
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
    ensure_kitty_conf()?;
    kitty_conf_set("background_opacity", &format!("{:.2}", clamped))?;
    kitty_conf_set("dynamic_background_opacity", "yes")?;
    kitty_conf_set("background_blur", "32")?;
    // Try live runtime change via remote control first (no flash)
    let live = Command::new("kitty")
        .args(["@", "set-background-opacity", &format!("{:.2}", clamped)])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .ok()
        .map(|s| s.success())
        .unwrap_or(false);
    if !live {
        // Fallback: SIGUSR1 reload (works without remote control)
        kitty_reload();
    }
    ui::ok(&format!("kitty opacity = {:.2} {}", clamped,
        if live { "(live)" } else { "(SIGUSR1 reload)" }));
    if !live {
        warn_opacity_needs_restart();
    }
    let mut st = load_state();
    st.opacity = Some(clamped);
    save_state(&st)?;
    Ok(())
}

/// Kitty trade-off: `background_image` swaps live via SIGUSR1, but
/// `background_opacity` is baked in at window creation. Reloading
/// conf does NOT change opacity for already-open windows — only new
/// ones pick it up. Let the user know exactly what to do.
fn warn_opacity_needs_restart() {
    eprintln!(
        "\x1b[33m! Opacity is baked in at window creation. Current kitty windows will\n  stay opaque until you close & reopen kitty. Quick test:\n    \x1b[1msetsid -f kitty\x1b[0m\x1b[33m       (opens a new window with the new opacity)\n  Or run `8sync bg apply-now` to spawn a test window automatically.\x1b[0m"
    );
}

fn nudge_opacity(d: f32) -> Result<()> {
    let cur = load_state().opacity.unwrap_or(0.85);
    set_opacity(cur + d)
}

fn handle_tint(v: Option<&str>) -> Result<()> {
    let v = v.ok_or_else(|| anyhow::anyhow!("usage: 8sync bg overlay <0..1>  (0 = image full visible, 1 = fully dark)"))?;
    let parsed: f32 = v.parse().context("overlay must be 0..1")?;
    let clamped = parsed.clamp(0.0, 1.0);
    ensure_kitty_conf()?;

    // FLASH-WEZTERM SECRET: WezTerm has `hsb.brightness` on image source —
    // kitty doesn't. So we PRE-DARKEN the image file itself with ImageMagick,
    // then point kitty to the darkened copy. This is the ONLY way to get
    // equivalent of wezterm hsb on kitty.
    //
    // Brightness of darkened image = (1 - overlay) * 100  (modulate %).
    // overlay 0.6 → image brightness 40% → much easier to read code on top.
    let st_orig = load_state();
    if let Some(orig) = st_orig.last_path.as_ref() {
        let orig_path = std::path::Path::new(orig);
        // Find the TRUE original (un-darkened) — if last_path is already a darkened
        // version in cache, use the stored orig instead.
        let true_orig = st_orig.original_path.as_ref()
            .map(std::path::Path::new)
            .filter(|p| p.exists())
            .unwrap_or(orig_path);
        if true_orig.exists() && which("magick").is_some() {
            let bright_pct = ((1.0 - clamped) * 100.0).clamp(5.0, 100.0) as u32;
            let cache = dirs::cache_dir().unwrap_or_default().join("8sync/bg/dim");
            std::fs::create_dir_all(&cache)?;
            let stem = true_orig.file_stem()
                .and_then(|s| s.to_str()).unwrap_or("img");
            let dimmed = cache.join(format!("{}-b{}.jpg", stem, bright_pct));
            if !dimmed.exists() {
                ui::info(&format!("pre-darkening image to {}% brightness ...", bright_pct));
                let status = Command::new("magick")
                    .arg(true_orig)
                    .args(["-modulate", &format!("{},90,100", bright_pct)])
                    .args(["-quality", "85"])
                    .arg(&dimmed)
                    .status();
                if !matches!(status, Ok(s) if s.success()) {
                    ui::warn("ImageMagick failed — falling back to kitty background_tint");
                } else {
                    // Point kitty to the darkened image, NOT the original.
                    kitty_conf_set("background_image", dimmed.to_str().unwrap())?;
                    // Reset native tint to 0 — darkening already baked in.
                    kitty_conf_set("background_tint", "0.0")?;
                    kitty_conf_set("background_tint_gaps", "0.0")?;
                    let _ = Command::new("kitty")
                        .args(["@", "set-background-image", dimmed.to_str().unwrap()])
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                    kitty_reload();
                    ui::ok(&format!("overlay = {:.2} (image pre-darkened to {}% — code is now {} to read)",
                        clamped, bright_pct,
                        if clamped > 0.5 { "much easier" } else { "easier" }));
                    let mut st = load_state();
                    st.tint = Some(clamped);
                    st.last_path = Some(dimmed.to_string_lossy().to_string());
                    st.original_path = Some(true_orig.to_string_lossy().to_string());
                    save_state(&st)?;
                    return Ok(());
                }
            } else {
                // Cache hit — instant swap
                kitty_conf_set("background_image", dimmed.to_str().unwrap())?;
                kitty_conf_set("background_tint", "0.0")?;
                kitty_conf_set("background_tint_gaps", "0.0")?;
                let _ = Command::new("kitty")
                    .args(["@", "set-background-image", dimmed.to_str().unwrap()])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                kitty_reload();
                ui::ok(&format!("overlay = {:.2} (cached, instant swap)", clamped));
                let mut st = load_state();
                st.tint = Some(clamped);
                st.last_path = Some(dimmed.to_string_lossy().to_string());
                st.original_path = Some(true_orig.to_string_lossy().to_string());
                save_state(&st)?;
                return Ok(());
            }
        }
    }

    // Fallback: no image or ImageMagick missing → use kitty native tint
    kitty_conf_set("background_tint", &format!("{:.2}", clamped))?;
    kitty_conf_set("background_tint_gaps", &format!("{:.2}", clamped))?;
    kitty_reload();
    ui::ok(&format!("overlay = {:.2} (via kitty tint — install imagemagick for true pre-darken)", clamped));
    let mut st = load_state();
    st.tint = Some(clamped);
    save_state(&st)?;
    Ok(())
}

/// `bg fit <mode>` — control how image fills kitty window.
///   scaled       fill window exactly (may stretch — best for "đúng cửa sổ")
///   cscaled      cover-scaled, keep aspect ratio, crop edges (default)
///   centered     center, no scale (good for small images)
///   tiled        repeat
///   mirror-tiled mirror repeat
///   clamped      no scale, no center
fn handle_fit(v: Option<&str>) -> Result<()> {
    let mode = v.unwrap_or("scaled");
    let valid = ["scaled", "cscaled", "centered", "tiled", "mirror-tiled", "clamped"];
    if !valid.contains(&mode) {
        return Err(anyhow::anyhow!(
            "invalid mode '{}'. Valid: {}",
            mode, valid.join(" | ")
        ));
    }
    ensure_kitty_conf()?;
    kitty_conf_set("background_image_layout", mode)?;
    kitty_conf_set("background_image_linear", "yes")?;
    kitty_reload();
    ui::ok(&format!("layout = {} (SIGUSR1 reload)", mode));
    println!("  scaled       fill window exactly (may stretch, no empty space)");
    println!("  cscaled      cover-scaled, keep aspect, crop edges");
    println!("  centered     center, no scale");
    println!("  tiled        repeat image");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// kitty.conf live editor (no restart needed thanks to SIGUSR1)
// ─────────────────────────────────────────────────────────────────
fn kitty_conf_path() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join("kitty/kitty.conf")
}

/// Create a minimal kitty.conf if missing, so subsequent edits have something to patch.
/// Contains the keys needed for live wallpaper/opacity reload + remote control.
fn ensure_kitty_conf() -> Result<()> {
    let p = kitty_conf_path();
    if p.exists() { return Ok(()); }
    std::fs::create_dir_all(p.parent().unwrap())?;
    let stub = r#"# ~/.config/kitty/kitty.conf — bootstrapped by 8sync
# Run `8sync setup` for the full managed config.
font_family       JetBrainsMono Nerd Font
font_size         12.0

# Background — see-through model for KDE Plasma Wayland
background         #0b1220
# CRITICAL for translucency on KDE Plasma Wayland:
# - dynamic_background_opacity MUST be set in initial config (can't be added via reload)
# - background_opacity < 1 makes bg color transparent (but NOT background_image)
# - background_tint > 0 mixes image with bg color → image becomes semi-transparent
#   Formula per kitty docs: pixel = image*(1-tint) + bg_color*tint
#   Since bg_color has alpha=opacity, effective image transparency = tint*(1-opacity)
background_opacity 0.75
dynamic_background_opacity yes
background_blur    32
background_tint    0.0
background_tint_gaps 0.0
background_image_layout scaled
background_image_linear yes
# transparent_background_colors lets specific cell bg colors be semi-transparent
# even when they don't match the default terminal bg. Critical for TUIs like
# forge/helix that paint their own bg. Format: color@opacity.
transparent_background_colors #000000@0.5 #0b1220@0.5 #1e1e2e@0.5 #282a36@0.5 #1a1b26@0.5

# Remote control (kitty @ ...)
allow_remote_control yes
listen_on          unix:@kitty
clipboard_control  write-clipboard write-primary read-clipboard read-primary

# Wayland: explicit so KDE/Plasma6 + Hyprland behave the same
linux_display_server wayland
"#;
    std::fs::write(&p, stub)?;
    ui::ok(&format!("created {}", p.display()));
    Ok(())
}

/// Set or replace a single `key value` line in kitty.conf (idempotent).
fn kitty_conf_set(key: &str, value: &str) -> Result<()> {
    let p = kitty_conf_path();
    let content = std::fs::read_to_string(&p).unwrap_or_default();
    let mut found = false;
    let mut lines: Vec<String> = content
        .lines()
        .map(|l| {
            let lt = l.trim_start();
            if lt.starts_with(&format!("{} ", key)) || lt == key {
                found = true;
                format!("{} {}", key, value)
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(format!("{} {}", key, value));
    }
    let mut joined = lines.join("\n");
    if !joined.ends_with('\n') { joined.push('\n'); }
    std::fs::write(&p, joined)?;
    Ok(())
}

/// Reload kitty config in every running kitty instance.
/// `SIGUSR1` triggers a config re-read across all OS windows (since v0.21+).
/// Works even WITHOUT `allow_remote_control` — no restart required.
fn kitty_reload() {
    let _ = Command::new("pkill")
        .args(["-USR1", "-x", "kitty"])
        .status();
}

#[allow(dead_code)]
fn kitty_pid() -> String {
    std::env::var("KITTY_PID").unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────
// Set bg
// ─────────────────────────────────────────────────────────────────
fn set_bg_file(path: &Path) -> Result<()> {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    ensure_kitty_conf()?;
    kitty_conf_set("background_image", abs.to_str().unwrap())?;
    kitty_conf_set("background_image_layout", "scaled")?;
    kitty_conf_set("background_image_linear", "yes")?;
    kitty_conf_set("background_tint", "0.0")?;
    kitty_conf_set("background_tint_gaps", "0.0")?;
    let live = Command::new("kitty")
        .args(["@", "set-background-image", abs.to_str().unwrap()])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .ok()
        .map(|s| s.success())
        .unwrap_or(false);
    if !live { kitty_reload(); }
    ui::ok(&format!("kitty bg ← {} {}", abs.display(),
        if live { "(live)" } else { "(SIGUSR1 reload)" }));
    let mut st = load_state();
    st.last_path = Some(abs.to_string_lossy().to_string());
    st.original_path = Some(abs.to_string_lossy().to_string());
    save_state(&st)?;
    // Auto-reapply overlay if user previously set one
    let prev_tint = st.tint.unwrap_or(0.0);
    if prev_tint > 0.05 {
        let _ = handle_tint(Some(&format!("{:.2}", prev_tint)));
    }
    Ok(())
}

fn clear_bg() -> Result<()> {
    ensure_kitty_conf()?;
    kitty_conf_set("background_image", "none")?;
    kitty_conf_set("background_tint", "0.0")?;
    let _ = Command::new("kitty")
        .args(["@", "set-background-image", "none"])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status();
    kitty_reload();
    ui::ok("kitty bg cleared + tint=0 — desktop will show through opacity (SIGUSR1 reload)");
    let mut st = load_state();
    st.last_path = None;
    st.tint = Some(0.0);
    save_state(&st)?;
    Ok(())
}

/// Maximum-transparency mode: clear image, low opacity → desktop shows through.
/// No image = no tint conflict, opacity directly controls bg color alpha.
fn see_through(opacity: f32) -> Result<()> {
    ensure_kitty_conf()?;
    kitty_conf_set("background_image", "none")?;
    kitty_conf_set("background_tint", "0.0")?;
    kitty_conf_set("background_tint_gaps", "0.0")?;
    let clamped = opacity.clamp(0.2, 1.0);
    kitty_conf_set("background_opacity", &format!("{:.2}", clamped))?;
    kitty_conf_set("dynamic_background_opacity", "yes")?;
    let _ = Command::new("kitty")
        .args(["@", "set-background-image", "none"])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status();
    // Apply opacity runtime to existing windows (dynamic_background_opacity must be yes in initial conf)
    let _ = Command::new("kitty")
        .args(["@", "set-background-opacity", &format!("{:.2}", clamped)])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status();
    kitty_reload();
    ui::ok(&format!("see-through mode: image=none, opacity={:.2}", clamped));
    warn_opacity_needs_restart();
    let mut st = load_state();
    st.last_path = None;
    st.tint = Some(0.0);
    st.opacity = Some(clamped);
    save_state(&st)?;
    Ok(())
}

/// Spawn a fresh kitty window so the user can immediately see whether the
/// current opacity/image config produces translucency on their compositor.
fn apply_now() -> Result<()> {
    ensure_kitty_conf()?;
    let _ = Command::new("setsid")
        .args(["-f", "kitty"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    ui::ok("spawned a new kitty window — that's where opacity changes take effect");
    eprintln!(
        "\x1b[36m  tip: close your old kitty windows once the new one looks right,\n       then every subsequent `8sync bg ...` will apply live.\x1b[0m"
    );
    Ok(())
}

/// Full diagnostic for "why isn't my transparency working".
/// Spawns a test window with EXPLICIT --override flags so user can isolate
/// whether it's a config issue, a compositor issue, or a window-rule issue.
fn verify_transparency() -> Result<()> {
    println!("\x1b[1;36m=== 8sync bg verify — transparency diagnostic ===\x1b[0m");

    // 1. Kitty version
    let ver = Command::new("kitty").arg("--version").output()
        .ok().and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());
    println!("kitty:        {}", ver.trim());

    // 2. Session type / compositor
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "?".into());
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "?".into());
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "(none)".into());
    println!("session:      {} ({})", session, desktop);
    println!("wayland:      {}", wayland);

    // 3. Current window state
    let win_pid = std::env::var("KITTY_PID").unwrap_or_else(|_| "?".into());
    println!("current win:  KITTY_PID={}", win_pid);

    // 4. Conf path + key settings
    let conf = kitty_conf_path();
    println!("conf:         {}", conf.display());
    if conf.exists() {
        let content = std::fs::read_to_string(&conf).unwrap_or_default();
        for key in [
            "background_opacity",
            "background_image",
            "background_image_layout",
            "background_tint",
            "background_tint_gaps",
            "background_blur",
            "dynamic_background_opacity",
            "transparent_background_colors",
        ] {
            let found = content.lines().find(|l| {
                let t = l.trim_start();
                t.starts_with(&format!("{} ", key)) || t == key
            });
            println!("  {:30} {}", key, found.unwrap_or("(unset)"));
        }
    } else {
        println!("  \x1b[33m! conf file missing — run any `8sync bg ...` command to create it\x1b[0m");
    }

    // 5. Remote control reachability
    let rc = Command::new("kitty").args(["@", "ls"])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false);
    println!("remote ctrl:  {}", if rc { "\x1b[32mreachable\x1b[0m" }
                                  else { "\x1b[31munreachable\x1b[0m (need restart kitty after enabling)" });

    // 6. KDE window rules check (best-effort)
    if desktop.to_lowercase().contains("kde") {
        let rules = dirs::config_dir().unwrap_or_default().join("kwinrulesrc");
        if rules.exists() {
            let txt = std::fs::read_to_string(&rules).unwrap_or_default();
            if txt.to_lowercase().contains("kitty") && txt.contains("opacityactive") {
                println!("\x1b[33m! KDE window rule for kitty found in kwinrulesrc — may override opacity\x1b[0m");
                println!("  open: System Settings → Window Management → Window Rules");
            }
        }
    }

    println!();
    println!("\x1b[1;36m=== test 1: pure opacity (no image), via --override ===\x1b[0m");
    println!("running: setsid -f kitty --override background_opacity=0.4 \\");
    println!("                          --override background_blur=24 \\");
    println!("                          --override dynamic_background_opacity=yes");
    let _ = Command::new("setsid")
        .args(["-f", "kitty",
            "--override", "background_opacity=0.4",
            "--override", "background_blur=24",
            "--override", "dynamic_background_opacity=yes",
            "--title", "8sync-test-opacity",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    println!("→ new kitty window opened. \x1b[1mIf it shows the desktop through, your compositor works.\x1b[0m");
    println!("  If it does NOT show desktop:");
    println!("    a) KDE Plasma: check System Settings → Window Management → Compositor");
    println!("       (Compositor must be ON, hardware accel = OpenGL)");
    println!("    b) Check window rule for 'kitty' overriding opacity");
    println!("    c) Try `kitty --debug-config | grep background_opacity` to confirm value");
    println!();
    println!("\x1b[1;36mtest 2:\x1b[0m once test 1 works, run:");
    println!("  8sync bg apply-now    # spawn window with full conf");
    Ok(())
}

/// Glass-with-image: keeps a background image AND blends the whole thing
/// with the desktop via low opacity. `arg1` can be a path, a number
/// (opacity), or empty (re-use last image + 0.6 opacity).
fn blend_mode(arg1: Option<&str>, arg2: Option<&str>) -> Result<()> {
    // Parse: (path?, opacity?)
    let mut path: Option<PathBuf> = None;
    let mut opacity: f32 = 0.6;
    for a in [arg1, arg2].into_iter().flatten() {
        if let Ok(v) = a.parse::<f32>() {
            opacity = v;
        } else {
            let p = PathBuf::from(a);
            if p.exists() {
                path = Some(p);
            }
        }
    }
    let path = path.or_else(|| load_state().last_path.map(PathBuf::from));
    let Some(path) = path else {
        return Err(anyhow::anyhow!(
            "no image to blend — try `8sync bg <kw>` first or pass a path: `8sync bg blend /img.jpg 0.5`"
        ));
    };
    let abs = std::fs::canonicalize(&path).unwrap_or(path.clone());
    ensure_kitty_conf()?;
    kitty_conf_set("background_image", abs.to_str().unwrap())?;
    kitty_conf_set("background_image_layout", "scaled")?;
    kitty_conf_set("background_image_linear", "yes")?;
    let clamped = opacity.clamp(0.2, 1.0);
    // KEY FIX: tint must be > 0 for image to mix with bg color → semi-transparent.
    // tint=(1-opacity) gives intuitive "opacity" = how visible image is.
    // Lower opacity → higher tint → more desktop bleeding through image.
    let tint = (1.0 - clamped).clamp(0.3, 0.85);
    kitty_conf_set("background_tint", &format!("{:.2}", tint))?;
    kitty_conf_set("background_tint_gaps", &format!("{:.2}", tint))?;
    kitty_conf_set("background_opacity", &format!("{:.2}", clamped))?;
    kitty_conf_set("dynamic_background_opacity", "yes")?;
    let _ = Command::new("kitty")
        .args(["@", "set-background-image", abs.to_str().unwrap()])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status();
    let _ = Command::new("kitty")
        .args(["@", "set-background-opacity", &format!("{:.2}", clamped)])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status();
    kitty_reload();
    ui::ok(&format!(
        "blend: image={} opacity={:.2} tint={:.2} (image semi-transparent + desktop visible)",
        abs.display(),
        clamped,
        tint
    ));
    warn_opacity_needs_restart();
    let mut st = load_state();
    st.last_path = Some(abs.to_string_lossy().to_string());
    st.tint = Some(tint);
    st.opacity = Some(clamped);
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
