use anyhow::Result;
use clap::Args as ClapArgs;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync diff-img                          render the current uncommitted diff
      8sync diff-img HEAD~1                   render diff against the previous commit
      8sync diff-img main..feature            render diff between two refs
      8sync diff-img -o /tmp/review.png       custom output path

    USE CASE
      Turn `git diff` into a PNG so forge can review code changes visually.
      Useful when the diff is too large to fit in the AI context window as text.

    STATUS
      Phase 2 — needs `delta` + terminal-to-png pipeline. Currently a no-op stub.
"})]
pub struct Args {
    /// Optional git revision range (e.g. `HEAD~1`, `main..feature`). Default: working tree vs HEAD.
    pub git_range: Option<String>,
    /// Output PNG path.
    #[arg(short, long, default_value = "/tmp/8sync-diff.png")]
    pub output: String,
}

pub fn run(a: Args) -> Result<()> {
    let _ = a;
    ui::warn("diff-img phase 2 — needs delta + terminal-to-png pipeline");
    Ok(())
}
