use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync ship                       # auto-generate msg, commit, push, open PR
      8sync ship \"feat: dark mode\"     # commit + push + PR with message
      8sync ship --no-pr               # commit + push only
"})]
pub struct Args {
    /// Commit message (auto-generated if omitted)
    pub message: Option<String>,
    #[arg(long)]
    pub no_pr: bool,
    #[arg(long)]
    pub draft: bool,
}

pub fn run(a: Args) -> Result<()> {
    ui::header("8sync ship");
    Command::new("git").args(["add", "-A"]).status()?;
    let msg = a.message.unwrap_or_else(|| "chore: 8sync ship".to_string());
    Command::new("git").args(["commit", "-m", &msg]).status()?;
    Command::new("git").args(["push"]).status()?;
    if !a.no_pr && which::which("gh").is_ok() {
        let mut args = vec!["pr", "create", "--fill"];
        if a.draft { args.push("--draft"); }
        Command::new("gh").args(&args).status()?;
    }
    Ok(())
}
