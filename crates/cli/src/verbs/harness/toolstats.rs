//! `8sync harness toolstats` — track omp tool-call usage for the current project
//! in SQLite, exposing the **optimizer** (codegraph / codebase-memory-mcp / serena /
//! headroom) vs **fallback** (grep / read / search / find / glob) ratio + per-tool
//! failures. The source of truth is omp's own session JSONL — what the agent
//! *actually* called — so you can see whether the token-optimization stack (STEP 0)
//! is being used, and catch failing tool calls (e.g. a dead MCP server).
//!
//! DB: `<repo>/.cache/8sync/toolstats.db` (gitignored). Idempotent: re-ingest
//! is keyed on (session, seq), so re-running only adds new calls.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

use crate::{env_detect, ui, verbs::skill::discover};

pub(crate) fn harness_toolstats(env: &env_detect::Env) -> Result<()> {
    ui::header("8sync harness toolstats");
    let root = discover::detect_current_project_root()
        .context("not inside a project — cd into your repo first")?;
    let slug = session_slug(&env.home, &root);
    let sess_dir = env.home.join(format!(".omp/agent/sessions/{}", slug));

    let db_path = root.join(".cache/8sync/toolstats.db");
    if let Some(p) = db_path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let conn = Connection::open(&db_path).context("open toolstats.db")?;
    init_schema(&conn)?;

    if !sess_dir.is_dir() {
        ui::warn(&format!(
            "no omp sessions yet for this project ({}). Run omp here, then re-run.",
            sess_dir.display()
        ));
        return Ok(());
    }
    let (new_calls, n_sessions) = ingest(&conn, &sess_dir)?;
    ui::ok(&format!(
        "tracked {} call(s) from {} session(s) → {}",
        new_calls,
        n_sessions,
        db_path.display()
    ));
    report(&conn, &root)
}

/// `~/.omp/agent/sessions/<slug>` for a project root (mirrors the web dashboard).
fn session_slug(home: &Path, root: &Path) -> String {
    match root.strip_prefix(home) {
        Ok(rel) => format!("-{}", rel.to_string_lossy().replace('/', "-")),
        Err(_) => format!("-{}", root.to_string_lossy().trim_start_matches('/').replace('/', "-")),
    }
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS calls (
            session  TEXT    NOT NULL,
            seq      INTEGER NOT NULL,
            ts       INTEGER NOT NULL,
            tool     TEXT    NOT NULL,
            category TEXT    NOT NULL,  -- optimizer | fallback | edit | other
            detail   TEXT    NOT NULL,  -- codegraph|serena|cbm|headroom, or the tool name
            ok       INTEGER NOT NULL,  -- 1 success, 0 error
            PRIMARY KEY (session, seq)
        );",
    )?;
    Ok(())
}

/// Parse each `<slug>/*.jsonl` and upsert its tool calls. Returns (new rows, sessions).
fn ingest(conn: &Connection, sess_dir: &Path) -> Result<(usize, usize)> {
    let mut new_rows = 0usize;
    let mut n_sessions = 0usize;
    conn.execute("DELETE FROM calls", [])?; // rebuild from current sessions (re-categorize)
    let rd = std::fs::read_dir(sess_dir)?;
    for ent in rd.flatten() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let session = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let ts = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        n_sessions += 1;

        // First pass: collect tool calls (in order) + a toolCallId → isError map.
        let mut calls: Vec<(String, String, String)> = Vec::new(); // (id, name, command)
        let mut errors: HashMap<String, bool> = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if v.get("type").and_then(|t| t.as_str()) != Some("message") {
                continue;
            }
            let m = match v.get("message") {
                Some(m) => m,
                None => continue,
            };
            let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("");
            if role == "toolResult" {
                if let Some(id) = m.get("toolCallId").and_then(|i| i.as_str()) {
                    let is_err = m.get("isError").and_then(|e| e.as_bool()).unwrap_or(false);
                    errors.insert(id.to_string(), is_err);
                }
                continue;
            }
            if let Some(content) = m.get("content").and_then(|c| c.as_array()) {
                for c in content {
                    if c.get("type").and_then(|t| t.as_str()) != Some("toolCall") {
                        continue;
                    }
                    let id = c.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                    let name = c.get("name").and_then(|n| n.as_str()).unwrap_or("?").to_string();
                    let cmd = c
                        .get("arguments")
                        .and_then(|a| a.get("command").or_else(|| a.get("cmd")))
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    calls.push((id, name, cmd));
                }
            }
        }

        // Second pass: categorize + upsert.
        let tx = conn.unchecked_transaction()?;
        for (seq, (id, name, cmd)) in calls.iter().enumerate() {
            let (category, detail) = categorize(name, cmd);
            let ok = !errors.get(id).copied().unwrap_or(false);
            let changed = tx.execute(
                "INSERT OR IGNORE INTO calls (session, seq, ts, tool, category, detail, ok)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![session, seq as i64, ts, name, category, detail, ok as i64],
            )?;
            new_rows += changed;
        }
        tx.commit()?;
    }
    Ok((new_rows, n_sessions))
}

