//! Force-load block injection into root agent entry files (AGENTS.md, CLAUDE.md,
//! …) + always-on ranking + project-tech gating for conditional skills.
use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::discover::list_installed_skill_dirs;
use super::meta::meta_for_dir;
use crate::ui;

pub(crate) const BEGIN: &str = "<!-- 8sync:skills:begin -->";
pub(crate) const END: &str = "<!-- 8sync:skills:end -->";

/// Find the byte offset of `needle` only when it occurs as the start of a line
/// (offset 0, or the byte immediately before is '\n'). Returns the offset of
/// the needle itself (not the newline).
fn find_line(hay: &str, needle: &str) -> Option<usize> {
    let bytes = hay.as_bytes();
    let mut start = 0;
    while let Some(rel) = hay[start..].find(needle) {
        let pos = start + rel;
        if pos == 0 || bytes[pos - 1] == b'\n' {
            return Some(pos);
        }
        start = pos + needle.len();
    }
    None
}

/// Priority rank for always-on skills (lower = read earlier). The force-load
/// block lists always-on skills in EXACTLY this order; agents read top-down so
/// position == priority. The canonical chain is:
///   codegraph → karpathy → ponytail → assp → impeccable → taste → 8sync-cli → image-routing
/// codegraph (code intel), karpathy (engineering discipline) and ponytail
/// (lazy-senior YAGNI) form the engineering core; the brand + frontend-design
/// skills follow; the harness/token-routing tooling closes. Skills not listed
/// here are on-demand (read only when the task matches their description) and
/// rank `usize::MAX`.
pub(crate) fn always_on_rank(dirname: &str) -> usize {
    match dirname {
        "codegraph" => 0,
        "karpathy-guidelines" => 1,
        "ponytail" => 2,
        "assp-skill" => 3,
        "impeccable" => 4,
        "taste-skill" => 5,
        "8sync-cli" => 6,
        "image-routing" => 7,
        _ => usize::MAX,
    }
}

/// Always-on skills are read on EVERY session before the first tool call.
pub(crate) fn is_always_on(dirname: &str) -> bool {
    always_on_rank(dirname) != usize::MAX
}

/// On-demand skills gated by a project-tech predicate: only surfaced in the
/// force-load block when the tech is actually present (keeps a non-Encore
/// project from listing the Encore deploy runbook). Returns true = hide it.
fn skill_gated_out(dirname: &str, root: &Path) -> bool {
    match dirname {
        "encore-deploy" => !project_uses_encore(root),
        _ => false,
    }
}

/// Detect whether the project uses the Encore.dev framework.
fn project_uses_encore(root: &Path) -> bool {
    if root.join("encore.app").is_file() {
        return true;
    }
    for sub in ["backend", "api", "server"] {
        if root.join(sub).join("encore.app").is_file() {
            return true;
        }
    }
    for f in ["go.mod", "package.json"] {
        if let Ok(s) = std::fs::read_to_string(root.join(f)) {
            if s.contains("encore.dev") || s.contains("encore.app") {
                return true;
            }
        }
    }
    false
}

