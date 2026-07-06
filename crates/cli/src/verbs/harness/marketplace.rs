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
        "install": install,  // { type:"stdio", command, args, env:{..} } | { type:"http"|"sse", url, headers:{..} } | { type:"skill", spec }
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

/// Map a package's `registryType` (npm/pypi/oci/nuget/mcpb) — or an explicit
/// `runtimeHint` when present — to the runtime command, its leading args, and
/// whether env vars must be forwarded with `-e NAME` (docker). Returns `None`
/// for package kinds we can't launch directly (e.g. `mcpb` bundles).
fn runtime_for(registry_type: &str, runtime_hint: &str) -> Option<(String, Vec<String>, bool)> {
    let hint = if !runtime_hint.is_empty() {
        runtime_hint.to_string()
    } else {
        match registry_type {
            "npm" => "npx",
            "pypi" => "uvx",
            "oci" => "docker",
            "nuget" => "dnx",
            _ => return None, // mcpb / unknown → not directly runnable
        }
        .to_string()
    };
    Some(match hint.as_str() {
        "npx" => ("npx".into(), vec!["-y".into()], false),
        "uvx" => ("uvx".into(), vec![], false),
        "docker" => ("docker".into(), vec!["run".into(), "-i".into(), "--rm".into()], true),
        "dnx" => ("dnx".into(), vec![], false),
        other => (other.to_string(), vec![], false),
    })
}

/// Render a spec `Argument[]` (NamedArgument | PositionalArgument, each
/// extending Input) into CLI tokens. Named → `--flag` [value]; positional →
/// value. Value = `value` ?? `default` (a named flag with no value stays a bare
/// flag). Best-effort: most registry servers ship no arguments.
fn render_args(arr: Option<&Vec<serde_json::Value>>) -> Vec<String> {
    let mut out = Vec::new();
    let Some(arr) = arr else { return out };
    for a in arr {
        let val = a
            .get("value")
            .and_then(|v| v.as_str())
            .or_else(|| a.get("default").and_then(|v| v.as_str()));
        match a.get("type").and_then(|v| v.as_str()).unwrap_or("positional") {
            "named" => {
                if let Some(name) = a.get("name").and_then(|v| v.as_str()) {
                    out.push(name.to_string());
                    if let Some(v) = val {
                        out.push(v.to_string());
                    }
                }
            }
            _ => {
                if let Some(v) = val {
                    out.push(v.to_string());
                }
            }
        }
    }
    out
}

/// Build a `{NAME: value}` map from a `KeyValueInput[]` (env vars or HTTP
/// headers). Value = `value` ?? `default` ?? "" (an empty required entry is a
/// placeholder the user fills in `mcp.json`). Also returns the names of required
/// entries left empty, for a post-install hint. `additionalProperties` order is
/// irrelevant here — a JSON object is the shape every MCP client expects.
fn kv_map(arr: Option<&Vec<serde_json::Value>>) -> (serde_json::Map<String, serde_json::Value>, Vec<String>) {
    let mut map = serde_json::Map::new();
    let mut required_empty = Vec::new();
    let Some(arr) = arr else { return (map, required_empty) };
    for e in arr {
        let Some(name) = e.get("name").and_then(|v| v.as_str()) else { continue };
        let val = e
            .get("value")
            .and_then(|v| v.as_str())
            .or_else(|| e.get("default").and_then(|v| v.as_str()))
            .unwrap_or("");
        if val.is_empty() && e.get("isRequired").and_then(|v| v.as_bool()).unwrap_or(false) {
            required_empty.push(name.to_string());
        }
        map.insert(name.to_string(), serde_json::Value::String(val.to_string()));
    }
    (map, required_empty)
}

