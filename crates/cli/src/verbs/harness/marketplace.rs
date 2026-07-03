//! Marketplace catalog for the dashboard — discover + install skills and MCP
//! servers from external registries, without leaving the harness.
//!
//! Sources (all API-based, no HTML scraping so they don't rot on layout changes):
//!   • MCP — official registry (`registry.modelcontextprotocol.io`) + Smithery
//!     (`registry.smithery.ai`).
//!   • Skills — GitHub repo search (`api.github.com/search/repositories`),
//!     ranked by stars; installable via the existing `8sync skill add <url>`.
//!
//! HTTP goes through `curl` shell-out (project rule: no `reqwest`, keep the
//! binary small). Results are normalized to one `Entry` shape and cached under
//! `.cache/8sync/marketplace/*.json` with a 1h TTL (the MCP registry maintainers
//! explicitly ask aggregators to poll infrequently + persist locally).

use std::path::{Path, PathBuf};

/// A normalized catalog row the dashboard renders identically regardless of
/// source. `install` carries everything `/api/mcp/add` or `skill add` needs.
fn entry(
    id: &str,
    name: &str,
    description: &str,
    kind: &str,
    source: &str,
    stars: u64,
    updated: &str,
    url: &str,
    install: serde_json::Value,
) -> serde_json::Value {
    // "new" = touched within 30 days (RFC3339 date prefix compare is enough).
    let is_new = updated
        .get(0..10)
        .map(|d| d >= thirty_days_ago().as_str())
        .unwrap_or(false);
    serde_json::json!({
        "id": id,
        "name": name,
        "description": description,
        "kind": kind,        // "mcp" | "skill"
        "source": source,    // "official" | "smithery" | "github"
        "stars": stars,
        "updated": updated,
        "url": url,
        "new": is_new,
        "install": install,  // { type:"stdio", command, args, env:[..] } | { type:"http", url } | { type:"skill", spec }
    })
}

fn thirty_days_ago() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let then = now.saturating_sub(30 * 86_400);
    ymd_utc(then)
}

/// Minimal epoch→`YYYY-MM-DD` (UTC), no chrono dep. Civil-date algorithm
/// (Howard Hinnant's `days_from_civil` inverse).
fn ymd_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// `curl` a URL and parse the body as JSON. Times out fast so the dashboard
/// stays responsive when a registry is down.
fn curl_json(url: &str) -> Result<serde_json::Value, String> {
    let out = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "12",
            "-A",
            "8sync-harness-marketplace",
            "-H",
            "Accept: application/json",
            url,
        ])
        .output()
        .map_err(|e| format!("spawn curl: {e}"))?;
    if !out.status.success() {
        return Err(format!("curl failed ({})", out.status));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("bad JSON from {url}: {e}"))
}

// ── caching ──────────────────────────────────────────────────────────────

fn cache_dir(root: &Path) -> PathBuf {
    root.join(".cache/8sync/marketplace")
}

/// Read a cache file if younger than `ttl_secs`.
fn cache_read(root: &Path, key: &str, ttl_secs: u64) -> Option<Vec<serde_json::Value>> {
    let p = cache_dir(root).join(format!("{key}.json"));
    let meta = std::fs::metadata(&p).ok()?;
    let age = meta
        .modified()
        .ok()?
        .elapsed()
        .map(|d| d.as_secs())
        .unwrap_or(u64::MAX);
    if age > ttl_secs {
        return None;
    }
    let raw = std::fs::read_to_string(&p).ok()?;
    serde_json::from_str(&raw).ok()
}

fn cache_write(root: &Path, key: &str, rows: &[serde_json::Value]) {
    let dir = cache_dir(root);
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(s) = serde_json::to_string(rows) {
        let _ = std::fs::write(dir.join(format!("{key}.json")), s);
    }
}

// ── MCP: official registry ─────────────────────────────────────────────────

