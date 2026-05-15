use anyhow::Result;
use clap::Args as ClapArgs;
use std::path::{Path, PathBuf};

use crate::{assets, env_detect, ui};

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync skill                       # list all: installed skills/tools/rules + injection model
      8sync skill list                  # same as default
      8sync skill help                  # show usage + how auto inject works
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
        None | Some("list") => list_skills(&env, &skills_toml),
        Some("help") => print_help(&env, &skills_toml),
        Some("add") => add_skill(&skills_toml, a.arg.as_deref()),
        Some("sync") => sync_skills(&env),
        Some(other) => {
            ui::warn(&format!("unknown subcommand: {}", other));
            ui::info("try: 8sync skill help");
            Ok(())
        }
    }
}

fn list_skills(env: &env_detect::Env, toml_path: &Path) -> Result<()> {
    ui::header("8sync skill");

    println!("[config] {}", toml_path.display());
    if toml_path.exists() {
        let s = std::fs::read_to_string(toml_path)?;
        if s.trim().is_empty() {
            println!("  (empty)");
        } else {
            for line in s.lines() {
                println!("  {}", line);
            }
        }
    } else {
        println!("  (missing) — run `8sync setup` first");
    }

    let forge_skills = env.home.join(".forge/skills");
    println!("\n[global installed skills] {}", forge_skills.display());
    let installed = list_installed_skill_files(&forge_skills)?;
    if installed.is_empty() {
        println!("  (none)");
    } else {
        for p in installed {
            println!("  - {}", p.display());
        }
    }

    let force_load = forge_skills.join("00-force-load.md");
    println!("\n[rules: force-load] {}", force_load.display());
    if force_load.exists() {
        println!("  status: present (global auto-inject entrypoint)");
    } else {
        println!("  status: missing (run `8sync skill sync`) ");
    }

    println!("\n[tools bundled by 8sync]");
    println!("  - 8sync shot      (render web/file to PNG)");
    println!("  - 8sync diff-img  (render git diff to PNG)");
    println!("  - 8sync pdf-img   (render PDF pages to PNG)");
    println!("  - 8sync find      (code/file search + picker)");
    println!("  - 8sync note      (append session note)");
    println!("  - 8sync ship      (commit/push/PR shortcut)");

    println!("\n[injection model]");
    println!("  global: ~/.forge/skills/00-force-load.md loads always for every session");
    println!("  local : project AGENTS.md points AI to agents/* memory files");

    if let Some(root) = detect_current_project_root() {
        println!("\n[current project context]");
        println!("  root   : {}", root.display());
        println!("  agents : {}", root.join("agents").display());
        println!("  anchor : {}", root.join("AGENTS.md").display());
    } else {
        println!("\n[current project context]");
        println!("  not inside a detected project root");
    }

    println!("\nUse `8sync skill help` for add/sync and auto-inject details.");
    Ok(())
}

fn add_skill(toml_path: &Path, spec: Option<&str>) -> Result<()> {
    let Some(spec) = spec else {
        ui::err("usage: 8sync skill add <gh:owner/repo|path:/abs|builtin:name>");
        return Ok(());
    };
    if let Some(parent) = toml_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut s = std::fs::read_to_string(toml_path).unwrap_or_default();
    let name = spec.split(['/', ':']).last().unwrap_or("skill");
    s.push_str(&format!("\n[{}]\nsrc = \"{}\"\nwhen = \"always\"\n", name, spec));
    std::fs::write(toml_path, s)?;
    ui::ok(&format!("added '{}' → {}", name, toml_path.display()));
    ui::info("next: 8sync skill sync");
    ui::info("after sync: global force-load is updated for all future sessions");
    Ok(())
}

fn sync_skills(env: &env_detect::Env) -> Result<()> {
    let target = env.home.join(".forge/skills/00-force-load.md");
    std::fs::create_dir_all(target.parent().unwrap())?;
    let content = assets::read("skills/00-force-load.md").unwrap_or_default();
    std::fs::write(&target, content)?;
    ui::ok(&format!("synced master skill list → {}", target.display()));
    ui::info("global inject: every new forge session will read this file first");
    Ok(())
}

fn print_help(env: &env_detect::Env, toml_path: &Path) -> Result<()> {
    ui::header("8sync skill help");
    println!("SYNTAX");
    println!("  8sync skill");
    println!("  8sync skill list");
    println!("  8sync skill help");
    println!("  8sync skill add <gh:owner/repo|path:/abs|builtin:name>");
    println!("  8sync skill sync");

    println!("\nAUTO-INJECT FLOW");
    println!("  1) Add source into {}", toml_path.display());
    println!("  2) Run `8sync skill sync` to write ~/.forge/skills/00-force-load.md");
    println!("  3) New AI sessions read global force-load automatically");
    println!("  4) In each project, `8sync .` seeds AGENTS.md + agents/* for local memory");

    println!("\nPATHS");
    println!("  global skills dir : {}", env.home.join(".forge/skills").display());
    println!("  global rules file : {}", env.home.join(".forge/skills/00-force-load.md").display());
    println!("  config registry   : {}", toml_path.display());

    if let Some(root) = detect_current_project_root() {
        println!("  local project root: {}", root.display());
        println!("  local anchor file : {}", root.join("AGENTS.md").display());
        println!("  local memory dir  : {}", root.join("agents").display());
    }
    Ok(())
}

fn list_installed_skill_files(skills_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !skills_dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            let skill_md = p.join("SKILL.md");
            if skill_md.exists() {
                out.push(skill_md);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn detect_current_project_root() -> Option<PathBuf> {
    let markers = [
        ".git",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "deno.json",
        "go.mod",
    ];
    let mut p = std::env::current_dir().ok()?;
    loop {
        if markers.iter().any(|m| p.join(m).exists()) {
            return Some(p);
        }
        if !p.pop() {
            return None;
        }
    }
}
