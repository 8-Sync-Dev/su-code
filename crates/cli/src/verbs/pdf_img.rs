use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync pdf-img doc.pdf                       render every page → /tmp/8sync-pdf/page-*.png
      8sync pdf-img doc.pdf --page 3              render only page 3
      8sync pdf-img spec.pdf -o /tmp/spec         custom output directory
      8sync pdf-img book.pdf --page 12 -o /tmp/p  render one page to a custom dir

    USE CASE
      Convert PDF pages to PNGs so forge can read figures, diagrams, scanned docs
      etc. visually — much cheaper than OCR + sending raw text.

    REQUIREMENTS
      · `pdftocairo` from `poppler` (`pacman -S poppler`).
        poppler is usually already installed on HyDE/CachyOS as a dependency.
"})]
pub struct Args {
    /// Path to the source PDF.
    pub file: String,
    /// Render only one page (1-based).
    #[arg(long)]
    pub page: Option<u32>,
    /// Output directory (will be created).
    #[arg(short, long, default_value = "/tmp/8sync-pdf")]
    pub output: String,
}

pub fn run(a: Args) -> Result<()> {
    if which::which("pdftocairo").is_err() {
        ui::err("pdftocairo missing — install poppler (`pacman -S poppler`)");
        return Ok(());
    }
    std::fs::create_dir_all(&a.output)?;
    let mut args = vec!["-png".to_string(), "-r".into(), "150".into()];
    if let Some(p) = a.page {
        args.push("-f".into()); args.push(p.to_string());
        args.push("-l".into()); args.push(p.to_string());
    }
    args.push(a.file.clone());
    args.push(format!("{}/page", a.output));
    let status = Command::new("pdftocairo").args(&args).status()?;
    if status.success() {
        ui::ok(&format!("PDF rendered → {}/page-*.png", a.output));
    }
    Ok(())
}
