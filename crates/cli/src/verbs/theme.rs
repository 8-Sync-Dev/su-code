//! `8sync theme` — switch the kitty color palette (live, no restart).
//!
//! Curated dark palettes tuned for wallpaper-overlay readability (AA-contrast
//! foreground, high tint). Structure (opacity/blur/splits/font) stays in the
//! managed 8sync.conf; this swaps ONLY the palette fragment
//! (`8sync-theme.conf`), then SIGUSR1-reloads kitty so the change is instant.
//!
//! `hydectl theme set <name>` governs Hyprland/UI; this governs kitty — they
//! are distinct surfaces and do not collide.

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use crate::{assets, env_detect::Env, ui};

const THEMES_PREFIX: &str = "configs/kitty/themes/";
const DEFAULT_THEME: &str = "tokyo-night";

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync theme                 list palettes (★ = active)
          8sync theme list            same as above
          8sync theme set dracula     switch palette + reload kitty (instant)
          8sync theme show            show the active palette
          8sync theme show nord       preview a palette without applying

        NOTES
          Palettes are pure color fragments (background + foreground + 16 ANSI
          colors + cursor + tabs), curated for wallpaper-overlay readability.
          Structure (transparency, blur, splits, font) lives in 8sync.conf and
          is untouched — only colors change.
    "}
)]
pub struct Args {
    /// sub-action: list | set <name> | show [name]. Empty = list.
    pub action: Vec<String>,
}

pub fn run(a: Args) -> Result<()> {
    let env = Env::detect()?;
    let action = a.action.first().map(|s| s.as_str()).unwrap_or("list");
    match action {
        "list" => list(&env),
        "set" => {
            let name = a.action.get(1).map(|s| s.as_str()).unwrap_or("");
            if name.is_empty() {
                bail!("usage: 8sync theme set <name> — see `8sync theme list`");
            }
            set(&env, name)
        }
        "show" => {
            let name = a.action.get(1).map(|s| s.as_str()).unwrap_or("");
            let name = if name.is_empty() { active_name(&env) } else { name.to_string() };
            show(&name)
        }
        other => {
            ui::warn(&format!("unknown action `{other}` — try: list | set <name> | show"));
            Ok(())
        }
    }
}

// ─── public helpers (also used by setup.rs) ─────────────────────────────

/// Embedded theme names, sorted. Sourced from `assets/configs/kitty/themes/*.conf`.
pub(crate) fn list_themes() -> Vec<String> {
    let mut v: Vec<String> = assets::iter_under(THEMES_PREFIX)
        .iter()
        .filter_map(|p| {
            let rel = p.strip_prefix(THEMES_PREFIX)?;
            rel.strip_suffix(".conf").map(|s| s.to_string())
        })
        .collect();
    v.sort();
    v
}

/// Active theme name from `~/.config/8sync/kitty-theme`. Defaults to
/// `tokyo-night` when unset or pointing at a removed palette.
pub(crate) fn active_name(env: &Env) -> String {
    let stored = std::fs::read_to_string(env.xdg_config.join("8sync/kitty-theme"))
        .unwrap_or_default()
        .trim()
        .to_string();
    if list_themes().iter().any(|t| t == &stored) {
        stored
    } else {
        DEFAULT_THEME.to_string()
    }
}

/// Write the active palette to `~/.config/kitty/8sync-theme.conf` and record
/// its name. Called by `8sync setup --profile terminal` so a fresh deploy
/// ships a readable palette alongside the structure config. Returns the name.
pub(crate) fn deploy(env: &Env) -> Result<String> {
    let name = active_name(env);
    let body = assets::read(&format!("{THEMES_PREFIX}{name}.conf"))
        .context("default theme asset missing")?;
    let kitty_dir = env.xdg_config.join("kitty");
    std::fs::create_dir_all(&kitty_dir)?;
    std::fs::write(kitty_dir.join("8sync-theme.conf"), body)?;
    let cfg8 = env.xdg_config.join("8sync");
    std::fs::create_dir_all(&cfg8)?;
    std::fs::write(cfg8.join("kitty-theme"), &name)?;
    Ok(name)
}

// ─── sub-actions ────────────────────────────────────────────────────────

fn list(env: &Env) -> Result<()> {
    let active = active_name(env);
    ui::step("kitty themes (★ = active, curated for wallpaper-overlay readability)");
    for t in list_themes() {
        let mark = if t == active { "★" } else { " " };
        println!("  {mark} {t}");
    }
    println!("\n  switch: 8sync theme set <name>     preview: 8sync theme show <name>");
    Ok(())
}

fn set(env: &Env, name: &str) -> Result<()> {
    let avail = list_themes();
    if !avail.iter().any(|t| t == name) {
        bail!("unknown theme `{name}`. Available: {}", avail.join(", "));
    }
    let body = assets::read(&format!("{THEMES_PREFIX}{name}.conf"))
        .with_context(|| format!("theme asset `{name}` missing"))?;
    let kitty_dir = env.xdg_config.join("kitty");
    std::fs::create_dir_all(&kitty_dir)?;
    std::fs::write(kitty_dir.join("8sync-theme.conf"), body)?;
    let cfg8 = env.xdg_config.join("8sync");
    std::fs::create_dir_all(&cfg8)?;
    std::fs::write(cfg8.join("kitty-theme"), name)?;
    ui::ok(&format!("kitty theme → {name}"));
    reload_kitty();
    Ok(())
}

fn show(name: &str) -> Result<()> {
    let avail = list_themes();
    if !avail.iter().any(|t| t == name) {
        bail!("unknown theme `{name}`. Available: {}", avail.join(", "));
    }
    let body =
        assets::read(&format!("{THEMES_PREFIX}{name}.conf")).context("theme asset missing")?;
    println!("# 8sync kitty theme: {name}\n\n{body}");
    Ok(())
}

/// SIGUSR1 reloads kitty.conf (and its includes) across all kitty windows —
/// instant, no restart, no remote-control socket needed. Graceful if kitty
/// isn't running (the file is written; it applies on next start).
fn reload_kitty() {
    let ok = std::process::Command::new("pkill")
        .args(["-SIGUSR1", "-x", "kitty"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        ui::info("kitty reloaded (SIGUSR1) — palette applied live");
    } else {
        ui::info("kitty not running — palette applies on next start");
    }
}
