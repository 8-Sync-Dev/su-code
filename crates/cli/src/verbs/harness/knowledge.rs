//! Curated-knowledge catalog for the dashboard — auto-fetch `sindresorhus/awesome`
//! (the canonical "awesome list of awesome lists") and let the user browse,
//! search, and save selected entries into a project's `su-code/REFERENCES.md`.
//!
//! The source is one big README, so we `curl` the raw markdown (project rule:
//! no `reqwest`) and cache it under `.cache/8sync/knowledge/` with a 6h TTL,
//! then parse `##`/`###` headings → `- [name](url) - desc` bullets on each read.
use std::path::{Path, PathBuf};

/// Raw README candidates (branch/filename drift over time — try in order).
const SOURCES: &[&str] = &[
    "https://raw.githubusercontent.com/sindresorhus/awesome/main/readme.md",
    "https://raw.githubusercontent.com/sindresorhus/awesome/master/readme.md",
    "https://raw.githubusercontent.com/sindresorhus/awesome/main/README.md",
];

fn cache_dir(root: &Path) -> PathBuf {
    root.join(".cache/8sync/knowledge")
}

/// Fetch the awesome README, preferring a fresh cache (6h TTL). On a network
/// miss, fall back to a stale cache so the dashboard still renders.
fn fetch_readme(root: &Path, refresh: bool) -> Result<String, String> {
    let cache = cache_dir(root).join("awesome-readme.md");
    if !refresh {
        if let Ok(meta) = std::fs::metadata(&cache) {
            let fresh = meta
                .modified()
                .ok()
                .and_then(|m| m.elapsed().ok())
                .map(|d| d.as_secs() < 6 * 3600)
                .unwrap_or(false);
            if fresh {
                if let Ok(s) = std::fs::read_to_string(&cache) {
                    return Ok(s);
                }
            }
        }
    }
    let mut last_err = String::from("no source reachable");
    for url in SOURCES {
        match curl_text(url) {
            Ok(body) if body.contains("# Awesome") || body.len() > 2000 => {
                let _ = std::fs::create_dir_all(cache_dir(root));
                let _ = std::fs::write(&cache, &body);
                return Ok(body);
            }
            Ok(_) => last_err = format!("unexpected body from {url}"),
            Err(e) => last_err = e,
        }
    }
    // Network failed — serve a stale cache rather than nothing.
    if let Ok(s) = std::fs::read_to_string(&cache) {
        return Ok(s);
    }
    Err(last_err)
}

