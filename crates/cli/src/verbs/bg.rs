use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync bg                       # show current
      8sync bg cyberpunk city        # search & auto-set top result
      8sync bg pick                  # picker from cache
      8sync bg /path/to/img.jpg
      8sync bg https://example/x.jpg
      8sync bg 0.7                   # set opacity 0.7
      8sync bg +                     # nudge opacity +0.05
      8sync bg -                     # nudge opacity -0.05
      8sync bg off                   # remove image
"})]
pub struct Args {
    pub rest: Vec<String>,
}

pub fn run(a: Args) -> Result<()> {
    let joined = a.rest.join(" ");
    let trimmed = joined.trim();

    // opacity
    if let Ok(v) = trimmed.parse::<f32>() {
        return set_opacity(v);
    }
    if trimmed == "+" { return nudge_opacity(0.05); }
    if trimmed == "-" { return nudge_opacity(-0.05); }
    if trimmed == "off" { return clear_bg(); }
    if trimmed.is_empty() { return show_status(); }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        ui::info("URL set not implemented in phase 1; download then set path");
        return Ok(());
    }
    let path = std::path::Path::new(trimmed);
    if path.exists() {
        return set_bg(path);
    }

    ui::warn(&format!("bg search not implemented in phase 1: '{}'", trimmed));
    ui::info("Manually: 8sync bg /path/to/image.jpg");
    Ok(())
}

fn set_opacity(v: f32) -> Result<()> {
    let clamped = v.clamp(0.0, 1.0);
    Command::new("kitty")
        .args(["@", "set-background-opacity", &format!("{:.2}", clamped)])
        .status()?;
    ui::ok(&format!("kitty opacity = {:.2}", clamped));
    Ok(())
}

fn nudge_opacity(_d: f32) -> Result<()> {
    ui::warn("read-current opacity not implemented; pass explicit value (e.g. `8sync bg 0.85`)");
    Ok(())
}

fn set_bg(path: &std::path::Path) -> Result<()> {
    Command::new("kitty")
        .args(["@", "set-background-image", path.to_str().unwrap()])
        .status()?;
    ui::ok(&format!("kitty bg ← {}", path.display()));
    Ok(())
}

fn clear_bg() -> Result<()> {
    Command::new("kitty")
        .args(["@", "set-background-image", "none"])
        .status()?;
    ui::ok("kitty bg cleared");
    Ok(())
}

fn show_status() -> Result<()> {
    let home = dirs::home_dir().unwrap();
    let wp = home.join(".local/share/8sync/wallpapers/default.jpg");
    if wp.exists() {
        ui::info(&format!("default wallpaper: {}", wp.display()));
    } else {
        ui::warn("no default wallpaper installed — run `8sync setup`");
    }
    println!("Try: 8sync bg <path|url|keywords|0..1|+|-|off>");
    Ok(())
}
