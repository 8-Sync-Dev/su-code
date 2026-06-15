//! Skill-directory listing + project-root detection.
use anyhow::Result;
use std::path::{Path, PathBuf};

/// List immediate sub-directories of `skills_dir` (one per skill), sorted.
pub(crate) fn list_installed_skill_dirs(skills_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !skills_dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

/// Walk up from the cwd to the nearest recognised project root.
pub(crate) fn detect_current_project_root() -> Option<PathBuf> {
    // Markers in priority order. AGENTS.md / CLAUDE.md / agents/ catch projects
    // already seeded by `8sync .` even when they lack a language manifest.
    // `.git` / `.hg` catch any VCS repo. The rest cover major ecosystems.
    let markers = [
        "AGENTS.md",
        "CLAUDE.md",
        "agents",
        ".git",
        ".hg",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "deno.json",
        "go.mod",
        "composer.json",
        "Gemfile",
        "mix.exs",
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
