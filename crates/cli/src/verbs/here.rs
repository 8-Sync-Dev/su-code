use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

use crate::{env_detect, ui};

pub fn run() -> Result<()> {
    let env = env_detect::Env::detect()?;
    let cwd = std::env::current_dir().context("no cwd")?;
    let root = detect_project_root(&cwd).unwrap_or(cwd.clone());
    ui::header("8sync .");
    ui::info(&format!("project: {}", root.display()));

    // Detect stack
    let stack = detect_stack(&root);
    if !stack.is_empty() {
        ui::ok(&format!("stack: {}", stack.join(", ")));
    }

    // Seed AGENTS.md + .gsd/PROJECT.md if missing
    seed_project_context(&root, &stack)?;

    // Open Kitty layout if in kitty, else just exec forge
    if env.kitty && std::env::var("KITTY_PID").is_ok() {
        open_kitty_layout(&root)?;
    } else {
        ui::info("not in kitty — exec forge directly");
        let _ = Command::new("forge").current_dir(&root).status();
    }

    Ok(())
}

fn detect_project_root(start: &std::path::Path) -> Option<PathBuf> {
    let markers = [".git", "Cargo.toml", "package.json", "pyproject.toml", "deno.json", "go.mod"];
    let mut p = start.to_path_buf();
    loop {
        for m in &markers {
            if p.join(m).exists() {
                return Some(p);
            }
        }
        if !p.pop() {
            return None;
        }
    }
}

fn detect_stack(root: &std::path::Path) -> Vec<String> {
    let mut s = Vec::new();
    if root.join("Cargo.toml").exists() { s.push("rust".into()); }
    if root.join("package.json").exists() { s.push("node".into()); }
    if root.join("next.config.js").exists() || root.join("next.config.ts").exists() || root.join("next.config.mjs").exists() {
        s.push("nextjs".into());
    }
    if root.join("pyproject.toml").exists() { s.push("python".into()); }
    if root.join("src-tauri").exists() || root.join("tauri.conf.json").exists() {
        s.push("tauri".into());
    }
    if root.join("app.json").exists() && root.join("metro.config.js").exists() {
        s.push("react-native".into());
    }
    if root.join("go.mod").exists() { s.push("go".into()); }
    s
}

fn seed_project_context(root: &std::path::Path, stack: &[String]) -> Result<()> {
    let agents = root.join("AGENTS.md");
    if !agents.exists() {
        let content = format!(
            "# Agents guide for {}\n\n\
             Auto-seeded by `8sync .` on first open.\n\n\
             ## Stack\n{}\n\n\
             ## Project state\nSee `.gsd/PROJECT.md`, `.gsd/KNOWLEDGE.md`, `.gsd/DECISIONS.md`.\n\n\
             ## Skill discipline\n\
             - Always read `~/.forge/skills/karpathy-guidelines/SKILL.md` first.\n\
             - Use `~/.forge/skills/image-routing/SKILL.md` to choose image vs text reads.\n\
             - Follow `~/.forge/skills/8sync-conventions/SKILL.md`.\n",
            root.file_name().and_then(|s| s.to_str()).unwrap_or("project"),
            stack.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n"),
        );
        std::fs::write(&agents, content)?;
        ui::ok(&format!("seeded {}", agents.display()));
    }

    let gsd_dir = root.join(".gsd");
    std::fs::create_dir_all(&gsd_dir)?;
    let project_md = gsd_dir.join("PROJECT.md");
    if !project_md.exists() {
        std::fs::write(&project_md, format!(
            "# Project facts\n\n- name: {}\n- stack: {}\n- created_by: 8sync .\n",
            root.file_name().and_then(|s| s.to_str()).unwrap_or("project"),
            stack.join(", ")
        ))?;
        ui::ok(&format!("seeded {}", project_md.display()));
    }
    for f in ["KNOWLEDGE.md", "DECISIONS.md", "PREFERENCES.md", "STATE.md"] {
        let p = gsd_dir.join(f);
        if !p.exists() {
            std::fs::write(&p, format!("# {} (8sync managed)\n\n_empty_\n", f.trim_end_matches(".md")))?;
        }
    }
    Ok(())
}

fn open_kitty_layout(root: &std::path::Path) -> Result<()> {
    // Try kitty remote control to spawn panes; fallback to plain forge.
    // Layout: left big = helix, right = forge, bottom-left = logs/run shell
    let editor = which::which("hx").map(|_| "hx").unwrap_or("vim");
    // Helix pane (left, current window if first call)
    let _ = Command::new("kitty")
        .args(["@", "launch", "--cwd", root.to_str().unwrap(), "--type=tab", "--tab-title=8sync", editor, "."])
        .status();
    let _ = Command::new("kitty")
        .args(["@", "launch", "--cwd", root.to_str().unwrap(), "--location=vsplit", "forge"])
        .status();
    let _ = Command::new("kitty")
        .args(["@", "launch", "--cwd", root.to_str().unwrap(), "--location=hsplit", "fish"])
        .status();
    ui::ok("kitty layout: hx | forge | fish");
    Ok(())
}