/// Build an `mcp.json` install descriptor from an official-registry `server.json`
/// (schema `2025-12-11`). Honors `registryType`→runtime, `version` pinning,
/// `transport`, `runtimeArguments`/`packageArguments`, and `environmentVariables`
/// (as a proper `{NAME: value}` map). Prefers a runnable package; falls back to a
/// remote HTTP/SSE endpoint.
fn official_install(server: &serde_json::Value) -> Option<serde_json::Value> {
    if let Some(pkgs) = server.get("packages").and_then(|v| v.as_array()) {
        for p in pkgs {
            let id = p.get("identifier").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                continue;
            }
            let registry_type = p.get("registryType").and_then(|v| v.as_str()).unwrap_or("");
            let runtime_hint = p.get("runtimeHint").and_then(|v| v.as_str()).unwrap_or("");
            let version = p.get("version").and_then(|v| v.as_str()).unwrap_or("");
            let transport = p.get("transport");
            let ttype = transport.and_then(|t| t.get("type")).and_then(|v| v.as_str()).unwrap_or("stdio");

            // A package that serves over HTTP/SSE after launch → remote-style entry.
            if ttype == "streamable-http" || ttype == "sse" {
                if let Some(u) = transport.and_then(|t| t.get("url")).and_then(|v| v.as_str()) {
                    let (headers, _) = kv_map(transport.and_then(|t| t.get("headers")).and_then(|v| v.as_array()));
                    let mut o = serde_json::json!({ "type": if ttype == "sse" { "sse" } else { "http" }, "url": u });
                    if !headers.is_empty() {
                        o["headers"] = serde_json::Value::Object(headers);
                    }
                    return Some(o);
                }
            }

            let Some((command, mut args, docker_env_forward)) = runtime_for(registry_type, runtime_hint) else {
                continue;
            };
            args.extend(render_args(p.get("runtimeArguments").and_then(|v| v.as_array())));
            let (env, _required) = kv_map(p.get("environmentVariables").and_then(|v| v.as_array()));
            // docker: forward each env var into the container with `-e NAME`.
            if docker_env_forward {
                for k in env.keys() {
                    args.push("-e".into());
                    args.push(k.clone());
                }
            }
            // Package spec, version-pinned: `id@version` (npm/pypi/nuget) or
            // `id:version` (docker image), unless already tagged.
            let spec = if version.is_empty() {
                id.to_string()
            } else if command == "docker" {
                if id.contains(':') { id.to_string() } else { format!("{id}:{version}") }
            } else if id.strip_prefix('@').unwrap_or(id).contains('@') {
                id.to_string()
            } else {
                format!("{id}@{version}")
            };
            args.push(spec);
            args.extend(render_args(p.get("packageArguments").and_then(|v| v.as_array())));

            let mut o = serde_json::json!({ "type": "stdio", "command": command, "args": args });
            if !env.is_empty() {
                o["env"] = serde_json::Value::Object(env);
            }
            return Some(o);
        }
    }
    if let Some(remotes) = server.get("remotes").and_then(|v| v.as_array()) {
        if let Some(r) = remotes.first() {
            if let Some(u) = r.get("url").and_then(|v| v.as_str()) {
                let t = r.get("type").and_then(|v| v.as_str()).unwrap_or("streamable-http");
                let (headers, _) = kv_map(r.get("headers").and_then(|v| v.as_array()));
                let mut o = serde_json::json!({ "type": if t.contains("sse") { "sse" } else { "http" }, "url": u });
                if !headers.is_empty() {
                    o["headers"] = serde_json::Value::Object(headers);
                }
                return Some(o);
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── runtime_for ─────────────────────────────────────────────────────────

    #[test]
    fn runtime_for_maps_each_registry_type() {
        assert_eq!(runtime_for("npm", ""), Some(("npx".into(), vec!["-y".into()], false)));
        assert_eq!(runtime_for("pypi", ""), Some(("uvx".into(), vec![], false)));
        assert_eq!(
            runtime_for("oci", ""),
            Some(("docker".into(), vec!["run".into(), "-i".into(), "--rm".into()], true))
        );
        assert_eq!(runtime_for("nuget", ""), Some(("dnx".into(), vec![], false)));
    }

    #[test]
    fn runtime_for_unknown_kinds_are_unlaunchable() {
        assert_eq!(runtime_for("mcpb", ""), None);
        assert_eq!(runtime_for("weird", ""), None);
        assert_eq!(runtime_for("", ""), None);
    }

    #[test]
    fn runtime_hint_overrides_registry_type() {
        // npm package but explicit docker hint → docker triple, env forwarding on.
        assert_eq!(
            runtime_for("npm", "docker"),
            Some(("docker".into(), vec!["run".into(), "-i".into(), "--rm".into()], true))
        );
        // A hint even rescues an otherwise-unlaunchable registry type.
        assert_eq!(runtime_for("mcpb", "uvx"), Some(("uvx".into(), vec![], false)));
    }

    // ── render_args ─────────────────────────────────────────────────────────

    #[test]
    fn render_args_named_and_positional_forms() {
        // named with value → two tokens.
        let a = serde_json::json!([{"type":"named","name":"--flag","value":"v"}]);
        assert_eq!(render_args(a.as_array()), vec!["--flag".to_string(), "v".into()]);

        // named without value → bare flag, no trailing token.
        let a = serde_json::json!([{"type":"named","name":"--verbose"}]);
        assert_eq!(render_args(a.as_array()), vec!["--verbose".to_string()]);

        // named falls back to `default` when `value` absent.
        let a = serde_json::json!([{"type":"named","name":"--level","default":"info"}]);
        assert_eq!(render_args(a.as_array()), vec!["--level".to_string(), "info".into()]);

        // positional value → single token.
        let a = serde_json::json!([{"type":"positional","value":"x"}]);
        assert_eq!(render_args(a.as_array()), vec!["x".to_string()]);

        // positional falls back to `default`.
        let a = serde_json::json!([{"type":"positional","default":"d"}]);
        assert_eq!(render_args(a.as_array()), vec!["d".to_string()]);

        // positional with neither value nor default → skipped entirely.
        let a = serde_json::json!([{"type":"positional"}]);
        assert!(render_args(a.as_array()).is_empty());
    }

    #[test]
    fn render_args_none_is_empty() {
        assert!(render_args(None).is_empty());
    }

    // ── kv_map ──────────────────────────────────────────────────────────────

    #[test]
    fn kv_map_value_default_and_required_empty_flagging() {
        let a = serde_json::json!([
            {"name":"WITH_VALUE","value":"v"},
            {"name":"WITH_DEFAULT","default":"d"},
            {"name":"REQ_EMPTY","isRequired":true},
            {"name":"OPT_EMPTY"},
            {"name":"REQ_WITH_VALUE","value":"x","isRequired":true}
        ]);
        let (map, required) = kv_map(a.as_array());
        assert_eq!(map.get("WITH_VALUE").and_then(|v| v.as_str()), Some("v"));
        assert_eq!(map.get("WITH_DEFAULT").and_then(|v| v.as_str()), Some("d"));
        // required + empty → "" placeholder in the map.
        assert_eq!(map.get("REQ_EMPTY").and_then(|v| v.as_str()), Some(""));
        // non-required empty → "" placeholder too.
        assert_eq!(map.get("OPT_EMPTY").and_then(|v| v.as_str()), Some(""));
        assert_eq!(map.get("REQ_WITH_VALUE").and_then(|v| v.as_str()), Some("x"));
        // Only the required-AND-empty entry is flagged: not the optional-empty
        // one, not the required-but-filled one.
        assert_eq!(required, vec!["REQ_EMPTY".to_string()]);
    }

    // ── official_install ────────────────────────────────────────────────────

    #[test]
    fn official_install_npm_env_is_object_not_array() {
        let server = serde_json::json!({
            "packages": [{
                "identifier": "server-everything",
                "registryType": "npm",
                "version": "1.0.0",
                "environmentVariables": [
                    {"name":"FOO","value":"bar"},
                    {"name":"SECRET","isRequired":true}
                ]
            }]
        });
        let v = official_install(&server).expect("npm package should project to an install");
        assert_eq!(v["type"], "stdio");
        assert_eq!(v["command"], "npx");
        assert_eq!(v["args"], serde_json::json!(["-y", "server-everything@1.0.0"]));
        // Regression under test: `env` MUST be a JSON object map, never an
        // array of environment-variable descriptors.
        assert!(v["env"].is_object());
        assert_eq!(v["env"]["FOO"], "bar");
        assert_eq!(v["env"]["SECRET"], "");
    }

    #[test]
    fn official_install_npm_scoped_identifier_version_pinning() {
        // Regression: a scoped npm id (leading `@scope/`) must still get its
        // version pinned — the already-tagged check strips the scope `@` first.
        let server = serde_json::json!({
            "packages": [{
                "identifier": "@scope/pkg",
                "registryType": "npm",
                "version": "1.2.3"
            }]
        });
        let v = official_install(&server).expect("scoped npm package should project to an install");
        assert_eq!(v["args"], serde_json::json!(["-y", "@scope/pkg@1.2.3"]));

        // An already-tagged scoped id (`@scope/pkg@x.y.z`) stays untouched.
        let server = serde_json::json!({
            "packages": [{
                "identifier": "@scope/pkg@9.9.9",
                "registryType": "npm",
                "version": "1.2.3"
            }]
        });
        let v = official_install(&server).expect("tagged scoped npm package should project to an install");
        assert_eq!(v["args"], serde_json::json!(["-y", "@scope/pkg@9.9.9"]));
    }

    #[test]
    fn official_install_pypi_uses_uvx_and_pins_version() {
        let server = serde_json::json!({
            "packages": [{
                "identifier": "mcp-server-git",
                "registryType": "pypi",
                "version": "2.0.0"
            }]
        });
        let v = official_install(&server).expect("pypi package should project to an install");
        assert_eq!(v["command"], "uvx");
        assert_eq!(v["args"], serde_json::json!(["mcp-server-git@2.0.0"]));
        // No env vars → no `env` key emitted at all.
        assert!(v.get("env").is_none());
    }

    #[test]
    fn official_install_oci_docker_forwards_env_and_appends_package_args() {
        let server = serde_json::json!({
            "packages": [{
                "identifier": "mcp/everything",
                "registryType": "oci",
                "version": "1.2.3",
                "environmentVariables": [{"name":"API_KEY","isRequired":true}],
                "packageArguments": [{"type":"positional","value":"--verbose"}]
            }]
        });
        let v = official_install(&server).expect("oci package should project to an install");
        assert_eq!(v["command"], "docker");
        // Order contract: run/-i/--rm, then `-e NAME` per env var, then the
        // version-pinned image `id:version`, then packageArguments AFTER it.
        assert_eq!(
            v["args"],
            serde_json::json!(["run", "-i", "--rm", "-e", "API_KEY", "mcp/everything:1.2.3", "--verbose"])
        );
        assert!(v["env"].is_object());
        assert_eq!(v["env"]["API_KEY"], "");
    }

    #[test]
    fn official_install_package_streamable_http_returns_remote() {
        let server = serde_json::json!({
            "packages": [{
                "identifier": "some-server",
                "registryType": "npm",
                "transport": {"type":"streamable-http","url":"https://example.com/mcp"}
            }]
        });
        let v = official_install(&server).expect("http-transport package should project to a remote");
        assert_eq!(v["type"], "http");
        assert_eq!(v["url"], "https://example.com/mcp");
        // Remote descriptor, not stdio: no launch command/args.
        assert!(v.get("command").is_none());
        assert!(v.get("args").is_none());
    }

    #[test]
    fn official_install_remotes_only_maps_headers_to_object() {
        let server = serde_json::json!({
            "remotes": [{
                "type":"sse",
                "url":"https://remote.example.com/sse",
                "headers":[{"name":"Authorization","value":"Bearer xyz"}]
            }]
        });
        let v = official_install(&server).expect("remote-only server should project to a remote");
        assert_eq!(v["type"], "sse");
        assert_eq!(v["url"], "https://remote.example.com/sse");
        assert!(v["headers"].is_object());
        assert_eq!(v["headers"]["Authorization"], "Bearer xyz");
    }

    #[test]
    fn official_install_mcpb_only_without_remote_is_none() {
        let server = serde_json::json!({
            "packages": [{
                "identifier": "some-bundle",
                "registryType": "mcpb"
            }]
        });
        assert!(official_install(&server).is_none());
    }
}
