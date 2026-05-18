use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync ai                                  resume the last forge chat in this project
          8sync ai \"explain this codebase\"          one-shot prompt — forge replies, you continue
          8sync ai \"add a login form with email + password validation\"
          8sync ai \"refactor src/auth.rs into smaller files\"
          8sync ai \"why does the build fail on macOS?\"
          8sync ai cost                             show today's token usage (calls `forge usage`)
          8sync ai end                              hint — use `8sync end` to capture knowledge

        NOTES
          · forge auto-loads project context from AGENTS.md + agents/* (memory files).
          · run inside the project root after `8sync .` for best results.
          · pass the prompt as ONE quoted argument so the shell doesn't split it.
    "}
)]
pub struct Args {
    /// Prompt to send to forge. Special words: `cost`, `end`. Empty = resume last session.
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
