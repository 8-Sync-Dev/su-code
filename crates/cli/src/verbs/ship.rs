use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync ship                          stage all, auto-generated commit message, push, open PR
      8sync ship \"feat: dark mode\"        commit with message, push, open PR
      8sync ship \"fix(auth): token expiry off-by-one\"
      8sync ship --no-pr                  commit + push, but do NOT open a PR
      8sync ship --draft \"wip: refactor\"  open a draft PR (review-ready later)

    REQUIREMENTS
      · git must be configured (`git config --global user.email/name`).
      · `gh` (github-cli) must be installed and logged in: `gh auth login`.
      · current directory must be a git repo with a remote.
"})]
pub struct Args {
    /// Commit message. Auto-generated as `chore: 8sync ship` if omitted.
    pub message: Option<String>,
    /// Skip opening a pull request (commit + push only).
    #[arg(long)]
    pub no_pr: bool,
    /// Open the PR as a draft.
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