/// Build an `mcp.json` install descriptor from an official-registry package or
/// remote. Prefers a stdio package (npx/uvx) since those need no auth; falls
/// back to a remote HTTP/SSE endpoint.
fn official_install(server: &serde_json::Value) -> Option<serde_json::Value> {
    if let Some(pkgs) = server.get("packages").and_then(|v| v.as_array()) {
        if let Some(p) = pkgs.first() {
            let id = p.get("identifier").and_then(|v| v.as_str()).unwrap_or("");
            let hint = p.get("runtimeHint").and_then(|v| v.as_str()).unwrap_or("");
            let env: Vec<serde_json::Value> = p
                .get("environmentVariables")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|e| {
                            let name = e.get("name").and_then(|v| v.as_str())?;
                            Some(serde_json::json!({
                                "name": name,
                                "required": e.get("isRequired").and_then(|v| v.as_bool()).unwrap_or(false),
                                "description": e.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                            }))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let (command, args) = match hint {
                "uvx" => ("uvx".to_string(), vec![id.to_string()]),
                _ => ("npx".to_string(), vec!["-y".to_string(), id.to_string()]),
            };
            if !id.is_empty() {
                return Some(serde_json::json!({
                    "type": "stdio", "command": command, "args": args, "env": env,
                }));
            }
        }
    }
    if let Some(remotes) = server.get("remotes").and_then(|v| v.as_array()) {
        if let Some(r) = remotes.first() {
            if let Some(u) = r.get("url").and_then(|v| v.as_str()) {
                let t = r.get("type").and_then(|v| v.as_str()).unwrap_or("http");
                let t = if t.contains("sse") { "sse" } else { "http" };
                return Some(serde_json::json!({ "type": t, "url": u }));
            }
        }
    }
    None
}

fn fetch_mcp_official(search: &str) -> Vec<serde_json::Value> {
    let q = if search.is_empty() {
        "https://registry.modelcontextprotocol.io/v0/servers?limit=60".to_string()
    } else {
        format!(
            "https://registry.modelcontextprotocol.io/v0/servers?limit=60&search={}",
            urlencoding::encode(search)
        )
    };
    let Ok(v) = curl_json(&q) else { return Vec::new() };
    let Some(servers) = v.get("servers").and_then(|v| v.as_array()) else { return Vec::new() };
    let mut out = Vec::new();
    for e in servers {
        let s = e.get("server").unwrap_or(e);
        let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let Some(install) = official_install(s) else { continue };
        // Short display name = last path segment of the reverse-DNS id.
        let short = name.rsplit(['/', '.']).next().unwrap_or(name);
        let updated = e
            .get("_meta")
            .and_then(|m| m.get("io.modelcontextprotocol.registry/official"))
            .and_then(|o| o.get("updatedAt").or_else(|| o.get("publishedAt")))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        out.push(entry(
            name,
            short,
            s.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "mcp",
            "official",
            0,
            updated,
            &format!("https://registry.modelcontextprotocol.io/?search={}", urlencoding::encode(name)),
            install,
        ));
    }
    out
}

// ── MCP: Smithery aggregator ────────────────────────────────────────────────

fn fetch_mcp_smithery(search: &str) -> Vec<serde_json::Value> {
    let q = if search.is_empty() {
        "https://registry.smithery.ai/servers?pageSize=40".to_string()
    } else {
        format!(
            "https://registry.smithery.ai/servers?pageSize=40&q={}",
            urlencoding::encode(search)
        )
    };
    let Ok(v) = curl_json(&q) else { return Vec::new() };
    let Some(servers) = v.get("servers").and_then(|v| v.as_array()) else { return Vec::new() };
    let mut out = Vec::new();
    for s in servers {
        let qn = s.get("qualifiedName").and_then(|v| v.as_str()).unwrap_or("");
        if qn.is_empty() {
            continue;
        }
        let uses = s.get("useCount").and_then(|v| v.as_u64()).unwrap_or(0);
        // Smithery servers install through its CLI runner (stdio).
        let install = serde_json::json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "@smithery/cli@latest", "run", qn],
            "env": [],
        });
        out.push(entry(
            qn,
            s.get("displayName").and_then(|v| v.as_str()).unwrap_or(qn),
            s.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "mcp",
            "smithery",
            uses,
            s.get("createdAt").and_then(|v| v.as_str()).unwrap_or(""),
            &format!("https://smithery.ai/server/{}", urlencoding::encode(qn)),
            install,
        ));
    }
    out
}

// ── Skills: GitHub search ───────────────────────────────────────────────────

fn fetch_skills_github(search: &str) -> Vec<serde_json::Value> {
    // Default query surfaces agent-skill repos; a user search narrows it.
    let query = if search.is_empty() {
        "agent skills SKILL.md".to_string()
    } else {
        format!("{search} skill")
    };
    let url = format!(
        "https://api.github.com/search/repositories?sort=stars&order=desc&per_page=40&q={}",
        urlencoding::encode(&query)
    );
    let Ok(v) = curl_json(&url) else { return Vec::new() };
    let Some(items) = v.get("items").and_then(|v| v.as_array()) else { return Vec::new() };
    let mut out = Vec::new();
    for r in items {
        let full = r.get("full_name").and_then(|v| v.as_str()).unwrap_or("");
        let clone = r.get("clone_url").and_then(|v| v.as_str()).unwrap_or("");
        if full.is_empty() || clone.is_empty() {
            continue;
        }
        out.push(entry(
            full,
            r.get("name").and_then(|v| v.as_str()).unwrap_or(full),
            r.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "skill",
            "github",
            r.get("stargazers_count").and_then(|v| v.as_u64()).unwrap_or(0),
            r.get("pushed_at").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("html_url").and_then(|v| v.as_str()).unwrap_or(""),
            // `8sync skill add <url>` handles collection-aware clone.
            serde_json::json!({ "type": "skill", "spec": r.get("html_url").and_then(|v| v.as_str()).unwrap_or(clone) }),
        ));
    }
    out
}