fn curl_text(url: &str) -> Result<String, String> {
    let out = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "15", "-A", "8sync-harness-knowledge", url])
        .output()
        .map_err(|e| format!("spawn curl: {e}"))?;
    if !out.status.success() {
        return Err(format!("curl failed for {url} ({})", out.status));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Parse one markdown list line into `(name, url, desc)`. Handles `- [x](y) - z`,
/// `* [x](y)`, and bold `- **[x](y)**`. Returns `None` for non-link bullets.
fn parse_link(line: &str) -> Option<(String, String, String)> {
    let t = line.trim_start();
    let t = t.strip_prefix("- ").or_else(|| t.strip_prefix("* "))?;
    let lb = t.find('[')?;
    let mid = t[lb + 1..].find("](")? + lb + 1;
    let name = t[lb + 1..mid].trim().trim_matches('*').trim().to_string();
    let after = &t[mid + 2..];
    let close = after.find(')')?;
    let url = after[..close].trim().to_string();
    let mut desc = after[close + 1..].trim().to_string();
    for p in ["- ", "— ", "– ", ": ", "* "] {
        if let Some(s) = desc.strip_prefix(p) {
            desc = s.trim().to_string();
            break;
        }
    }
    desc = desc.trim_matches('*').trim().to_string();
    if name.is_empty() || url.is_empty() {
        return None;
    }
    Some((name, url, desc))
}

/// Parse the README into `[{ name, entries: [{name,url,desc}] }]`. Skips the
/// table-of-contents (anchor `#` links) and the `Contents` section.
fn parse(md: &str, search: &str) -> Vec<serde_json::Value> {
    let needle = search.trim().to_lowercase();
    let mut cats: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
    let mut current: Option<usize> = None;
    let skip = |name: &str| {
        matches!(
            name.to_lowercase().as_str(),
            "contents" | "footnotes" | "license" | "related" | "contributing"
        )
    };
    for line in md.lines() {
        if let Some(h) = line.strip_prefix("## ").or_else(|| line.strip_prefix("### ")) {
            let name = h.trim().trim_end_matches('#').trim().to_string();
            if name.is_empty() || skip(&name) {
                current = None;
                continue;
            }
            cats.push((name, Vec::new()));
            current = Some(cats.len() - 1);
            continue;
        }
        if line.starts_with("# ") {
            current = None;
            continue;
        }
        let Some(idx) = current else { continue };
        if let Some((name, url, desc)) = parse_link(line) {
            if url.starts_with('#') || url.starts_with("./") {
                continue; // TOC anchor / in-repo relative link
            }
            if !needle.is_empty() {
                let hay = format!("{name} {desc} {}", cats[idx].0).to_lowercase();
                if !hay.contains(&needle) {
                    continue;
                }
            }
            cats[idx].1.push(serde_json::json!({
                "name": name, "url": url, "desc": desc,
            }));
        }
    }
    cats.into_iter()
        .filter(|(_, e)| !e.is_empty())
        .map(|(name, entries)| serde_json::json!({ "name": name, "count": entries.len(), "entries": entries }))
        .collect()
}

/// Public catalog for `/api/knowledge`. `search` filters entries; `refresh`
/// bypasses the cache.
pub(crate) fn catalog(root: &Path, search: &str, refresh: bool) -> Result<serde_json::Value, String> {
    let md = fetch_readme(root, refresh)?;
    let categories = parse(&md, search);
    let total: usize = categories
        .iter()
        .filter_map(|c| c.get("count").and_then(|v| v.as_u64()))
        .sum::<u64>() as usize;
    Ok(serde_json::json!({
        "source": "sindresorhus/awesome",
        "categories": categories,
        "category_count": categories.len(),
        "total": total,
    }))
}

/// Append selected entries to `<proj>/su-code/REFERENCES.md`, grouped by category
/// and de-duplicated by URL against what's already there. Returns (added, path).
pub(crate) fn apply_entries(
    proj: &Path,
    items: &[serde_json::Value],
) -> Result<(usize, PathBuf), String> {
    let dir = proj.join("su-code");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("REFERENCES.md");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut out = if existing.trim().is_empty() {
        "# REFERENCES (8sync — curated external knowledge)\n\n\
         > Saved from the dashboard Knowledge browser (`sindresorhus/awesome`).\n\
         > High-signal links the AI can consult for this project.\n"
            .to_string()
    } else {
        let mut s = existing.clone();
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s
    };

    // Group incoming items by category, preserving first-seen order.
    let mut order: Vec<String> = Vec::new();
    let mut groups: std::collections::HashMap<String, Vec<&serde_json::Value>> =
        std::collections::HashMap::new();
    for it in items {
        let cat = it.get("category").and_then(|v| v.as_str()).unwrap_or("Misc").to_string();
        if !groups.contains_key(&cat) {
            order.push(cat.clone());
        }
        groups.entry(cat).or_default().push(it);
    }

    let mut added = 0usize;
    for cat in &order {
        let mut section = String::new();
        for it in &groups[cat] {
            let url = it.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
            let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
            if url.is_empty() || name.is_empty() {
                continue;
            }
            if existing.contains(url) || out.contains(url) {
                continue; // already saved
            }
            let desc = it.get("desc").and_then(|v| v.as_str()).unwrap_or("").trim();
            if desc.is_empty() {
                section.push_str(&format!("- [{name}]({url})\n"));
            } else {
                section.push_str(&format!("- [{name}]({url}) — {desc}\n"));
            }
            added += 1;
        }
        if !section.is_empty() {
            // Reuse an existing `### <cat>` header if present, else add one.
            let header = format!("### {cat}");
            if !out.contains(&header) {
                out.push_str(&format!("\n{header}\n"));
            }
            out.push_str(&section);
        }
    }

    if added > 0 {
        std::fs::write(&path, out).map_err(|e| e.to_string())?;
    }
    Ok((added, path))
}
