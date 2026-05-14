use anyhow::Result;
use clap::Args as ClapArgs;
use std::path::PathBuf;
use std::process::Command;

use crate::{assets, ui};

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync look                    show current preset
      8sync look neon               apply neon_glass (cyber pink/cyan, transparent)
      8sync look ice                apply ice_glass (cool blue-white)
      8sync look mint               apply mint_glass (mint green)
      8sync look dark               apply dark (solid, focus mode)
      8sync look dim                apply dim (mid transparency, neutral)
      8sync look list               list available presets
"})]
pub struct Args {
    pub name: Option<String>,
}

const PRESETS: &[(&str, &str)] = &[
    ("neon", "neon_glass.conf"),
    ("neon_glass", "neon_glass.conf"),
    ("ice", "ice_glass.conf"),
    ("ice_glass", "ice_glass.conf"),
    ("mint", "mint_glass.conf"),
    ("mint_glass", "mint_glass.conf"),
    ("dark", "dark.conf"),
    ("dim", "dim.conf"),
];

pub fn run(a: Args) -> Result<()> {
    match a.name.as_deref() {
        None => show_current(),
        Some("list") => list_presets(),
        Some(name) => apply(name),
    }
}

fn list_presets() -> Result<()> {
    println!("Available presets:");
    for (alias, _) in PRESETS.iter().filter(|(a, _)| !a.contains('_')) {
        println!("  {}", alias);
    }
    Ok(())
}

fn show_current() -> Result<()> {
    let cur = current_preset_marker().unwrap_or_else(|| "(unset)".into());
    println!("current preset: {}", cur);
    println!();
    list_presets()
}

fn apply(name: &str) -> Result<()> {
    let asset_name = PRESETS
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, f)| *f)
        .ok_or_else(|| anyhow::anyhow!("unknown preset: {} (try `8sync look list`)", name))?;

    let preset_content = assets::read(&format!("presets/{}", asset_name))
        .ok_or_else(|| anyhow::anyhow!("preset asset missing: {}", asset_name))?;

    let conf_dir = dirs::config_dir().unwrap_or_default().join("kitty");
    std::fs::create_dir_all(&conf_dir)?;
    let preset_path = conf_dir.join("8sync-preset.conf");
    let marker = format!("# 8sync-look: {}\n", name);
    std::fs::write(&preset_path, format!("{}{}", marker, preset_content))?;

    let main_conf = conf_dir.join("kitty.conf");
    if main_conf.exists() {
        ensure_include(&main_conf, "8sync-preset.conf")?;
    }
    // SIGUSR1 to running kitty triggers reload
    let _ = Command::new("pkill").args(["-SIGUSR1", "kitty"]).status();

    ui::ok(&format!("applied preset `{}` → {}", name, preset_path.display()));
    ui::info("SIGUSR1 sent to running kitty for live reload");
    Ok(())
}

fn ensure_include(main_conf: &PathBuf, fname: &str) -> Result<()> {
    let line = format!("include {}", fname);
    let content = std::fs::read_to_string(main_conf)?;
    if content.lines().any(|l| l.trim() == line) {
        return Ok(());
    }
    let mut new = content;
    if !new.ends_with('\n') {
        new.push('\n');
    }
    new.push_str(&format!("\n# managed by `8sync look`\n{}\n", line));
    std::fs::write(main_conf, new)?;
    Ok(())
}

fn current_preset_marker() -> Option<String> {
    let path = dirs::config_dir()?.join("kitty/8sync-preset.conf");
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .find_map(|l| l.strip_prefix("# 8sync-look: ").map(|s| s.trim().to_string()))
}
