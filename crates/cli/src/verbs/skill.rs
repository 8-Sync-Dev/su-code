use anyhow::Result;
use clap::Args as ClapArgs;
use crate::{assets, env_detect, ui};

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync skill                       # list skills + status
      8sync skill add gh:owner/repo
      8sync skill add path:/path/to/skill
      8sync skill sync                  # re-sync ~/.forge/skills/00-force-load.md
"})]
pub struct Args {
    pub sub: Option<String>,
    pub arg: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    let skills_toml = env.xdg_config.join("8sync/skills.toml");
    match a.sub.as_deref() {
        None | Some("list") => list_skills(&skills_toml),
        Some("add") => add_skill(&skills_toml, a.arg.as_deref()),
        Some("sync") => sync_skills(&env),
        Some(other) => {
            ui::warn(&format!("unknown subcommand: {}", other));
            Ok(())
        }
    }
}

fn list_skills(toml_path: &std::path::Path) -> Result<()> {
    if !toml_path.exists() {
        ui::warn(&format!("no {} — run `8sync setup`", toml_path.display()));
        return Ok(());
    }
    let s = std::fs::read_to_string(toml_path)?;
    println!("{}", s);
    Ok(())
}

fn add_skill(toml_path: &std::path::Path, spec: Option<&str>) -> Result<()> {
    let Some(spec) = spec else {
        ui::err("usage: 8sync skill add <gh:owner/repo|path:/abs|builtin:name>");
        return Ok(());
    };
    let mut s = std::fs::read_to_string(toml_path).unwrap_or_default();
    let name = spec.split(['/', ':']).last().unwrap_or("skill");
    s.push_str(&format!("\n[{}]\nsrc = \"{}\"\nwhen = \"always\"\n", name, spec));
    std::fs::write(toml_path, s)?;
    ui::ok(&format!("added '{}' → {}", name, toml_path.display()));
    ui::info("Now run: 8sync skill sync");
    Ok(())
}

fn sync_skills(env: &env_detect::Env) -> Result<()> {
    let target = env.home.join(".forge/skills/00-force-load.md");
    std::fs::create_dir_all(target.parent().unwrap())?;
    let content = assets::read("skills/00-force-load.md").unwrap_or_default();
    std::fs::write(&target, content)?;
    ui::ok(&format!("synced master skill list → {}", target.display()));
    Ok(())
}