/// Rewrite (or insert) the force-load block in **every** agent entry file at the
/// project root: AGENTS.md, CLAUDE.md, GEMINI.md, OPENCODE.md,
/// .github/copilot-instructions.md, .cursorrules, .windsurfrules. Always-on
/// skills are listed in canonical order; on-demand skills only when relevant
/// (tech-gated ones hidden unless the project uses that tech).
pub(crate) fn inject_agents_md(home: &Path, root: &Path) -> Result<()> {
    let global_dir = home.join(".omp/skills");
    let local_dir = root.join("agents/skills");

    let globals = list_installed_skill_dirs(&global_dir).unwrap_or_default();
    let locals = list_installed_skill_dirs(&local_dir).unwrap_or_default();

    // Dedupe global vs local (locals mirror globals): list each skill once,
    // preferring the committed local copy under agents/skills/.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut chosen: Vec<(PathBuf, bool)> = Vec::new(); // (dir, is_local)
    for p in locals.iter() {
        if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
            if seen.insert(n.to_string()) { chosen.push((p.clone(), true)); }
        }
    }
    for p in globals.iter() {
        if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
            if seen.insert(n.to_string()) { chosen.push((p.clone(), false)); }
        }
    }
    // Always-on lead in canonical force-load order; on-demand (rank usize::MAX)
    // trail, ordered alphabetically by directory name.
    chosen.sort_by(|(a, _), (b, _)| {
        let an = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let bn = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        always_on_rank(an).cmp(&always_on_rank(bn)).then_with(|| an.cmp(bn))
    });

    let mut always_lines = String::new();
    let mut ondemand_lines = String::new();
    let mut a_idx = 1usize;
    let mut on_count = 0usize;
    for (p, is_local) in chosen.iter() {
        let dirname = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        if is_always_on(dirname) {
            let (_m, entry) = meta_for_dir(p);
            let abs = p.join(entry);
            always_lines.push_str(&format!("  {}. `{}`\n", a_idx, abs.display()));
            a_idx += 1;
        } else {
            // Tech-gated on-demand skill in a project that doesn't use that tech → hide.
            if skill_gated_out(dirname, root) {
                continue;
            }
            let (m, entry) = meta_for_dir(p);
            let rel = if *is_local {
                format!("agents/skills/{}/{}", dirname, entry)
            } else {
                format!("~/.omp/skills/{}/{}", dirname, entry)
            };
            ondemand_lines.push_str(&format!("- `{}` — `{}`\n", m.name, rel));
            on_count += 1;
        }
    }
    if a_idx == 1 {
        always_lines.push_str("  _(no always-on skills — run `8sync harness init`)_\n");
    }
    if on_count == 0 {
        ondemand_lines.push_str("- _(none — add via `8sync skill add <github-url>`)_\n");
    }
    let always_count = a_idx - 1;

    let has_codegraph_bin = which::which("codegraph").is_ok();
    let codegraph_install_hint: &str = if has_codegraph_bin {
        ""
    } else {
        "> ⚠ `codegraph` binary chưa cài. Chạy `8sync harness init` (auto cài) HOẶC `npx -y @colbymchenry/codegraph install` rồi quay lại đọc tiếp.\n\n"
    };

    let block = format!(
        "{BEGIN}\n\
## 🚨 STEP 0 — CODE INTELLIGENCE FIRST (codegraph + codebase-memory-mcp; bắt buộc)\n\
\n\
Mọi câu hỏi về code → dùng code-intelligence engine TRƯỚC grep/read (tiết kiệm ~99% token). Bạn (AI) **PHẢI**:\n\
\n\
1. **codegraph** (local index): `codegraph index .` 1 lần/session; rồi `codegraph search/deps/callers/defs` thay cho `grep`/`rg`/`fd`/`Grep`/`Glob` và `Read` toàn file.\n\
2. **codebase-memory-mcp** (MCP, auto-setup bởi `8sync harness`): `search_graph`, `semantic_query`, `trace_path`, `get_architecture`, `detect_changes`, `query_graph`, `get_code_snippet` — knowledge graph 158 ngôn ngữ, query sub-ms.\n\
3. Tìm/hiểu/định vị code · impact · route→handler · dead code · architecture → ƯU TIÊN 2 engine trên. Chỉ `Read` raw file khi sắp SỬA nó (read-before-edit).\n\
4. **Output dài** (logs/diff/test/tool output) → nén bằng `headroom` MCP (`headroom_compress`) thay vì dump cả khối vào context (60–95% ít token).\n\
\n\
Lý do: 5 query cấu trúc ≈ 3.4k token vs ≈ 412k token grep từng file (−99%). Dump cả file / grep mù = đốt token = bug.\n\
\n\
{codegraph_install_hint}\
## 🚨 STEP 1 — skills: always-on (đọc ngay) vs on-demand (đọc khi cần)\n\
\n\
Mỗi skill = 1 directory (Agent Skills open standard): `SKILL.md` có frontmatter `name`+`description`. Skill vendored ở `agents/skills/<name>/` (bản commit trong repo, mirror từ `~/.omp/skills/`). Mỗi skill liệt kê 1 lần.\n\
\n\
### ⛔ Always-on — ĐỌC NGAY, trước tool call đầu tiên (không skip)\n\
\n\
**Thứ tự = ưu tiên (đọc top-down, không đảo).** Mở đúng file `SKILL.md` ở path bên dưới rồi mới được gọi tool đầu tiên:\n\
\n\
{always_lines}\n\
### 🔎 On-demand — tên = trigger; mở `SKILL.md` của skill khi task khớp (mô tả ở frontmatter, KHÔNG nhồi ở đây)\n\
\n\
{ondemand_lines}\n\
### Quy tắc bất biến\n\
\n\
- **Code-intelligence FIRST** (codegraph + codebase-memory-mcp) cho mọi câu hỏi explore code (Step 0). Bypass = bug.\n\
- Đọc TẤT CẢ skill **always-on** TRƯỚC tool call đầu tiên, ĐÚNG thứ tự: codegraph → karpathy → ponytail → assp → impeccable + taste → 8sync-cli → image-routing.\n\
- **Cách tận dụng (luôn nhớ):** `codegraph` = explore code (search/deps/callers, KHÔNG grep) · `karpathy` + `ponytail` = YAGNI, làm ít nhất, xoá > thêm · `assp` = copy/offer hướng người dùng · **`impeccable` = design system CHUẨN, BẮT BUỘC cho MỌI UI/design/redesign/audit (đọc kèm `references/house/*`)** + `taste` chống slop.\n\
- Skill **on-demand**: chỉ mở khi description khớp task hiện tại — đừng đọc thừa.\n\
- Nếu skill có `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.\n\
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.\n\
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (mục Unreleased) + ghi học được vào `agents/KNOWLEDGE.md`.\n\
{END}"
    );

    // Inject into every known agent entry file (markdown: sentinel rewrite or
    // skeleton; plain text: prepend). AGENTS.md + CLAUDE.md are stub-created;
    // the rest are touched only if they already exist.
    let targets: &[(&str, EntryKind)] = &[
        ("AGENTS.md",                       EntryKind::Markdown { h1: "AGENTS.md — guidance for AI" }),
        ("CLAUDE.md",                       EntryKind::Markdown { h1: "CLAUDE.md — guidance for Claude Code" }),
        ("GEMINI.md",                       EntryKind::Markdown { h1: "GEMINI.md — guidance for Gemini" }),
        ("OPENCODE.md",                     EntryKind::Markdown { h1: "OPENCODE.md — guidance for opencode" }),
        (".github/copilot-instructions.md", EntryKind::Markdown { h1: "GitHub Copilot instructions" }),
        (".cursorrules",                    EntryKind::Plain),
        (".windsurfrules",                  EntryKind::Plain),
    ];

    let mut written = 0usize;
    for (name, kind) in targets {
        let path = root.join(name);
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let stub_create = matches!(*name, "AGENTS.md" | "CLAUDE.md");
        if existing.is_empty() && !stub_create { continue; }

        let new_contents = match kind {
            EntryKind::Markdown { h1 } => rewrite_md_with_block(&existing, &block, h1),
            EntryKind::Plain => rewrite_plain_with_block(&existing, &block),
        };
        if new_contents != existing {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &new_contents)?;
            ui::ok(&format!("injected force-load → {}", path.display()));
            written += 1;
        }
    }

    ui::info(&format!(
        "force-load: {} always-on, {} on-demand skill(s), written into {} file(s)",
        always_count, on_count, written,
    ));
    Ok(())
}

