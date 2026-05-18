use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync run                  run the default recipe (`dev` for npm/cargo projects)
      8sync run dev              start the dev server  (cargo run | npm/pnpm/bun dev)
      8sync run build            build the project     (cargo build | npm build)
      8sync run test             run the test suite    (cargo test | npm test)
      8sync run fmt              format the code       (cargo fmt | npm run fmt)
      8sync run lint             lint the code         (cargo clippy | npm run lint)
      8sync run \"echo hi\"        no recipe? falls back to plain shell

    PROJECT DETECTION
      · Cargo.toml present  → uses `cargo <recipe>`
      · package.json present→ uses bun > pnpm > npm (whichever is on PATH)
      · neither             → runs the recipe as a shell command
"})]
pub struct Args {
    /// Recipe name: dev | build | test | fmt | lint  (or any custom shell command).
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
