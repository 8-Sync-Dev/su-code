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
