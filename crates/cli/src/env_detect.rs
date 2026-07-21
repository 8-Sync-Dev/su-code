// Environment & system detection
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub struct Env {
    pub home: PathBuf,
    pub xdg_config: PathBuf,
    pub os_id: String,
}

impl Env {
    pub fn detect() -> Result<Self> {
        let home = dirs::home_dir().context("no HOME")?;
        let xdg_config = dirs::config_dir().unwrap_or_else(|| home.join(".config"));

        let os_id = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("ID=").map(|v| v.trim_matches('"').to_string()))
            })
            .unwrap_or_else(|| "unknown".to_string());

        Ok(Self { home, xdg_config, os_id })
    }

    pub fn is_cachyos_or_arch(&self) -> bool {
        matches!(self.os_id.as_str(), "cachyos" | "arch" | "manjaro" | "endeavouros")
    }
}


pub fn cmd_version(name: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(name).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let first = s.lines().next()?.trim().to_string();
    Some(first)
}

/// omp's major version (e.g. `17` from `omp/17.0.6`), or None if omp isn't on PATH.
/// omp ≥17 mounts MCP tools as `xd://` device URLs (`tools.xdev`, default on) and
/// dropped the pre-17 bm25 discovery hop + `mcp.discoveryDefaultServers` key.
pub fn omp_major() -> Option<u32> {
    let v = cmd_version("omp", &["--version"])?; // "omp/17.0.6" (or "omp 17.0.6")
    let digits: String = v
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Detect HyDE-Project setup (hyprland + wallbash theme engine).
pub fn is_hyde() -> bool {
    let home = match dirs::home_dir() { Some(h) => h, None => return false };
    home.join(".config/hyde/wallbash").exists()
        || home.join(".config/hyde").exists() && which::which("hydectl").is_ok()
}

/// True on a tiling Wayland compositor (Hyprland, sway, river, wayfire) that
/// manages its own borders/gaps and expects clients to hide their own chrome.
/// False on a stacking desktop (KDE/kwin, GNOME/mutter, Xfce) where the
/// compositor does NOT draw decorations for kitty either — hiding kitty's own
/// title bar there leaves the window with no title bar, no min/max/close
/// buttons, and no drag-to-resize border at all.
pub fn is_tiling_wm() -> bool {
    if is_hyde() {
        return true;
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    let session = std::env::var("DESKTOP_SESSION").unwrap_or_default().to_lowercase();
    let hay = format!("{desktop} {session}");
    ["hyprland", "sway", "river", "wayfire", "qtile", "i3", "bspwm", "awesome"]
        .iter()
        .any(|wm| hay.contains(wm))
}

/// True when stdin/stdout is a real TTY (so we can prompt y/N).
pub fn has_tty() -> bool {
    // Use the simple `isatty(0)` trick via /proc.
    // unistd::isatty would need a new dep — keep it tiny.
    std::io::IsTerminal::is_terminal(&std::io::stdin())
        && std::io::IsTerminal::is_terminal(&std::io::stdout())
}

/// Return preferred AUR helper on PATH (`paru` > `yay`), or None.
pub fn aur_helper() -> Option<&'static str> {
    if which::which("paru").is_ok() { return Some("paru"); }
    if which::which("yay").is_ok()  { return Some("yay"); }
    None
}

