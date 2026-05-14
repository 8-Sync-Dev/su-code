use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync pdf-img doc.pdf
      8sync pdf-img doc.pdf --page 3
      8sync pdf-img doc.pdf -o /tmp/pages
"})]
pub struct Args {
    pub file: String,
    #[arg(long)]
    pub page: Option<u32>,
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
