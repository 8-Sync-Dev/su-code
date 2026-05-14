use anyhow::Result;
use clap::Args as ClapArgs;
use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync look          # list presets
      8sync look neon
      8sync look ice
      8sync look mint
      8sync look dark
      8sync look dim
"})]
pub struct Args {
    pub name: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    match a.name.as_deref() {
        None | Some("list") => {
            ui::info("Presets: neon | ice | mint | dark | dim");
            ui::info("Phase 1: presets stubbed. `8sync bg <path>` for manual.");
        }
        Some(name) => {
            ui::warn(&format!("preset `{}` not implemented in phase 1", name));
        }
    }
    Ok(())
}
