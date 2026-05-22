use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync ai                                  resume the last omp chat in this project (omp --continue)
          8sync ai \"explain this codebase\"          one-shot prompt — omp replies, you continue
          8sync ai \"add a login form with email + password validation\"
          8sync ai \"refactor src/auth.rs into smaller files\"
          8sync ai \"why does the build fail on macOS?\"

        NOTES
          · omp auto-loads project context from AGENTS.md + agents/* (memory + skills).
          · run inside the project root after `8sync .` for best results.
          · pass the prompt as ONE quoted argument so the shell doesn't split it.
    "}
)]
pub struct Args {
    /// Prompt to send to omp. Empty (or `continue`/`resume`) = resume last session via `omp --continue`.
    pub rest: Vec<String>,
}

pub fn run(a: Args) -> Result<()> {
    let arg_joined = a.rest.join(" ");
    let trimmed = arg_joined.trim();

    if which::which("omp").is_err() {
        ui::err("omp not installed. Run `8sync setup` first.");
        return Ok(());
    }

    let status = if trimmed.is_empty() || trimmed == "continue" || trimmed == "resume" {
        ui::info("omp — continue previous session");
        Command::new("omp").arg("--continue").status()?
    } else {
        ui::info(&format!("omp — prompt: {}", trimmed));
        Command::new("omp").arg("-p").arg(trimmed).status()?
    };

    if !status.success() {
        ui::warn("omp exited non-zero");
    }
    Ok(())
}
