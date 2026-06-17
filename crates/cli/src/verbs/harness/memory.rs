//! Agent-memory + CHANGELOG seeding and the managed harness breadcrumb in
//! agents/KNOWLEDGE.md.
use anyhow::Result;
use std::path::Path;

use crate::ui;
use crate::verbs::skill::index::always_on_names_in_order;

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

/// Ensure the project carries the 8sync agent-memory files + a CHANGELOG, and
/// refresh the managed harness breadcrumb in agents/KNOWLEDGE.md. Memory files
/// are seeded only when missing; the KNOWLEDGE block is a sentinel-bounded
/// managed region (always current, never spam-appended).
pub(crate) fn seed_harness_memory(root: &Path) -> Result<()> {
    let agents_dir = root.join("agents");
    std::fs::create_dir_all(&agents_dir)?;
    for f in ["PROJECT.md", "KNOWLEDGE.md", "DECISIONS.md", "PREFERENCES.md", "STATE.md", "NOTES.md"] {
        let p = agents_dir.join(f);
        if !p.exists() {
            std::fs::write(
                &p,
                format!("# {} (8sync managed — append-only)\n\n_empty_\n", f.trim_end_matches(".md")),
            )?;
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
        "## 🧠 8sync harness ({})\n\n\
- **Always-on skills (đọc trước tool call đầu tiên, đúng thứ tự):** {}.\n\
- **Cách tận dụng:** codegraph = explore code (search/deps/callers, không grep) · \
karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · assp = copy/offer · \
impeccable = design system CHUẨN, BẮT BUỘC cho mọi UI/design (kèm references/house/*) + taste chống slop.\n\
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (mục Unreleased) + ghi học được vào file này.",
        now_stamp(),
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