/// Map a tool call to (category, detail). codegraph runs via `bash`, so its
/// command string is inspected; serena/cbm/headroom are MCP tools.
fn categorize(name: &str, cmd: &str) -> (&'static str, String) {
    const SERENA: &[&str] = &[
        "find_symbol", "find_referencing_symbols", "find_implementations", "find_declaration",
        "get_symbols_overview", "get_diagnostics_for_file", "get_diagnostics_for_symbol",
        "rename_symbol", "replace_symbol_body", "insert_after_symbol", "insert_before_symbol",
        "safe_delete_symbol",
    ];
    const CBM: &[&str] = &[
        "search_graph", "semantic_query", "trace_path", "get_architecture", "query_graph",
        "get_code_snippet", "detect_changes", "manage_adr",
    ];
    const SEARCH: &[&str] = &["grep", "glob", "search", "find"];
    let n = name.to_lowercase();
    if name == "bash" && cmd.contains("codegraph") {
        return ("optimizer", "codegraph".into());
    }
    if n.contains("serena") || SERENA.contains(&name) {
        return ("optimizer", "serena".into());
    }
    if n.contains("codebase") || n.contains("cbm") || CBM.contains(&name) {
        return ("optimizer", "cbm".into());
    }
    if n.contains("headroom") {
        return ("compress", "headroom".into()); // auto-compression, not a lookup tool
    }
    if name == "read" {
        return ("read", "read".into()); // often legit read-before-edit
    }
    if SEARCH.contains(&name) {
        return ("search", name.to_string()); // raw search the optimizer should replace
    }
    if name == "edit" || name == "write" {
        return ("edit", name.to_string());
    }
    ("other", name.to_string())
}

fn report(conn: &Connection, root: &Path) -> Result<()> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM calls", [], |r| r.get(0))?;
    if total == 0 {
        ui::info("no tool calls tracked yet.");
        return Ok(());
    }
    let cat = |c: &str| -> (i64, i64) {
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(1-ok),0) FROM calls WHERE category=?1",
            [c],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, 0))
    };
    let (opt, opt_fail) = cat("optimizer");
    let (search, search_fail) = cat("search");
    let (read, _) = cat("read");
    let (compress, _) = cat("compress");
    let (edit, _) = cat("edit");
    let (other, _) = cat("other");

    // The actionable ratio: of code-LOOKUP calls (optimizer + raw search), how many
    // used the token-optimized engines? read/edit/bash aren't lookups → excluded.
    let lookup = opt + search;
    let lookup_pct = if lookup > 0 { 100.0 * opt as f64 / lookup as f64 } else { 0.0 };

    ui::step(&format!("project: {}  ·  {} tracked tool calls", root.display(), total));
    println!();
    println!("  CODE-LOOKUP calls (optimizer + raw-search) = {}", lookup);
    println!("  ┌ OPTIMIZER  (codegraph·cbm·serena)   {:>6}   {:>5.1}% of lookups   {} fail", opt, lookup_pct, opt_fail);
    for d in ["codegraph", "cbm", "serena"] {
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM calls WHERE detail=?1", [d], |r| r.get(0))
            .unwrap_or(0);
        let flag = if n == 0 { "  ← never called" } else { "" };
        println!("  │    {:<10} {:>6}{}", d, n, flag);
    }
    println!("  └ RAW SEARCH (grep·search·find·glob)  {:>6}   {:>5.1}% of lookups   {} fail", search, 100.0 - lookup_pct, search_fail);
    for (d, n) in detail_counts(conn, "search")? {
        println!("       {:<10} {:>6}", d, n);
    }
    println!();
    println!("  read (read-before-edit, ranges)  {:>6}   (often legit — not shamed)", read);
    println!("  edit / write                     {:>6}", edit);
    println!("  headroom (auto-compress)         {:>6}   (background, not agent-called)", compress);
    println!("  other (bash/todo/job/…)          {:>6}", other);
    println!();

    // Failing tools (any category) — fix these (e.g. a dead MCP server).
    let mut fails = conn.prepare(
        "SELECT detail, COUNT(*) FROM calls WHERE ok=0 GROUP BY detail ORDER BY 2 DESC LIMIT 8",
    )?;
    let frows: Vec<(String, i64)> = fails
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|x| x.ok())
        .collect();
    if !frows.is_empty() {
        let list: Vec<String> = frows.iter().map(|(t, n)| format!("{}×{}", t, n)).collect();
        ui::info(&format!("failing calls: {}", list.join(", ")));
    }

    // Verdict on the code-lookup ratio (the actionable number).
    if lookup == 0 {
        ui::info("no code-lookup calls yet.");
    } else if opt == 0 {
        ui::warn("0% of code-lookups used the optimizer — every where-is/who-calls went through raw grep/search.");
        ui::info("check `8sync doctor` (codegraph/cbm/serena reachable?). serena was fixed in 0.29.3 — re-measure after omp usage.");
    } else if lookup_pct < 50.0 {
        ui::warn(&format!("optimizer = {:.0}% of code-lookups — STEP-0 under-used; agent still defaults to raw search.", lookup_pct));
    } else {
        ui::ok(&format!("optimizer = {:.0}% of code-lookups — STEP-0 is working.", lookup_pct));
    }
    Ok(())
}

fn detail_counts(conn: &Connection, category: &str) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT detail, COUNT(*) FROM calls WHERE category=?1 GROUP BY detail ORDER BY 2 DESC",
    )?;
    let rows = stmt
        .query_map([category], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .filter_map(|x| x.ok())
        .collect();
    Ok(rows)
}
