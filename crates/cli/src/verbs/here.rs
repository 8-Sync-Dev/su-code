use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{env_detect, ui, verbs::skill};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync .                       seed su-code/* context for the current project and run `omp --continue`

        BEHAVIOR
          · Walks up from cwd to find the project root (.git / Cargo.toml / package.json / pyproject.toml / go.mod / deno.json).
          · Detects stack (rust/node/python/nextjs/tauri/react-native/go) and seeds AGENTS.md + su-code/{PROJECT,KNOWLEDGE,DECISIONS,PREFERENCES,STATE,NOTES}.md when missing.
          · Re-injects the dynamic skills block in AGENTS.md so omp sees up-to-date skill list.
          · Execs `omp --continue` in the project root. Session lifetime is owned by omp (retain/recall/auto-compact); 8sync no longer manages abduco sockets or kitty panes.
          · If omp is missing, drops into the user shell instead (run `8sync setup` to fix).
    "}
)]
pub struct Args {}

pub fn run(_args: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    let cwd = std::env::current_dir().context("no cwd")?;
    let root = detect_project_root(&cwd).unwrap_or(cwd.clone());

    ui::header("8sync .");
    ui::info(&format!("project: {}", root.display()));

    let stack = detect_stack(&root);
    if !stack.is_empty() {
        ui::ok(&format!("stack: {}", stack.join(", ")));
    }

    seed_project_context(&env, &root, &stack)?;

    if which::which("omp").is_ok() {
        ui::ok("→ exec: omp --continue");
        let cfg = crate::models::ModelConfig::load();
        let err = Command::new("omp").args(cfg.resume_flags()).arg("--continue").current_dir(&root).status();
        match err {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(anyhow::anyhow!("omp exited with {}", s)),
            Err(e) => Err(anyhow::anyhow!("could not exec omp: {}", e)),
        }
    } else {
        ui::warn("omp not installed — run `8sync setup` first. Falling back to $SHELL.");
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let _ = Command::new(&shell).current_dir(&root).status();
        Ok(())
    }
}

// ═════════════════════════════════════════════════════════════════
// helpers
// ═════════════════════════════════════════════════════════════════

fn detect_project_root(start: &Path) -> Option<PathBuf> {
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

fn detect_stack(root: &Path) -> Vec<String> {
    let mut s = Vec::new();
    if root.join("Cargo.toml").exists() { s.push("rust".into()); }
    if root.join("package.json").exists() { s.push("node".into()); }
    if root.join("next.config.js").exists()
        || root.join("next.config.ts").exists()
        || root.join("next.config.mjs").exists()
    {
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

fn seed_project_context(env: &env_detect::Env, root: &Path, stack: &[String]) -> Result<()> {
    let _ = crate::verbs::harness::memory::migrate_legacy_layout(root);
    let agents = root.join("AGENTS.md");
    if !agents.exists() {
        let name = root.file_name().and_then(|s| s.to_str()).unwrap_or("project");
        let stack_lines = if stack.is_empty() {
            "- (auto-detect failed, please fill in)".to_string()
        } else {
            stack.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n")
        };
        let content = format!(
            r#"# AGENTS.md — guidance for AI working in `{name}`

> Managed by **8sync**. AI tooling (omp, claude-code, cursor, opencode) MUST
> read this file at the start of every session.

<!-- 8sync:skills:begin -->
<!-- 8sync:skills:end -->

## Stack (auto-detected)
{stack_lines}

## Project memory (đọc TRƯỚC khi bắt đầu bất kỳ task)

| File | Mục đích |
|---|---|
| `su-code/PROJECT.md`     | facts cố định (stack, entrypoint, conventions) |
| `su-code/KNOWLEDGE.md`   | append-only: AI học được gì về codebase |
| `su-code/DECISIONS.md`   | append-only: quyết định kiến trúc |
| `su-code/PREFERENCES.md` | append-only: user style preferences |
| `su-code/STATE.md`       | việc đang dở, next-step concrete |
| `su-code/NOTES.md`       | quick notes appended via `8sync note` |

Session memory được omp tự quản (retain/recall/auto-compact). Không cần capture tay.

## Conventions

- Cite code dạng `path/to/file.rs:23-58` hoặc `file.rs:23`.
- Commit + push + PR qua `8sync ship "msg"` (không git push thô).
- Screenshot UI / PDF / diff: ưu tiên `8sync shot|pdf-img|diff-img` thay vì
  dump text (tiết kiệm token 3-10×).
- Tìm symbol/file: `8sync find <kw>` (không gọi `rg`/`fd` thô).
- Ghi nhớ ý tưởng nhanh: `8sync note "..."` (append vào `su-code/NOTES.md`).
"#
        );
        std::fs::write(&agents, content)?;
        ui::ok(&format!("seeded {}", agents.display()));
    }

    let agents_dir = root.join("su-code");
    std::fs::create_dir_all(&agents_dir)?;
    let project_md = agents_dir.join("PROJECT.md");
    if !project_md.exists() {
        std::fs::write(
            &project_md,
            format!(
                "# Project facts\n\n- name: {}\n- stack: {}\n- created_by: 8sync .\n",
                root.file_name().and_then(|s| s.to_str()).unwrap_or("project"),
                stack.join(", ")
            ),
        )?;
        ui::ok(&format!("seeded {}", project_md.display()));
    }
    for f in ["KNOWLEDGE.md", "DECISIONS.md", "PREFERENCES.md", "STATE.md", "NOTES.md"] {
        let p = agents_dir.join(f);
        if !p.exists() {
            std::fs::write(
                &p,
                format!("# {} (8sync managed — append-only)\n\n_empty_\n", f.trim_end_matches(".md")),
            )?;
        }
    }

    // Re-inject the dynamic skills block (root) + a compact index into every
    // significant sub-folder, so an agent opening any sub-tree still sees the
    // skill rules (progressive disclosure: nearest AGENTS.md wins).
    if let Err(e) = skill::inject_agents_md(&env.home, root) {
        ui::warn(&format!("could not inject AGENTS.md skills block: {}", e));
    }
    if let Err(e) = skill::inject_subfolder_indexes(root) {
        ui::warn(&format!("could not inject sub-folder skill indexes: {}", e));
    }
    Ok(())
}