// ── MCP: Glama aggregator (JSON) ────────────────────────────────────────────

fn fetch_mcp_glama(search: &str) -> Vec<serde_json::Value> {
    let url = if search.is_empty() {
        "https://glama.ai/api/mcp/v1/servers?first=40".to_string()
    } else {
        format!(
            "https://glama.ai/api/mcp/v1/servers?first=40&query={}",
            urlencoding::encode(search)
        )
    };
    let Ok(v) = curl_json(&url) else { return Vec::new() };
    let Some(servers) = v.get("servers").and_then(|v| v.as_array()) else { return Vec::new() };
    let mut out = Vec::new();
    for s in servers {
        let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() || name.is_empty() {
            continue;
        }
        let page = s.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let repo = s.get("repository").and_then(|r| r.get("url")).and_then(|v| v.as_str()).unwrap_or("");
        // Glama's list endpoint carries no run-command → discovery entry that
        // opens the config page (honest: don't fabricate an install command).
        let install = serde_json::json!({ "type": "link", "url": if page.is_empty() { repo } else { page } });
        out.push(entry(
            id, name,
            s.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "mcp", "glama", 0, "", page, install,
        ));
    }
    out
}

// ── MCP: mcp.so aggregator (HTML scrape, Rust `scraper`) ────────────────────
// mcp.so is a Next.js app with no clean JSON API — its server cards are SSR'd
// as `<a href="/server/<slug>/<author>">` anchors. We fetch the HTML via curl
// and parse the DOM with the pure-Rust `scraper` crate (CSS selectors), which
// survives cosmetic class changes as long as the anchor href scheme holds.
// Scraped entries carry no run-command → discovery-only (opens the page).

fn fetch_mcp_mcpso(search: &str) -> Vec<serde_json::Value> {
    let url = if search.is_empty() {
        "https://mcp.so/".to_string()
    } else {
        format!("https://mcp.so/search?q={}", urlencoding::encode(search))
    };
    let html = match std::process::Command::new("curl")
        .args(["-s", "--max-time", "12", "-A", "8sync-harness-marketplace", &url])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => return Vec::new(),
    };
    let doc = scraper::Html::parse_document(&html);
    let Ok(sel) = scraper::Selector::parse(r#"a[href^="/server/"]"#) else { return Vec::new() };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for a in doc.select(&sel) {
        let Some(href) = a.value().attr("href") else { continue };
        // href = /server/<slug>/<author> → id = "<slug>/<author>"
        let id = href.trim_start_matches("/server/").trim_end_matches('/').to_string();
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        // Card text: first non-empty line = name, the rest = description.
        let text: String = a.text().collect::<Vec<_>>().join(" ");
        let mut parts = text.split_whitespace().collect::<Vec<_>>().join(" ");
        parts.truncate(300);
        let slug = id.split('/').next().unwrap_or(&id);
        out.push(entry(
            &id, slug, parts.trim(), "mcp", "mcp.so", 0, "",
            &format!("https://mcp.so{href}"),
            serde_json::json!({ "type": "link", "url": format!("https://mcp.so{href}") }),
        ));
    }
    out
}

// ── public catalog ──────────────────────────────────────────────────────────

/// Fetch + merge + sort a marketplace catalog. `kind` = "mcp" | "skill".
/// `sort` = "top" (stars/uses desc) | "new" (updated desc). Cached 1h per
/// (kind, search) key so repeated dashboard views don't hammer registries.
pub(crate) fn catalog(root: &Path, kind: &str, search: &str, sort: &str) -> Vec<serde_json::Value> {
    let key = format!(
        "{kind}-{}",
        if search.is_empty() { "_all".into() } else { search.replace(|c: char| !c.is_alphanumeric(), "_") }
    );
    let mut rows = cache_read(root, &key, 3600).unwrap_or_else(|| {
        let fresh = match kind {
            "skill" => fetch_skills_github(search),
            _ => {
                let mut m = fetch_mcp_official(search);
                m.extend(fetch_mcp_smithery(search));
                m.extend(fetch_mcp_glama(search));
                m.extend(fetch_mcp_mcpso(search));
                // Dedup by id (official wins — inserted first).
                let mut seen = std::collections::HashSet::new();
                m.retain(|e| {
                    let id = e.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    seen.insert(id)
                });
                m
            }
        };
        if !fresh.is_empty() {
            cache_write(root, &key, &fresh);
        }
        fresh
    });
    // Sort: "new" by updated date desc, else by stars/uses desc.
    if sort == "new" {
        rows.sort_by(|a, b| {
            b.get("updated").and_then(|v| v.as_str()).unwrap_or("")
                .cmp(a.get("updated").and_then(|v| v.as_str()).unwrap_or(""))
        });
    } else {
        rows.sort_by(|a, b| {
            b.get("stars").and_then(|v| v.as_u64()).unwrap_or(0)
                .cmp(&a.get("stars").and_then(|v| v.as_u64()).unwrap_or(0))
        });
    }
    rows
}
