//! Agent-memory + CHANGELOG seeding and the managed harness breadcrumb in
//! agents/KNOWLEDGE.md.
use anyhow::Result;
use std::path::Path;

use crate::ui;
use crate::verbs::skill::index::always_on_names_in_order;

/// Structured live-plan seed for `agents/STATE.md` — the loop-engineering
/// recitation anchor (Manus todo.md pattern): the agent rewrites it at each
/// phase boundary and reads it at session start, keeping the plan in recent
/// context (anti lost-in-the-middle). Seeded once; never overwritten if present.
const STATE_TEMPLATE: &str = "\
# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
_một câu: kết quả cần đạt_

## Definition of Done
- [ ] _tiêu chí nghiệm thu_

## Checklist
- [ ] _bước 1_

## Current step
_đang làm gì_

## Next
_bước kế tiếp_

## Open questions / blockers
_none_

## Handoff (compaction)
_none — khi context gần đầy: ghi Done · In-flight · Next · Open-questions vào đây + bài học vào KNOWLEDGE, rồi reinit phiên mới chỉ đọc spine._
";

/// Procedural-memory seed for `agents/PLAYBOOKS.md` (Voyager-style skill
/// library): validated multi-step procedures distilled into reusable runbooks
/// indexed by a `When:` line. Seeded once; appended to by the agent.
const PLAYBOOKS_TEMPLATE: &str = "\
# PLAYBOOKS (8sync managed — procedural memory, append-only)

Runbook tái dùng cho quy trình ĐÃ `validated:`. Index theo `When:` để retrieve;
Voyager-style: lưu cái đã chạy được, lần sau adapt thay vì suy luận lại.

## Template
### <tên ngắn>
- **When:** _tình huống kích hoạt (1 dòng để match)_
- **Steps:** _các bước đã verify_
- **Verify:** _cách kiểm chứng_
- **Pitfalls:** _bẫy đã gặp_

_empty_
";

/// Epoch-seconds stamp in the repo's `epoch:<n>` convention (no chrono dep).
pub(crate) fn now_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{}", secs)
}

/// Replace the text between `begin`/`end` sentinels in `path`, or prepend the
/// block at the top when the sentinels are absent. Creates the file if missing.
pub(crate) fn upsert_block(path: &Path, begin: &str, end: &str, body: &str) -> Result<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let block = format!("{begin}\n{body}\n{end}");
    let new = match (existing.find(begin), existing.find(end)) {
        (Some(b), Some(e)) if b < e => {
            let mut s = String::with_capacity(existing.len() + block.len());
            s.push_str(&existing[..b]);
            s.push_str(&block);
            s.push_str(&existing[e + end.len()..]);
            s
        }
        _ if existing.is_empty() => format!("{block}\n"),
        _ => format!("{block}\n\n{existing}"),
    };
    if new != existing {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(path, new)?;
    }
    Ok(())
}

/// Seed/refresh a managed block in `<root>/.gitignore` so durable agent memory
/// + skills stay committed (portable to a new machine) while derived caches and
/// secrets are ignored. Only the sentinel-bounded block is owned; any user
/// entries outside it (incl. a tool-repo's own `agents/skills/` rule) survive.
pub(crate) fn seed_gitignore(root: &Path) -> Result<()> {
    let body = concat!(
        "# Derived / machine-local — rebuilt by `8sync harness init` + codegraph. Safe to ignore:\n",
        ".codegraph/\n",
        ".cache/8sync/\n",
        "# Secrets — NEVER commit:\n",
        ".env\n",
        ".env.*\n",
        "!.env.example\n",
        "# KEEP COMMITTED (do NOT add here): agents/ (memory), agents/skills/, AGENTS.md, CHANGELOG.md",
    );
    upsert_block(
        &root.join(".gitignore"),
        "# >>> 8sync (managed) >>>",
        "# <<< 8sync <<<",
        body,
    )
}

