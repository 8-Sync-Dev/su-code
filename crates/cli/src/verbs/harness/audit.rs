//! `8sync harness audit` — code-backed doc-hygiene. Finds stale file-path
//! references, oversized docs, and recent churn hotspots so the agent can
//! delete junk and update stale docs instead of trusting them. Report-only —
//! NEVER auto-deletes (heuristic path detection has false positives on
//! illustrative paths); the agent/user acts on the findings.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;

use crate::verbs::skill::discover::detect_current_project_root;
use crate::{env_detect, ui};

/// File extensions that mark a token as a real source/doc path (not prose).
const EXTS: &[&str] = &[
    ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".toml", ".json", ".sh", ".yml", ".yaml",
    ".md", ".css", ".html", ".c", ".h", ".cpp",
];

/// True for a token that looks like a repo-local source/doc path: contains a
/// `/` (so bare filenames mentioned in prose aren't flagged) and ends in a
/// known extension.
fn looks_like_path(tok: &str) -> bool {
    tok.contains('/') && EXTS.iter().any(|e| tok.ends_with(e))
}

/// Absolute prefixes that only ever resolve on the machine that generated them:
/// a doc pointing at `/home/<user>/…` is dead for every other user, clone and
/// box — the agent is told to read a file that is not there and silently skips
/// it. Exactly the rot this audit exists to catch, so these are FLAGGED.
const MACHINE_PREFIXES: &[&str] = &["/home/", "/Users/", "/root/"];

/// True for an absolute path baked to one machine's `$HOME` layout.
fn is_machine_specific(tok: &str) -> bool {
    MACHINE_PREFIXES.iter().any(|p| tok.starts_with(p))
}

/// External / non-repo references we never treat as stale (URLs, and
/// `~`-anchored home paths, which are portable by construction).
fn is_external(tok: &str) -> bool {
    tok.starts_with("http")
        || tok.starts_with('~')
        || tok.starts_with("//")
        || tok.starts_with("mailto:")
        || tok.contains("://")
}

/// Extract unique path-candidate tokens from a doc body (hand-rolled; no regex
/// crate). Splits on every char outside `[A-Za-z0-9_./-]`, trims stray edge
/// punctuation, and keeps tokens that look like a repo-local path. A first
/// segment containing a `.` (e.g. `github.com`, `.github`) is treated as a
/// domain/dotdir and skipped to avoid URL false-positives.
fn path_candidates(body: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for raw in body.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))) {
        if is_external(raw) {
            continue;
        }
        // Trim only trailing sentence punctuation; keep leading dots/slashes.
        let tok = raw.trim_end_matches(|c: char| c == '.' || c == ',' || c == ';');
        if tok.is_empty() || !looks_like_path(tok) {
            continue;
        }
        // Absolute paths split two ways. Generic system paths (`/etc/…`,
        // `/usr/…`, `/tmp/…`, `/opt/…`) are environment facts, not authored repo
        // references, so they carry no doc-rot signal → keep skipping them. The
        // same goes for `~`/`<placeholder>`-derived `/…` fragments. Machine-
        // specific home prefixes are the opposite: they resolve only on the box
        // that generated them, so they must be reported.
        if tok.starts_with('/') && !is_machine_specific(tok) {
            continue;
        }
        let first = tok.split('/').next().unwrap_or("");
        if first.contains('.') {
            continue; // domain (github.com/…) or dotdir (.cargo/, .github/…)
        }
        out.insert(tok.to_string());
    }
    out
}

/// A path candidate is doc-rot when it does not resolve under the repo root —
/// or when it is machine-specific, which is stale even while it happens to
/// exist locally, because it breaks on every other machine.
fn is_stale_ref(root: &Path, cand: &str) -> bool {
    is_machine_specific(cand) || !root.join(cand).exists()
}

/// Collect the docs to audit: fixed top-level docs, every `*.md` at the repo
/// root, and every `su-code/*.md`. Non-recursive (skills/reference trees are
/// vendored/derived, not authored docs).
fn scan_docs(root: &Path) -> Vec<String> {
    let mut docs: BTreeSet<String> = BTreeSet::new();
    for f in ["AGENTS.md", "CLAUDE.md", "README.md", "CHANGELOG.md"] {
        if root.join(f).exists() {
            docs.insert(f.to_string());
        }
    }
    // Archives are exempt. An `*-ARCHIVE.md` is append-only history that no agent
    // force-loads, so the oversized-doc rule does not apply, and the paths quoted
    // inside it are accurate records of a layout that existed then — reporting
    // them as "stale" is noise that trains people to ignore the audit. Without
    // this, splitting an oversized CHANGELOG just moves the warning to the file
    // the split created.
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    if !name.ends_with("-ARCHIVE.md") {
                        docs.insert(name.to_string());
                    }
                }
            }
        }
    }
    if let Ok(rd) = std::fs::read_dir(root.join("su-code")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    docs.insert(format!("su-code/{}", name));
                }
            }
        }
    }
    docs.into_iter().collect()
}

/// Line count of the managed force-load block in AGENTS.md, if present.
fn agents_block_lines(root: &Path) -> Option<usize> {
    let s = std::fs::read_to_string(root.join("AGENTS.md")).ok()?;
    let b = s.find(crate::brand::sentinel_begin().as_str()).or_else(|| s.find(crate::brand::LEGACY_SENTINEL_BEGIN))?;
    let e = s.find(crate::brand::sentinel_end().as_str()).or_else(|| s.find(crate::brand::LEGACY_SENTINEL_END))?;
    (b < e).then(|| s[b..e].lines().count())
}