enum EntryKind {
    Markdown { h1: &'static str },
    Plain,
}

/// Markdown injection: replace existing sentinel block, or insert after the first
/// H1, or create a minimal skeleton if the file is empty.
pub(crate) fn rewrite_md_with_block(existing: &str, block: &str, h1_title: &str) -> String {
    if let (Some(b), Some(e)) = (find_line(existing, BEGIN), find_line(existing, END)) {
        if b < e {
            let mut s = String::with_capacity(existing.len() + block.len());
            s.push_str(&existing[..b]);
            s.push_str(block);
            s.push_str(&existing[e + END.len()..]);
            return s;
        }
    }
    if existing.is_empty() {
        return format!("# {h1_title}\n\n{block}\n");
    }
    insert_block_after_h1(existing, block)
}

/// Plain-text injection (.cursorrules / .windsurfrules): replace existing
/// sentinel block, or prepend the block at the top of the file.
fn rewrite_plain_with_block(existing: &str, block: &str) -> String {
    if let (Some(b), Some(e)) = (find_line(existing, BEGIN), find_line(existing, END)) {
        if b < e {
            let mut s = String::with_capacity(existing.len() + block.len());
            s.push_str(&existing[..b]);
            s.push_str(block);
            s.push_str(&existing[e + END.len()..]);
            return s;
        }
    }
    let sep = if existing.is_empty() { "" } else { "\n\n" };
    format!("{block}{sep}{existing}")
}

fn insert_block_after_h1(existing: &str, block: &str) -> String {
    if existing.is_empty() {
        return format!("# AGENTS.md\n\n{block}\n");
    }
    let mut out = String::with_capacity(existing.len() + block.len() + 8);
    let mut inserted = false;
    let mut prev_was_h1 = false;
    for line in existing.lines() {
        out.push_str(line);
        out.push('\n');
        if !inserted {
            if prev_was_h1 {
                out.push('\n');
                out.push_str(block);
                out.push_str("\n\n");
                inserted = true;
                prev_was_h1 = false;
            } else if line.starts_with("# ") {
                prev_was_h1 = true;
            }
        }
    }
    if !inserted {
        let mut s = String::with_capacity(existing.len() + block.len() + 4);
        s.push_str(block);
        s.push_str("\n\n");
        s.push_str(existing);
        return s;
    }
    out
}