/// Ensure the project carries the 8sync agent-memory files + a CHANGELOG, and
/// refresh the managed harness breadcrumb in agents/KNOWLEDGE.md. Memory files
/// are seeded only when missing; the KNOWLEDGE block is a sentinel-bounded
/// managed region (always current, never spam-appended).
pub(crate) fn seed_harness_memory(root: &Path) -> Result<()> {
    let agents_dir = root.join("agents");
    std::fs::create_dir_all(&agents_dir)?;
    seed_gitignore(root)?;
    for f in ["PROJECT.md", "KNOWLEDGE.md", "DECISIONS.md", "PREFERENCES.md", "STATE.md", "PLAYBOOKS.md", "NOTES.md"] {
        let p = agents_dir.join(f);
        if !p.exists() {
            // KNOWLEDGE.md carries an append-only "Learnings" zone below the managed
            // breadcrumb block (which `harness up` overwrites) so learnings persist.
            let content = if f == "KNOWLEDGE.md" {
                "# KNOWLEDGE (8sync managed — append-only)\n\n## Learnings (append-only — ghi DƯỚI đây; KHÔNG sửa block `8sync:harness` ở trên)\n\nMỗi entry prefix `validated:` (test/build xác nhận) · `hypothesis:` (chưa) · `failure:` (lỗi đã gặp + cách sửa; đọc đầu phiên để khỏi lặp).\n\n_empty_\n".to_string()
            } else if f == "STATE.md" {
                STATE_TEMPLATE.to_string()
            } else if f == "PLAYBOOKS.md" {
                PLAYBOOKS_TEMPLATE.to_string()
            } else {
                format!("# {} (8sync managed — append-only)\n\n_empty_\n", f.trim_end_matches(".md"))
            };
            std::fs::write(&p, content)?;
        }
    }
    // CHANGELOG.md — Keep a Changelog skeleton, created once.
    let changelog = root.join("CHANGELOG.md");
    if !changelog.exists() {
        std::fs::write(
            &changelog,
            concat!(
                "# Changelog\n\n",
                "Mọi thay đổi đáng kể ghi vào đây — format [Keep a Changelog](https://keepachangelog.com), ",
                "versioning [SemVer](https://semver.org).\n",
                "**8sync rule:** mỗi PR cập nhật mục `Unreleased` bên dưới.\n\n",
                "## [Unreleased]\n\n",
            ),
        )?;
        ui::ok(&format!("seeded {}", changelog.display()));
    }
    // KNOWLEDGE.md — managed harness breadcrumb (always current).
    let chain = {
        let names = always_on_names_in_order(root);
        if names.is_empty() {
            "codegraph → karpathy → ponytail → assp → impeccable → taste → 8sync-cli → image-routing".to_string()
        } else {
            names.join(" → ")
        }
    };
    let body = format!(
        "## 🧠 8sync harness\n\n\
- **Always-on (đọc theo thứ tự; CORE đọc body ngay, SPECIALIST đọc khi task khớp):** {}.\n\
- **Cách tận dụng:** codegraph = explore code (search/deps/callers, không grep) · karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · impeccable = design CHUẨN, BẮT BUỘC khi UI/design (đọc body lúc đó) + taste chống slop.\n\
- **Output lớn (>~50 dòng) → BẮT BUỘC `headroom_compress`** trước khi vào context.\n\
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (Unreleased) + ghi học được vào file này (prefix `validated:` nếu test/build xác nhận, `hypothesis:` nếu chưa).",
        chain,
    );
    upsert_block(
        &agents_dir.join("KNOWLEDGE.md"),
        "<!-- 8sync:harness:begin -->",
        "<!-- 8sync:harness:end -->",
        &body,
    )?;
    Ok(())
}

/// Bound the append-only `## Learnings` zone in agents/KNOWLEDGE.md (anti
/// context-rot, Hindsight 4-lever): when it exceeds the budget, archive the
/// OLDER lines to agents/archive/ and keep the most recent, leaving a pointer.
/// Best-effort; git history preserves the full trail.
pub(crate) fn consolidate_learnings(root: &Path) -> Result<()> {
    const BUDGET: usize = 200;
    let path = root.join("agents/KNOWLEDGE.md");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let Some(hpos) = content.find("\n## Learnings") else {
        return Ok(());
    };
    // Body = everything after the header line's trailing newline.
    let Some(nl) = content[hpos + 1..].find('\n') else {
        return Ok(());
    };
    let after_header = hpos + 1 + nl + 1;
    let head = &content[..after_header];
    let body_lines: Vec<&str> = content[after_header..].lines().collect();
    if body_lines.len() <= BUDGET {
        return Ok(());
    }
    let keep_from = body_lines.len() - BUDGET;
    let archived = body_lines[..keep_from].join("\n");
    let kept = body_lines[keep_from..].join("\n");
    let stamp = now_stamp().trim_start_matches("epoch:").to_string();
    let archive_dir = root.join("agents/archive");
    std::fs::create_dir_all(&archive_dir)?;
    std::fs::write(
        archive_dir.join(format!("KNOWLEDGE-{}.md", stamp)),
        format!("# Archived learnings ({})\n\n{}\n", now_stamp(), archived),
    )?;
    let new = format!(
        "{}_(consolidated {} dòng cũ → agents/archive/KNOWLEDGE-{}.md)_\n{}",
        head, keep_from, stamp, kept
    );
    std::fs::write(&path, new)?;
    ui::ok(&format!(
        "consolidated KNOWLEDGE learnings → archived {} older line(s)",
        keep_from
    ));
    Ok(())
}

/// Install a gitleaks pre-commit hook so any commit (incl. `harness up --commit`)
/// is secret-scanned. Non-destructive: only when gitleaks is installed,
/// `.git/hooks/` exists, and no pre-commit hook is already present.
pub(crate) fn seed_gitleaks_hook(root: &Path) {
    let hooks = root.join(".git/hooks");
    if !hooks.is_dir() {
        return;
    }
    let hook = hooks.join("pre-commit");
    if hook.exists() || which::which("gitleaks").is_err() {
        return;
    }
    let body = "#!/bin/sh\n# 8sync: block commits containing secrets (gitleaks).\ncommand -v gitleaks >/dev/null 2>&1 || exit 0\ngitleaks protect --staged --no-banner\n";
    if std::fs::write(&hook, body).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755));
        }
        ui::ok("installed gitleaks pre-commit hook (.git/hooks/pre-commit)");
    }
}
