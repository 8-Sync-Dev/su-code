// Environment & system detection
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub struct Env {
    pub home: PathBuf,
    pub xdg_config: PathBuf,
    pub xdg_data: PathBuf,
    pub xdg_state: PathBuf,
    pub os_id: String,
    pub kitty: bool,
}

impl Env {
    pub fn detect() -> Result<Self> {
        let home = dirs::home_dir().context("no HOME")?;
        let xdg_config = dirs::config_dir().unwrap_or_else(|| home.join(".config"));
        let xdg_data = dirs::data_dir().unwrap_or_else(|| home.join(".local/share"));
        let xdg_state = dirs::state_dir().unwrap_or_else(|| home.join(".local/state"));

        let os_id = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("ID=").map(|v| v.trim_matches('"').to_string()))
            })
            .unwrap_or_else(|| "unknown".to_string());

        let kitty = std::env::var("TERM").map(|t| t.contains("kitty")).unwrap_or(false)
            || which::which("kitty").is_ok();

        Ok(Self { home, xdg_config, xdg_data, xdg_state, os_id, kitty })
    }

    pub fn is_cachyos_or_arch(&self) -> bool {
        matches!(self.os_id.as_str(), "cachyos" | "arch" | "manjaro" | "endeavouros")
    }
}

pub fn cmd_exists(name: &str) -> bool {
    which::which(name).is_ok()
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

/// Detect HyDE-Project setup (hyprland + wallbash theme engine).
pub fn is_hyde() -> bool {
    let home = match dirs::home_dir() { Some(h) => h, None => return false };
    home.join(".config/hyde/wallbash").exists()
        || home.join(".config/hyde").exists() && which::which("hydectl").is_ok()
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

/// Kitty `allow_remote_control` enabled? Detected from `kitty.conf` chain.
pub fn kitty_remote_on() -> bool {
    let home = match dirs::home_dir() { Some(h) => h, None => return false };
    let main = home.join(".config/kitty/kitty.conf");
    let candidates: Vec<PathBuf> = vec![
        main.clone(),
        home.join(".config/kitty/hyde.conf"),
    ];
    for p in candidates {
        if let Ok(c) = std::fs::read_to_string(&p) {
            for line in c.lines() {
                let l = line.trim();
                if l.starts_with('#') { continue; }
                if let Some(rest) = l.strip_prefix("allow_remote_control") {
                    let v = rest.trim().to_lowercase();
                    if v.starts_with("yes") || v.starts_with("socket") || v.starts_with("password") {
                        return true;
                    }
                    if v.starts_with("no") { return false; }
                }
            }
        }
    }
    false
}
