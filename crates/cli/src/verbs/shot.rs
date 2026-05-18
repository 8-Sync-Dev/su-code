use anyhow::Result;
use clap::Args as ClapArgs;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync shot http://localhost:3000/dashboard
      8sync shot http://localhost:5173/login -o /tmp/login.png
      8sync shot ./index.html
      8sync shot ./docs/index.html -o /tmp/docs.png

    USE CASE
      Render a URL or local HTML file to PNG so that forge / claude-code can review the
      visual result cheaply (one image is far fewer tokens than parsing the full DOM).

    STATUS
      Phase 2 — needs a chromium/firefox headless wrapper. Currently a no-op stub.
"})]
pub struct Args {
    /// URL or local HTML file to render.
    pub target: String,
    /// Output PNG path.
    #[arg(short, long, default_value = "/tmp/8sync-shot.png")]
    pub output: String,
}

pub fn run(a: Args) -> Result<()> {
    ui::info(&format!("shot {} → {}", a.target, a.output));
    ui::warn("shot uses headless browser — phase 2 (need chromium/firefox headless wrapper)");
    Ok(())
}
