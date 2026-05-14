use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync ai                          # resume forge session
          8sync ai \"add dark mode toggle\"   # one-shot prompt
          8sync ai cost                     # token usage today
          8sync ai end                      # close session
    "}
)]
pub struct Args {
    /// Prompt (or special: 'cost', 'end'); empty = resume
    pub rest: Vec<String>,
}

pub fn run(a: Args) -> Result<()> {
    let arg_joined = a.rest.join(" ");
    let trimmed = arg_joined.trim();

    if trimmed == "cost" {
        return show_cost();
    }
    if trimmed == "end" {
        ui::info("Use `8sync end` to capture knowledge & close session.");
        return Ok(());
    }

    if which::which("forge").is_err() {
        ui::err("forge not installed. Run `8sync setup` then `forge login`.");
        return Ok(());
    }

    let status = if trimmed.is_empty() {
        // resume
        ui::info("forge — resume last session");
        Command::new("forge").status()?
    } else {
        // one-shot
        ui::info(&format!("forge — prompt: {}", trimmed));
        Command::new("forge")
            .arg("-p")
            .arg(trimmed)
            .status()?
    };

    if !status.success() {
        ui::warn("forge exited non-zero");
    }
    Ok(())
}

fn show_cost() -> Result<()> {
    // Forge writes usage logs under ~/.forge/; we just shell out to `forge usage` if it exists
    let status = Command::new("forge").arg("usage").status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }
    ui::warn("`forge usage` not available — open forge directly to see token usage");
    Ok(())
}
