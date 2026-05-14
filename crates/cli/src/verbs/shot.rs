use anyhow::Result;
use clap::Args as ClapArgs;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync shot http://localhost:3000/dashboard
      8sync shot ./index.html -o /tmp/page.png
"})]
pub struct Args {
    pub target: String,
    #[arg(short, long, default_value = "/tmp/8sync-shot.png")]
    pub output: String,
}

pub fn run(a: Args) -> Result<()> {
    ui::info(&format!("shot {} → {}", a.target, a.output));
    ui::warn("shot uses headless browser — phase 2 (need chromium/firefox headless wrapper)");
    Ok(())
}
