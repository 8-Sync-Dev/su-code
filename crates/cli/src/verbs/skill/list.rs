//! `8sync skill list` + `8sync skill help` output.
use anyhow::Result;
use std::path::Path;

use super::discover::{detect_current_project_root, list_installed_skill_dirs};
use super::meta::meta_for_dir;
use crate::{env_detect, ui};

pub(crate) fn list_skills(env: &env_detect::Env, toml_path: &Path) -> Result<()> {
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
        println!("{}", crate::brand::render("  (missing) — run `8sync setup` first"));
    }

    let omp_skills = env.home.join(".omp/skills");
    println!("\n[global installed skills] {}", omp_skills.display());
    print_skill_list(&omp_skills);

    let force_load = omp_skills.join("00-force-load.md");
    println!("\n[rules: force-load] {}", force_load.display());
    if force_load.exists() {
        println!("  status: present (global auto-inject entrypoint)");
    } else {
        println!("{}", crate::brand::render("  status: missing (run `8sync harness init`)"));
    }

    if let Some(root) = detect_current_project_root() {
        println!("\n[current project context]");
        println!("  root   : {}", root.display());
        println!("  agents : {}", root.join("su-code").display());
        println!("  anchor : {}", root.join("AGENTS.md").display());
        println!("  skills : {}", root.join("su-code/skills").display());
        print_skill_list(&root.join("su-code/skills"));
    } else {
        println!("\n[current project context]");
        println!("  not inside a detected project root");
    }

    println!("\n[injection model]");
    println!("  global : ~/.omp/skills/00-force-load.md loads on every omp session");
    println!("{}", crate::brand::render("  local  : <repo>/AGENTS.md has a `<!-- 8sync:skills:* -->` block listing local skills"));
    println!("  spec   : Agent Skills open standard — each skill dir has `SKILL.md` with YAML frontmatter");

    println!("{}", crate::brand::render("\nUse `8sync skill help` (add/gen) or `8sync harness init` (deploy + force-load)."));
    Ok(())
}

fn print_skill_list(dir: &Path) {
    let dirs = list_installed_skill_dirs(dir).unwrap_or_default();
    if dirs.is_empty() {
        println!("  (none)");
        return;
    }
    for (i, p) in dirs.iter().enumerate() {
        let (m, _entry) = meta_for_dir(p);
        let short = truncate(&m.description, 100);
        println!("  {:>2}. {} — {}", i + 1, m.name, short);
        println!("      {}", p.display());
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

pub(crate) fn print_help(env: &env_detect::Env, toml_path: &Path) -> Result<()> {
    ui::header("8sync skill help");
    println!("SYNTAX");
    println!("{}", crate::brand::render("  8sync skill"));
    println!("{}", crate::brand::render("  8sync skill list"));
    println!("{}", crate::brand::render("  8sync skill help"));
    println!("{}", crate::brand::render("  8sync skill add <https URL|gh:owner/repo|path:/abs|builtin:name>"));
    println!("{}", crate::brand::render("  8sync skill gen <id1> <id2> [id3 …]   # fuse N local skills into one combined SKILL.md"));
    println!("{}", crate::brand::render("  8sync skill update [name]              # re-pull registered skills from skills.toml src"));
    println!("{}", crate::brand::render("  (deploy + force-load + memory + CHANGELOG → `8sync harness init`)"));

    println!("\nSPEC (Agent Skills open standard)");
    println!("  Each skill is a directory containing `SKILL.md` at its root.");
    println!("  SKILL.md MUST start with YAML frontmatter:");
    println!("    ---");
    println!("    name: skill-name          # lowercase, digits, hyphens, ≤64 chars");
    println!("    description: pushy 3rd-person sentence describing what + WHEN to use");
    println!("    ---");
    println!("  Optional subdirs: scripts/, references/, assets/ (progressive disclosure).");
    println!("  Collection repos (skills/<name>/SKILL.md) install every sub-skill.");

    println!("\nAUTO-INJECT FLOW");
    println!("{}", crate::brand::render("  1) `8sync skill add <url>` clones into ~/.omp/skills/<name>/  (global)"));
    println!("     and (if inside a project) <root>/su-code/skills/<name>/    (local)");
    println!("{}", crate::brand::render("  2) <root>/AGENTS.md is rewritten between `<!-- 8sync:skills:* -->` sentinels"));
    println!("     to list every global + local skill with its frontmatter description.");
    println!("  3) omp reads ~/.omp/skills/00-force-load.md + AGENTS.md every session.");

    println!("\nPATHS");
    println!("  global skills dir : {}", env.home.join(".omp/skills").display());
    println!("  global rules file : {}", env.home.join(".omp/skills/00-force-load.md").display());
    println!("  config registry   : {}", toml_path.display());

    if let Some(root) = detect_current_project_root() {
        println!("  local project root: {}", root.display());
        println!("  local anchor file : {}", root.join("AGENTS.md").display());
        println!("  local skills dir  : {}", root.join("su-code/skills").display());
    }
    Ok(())
}
