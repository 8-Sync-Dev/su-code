use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{env_detect, ui};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync .                      attach hoặc tạo session ở project hiện tại
          8sync . ls                   liệt kê các session đang sống (abduco -l)
          8sync . to other-project     chuyển sang attach session khác
          8sync . new bg-fix forge     tạo session detached mới tên `bg-fix`
          8sync . rm bg-fix            kill + xoá session
          8sync . mv old new           đổi tên session
          8sync . wipe                 kill toàn bộ session của project hiện tại
          8sync . kick                 detach mọi attach hiện tại (để máy khác attach)
    "}
)]
pub struct Args {
    /// Subcommand: ls | to | new | rm | mv | wipe | kick (mặc định: attach/create)
    pub action: Option<String>,
    /// Tham số phụ (tên session, lệnh, target...)
    pub rest: Vec<String>,
}

pub fn run(args: Args) -> Result<()> {
    match args.action.as_deref() {
        None => open_or_attach(),
        Some("ls") => list_sessions(),
        Some("to") => switch_to(args.rest.first().cloned()),
        Some("new") => new_session(args.rest),
        Some("rm") => rm_session(args.rest.first().cloned()),
        Some("mv") => mv_session(args.rest.first().cloned(), args.rest.get(1).cloned()),
        Some("wipe") => wipe_project(),
        Some("kick") => kick_detach(args.rest.first().cloned()),
        Some(other) => {
            ui::warn(&format!("unknown action `{}` — try `8sync . -h`", other));
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// default: 8sync .
// ─────────────────────────────────────────────────────────────────
fn open_or_attach() -> Result<()> {
    let env = env_detect::Env::detect()?;
    let cwd = std::env::current_dir().context("no cwd")?;
    let root = detect_project_root(&cwd).unwrap_or(cwd.clone());
    let session_name = project_session_name(&root);

    ui::header("8sync .");
    ui::info(&format!("project: {}", root.display()));
    ui::info(&format!("session: {}", session_name));

    let stack = detect_stack(&root);
    if !stack.is_empty() {
        ui::ok(&format!("stack: {}", stack.join(", ")));
    }

    seed_project_context(&root, &stack)?;

    let has_abduco = which::which("abduco").is_ok();
    let has_forge = which::which("forge").is_ok();

    if !has_forge {
        ui::warn("forge not installed — run `8sync setup` first. Falling back to fish shell.");
    }

    // Open Kitty layout (3 panes) if running inside kitty
    let in_kitty = env.kitty && std::env::var("KITTY_PID").is_ok();
    if in_kitty {
        open_kitty_layout(&root, &session_name, has_abduco, has_forge)?;
    } else {
        // No kitty → just attach/create in current terminal
        exec_forge_in_session(&root, &session_name, has_abduco, has_forge)?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// 8sync . ls
// ─────────────────────────────────────────────────────────────────
fn list_sessions() -> Result<()> {
    ui::header("8sync . ls");
    if which::which("abduco").is_err() {
        ui::warn("abduco missing — run `8sync setup` to install");
        return Ok(());
    }
    let out = Command::new("abduco").output()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let mut found = false;
    for line in s.lines() {
        if line.contains("8sync-") {
            println!("  {}", line);
            found = true;
        }
    }
    if !found {
        ui::info("no 8sync sessions");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// 8sync . to <name>
// ─────────────────────────────────────────────────────────────────
fn switch_to(name: Option<String>) -> Result<()> {
    let n = name.ok_or_else(|| anyhow::anyhow!("usage: 8sync . to <name>"))?;
    let full = if n.starts_with("8sync-") { n } else { format!("8sync-{}", n) };
    ui::info(&format!("attaching → {}", full));
    Command::new("abduco").args(["-a", &full]).status()?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// 8sync . new <name> [cmd...]
// ─────────────────────────────────────────────────────────────────
fn new_session(rest: Vec<String>) -> Result<()> {
    if rest.is_empty() {
        ui::warn("usage: 8sync . new <name> [command...]");
        return Ok(());
    }
    let name = format!("8sync-{}", rest[0]);
    let cmd: Vec<&str> = if rest.len() > 1 {
        rest[1..].iter().map(|s| s.as_str()).collect()
    } else {
        vec!["forge"]
    };
    ui::info(&format!("create detached → {}", name));
    let mut a = Command::new("abduco");
    a.args(["-A", &name]);
    for c in &cmd {
        a.arg(c);
    }
    a.status()?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// 8sync . rm <name>
// ─────────────────────────────────────────────────────────────────
fn rm_session(name: Option<String>) -> Result<()> {
    let n = name.ok_or_else(|| anyhow::anyhow!("usage: 8sync . rm <name>"))?;
    let full = if n.starts_with("8sync-") { n } else { format!("8sync-{}", n) };
    let _ = Command::new("pkill")
        .args(["-f", &format!("abduco.*{}", full)])
        .status();
    ui::ok(&format!("killed {}", full));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// 8sync . mv <old> <new>
// ─────────────────────────────────────────────────────────────────
fn mv_session(old: Option<String>, new: Option<String>) -> Result<()> {
    let _o = old.ok_or_else(|| anyhow::anyhow!("usage: 8sync . mv <old> <new>"))?;
    let _n = new.ok_or_else(|| anyhow::anyhow!("usage: 8sync . mv <old> <new>"))?;
    // abduco doesn't support rename — workaround: kill old, attach new with same cmd
    ui::warn("abduco không hỗ trợ rename trực tiếp.");
    ui::info("Cách thủ công: `8sync . rm <old>` rồi `8sync . new <new> [cmd]`");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// 8sync . wipe
// ─────────────────────────────────────────────────────────────────
fn wipe_project() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = detect_project_root(&cwd).unwrap_or(cwd);
    let name = project_session_name(&root);
    let _ = Command::new("pkill")
        .args(["-f", &format!("abduco.*{}", name)])
        .status();
    ui::ok(&format!("wiped sessions matching {}", name));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// 8sync . kick [name]
// ─────────────────────────────────────────────────────────────────
fn kick_detach(name: Option<String>) -> Result<()> {
    let n = match name {
        Some(v) => v,
        None => {
            let cwd = std::env::current_dir()?;
            let root = detect_project_root(&cwd).unwrap_or(cwd);
            project_session_name(&root)
        }
    };
    let full = if n.starts_with("8sync-") { n } else { format!("8sync-{}", n) };
    // abduco sends SIGHUP to attached clients via -k (varies by version);
    // safer: pkill the attached abduco -a client
    let _ = Command::new("pkill")
        .args(["-f", &format!("abduco -a.*{}", full)])
        .status();
    ui::ok(&format!("kicked attached clients of {}", full));
    Ok(())
}

// ═════════════════════════════════════════════════════════════════
// helpers
// ═════════════════════════════════════════════════════════════════

pub fn project_session_name(root: &Path) -> String {
    let base = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("proj")
        .replace(['/', ' '], "_");
    // include a short hash of the full path so two folders with same name don't collide
    let h = short_hash(root.to_string_lossy().as_bytes());
    format!("8sync-{}-{}", base, h)
}

fn short_hash(bytes: &[u8]) -> String {
    // dependency-free FNV-1a 32-bit
    let mut h: u32 = 0x811c9dc5;
    for b in bytes {
        h ^= *b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    format!("{:08x}", h)
}

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

fn seed_project_context(root: &Path, stack: &[String]) -> Result<()> {
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

> Managed by **8sync**. AI tooling (forge, claude-code, cursor, opencode) MUST
> read this file at the start of every session.

## ⛔ FORCE-LOAD SKILLS (đọc theo thứ tự, không bỏ qua)

1. **`~/.forge/skills/karpathy-guidelines/SKILL.md`** — kỷ luật suy nghĩ.
2. **`~/.forge/skills/8sync-cli/SKILL.md`** — bạn đang chạy trong 8sync harness,
   dùng đúng các tool 8sync (shot/find/note/ship/diff-img/pdf-img/...).
3. **`~/.forge/skills/image-routing/SKILL.md`** — chọn đọc image hay text để
   tiết kiệm token.

Sau đó đọc memory project (mục dưới).

## Stack (auto-detected)
{stack_lines}

## Project memory (đọc TRƯỚC khi bắt đầu bất kỳ task)

| File | Mục đích |
|---|---|
| `agents/PROJECT.md`     | facts cố định (stack, entrypoint, conventions) |
| `agents/KNOWLEDGE.md`   | append-only: AI học được gì về codebase |
| `agents/DECISIONS.md`   | append-only: quyết định kiến trúc |
| `agents/PREFERENCES.md` | append-only: user style preferences |
| `agents/STATE.md`       | việc đang dở, next-step concrete |

**KHÔNG modify `agents/*.md` trực tiếp.** Chỉ append qua `8sync end` capture format
(xem `~/.forge/skills/8sync-cli/SKILL.md` mục 4).

## Conventions

- Cite code dạng `path/to/file.rs:23-58` hoặc `file.rs:23`.
- Commit + push + PR qua `8sync ship "msg"` (không git push thô).
- Screenshot UI / PDF / diff: ưu tiên `8sync shot|pdf-img|diff-img` thay vì
  dump text (tiết kiệm token 3-10×).
- Tìm symbol/file: `8sync find <kw>` (không gọi `rg`/`fd` thô).
- Ghi nhớ ý tưởng nhanh: `8sync note "..."` (append vào `agents/NOTES.md`).

## Session boundary

- `8sync .` = session bắt đầu → AI đọc tất cả file trên.
- `8sync end` = session kết thúc → AI output 4 block `<DECISIONS>`,
  `<KNOWLEDGE>`, `<PREFERENCES>`, `<STATE>` để 8sync append vào `agents/*.md`.
"#
        );
        std::fs::write(&agents, content)?;
        ui::ok(&format!("seeded {}", agents.display()));
    }

    let agents_dir = root.join("agents");
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
    Ok(())
}

fn open_kitty_layout(
    root: &Path,
    session_name: &str,
    has_abduco: bool,
    has_forge: bool,
) -> Result<()> {
    let editor = if which::which("hx").is_ok() { "hx" } else if which::which("helix").is_ok() { "helix" } else { "fish" };

    // Pane 1: editor (current tab → new tab so the chat session is preserved)
    let _ = Command::new("kitty")
        .args([
            "@", "launch",
            "--cwd", root.to_str().unwrap(),
            "--type=tab",
            "--tab-title=8sync",
            editor, ".",
        ])
        .status();

    // Pane 2: forge inside abduco (detached, survives close)
    let forge_cmd = forge_invocation(root, session_name, has_abduco, has_forge);
    let _ = Command::new("kitty")
        .args([
            "@", "launch",
            "--cwd", root.to_str().unwrap(),
            "--location=vsplit",
            "fish", "-c", &forge_cmd,
        ])
        .status();

    // Pane 3: fish for logs / run
    let _ = Command::new("kitty")
        .args([
            "@", "launch",
            "--cwd", root.to_str().unwrap(),
            "--location=hsplit",
            "fish",
        ])
        .status();

    let bg = if has_abduco { " (detached via abduco)" } else { " (no abduco — exec direct)" };
    ui::ok(&format!("kitty layout: {} | forge{} | fish", editor, bg));
    Ok(())
}

fn exec_forge_in_session(
    root: &Path,
    session_name: &str,
    has_abduco: bool,
    has_forge: bool,
) -> Result<()> {
    let cmd = forge_invocation(root, session_name, has_abduco, has_forge);
    Command::new("fish").arg("-c").arg(&cmd).current_dir(root).status()?;
    Ok(())
}

fn forge_invocation(root: &Path, session_name: &str, has_abduco: bool, has_forge: bool) -> String {
    let inner = if has_forge { "forge" } else { "fish" };
    let cd = format!("cd {}", shell_quote(root.to_str().unwrap()));
    if has_abduco {
        // abduco -A name cmd → attach if exists, else create
        format!("{cd}; and abduco -A {session_name} {inner}")
    } else {
        format!("{cd}; and {inner}")
    }
}

fn shell_quote(s: &str) -> String {
    let mut out = String::from("'");
    for c in s.chars() {
        if c == '\'' {
            out.push_str(r"'\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}
