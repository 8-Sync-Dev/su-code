use anyhow::Result;
use crate::{env_detect, pkg, ui};

pub fn run() -> Result<()> {
    ui::header("8sync doctor");
    let env = env_detect::Env::detect()?;

    check("OS",       &format!("{} (kitty: {})", env.os_id, env.kitty));
    check_cmd("kitty",      &["--version"]);
    check_cmd_any(&[("helix", &["--version"]), ("hx", &["--version"])], "helix");
    check_cmd("fish",       &["--version"]);
    check_cmd("git",        &["--version"]);
    check_cmd("gh",         &["--version"]);
    check_cmd("lazygit",    &["--version"]);
    check_cmd("docker",     &["--version"]);
    check_cmd("node",       &["--version"]);
    check_cmd("pnpm",       &["--version"]);
    check_cmd("bun",        &["--version"]);
    check_cmd("uv",         &["--version"]);
    check_cmd("forge",      &["--version"]);
    check_cmd("warp-cli",   &["--version"]);
    check_cmd("ufw",        &["--version"]);

    // gh auth
    if let Some(out) = env_detect::cmd_version("gh", &["auth", "status"]) {
        ui::info(&format!("gh: {}", out));
    }

    // Configs present?
    for path in [
        env.xdg_config.join("kitty/kitty.conf"),
        env.xdg_config.join("helix/config.toml"),
        env.xdg_config.join("fish/conf.d/8sync.fish"),
        env.xdg_config.join("8sync/global.toml"),
        env.xdg_config.join("8sync/skills.toml"),
        env.home.join(".forge/skills/00-force-load.md"),
    ] {
        if path.exists() {
            ui::ok(&format!("{}", path.display()));
        } else {
            ui::warn(&format!("missing: {}", path.display()));
        }
    }

    // Wallpaper
    let wp = env.xdg_data.join("8sync/wallpapers/default.jpg");
    if wp.exists() {
        ui::ok(&format!("wallpaper: {}", wp.display()));
    } else {
        ui::warn(&format!("missing wallpaper: {}", wp.display()));
    }

    // WARP status
    if let Some(s) = env_detect::cmd_version("warp-cli", &["status"]) {
        ui::info(&format!("warp: {}", s));
    }

    // MCP service
    let mcp_active = std::process::Command::new("systemctl")
        .args(["--user", "is-active", "8sync-mcp.service"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if mcp_active { ui::ok("8sync-mcp user service: active"); }
    else { ui::warn("8sync-mcp user service: inactive"); }

    // Suppress unused warning for pkg
    let _ = pkg::ver_at_least;
    Ok(())
}

fn check(label: &str, value: &str) {
    ui::ok(&format!("{}: {}", label, value));
}

fn check_cmd(name: &str, args: &[&str]) {
    match env_detect::cmd_version(name, args) {
        Some(v) => ui::ok(&format!("{}: {}", name, v)),
        None => ui::warn(&format!("{}: missing or broken", name)),
    }
}

fn check_cmd_any(candidates: &[(&str, &[&str])], label: &str) {
    for (name, args) in candidates {
        if let Some(v) = env_detect::cmd_version(name, args) {
            ui::ok(&format!("{} ({}): {}", label, name, v));
            return;
        }
    }
    ui::warn(&format!("{}: missing or broken", label));
}