/// Top-5 files changed in the last 30 days (history-awareness: docs referencing
/// churned code are the most likely to be stale). Best-effort; empty on
/// non-git repos.
fn churn_hotspots(root: &Path) -> Vec<(String, usize)> {
    let Ok(out) = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--since=30.days", "--name-only", "--pretty=format:"])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for line in text.lines() {
        let f = line.trim();
        if !f.is_empty() {
            *counts.entry(f.to_string()).or_insert(0) += 1;
        }
    }
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(5);
    v
}

/// `(stale_path_refs, oversized_docs)` counts for the current project — the
/// lightweight summary `8sync doctor` surfaces without the full report.
pub(crate) fn stale_summary(root: &Path) -> (usize, usize) {
    let docs = scan_docs(root);
    let mut stale = 0usize;
    for doc in &docs {
        let body = std::fs::read_to_string(root.join(doc)).unwrap_or_default();
        for cand in path_candidates(&body) {
            if is_stale_ref(root, &cand) {
                stale += 1;
            }
        }
    }
    let mut oversized = 0usize;
    if agents_block_lines(root).is_some_and(|n| n > 120) {
        oversized += 1;
    }
    for doc in &docs {
        let n = std::fs::read_to_string(root.join(doc))
            .map(|s| s.lines().count())
            .unwrap_or(0);
        if n > 400 {
            oversized += 1;
        }
    }
    (stale, oversized)
}

pub(crate) fn harness_audit(_env: &env_detect::Env) -> Result<()> {
    ui::header("8sync harness audit — doc-hygiene");
    let Some(root) = detect_current_project_root() else {
        ui::warn("not inside a project — cd into a repo root and re-run");
        return Ok(());
    };
    let docs = scan_docs(&root);
    println!();
    println!("  project: {}", root.display());
    println!("  docs scanned: {}", docs.len());
    println!();

    // A — stale path references.
    let mut stale: Vec<(String, String)> = Vec::new();
    for doc in &docs {
        let body = std::fs::read_to_string(root.join(doc)).unwrap_or_default();
        for cand in path_candidates(&body) {
            if is_stale_ref(&root, &cand) {
                stale.push((doc.clone(), cand));
            }
        }
    }
    println!("  ── stale path references (heuristic — review before editing) ──");
    if stale.is_empty() {
        println!("   none");
    } else {
        for (doc, cand) in &stale {
            println!("   {} → {}", doc, cand);
        }
    }
    println!();

    // B — oversized docs.
    let mut oversized: Vec<(String, usize)> = Vec::new();
    if let Some(n) = agents_block_lines(&root) {
        if n > 120 {
            oversized.push(("AGENTS.md force-load block".into(), n));
        }
    }
    for doc in &docs {
        let n = std::fs::read_to_string(root.join(doc))
            .map(|s| s.lines().count())
            .unwrap_or(0);
        if n > 400 {
            oversized.push((doc.clone(), n));
        }
    }
    println!("  ── oversized docs (>400 lines / >120-line block — trim or split) ──");
    if oversized.is_empty() {
        println!("   none");
    } else {
        for (doc, n) in &oversized {
            println!("   {} — {} lines", doc, n);
        }
    }
    println!();

    // C — churn hotspots (history-awareness).
    let churn = churn_hotspots(&root);
    println!("  ── churn hotspots (30d — docs near these are most likely stale) ──");
    if churn.is_empty() {
        println!("   none (or not a git repo)");
    } else {
        for (f, c) in &churn {
            println!("   {:>3}× {}", c, f);
        }
    }
    println!();

    if stale.is_empty() && oversized.is_empty() {
        ui::ok("docs clean — no stale paths or oversized docs");
    } else {
        ui::warn(&format!(
            "audit: {} stale path(s) · {} oversized doc(s) — fix stale, delete junk/superseded, trim oversized",
            stale.len(),
            oversized.len()
        ));
    }
    ui::info("report-only — never auto-deletes; verify each finding (illustrative paths can false-positive)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dead-skill-path bug: a doc carrying ANOTHER machine's `$HOME` layout
    /// must be reported, while generic system paths stay invisible to the audit
    /// (they are environment facts, not authored repo references).
    #[test]
    fn flags_machine_specific_absolutes_only() {
        let cands = path_candidates(
            "core: /home/alexng/x/SKILL.md · /Users/bob/y/SKILL.md · /root/z/SKILL.md — \
             env: /etc/os-release, /etc/profile.d/init.sh, /usr/share/doc/readme.md, \
             /tmp/scratch/note.md, /opt/tool/setup.sh — repo: su-code/STATE.md",
        );
        assert!(cands.contains("/home/alexng/x/SKILL.md"));
        assert!(cands.contains("/Users/bob/y/SKILL.md"));
        assert!(cands.contains("/root/z/SKILL.md"));
        for generic in [
            "/etc/os-release",
            "/etc/profile.d/init.sh",
            "/usr/share/doc/readme.md",
            "/tmp/scratch/note.md",
            "/opt/tool/setup.sh",
        ] {
            assert!(!cands.contains(generic), "generic absolute must stay skipped: {generic}");
        }
        assert!(cands.contains("su-code/STATE.md"), "repo-relative refs still audited");
    }

    /// A machine-specific path is stale even when it currently resolves — it
    /// breaks on every other machine. A real repo-relative file is not stale.
    #[test]
    fn stale_ref_ignores_existence_for_machine_paths() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(is_stale_ref(root, "/home/alexng/x/SKILL.md"));
        assert!(!is_stale_ref(root, "src/main.rs"));
        let here = root.join("src/main.rs");
        let here = here.to_string_lossy();
        if is_machine_specific(&here) {
            assert!(is_stale_ref(root, &here), "existing absolute home path is still rot");
        }
    }
}