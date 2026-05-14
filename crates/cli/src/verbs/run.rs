use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync run                   # default: dev
      8sync run dev
      8sync run build
      8sync run test
      8sync run fmt
      8sync run lint
"})]
pub struct Args {
    pub script: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    let s = a.script.unwrap_or_else(|| "dev".to_string());
    let root = std::env::current_dir()?;
    // Resolve runner
    if root.join("package.json").exists() {
        let runner = if which::which("bun").is_ok() { "bun" }
                     else if which::which("pnpm").is_ok() { "pnpm" }
                     else { "npm" };
        ui::info(&format!("$ {} {}", runner, s));
        Command::new(runner).arg(s).status()?;
    } else if root.join("Cargo.toml").exists() {
        let cmd = match s.as_str() {
            "dev" | "run" => vec!["run"],
            "build" => vec!["build"],
            "test"  => vec!["test"],
            "fmt"   => vec!["fmt"],
            "lint"  => vec!["clippy"],
            other   => vec![other],
        };
        ui::info(&format!("$ cargo {}", cmd.join(" ")));
        Command::new("cargo").args(&cmd).status()?;
    } else {
        ui::warn("no recipe found (package.json / Cargo.toml). Running shell:");
        Command::new("sh").args(["-c", &s]).status()?;
    }
    Ok(())
}
