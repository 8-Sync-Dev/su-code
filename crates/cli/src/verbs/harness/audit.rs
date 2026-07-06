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

/// External / non-repo references we never treat as stale (URLs, home paths).
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
        // Only flag REPO-RELATIVE paths. Absolute (`/home/…`, machine-generated
        // by the harness) and `~`/`<placeholder>`-derived `/…` fragments carry
        // no authored-doc-rot signal — skip them.
        if tok.starts_with('/') {
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
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    docs.insert(name.to_string());
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
    let b = s.find("<!-- 8sync:skills:begin -->")?;
    let e = s.find("<!-- 8sync:skills:end -->")?;
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
            if !root.join(&cand).exists() {
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
            if !root.join(&cand).exists() {
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
