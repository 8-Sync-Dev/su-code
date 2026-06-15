//! SKILL.md frontmatter parsing + skill-directory metadata/auditing.
use anyhow::Result;
use std::path::Path;

use crate::ui;

/// Minimal Agent-Skills frontmatter projection used by 8sync.
pub(crate) struct SkillMeta {
    pub(crate) name: String,
    pub(crate) description: String,
}

/// Parse YAML frontmatter at the top of `skill_md`. Supports the two fields
/// 8sync cares about (`name`, `description`), single-line or double-quoted.
/// Returns Ok(None) when the file has no frontmatter (i.e. doesn't start with
/// `---`) — caller decides whether that's fatal.
pub(crate) fn read_skill_meta(skill_md: &Path) -> Result<Option<SkillMeta>> {
    if !skill_md.exists() {
        return Ok(None);
    }
    let s = std::fs::read_to_string(skill_md)?;
    let body = s.strip_prefix("---\n").or_else(|| s.strip_prefix("---\r\n"));
    let Some(body) = body else { return Ok(None) };
    let mut name = String::new();
    let mut desc = String::new();
    for line in body.lines() {
        if line.trim() == "---" {
            break;
        }
        if let Some(rest) = line.strip_prefix("name:") {
            name = unquote(rest.trim()).to_string();
        } else if let Some(rest) = line.strip_prefix("description:") {
            desc = unquote(rest.trim()).to_string();
        }
    }
    if name.is_empty() {
        return Ok(None);
    }
    Ok(Some(SkillMeta { name, description: desc }))
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 {
        let bs = s.as_bytes();
        let first = bs[0];
        let last = bs[s.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

/// Best-effort metadata + entry-point filename for a skill directory.
/// Preference order:
///   1. `SKILL.md` with valid YAML frontmatter (Agent Skills open standard).
///   2. `SKILL.md` present but no frontmatter — flag, use dir name.
///   3. `CLAUDE.md` (Claude Code convention) — recommend as entrypoint.
///   4. `README.md` — recommend as entrypoint.
///   5. Nothing — warn that AI cannot auto-discover.
pub(crate) fn meta_for_dir(dir: &Path) -> (SkillMeta, &'static str) {
    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string();
    let skill_md = dir.join("SKILL.md");
    if let Ok(Some(m)) = read_skill_meta(&skill_md) {
        return (m, "SKILL.md");
    }
    if skill_md.exists() {
        return (
            SkillMeta {
                name,
                description: "(SKILL.md present but missing YAML frontmatter — non-standard)".to_string(),
            },
            "SKILL.md",
        );
    }
    for (file, label) in [
        ("CLAUDE.md", "Claude-Code-style skill — entrypoint: CLAUDE.md (no Agent-Skills SKILL.md)"),
        ("README.md", "Tool/repo with no SKILL.md — entrypoint: README.md (read it for usage)"),
        ("AGENTS.md", "AGENTS.md-style skill — entrypoint: AGENTS.md (no Agent-Skills SKILL.md)"),
    ] {
        if dir.join(file).exists() {
            return (SkillMeta { name, description: label.to_string() }, file);
        }
    }
    (
        SkillMeta {
            name,
            description: "(no SKILL.md / CLAUDE.md / README.md — AI cannot auto-discover)".to_string(),
        },
        "SKILL.md",
    )
}

/// After install, audit that the skill dir has a `SKILL.md` with frontmatter.
/// Emit a warning if not — the AI will be unable to auto-load it via the
/// open-standard discovery mechanism.
pub(crate) fn audit_skill_layout(dir: &Path) {
    let skill_md = dir.join("SKILL.md");
    if !skill_md.exists() {
        ui::warn(&format!(
            "{} has no SKILL.md — non-standard layout, AI auto-discovery may not work",
            dir.display()
        ));
        return;
    }
    match read_skill_meta(&skill_md) {
        Ok(Some(m)) => {
            ui::ok(&format!("SKILL.md valid: name={}", m.name));
            if m.description.is_empty() {
                ui::warn("SKILL.md frontmatter has empty `description` — AI won't know when to use it");
            }
        }
        Ok(None) => {
            ui::warn(&format!(
                "{} exists but has no YAML frontmatter (`---` header) — required by Agent Skills spec",
                skill_md.display()
            ));
        }
        Err(e) => ui::warn(&format!("could not read {}: {}", skill_md.display(), e)),
    }
}
