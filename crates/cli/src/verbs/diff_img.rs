use anyhow::Result;
use clap::Args as ClapArgs;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync diff-img
      8sync diff-img HEAD~1
      8sync diff-img main..feature -o /tmp/diff.png
"})]
pub struct Args {
    pub git_range: Option<String>,
    #[arg(short, long, default_value = "/tmp/8sync-diff.png")]
    pub output: String,
}

pub fn run(a: Args) -> Result<()> {
    let _ = a;
    ui::warn("diff-img phase 2 — needs delta + terminal-to-png pipeline");
    Ok(())
}
