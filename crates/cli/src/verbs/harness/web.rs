//! `8sync harness web` — local dashboard. axum serves the embedded Vite FE
//! (web/dist via rust-embed) + a JSON API over the harness state. Bound to
//! 127.0.0.1 only (single-user local tool). The API reuses the same data fns
//! as the CLI (`bench_metrics`, `eval_project_data`) and the skill registry
//! helpers (`discover::read_registry`/`write_registry`).
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::{assets, ui, verbs::skill::discover::{self, detect_current_project_root}};

#[derive(Clone)]
struct Ctx {
    home: std::path::PathBuf,
}

const MEMORY_ALLOWLIST: &[&str] = &["STATE", "KNOWLEDGE", "PLAYBOOKS", "DECISIONS", "PROJECT", "NOTES"];

pub(crate) fn harness_web(home: &std::path::Path, port: u16, no_open: bool) -> Result<()> {
    let ctx = Arc::new(Ctx { home: home.to_path_buf() });
    apply_active_project(home); // honor last-activated project across restarts
    let app = api_routes()
        .merge(Router::new().fallback(static_handler))
        .with_state(ctx);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    ui::ok(&format!("8sync harness web → http://127.0.0.1:{}  (Ctrl+C to stop)", port));
    if !no_open {
        let _ = std::process::Command::new("xdg-open")
            .arg(format!("http://127.0.0.1:{}", port))
            .spawn();
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

/// Honor the last-activated project (web-session.json) by chdir-ing into it, so
/// every cwd-based handler (detect_current_project_root) resolves to it. The
/// dashboard is single-user/local → a process-global cwd is the simplest reliable
/// switch and it persists across server restarts.
fn apply_active_project(home: &std::path::Path) {
    let sess = std::fs::read_to_string(home.join(".config/8sync/web-session.json")).unwrap_or_default();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&sess) {
        if let Some(p) = v.get("project").and_then(|x| x.as_str()) {
            if std::path::Path::new(p).is_dir() {
                let _ = std::env::set_current_dir(p);
            }
        }
    }
}


type ApiErr = (StatusCode, String);

fn api_routes() -> Router<Arc<Ctx>> {
    Router::new()
        .route("/api/state", get(api_state))
        .route("/api/skills", get(api_skills))
        .route("/api/skills/toggle", post(api_skill_toggle))
        .route("/api/skills/add", post(api_skill_add))
        .route("/api/skills/update", post(api_skill_update))
        .route("/api/engines", get(api_engines))
        .route("/api/engine", get(api_engine))
        .route("/api/bench", get(api_bench))
        .route("/api/eval", get(api_eval))
        .route("/api/memory/:file", get(api_memory_get).post(api_memory_set))
        .route("/api/workspaces", get(api_workspaces))
        .route("/api/workspaces/activate", post(api_workspace_activate))
        .route("/api/team", get(api_team))
        .route("/api/submodules", get(api_submodules))
        .route("/api/submodules/add", post(api_submodule_add))
        .route("/api/submodules/pull", post(api_submodule_pull))
        .route("/api/submodules/remove", post(api_submodule_remove))
        .route("/api/context", get(api_context))
        .route("/api/mcp", get(api_mcp))
        .route("/api/rules", get(api_rules))
        .route("/api/rules/add", post(api_rule_add))
        .route("/api/rules/delete", post(api_rule_delete))
        .route("/api/models", get(api_models_get).post(api_models_set))
        .route("/api/projects", get(api_projects))
        .route("/api/workflows", get(api_workflows))
        .route("/api/workflows/templates", get(api_workflow_templates))
        .route("/api/workflows/:name", get(api_workflow_get).post(api_workflow_save).delete(api_workflow_delete))
        .route("/api/workflows/:name/export", post(api_workflow_export))
}

async fn api_state(State(ctx): State<Arc<Ctx>>) -> Result<Json<serde_json::Value>, ApiErr> {
    let root = detect_current_project_root().unwrap_or_default();
    let profile = std::env::var("OMP_PROFILE").unwrap_or_else(|_| "default".to_string());
    let state_md = std::fs::read_to_string(root.join("agents/STATE.md")).unwrap_or_default();
    Ok(Json(serde_json::json!({
        "project": root.display().to_string(),
        "profile": profile,
        "state_md": state_md,
    })))
}

async fn api_skills(State(ctx): State<Arc<Ctx>>) -> Result<Json<Vec<serde_json::Value>>, ApiErr> {
    let root = detect_current_project_root().unwrap_or_default();
    let reg_g = discover::read_registry(&ctx.home.join(".config/8sync/skills.toml"));
    let proj_man = root.join("agents/skills.toml");
    let reg_p = if proj_man.exists() { discover::read_registry(&proj_man) } else { Default::default() };
    let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for base in [ctx.home.join(".omp/skills"), root.join("agents/skills")] {
        if let Ok(entries) = std::fs::read_dir(&base) {
            for e in entries.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    names.insert(n.to_string());
                }
            }
        }
    }
    let mut out = Vec::new();
    for name in names {
        let entry = reg_p.get(&name).or_else(|| reg_g.get(&name));
        let tier = match entry.and_then(|e| e.when.as_deref()) {
            Some("always") => "always",
            Some("on-demand") => "on-demand",
            _ => "off",
        };
        out.push(serde_json::json!({
            "name": name,
            "tier": tier,
            "source": entry.map(|e| e.src.clone()).unwrap_or_default(),
            "global": ctx.home.join(format!(".omp/skills/{}", name)).exists(),
            "local": root.join(format!("agents/skills/{}", name)).exists(),
        }));
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct ToggleBody {
    name: String,
    when: String,
}

async fn api_skill_toggle(
    State(ctx): State<Arc<Ctx>>,
    Json(body): Json<ToggleBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    if !matches!(body.when.as_str(), "always" | "on-demand" | "off") {
        return Err((StatusCode::BAD_REQUEST, "`when` must be always|on-demand|off".into()));
    }
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let path = root.join("agents/skills.toml");
    let mut reg = discover::read_registry(&path);
    let reg_g = discover::read_registry(&ctx.home.join(".config/8sync/skills.toml"));
    if body.when == "off" {
        reg.remove(&body.name);
    } else {
        let src = reg
            .get(&body.name)
            .map(|e| e.src.clone())
            .or_else(|| reg_g.get(&body.name).map(|e| e.src.clone()))
            .unwrap_or_else(|| format!("builtin:{}", body.name));
        reg.insert(
            body.name.clone(),
            discover::SkillEntry { src, when: Some(body.when.clone()), rev: None },
        );
    }
    discover::write_registry(&path, &reg).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "name": body.name, "tier": body.when })))
}

async fn api_memory_get(
    Path(file): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    if !MEMORY_ALLOWLIST.contains(&file.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "file not in allowlist".into()));
    }
    let content = std::fs::read_to_string(root.join(format!("agents/{}.md", file)))
        .map_err(|_| (StatusCode::NOT_FOUND, "file missing".into()))?;
    Ok(Json(serde_json::json!({ "file": file, "content": content })))
}

#[derive(Deserialize)]
struct MemoryBody {
    content: String,
}

async fn api_memory_set(
    Path(file): Path<String>,
    Json(body): Json<MemoryBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    if !MEMORY_ALLOWLIST.contains(&file.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "file not in allowlist".into()));
    }
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let target = root.join(format!("agents/{}.md", file));
    if let Some(p) = target.parent() {
        std::fs::create_dir_all(p).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    std::fs::write(&target, body.content)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn api_engines(State(ctx): State<Arc<Ctx>>) -> Json<serde_json::Value> {
    let ver = |b: &str| crate::env_detect::cmd_version(b, &["--version"]).unwrap_or_default();
    let eng = |b: &str| serde_json::json!({ "present": which::which(b).is_ok(), "version": ver(b).trim() });
    let cfg = std::fs::read_to_string(ctx.home.join(".omp/agent/config.yml")).unwrap_or_default();
    // serena runs via `uvx` (no `serena` binary on PATH), so `which serena`
    // always reports it off. Detect it instead by registration in mcp.json
    // (`mcpServers.serena`) AND a uv/uvx runner being present. Version is
    // best-effort (the uvx-launched server exposes none here) → left empty.
    let mcp_raw = std::fs::read_to_string(ctx.home.join(".omp/agent/mcp.json")).unwrap_or_default();
    let serena_registered = serde_json::from_str::<serde_json::Value>(&mcp_raw)
        .ok()
        .and_then(|v| v.get("mcpServers").and_then(|m| m.get("serena")).cloned())
        .is_some();
    let uv_present = which::which("uvx").is_ok() || which::which("uv").is_ok();
    let serena = serde_json::json!({
        "present": serena_registered && uv_present,
        "version": "",
        "registered": serena_registered,
        "runner": uv_present,
    });
    Json(serde_json::json!({
        "codegraph": eng("codegraph"),
        "cbm": eng("codebase-memory-mcp"),
        "headroom": eng("headroom"),
        "serena": serena,
        "mnemopi_on": cfg.contains("backend: mnemopi"),
    }))
}

/// Live `/auto` engine run — the REAL gsd-pi state machine the engine drives at
/// `<root>/.cache/8sync/engine/state.json` (NOT demo data). Read-only mirror of
/// the terminal board: goal · progress · slice/task tree · current task. Returns
/// `{active:false}` when no run exists. The engine (driven by `/auto` in omp) is
/// the source of truth; the dashboard displays it, never bypasses its verify gate.
async fn api_engine(State(_ctx): State<Arc<Ctx>>) -> Json<serde_json::Value> {
    let root = match detect_current_project_root() {
        Some(r) => r,
        None => return Json(serde_json::json!({ "active": false })),
    };
    let raw = match std::fs::read_to_string(root.join(".cache/8sync/engine/state.json")) {
        Ok(s) => s,
        Err(_) => return Json(serde_json::json!({ "active": false })),
    };
    let state: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Json(serde_json::json!({ "active": false })),
    };
    // Mirror 8sync-engine.ts counts()/findNext() over slices[].tasks[].
    let (mut total, mut done, mut blocked) = (0u64, 0u64, 0u64);
    let mut current = serde_json::Value::Null;
    if let Some(slices) = state.get("slices").and_then(|v| v.as_array()) {
        for s in slices {
            let Some(tasks) = s.get("tasks").and_then(|v| v.as_array()) else { continue };
            for t in tasks {
                total += 1;
                match t.get("status").and_then(|v| v.as_str()) {
                    Some("done") => done += 1,
                    Some("blocked") => blocked += 1,
                    Some("pending") | Some("in_progress") if current.is_null() => {
                        current = serde_json::json!({
                            "slice": s.get("title").cloned().unwrap_or(serde_json::Value::Null),
                            "task": t.get("title").cloned().unwrap_or(serde_json::Value::Null),
                            "status": t.get("status").cloned().unwrap_or(serde_json::Value::Null),
                        });
                    }
                    _ => {}
                }
            }
        }
    }
    Json(serde_json::json!({
        "active": true,
        "goal": state.get("goal").cloned().unwrap_or(serde_json::Value::Null),
        "updatedAt": state.get("updatedAt").cloned().unwrap_or(serde_json::Value::Null),
        "total": total,
        "done": done,
        "blocked": blocked,
        "current": current,
        "slices": state.get("slices").cloned().unwrap_or(serde_json::json!([])),
    }))
}

async fn api_bench(State(ctx): State<Arc<Ctx>>) -> Result<Json<super::bench::BenchMetrics>, ApiErr> {
    super::bench::bench_metrics(&ctx.home)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "not in a project".into()))
}

async fn api_eval(State(ctx): State<Arc<Ctx>>) -> Result<Json<super::eval::EvalData>, ApiErr> {
    super::eval::eval_project_data(&ctx.home)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "not in a project".into()))
}

async fn static_handler(uri: Uri) -> Response {
    let p = uri.path().trim_start_matches('/');
    let p = if p.is_empty() { "index.html" } else { p };
    if let Some(bytes) = assets::web_asset(p) {
        return ([(header::CONTENT_TYPE, mime_for(p))], bytes).into_response();
    }
    if let Some(bytes) = assets::web_asset("index.html") {
        return ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], bytes).into_response();
    }
    (StatusCode::NOT_FOUND, "8sync web FE not built — run `pnpm --dir web build`").into_response()
}

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

// ---- Phase C: workspaces / team / submodules / skill install ----

async fn api_workspaces(State(ctx): State<Arc<Ctx>>) -> Json<serde_json::Value> {
    let mut profiles = vec!["default".to_string()];
    if let Ok(entries) = std::fs::read_dir(ctx.home.join(".omp/profiles")) {
        for e in entries.flatten() {
            if let Some(n) = e.file_name().to_str() {
                if !profiles.iter().any(|p| p == n) {
                    profiles.push(n.to_string());
                }
            }
        }
    }
    let project = detect_current_project_root()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let sess = std::fs::read_to_string(ctx.home.join(".config/8sync/web-session.json"))
        .unwrap_or_default();
    Json(serde_json::json!({ "profiles": profiles, "project": project, "session": sess }))
}

#[derive(Deserialize)]
struct ActivateBody {
    profile: Option<String>,
    project: Option<String>,
}

async fn api_workspace_activate(
    State(ctx): State<Arc<Ctx>>,
    Json(body): Json<ActivateBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    // Advisory: records the chosen profile/project. Actual isolation happens
    // when omp runs with `--profile <name>` in that project dir.
    let path = ctx.home.join(".config/8sync/web-session.json");
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    let cur = std::fs::read_to_string(&path).unwrap_or_default();
    let mut obj: serde_json::Value = serde_json::from_str(&cur).unwrap_or(serde_json::json!({}));
    if !obj.is_object() {
        obj = serde_json::json!({});
    }
    if let Some(p) = body.profile {
        obj["profile"] = serde_json::Value::String(p);
    }
    if let Some(p) = body.project {
        obj["project"] = serde_json::Value::String(p);
    }
    std::fs::write(&path, serde_json::to_string_pretty(&obj).unwrap_or_default())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    apply_active_project(&ctx.home); // chdir now so every handler switches project
    Ok(Json(obj))
}

async fn api_team(State(ctx): State<Arc<Ctx>>) -> Result<Json<serde_json::Value>, ApiErr> {
    // Static subagent roster (omp task types) + per-project readiness.
    let roster = serde_json::json!([
        { "type": "explore", "role": "scout", "skills": "codegraph, cbm" },
        { "type": "plan", "role": "architect", "skills": "planning-*, spec-driven-*" },
        { "type": "reviewer", "role": "code review", "skills": "code-review-and-quality, senior-security" },
        { "type": "oracle", "role": "2nd opinion / debug", "skills": "debugging, performance" },
        { "type": "designer", "role": "UI/UX", "skills": "impeccable, taste, senior-frontend" },
        { "type": "librarian", "role": "research", "skills": "agent-reach, deep-research" },
        { "type": "task", "role": "implementer", "skills": "full-flow, tdd" },
        { "type": "quick_task", "role": "mechanical", "skills": "—" }
    ]);
    let readiness = super::eval::eval_project_data(&ctx.home);
    Ok(Json(serde_json::json!({ "roster": roster, "readiness": readiness })))
}

async fn api_submodules(State(_ctx): State<Arc<Ctx>>) -> Result<Json<Vec<serde_json::Value>>, ApiErr> {
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    Ok(Json(parse_gitmodules(&root)))
}

#[derive(Deserialize)]
struct SubmoduleBody {
    url: String,
    path: Option<String>,
}
#[derive(Deserialize)]
struct SubmodulePathBody {
    path: String,
}

async fn api_submodule_add(
    State(_ctx): State<Arc<Ctx>>,
    Json(body): Json<SubmoduleBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let path = body.path.unwrap_or_else(|| format!("reference/{}", basename(&body.url)));
    git(&root, &["submodule", "add", "-f", "--depth", "1", &body.url, &path])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({ "ok": true, "path": path })))
}

async fn api_submodule_pull(
    State(_ctx): State<Arc<Ctx>>,
    Json(body): Json<SubmodulePathBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    git(&root, &["submodule", "update", "--init", "--remote", &body.path])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn api_submodule_remove(
    State(_ctx): State<Arc<Ctx>>,
    Json(body): Json<SubmodulePathBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    git(&root, &["submodule", "deinit", "-f", &body.path])
        .and_then(|_| git(&root, &["rm", "-f", &body.path]))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
struct SkillSpecBody {
    spec: String,
    name: Option<String>,
}

async fn api_skill_add(
    State(_ctx): State<Arc<Ctx>>,
    Json(body): Json<SkillSpecBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    skill_cmd(&["skill", "add", &body.spec])
}

async fn api_skill_update(
    State(ctx): State<Arc<Ctx>>,
    Json(body): Json<SkillSpecBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let _ = &ctx; // touched for State symmetry
    let args: Vec<String> = match &body.name {
        Some(n) => vec!["skill".into(), "update".into(), n.clone()],
        None => vec!["skill".into(), "update".into()],
    };
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    skill_cmd(&args_ref)
}

/// Shell out to the 8sync binary itself for skill add/update (reuses the
/// tested CLI path rather than duplicating install logic in-process).
fn skill_cmd(args: &[&str]) -> Result<Json<serde_json::Value>, ApiErr> {
    let exe = std::env::current_exe().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let out = std::process::Command::new(&exe)
        .args(args)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if out.status.success() {
        Ok(Json(serde_json::json!({ "ok": true, "log": combined })))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, combined))
    }
}

fn git(root: &std::path::Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C").arg(root).args(args);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

fn basename(url: &str) -> String {
    url.rsplit(['/', ':']).next().unwrap_or(url).trim_end_matches(".git").to_string()
}

fn parse_gitmodules(root: &std::path::Path) -> Vec<serde_json::Value> {
    let s = std::fs::read_to_string(root.join(".gitmodules")).unwrap_or_default();
    let mut out = Vec::new();
    let mut name = String::new();
    let mut path = String::new();
    let mut url = String::new();
    let flush = |out: &mut Vec<serde_json::Value>, name: &str, path: &str, url: &str| {
        if !name.is_empty() {
            let dir = root.join(path);
            let initialized = dir.exists()
                && std::fs::read_dir(&dir).map(|mut it| it.next().is_some()).unwrap_or(false);
            out.push(serde_json::json!({
                "name": name, "path": path, "url": url, "initialized": initialized,
            }));
        }
    };
    for line in s.lines() {
        let l = line.trim();
        if l.starts_with("[submodule") {
            flush(&mut out, &name, &path, &url);
            name.clear();
            path.clear();
            url.clear();
            let start = match l.find('"') { Some(i) => i + 1, None => continue };
            let end = match l[start..].find('"') { Some(i) => start + i, None => continue };
            name = l[start..end].to_string();
        } else if let Some(v) = l.strip_prefix("path = ") {
            path = v.to_string();
        } else if let Some(v) = l.strip_prefix("url = ") {
            url = v.to_string();
        }
    }
    flush(&mut out, &name, &path, &url);
    out
}

// ---- Context tracker / MCP viz / Rules CRUD ----

/// Current omp session context usage for the active project. Reads the newest
/// session JSONL's last `contextSnapshot.promptTokens`. Window is assumed
/// (configurable later); threshold comes from compaction.thresholdPercent.
async fn api_context(State(ctx): State<Arc<Ctx>>) -> Json<serde_json::Value> {
    let root = detect_current_project_root();
    let home = &ctx.home;
    let model = read_model(home);
    let (window, assumed) = model_window(&model);
    let cfg_raw = std::fs::read_to_string(home.join(".omp/agent/config.yml")).unwrap_or_default();
    let threshold_pct = parse_threshold(&cfg_raw);
    let (used, last_compact_at, session, session_age) = match (&root, session_slug(home, root.as_deref())) {
        (Some(_r), Some(slug)) => {
            let dir = home.join(format!(".omp/agent/sessions/{}", slug));
            let newest = newest_session(&dir);
            let (used, last_compact) = newest.as_ref().and_then(|p| analyze_context(p)).unwrap_or((0, None));
            let session = newest.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("").to_string();
            let age = newest
                .as_ref()
                .and_then(|p| std::fs::metadata(p).ok())
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (used, last_compact, session, age)
        }
        _ => (0u64, None, String::new(), 0u64),
    };
    let pct = if window > 0 { used * 100 / window } else { 0 };
    let compact_at = window * threshold_pct / 100;
    let will_compact = compact_at > 0 && used >= compact_at;
    // A session not written for >10 min is a stored snapshot, not a live run. omp
    // compacts on the NEXT turn (or a safe mid-turn point), so a paused/ended session
    // legitimately sits above threshold until resumed — it is not "stuck".
    let stale = session_age > 600;
    let note = if assumed {
        "window is assumed (model not found in `omp models`); the % is approximate"
    } else if stale {
        "compaction is turn-triggered (omp compacts after a completed turn / safe mid-turn point when usage exceeds threshold, not as a hard cap). This session is idle/ended, so it sits above threshold until resumed — run /compact or continue to compact now. last_compact_at is observed history and may predate the current threshold."
    } else {
        "compaction is turn-triggered: omp compacts after a completed turn / safe mid-turn point once usage exceeds the threshold — not a hard cap. last_compact_at is observed history and may predate the current threshold."
    };
    Json(serde_json::json!({
        // explicit FE contract (camelCase)
        "usedTok": used,
        "windowTok": window,
        "pct": pct,
        "thresholdPct": threshold_pct,
        "willCompact": will_compact,
        "assumed": assumed,
        "stale": stale,
        "sessionAgeSecs": session_age,
        "model": model,
        "project": root.map(|p| p.display().to_string()).unwrap_or_default(),
        "session": session,
        // retained legacy/diagnostic fields
        "used": used,
        "window": window,
        "threshold_pct": threshold_pct,
        "compact_at": compact_at,
        "over_threshold": will_compact,
        "last_compact_at": last_compact_at,
        "compaction_observed": last_compact_at.is_some(),
        "note": note,
    }))
}

/// Per-model context window (tokens) parsed once from `omp models`, so the % is
/// real instead of a hardcoded 1M. Falls back to (1M, assumed=true) when the model
/// isn't in the catalog or `omp` is unavailable.
static MODEL_WINDOWS: std::sync::LazyLock<std::collections::HashMap<String, u64>> =
    std::sync::LazyLock::new(build_model_windows);

fn model_window(model: &str) -> (u64, bool) {
    // "zai/glm-5.2:high" → "glm-5.2"
    let bare = model.rsplit('/').next().unwrap_or(model);
    let bare = bare.split(':').next().unwrap_or(bare);
    match MODEL_WINDOWS.get(bare) {
        Some(&w) if w > 0 => (w, false),
        _ => (1_000_000, true),
    }
}

fn build_model_windows() -> std::collections::HashMap<String, u64> {
    let mut map = std::collections::HashMap::new();
    let Ok(out) = std::process::Command::new("omp").arg("models").output() else {
        return map;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        // table rows: `│ <id> │ <context> │ <max-out> │ ...`
        let cols: Vec<&str> = line.split('│').map(|s| s.trim()).collect();
        if cols.len() < 3 {
            continue;
        }
        let id = cols[1];
        if id.is_empty() || id == "model" || id.contains(' ') {
            continue;
        }
        if let Some(tok) = parse_token_count(cols[2]) {
            map.insert(id.to_string(), tok);
        }
    }
    map
}

/// "1M" → 1_000_000, "205K" → 205_000, "131072" → 131072, "1.5M" → 1_500_000.
fn parse_token_count(s: &str) -> Option<u64> {
    let s = s.trim();
    let last = s.chars().last()?;
    let (num, mult) = match last {
        'K' | 'k' => (&s[..s.len() - 1], 1_000f64),
        'M' | 'm' => (&s[..s.len() - 1], 1_000_000f64),
        'G' | 'g' => (&s[..s.len() - 1], 1_000_000_000f64),
        c if c.is_ascii_digit() => (s, 1f64),
        _ => return None,
    };
    let f: f64 = num.trim().parse().ok()?;
    if f <= 0.0 {
        return None;
    }
    Some((f * mult) as u64)
}

fn session_slug(home: &std::path::Path, root: Option<&std::path::Path>) -> Option<String> {
    let r = root?;
    let rel = r.strip_prefix(home).ok()?;
    Some(format!("-{}", rel.to_string_lossy().replace('/', "-")))
}

fn newest_session(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    let rd = std::fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        if let (Ok(m), Some(name)) = (e.metadata(), e.file_name().to_str()) {
            if !name.ends_with(".jsonl") {
                continue;
            }
            if let Ok(mtime) = m.modified() {
                if newest.as_ref().map_or(true, |(_, t)| mtime > *t) {
                    newest = Some((e.path(), mtime));
                }
            }
        }
    }
    newest.map(|(p, _)| p)
}

/// Scan the tail of a session JSONL for the last `contextSnapshot.promptTokens`.
/// Read the whole session JSONL, collect every `contextSnapshot.promptTokens`
/// in order, return (last = current usage, last_compact_at = pre-compact value
/// before the most recent >30% drop — empirical proof compaction fired).
fn analyze_context(path: &std::path::Path) -> Option<(u64, Option<u64>)> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let mut tokens: Vec<u64> = Vec::new();
    for line in text.lines() {
        if let Some(i) = line.find("\"promptTokens\"") {
            let rest = line[i + 14..].trim_start_matches([':', ' ', '"']);
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num.parse::<u64>() {
                tokens.push(n);
            }
        }
    }
    let used = *tokens.last()?;
    let mut last_compact: Option<u64> = None;
    for w in tokens.windows(2) {
        if w[0] > 0 && w[1] < w[0] * 7 / 10 {
            last_compact = Some(w[0]);
        }
    }
    Some((used, last_compact))
}

fn parse_threshold(cfg: &str) -> u64 {
    for line in cfg.lines() {
        let l = line.trim();
        if let Some(v) = l.strip_prefix("thresholdPercent:") {
            if let Ok(n) = v.trim().parse::<u64>() {
                return n;
            }
        }
    }
    50
}

fn read_model(home: &std::path::Path) -> String {
    let cfg = std::fs::read_to_string(home.join(".omp/agent/config.yml")).unwrap_or_default();
    let key = "default:";
    if let Some(i) = cfg.find(key) {
        let rest = cfg[i + key.len()..].trim_start();
        let v: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
        return v;
    }
    String::new()
}

async fn api_mcp(State(ctx): State<Arc<Ctx>>) -> Json<serde_json::Value> {
    let raw = std::fs::read_to_string(ctx.home.join(".omp/agent/mcp.json")).unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
    let mut servers = Vec::new();
    if let Some(map) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, v) in map {
            servers.push(serde_json::json!({
                "name": name,
                "command": v.get("command").and_then(|x| x.as_str()).unwrap_or(""),
                "args": v.get("args").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|y| y.as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default(),
                "type": v.get("type").and_then(|x| x.as_str()).unwrap_or("stdio"),
                "present": v.get("command").and_then(|x| x.as_str()).map(|c| which::which(c).is_ok()).unwrap_or(false),
            }));
        }
    }
    Json(serde_json::json!({ "servers": servers }))
}

async fn api_rules(State(ctx): State<Arc<Ctx>>) -> Result<Json<Vec<serde_json::Value>>, ApiErr> {
    let root = detect_current_project_root().unwrap_or_default();
    let mut out = Vec::new();
    for (scope, base) in [
        ("global", ctx.home.join(".omp/agent/rules")),
        ("project", root.join(".omp/rules")),
    ] {
        if let Ok(rd) = std::fs::read_dir(&base) {
            for e in rd.flatten() {
                let path = e.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !(name.ends_with(".md") || name.ends_with(".mdc")) {
                        continue;
                    }
                    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                    out.push(serde_json::json!({ "scope": scope, "name": name, "path": path.display().to_string(), "bytes": size }));
                }
            }
        }
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct RuleAddBody {
    name: String,
    content: String,
    scope: Option<String>, // "project" (default) | "global"
}

async fn api_rule_add(
    State(ctx): State<Arc<Ctx>>,
    Json(body): Json<RuleAddBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let dir = match body.scope.as_deref() {
        Some("global") => ctx.home.join(".omp/agent/rules"),
        _ => root.join(".omp/rules"),
    };
    std::fs::create_dir_all(&dir).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut name = body.name.trim().to_string();
    if name.is_empty() {
        name = format!("rule-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    }
    if !name.ends_with(".md") {
        name.push_str(".md");
    }
    let safe: String = name.chars().filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' )).collect();
    let target = dir.join(safe);
    std::fs::write(&target, body.content).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true, "path": target.display().to_string() })))
}

#[derive(Deserialize)]
struct RuleDelBody {
    path: String,
}

async fn api_rule_delete(
    State(_ctx): State<Arc<Ctx>>,
    Json(body): Json<RuleDelBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let p = std::path::Path::new(&body.path);
    // Only allow deleting files under a rules dir (defensive).
    let ok = p.to_string_lossy().contains("/.omp/rules/") || p.to_string_lossy().contains("/rules/");
    if !ok {
        return Err((StatusCode::BAD_REQUEST, "path not under a rules dir".into()));
    }
    std::fs::remove_file(p).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
// ---- Workflow viz (react-flow) + export → omp extension tool ----
//
// Workflows are stored as react-flow node/edge JSON in <root>/agents/workflows/.
// `export` generates a STANDALONE omp extension <root>/.omp/extensions/<name>.ts
// (NOT appended to the harness-managed 8sync-workflow.ts, which is redeployed
// verbatim) that registers a model-callable `<name>_run` tool dispatching the
// steps as followUp messages.

fn workflows_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join("agents/workflows")
}

fn validate_wf_name(name: &str) -> Result<(), ApiErr> {
    let ok = !name.is_empty()
        && !name.starts_with('-')
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return Err((StatusCode::BAD_REQUEST, "name must match ^[a-z0-9-]+$".into()));
    }
    Ok(())
}

async fn api_workflows(State(_ctx): State<Arc<Ctx>>) -> Result<Json<Vec<String>>, ApiErr> {
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let dir = workflows_dir(&root);
    let mut names = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if let Some(stem) = e.file_name().to_str().and_then(|n| n.strip_suffix(".json")) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(Json(names))
}

#[derive(Deserialize)]
struct WfBody {
    nodes: serde_json::Value,
    edges: serde_json::Value,
}

async fn api_workflow_get(
    State(_ctx): State<Arc<Ctx>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    validate_wf_name(&name)?;
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let path = workflows_dir(&root).join(format!("{}.json", name));
    let content = std::fs::read_to_string(&path)
        .map_err(|_| (StatusCode::NOT_FOUND, format!("workflow '{}' not found", name)))?;
    let mut v: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if v.get("name").is_none() {
        v["name"] = serde_json::Value::String(name);
    }
    Ok(Json(v))
}

async fn api_workflow_save(
    State(_ctx): State<Arc<Ctx>>,
    Path(name): Path<String>,
    Json(body): Json<WfBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    validate_wf_name(&name)?;
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let dir = workflows_dir(&root);
    std::fs::create_dir_all(&dir).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let v = serde_json::json!({ "name": name, "nodes": body.nodes, "edges": body.edges });
    let path = dir.join(format!("{}.json", name));
    std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true, "path": path.display().to_string() })))
}

async fn api_workflow_delete(
    State(_ctx): State<Arc<Ctx>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    validate_wf_name(&name)?;
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let path = workflows_dir(&root).join(format!("{}.json", name));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Export a workflow → a standalone omp extension `<root>/.omp/extensions/<name>.ts`
/// registering a model-callable `<name>_run` tool. The tool dispatches the steps
/// in topological order as followUp messages (subagents/skills can't be spawned
/// directly from a tool ctx, so the lead agent executes them).
async fn api_workflow_export(
    State(_ctx): State<Arc<Ctx>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    validate_wf_name(&name)?;
    let root = detect_current_project_root().ok_or((StatusCode::NOT_FOUND, "not in a project".into()))?;
    let path = workflows_dir(&root).join(format!("{}.json", name));
    let content = std::fs::read_to_string(&path)
        .map_err(|_| (StatusCode::NOT_FOUND, format!("workflow '{}' not found", name)))?;
    let wf: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let ts = generate_workflow_extension(&name, &wf)?;
    let ext_dir = root.join(".omp/extensions");
    std::fs::create_dir_all(&ext_dir).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let ext_path = ext_dir.join(format!("{}.ts", name));
    std::fs::write(&ext_path, ts).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "path": ext_path.display().to_string(),
        "tool": format!("{}_run", name)
    })))
}

/// Generate the TS body of a standalone omp extension for one workflow. Built
/// via push_str to avoid format! brace-escaping churn.
fn generate_workflow_extension(name: &str, wf: &serde_json::Value) -> Result<String, ApiErr> {
    let nodes = wf
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or((StatusCode::BAD_REQUEST, "workflow missing nodes[]".into()))?;
    let edges = wf
        .get("edges")
        .and_then(|v| v.as_array())
        .ok_or((StatusCode::BAD_REQUEST, "workflow missing edges[]".into()))?;
    let order = topo_order(nodes, edges);

    let esc = |s: &str| -> String { s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ") };
    let mut steps = String::new();
    for (i, idx) in order.iter().enumerate() {
        let n = &nodes[*idx];
        let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let data = n.get("data").cloned().unwrap_or_default();
        let label = data.get("label").and_then(|v| v.as_str()).unwrap_or("(untitled)");
        let kind = data.get("kind").and_then(|v| v.as_str()).unwrap_or("step");
        let refv = data.get("ref").and_then(|v| v.as_str()).unwrap_or("");
        let instr = match kind {
            "subagent" => format!("Spawn a task subagent to: {} (use: {})", label, refv),
            "tool" => format!("Call the `{}` tool to: {}", refv, label),
            _ => {
                let r = if refv.is_empty() { label } else { refv };
                format!("Use the `{}` skill to: {}", r, label)
            }
        };
        steps.push_str(&format!(
            "  // step {}: {} ({})\n  await pi.sendUserMessage(\"{}\", {{ deliverAs: \"followUp\" }});\n",
            i + 1,
            id,
            kind,
            esc(&instr)
        ));
    }
    let n = order.len();
    let mut out = String::new();
    out.push_str("// Auto-generated by `8sync harness web` — workflow \"");
    out.push_str(name);
    out.push_str("\". Do not edit by hand;\n// re-export from the Workflow page to regenerate.\n");
    out.push_str("// Each step dispatches as a followUp message the lead agent executes\n");
    out.push_str("// (skills/subagents can't be spawned directly from a tool ctx).\n");
    out.push_str("import type { ExtensionAPI } from \"@oh-my-pi/pi-coding-agent\";\n\n");
    out.push_str("export default function (pi: ExtensionAPI) {\n");
    out.push_str("  const { z } = pi.zod;\n");
    out.push_str("  pi.setLabel(\"workflow: ");
    out.push_str(&esc(name));
    out.push_str("\");\n");
    out.push_str("  pi.registerTool({\n");
    out.push_str("    name: \"");
    out.push_str(name);
    out.push_str("_run\",\n    label: \"Run workflow: ");
    out.push_str(&esc(name));
    out.push_str("\",\n    description: \"Execute the \\\"");
    out.push_str(&esc(name));
    out.push_str(&format!("\\\" workflow ({} step(s)) as queued followUp messages.\",\n", n));
    out.push_str("    parameters: z.object({}),\n");
    out.push_str("    async execute(_id, _p, _sig, _onUpd, ctx) {\n");
    out.push_str(&format!("      ctx.ui.notify(\"workflow {}: dispatching {} step(s)\", \"info\");\n", esc(name), n));
    out.push_str(&steps);
    out.push_str(&format!(
        "      return {{ content: [{{ type: \"text\", text: \"workflow {} dispatched ({} followUp step(s) queued)\" }}], details: {{ steps: {} }} }};\n",
        esc(name),
        n,
        n
    ));
    out.push_str("    },\n");
    out.push_str("  });\n");
    out.push_str("}\n");
    Ok(out)
}

/// Kahn's topological sort over workflow nodes/edges by id. Falls back to
/// original array order when a cycle is detected (graceful, not an error).
fn topo_order(nodes: &[serde_json::Value], edges: &[serde_json::Value]) -> Vec<usize> {
    use std::collections::{HashMap, HashSet, VecDeque};
    let id_to_idx: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| n.get("id").and_then(|v| v.as_str()).map(|s| (s, i)))
        .collect();
    let mut deps: Vec<HashSet<usize>> = vec![HashSet::new(); nodes.len()];
    let mut indeg = vec![0usize; nodes.len()];
    for e in edges {
        let s = e.get("source").and_then(|v| v.as_str());
        let t = e.get("target").and_then(|v| v.as_str());
        if let (Some(&si), Some(&ti)) = (s.and_then(|x| id_to_idx.get(x)), t.and_then(|x| id_to_idx.get(x))) {
            if si != ti && deps[si].insert(ti) {
                indeg[ti] += 1;
            }
        }
    }
    let mut q: VecDeque<usize> = (0..nodes.len()).filter(|&i| indeg[i] == 0).collect();
    let mut out = Vec::with_capacity(nodes.len());
    while let Some(i) = q.pop_front() {
        out.push(i);
        let next: Vec<usize> = deps[i].iter().copied().collect();
        for j in next {
            indeg[j] -= 1;
            if indeg[j] == 0 {
                q.push_back(j);
            }
        }
    }
    if out.len() == nodes.len() {
        out
    } else {
        (0..nodes.len()).collect() // cycle → array order
    }
}

// ---- Adaptive model config (models.toml) ----

/// Resolve `~/.config/8sync/models.toml` the same way `ModelConfig::load()`
/// does (XDG config dir), falling back to `<home>/.config` if XDG is absent.
fn models_toml_path(home: &std::path::Path) -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home.join(".config"))
        .join("8sync/models.toml")
}

/// The `/api/models` JSON shape: config path + parsed roles/tasks + the fixed
/// task-class list the FE renders as editable rows.
fn models_config_json(path: &std::path::Path) -> serde_json::Value {
    let cfg = crate::models::ModelConfig::load();
    serde_json::json!({
        "path": path.display().to_string(),
        "roles": {
            "default": cfg.roles.default,
            "plan": cfg.roles.plan,
            "smol": cfg.roles.smol,
            "slow": cfg.roles.slow,
        },
        "tasks": cfg.tasks,
        "classes": ["plan", "review", "debug", "code", "trivial"],
    })
}

async fn api_models_get(State(ctx): State<Arc<Ctx>>) -> Json<serde_json::Value> {
    Json(models_config_json(&models_toml_path(&ctx.home)))
}

#[derive(Deserialize)]
struct ModelSetBody {
    section: String,
    key: String,
    value: String,
}

async fn api_models_set(
    State(ctx): State<Arc<Ctx>>,
    Json(body): Json<ModelSetBody>,
) -> Result<Json<serde_json::Value>, ApiErr> {
    let section = body.section.trim();
    if section != "roles" && section != "tasks" {
        return Err((StatusCode::BAD_REQUEST, "section must be 'roles' or 'tasks'".into()));
    }
    let key = body.key.trim();
    if key.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "key must not be empty".into()));
    }
    let path = models_toml_path(&ctx.home);
    // Seed the user file from the embedded default on first touch (same as the
    // CLI set mode), so a fresh machine writes a complete config.
    if !path.exists() {
        if let (Some(def), Some(parent)) = (assets::read("configs/models.toml"), path.parent()) {
            std::fs::create_dir_all(parent).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            std::fs::write(&path, def).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }
    super::model::set_model_toml(&path, section, key, body.value.trim())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(models_config_json(&path)))
}

// ---- Projects (omp session directories) ----

/// Invert `session_slug`'s encoding (`-<rel-with-/replaced-by-->`) back to an
/// absolute project path under `home`. The encoding is lossy (a literal `-` in a
/// dir name is indistinguishable from a `/` separator), so resolve greedily
/// against the filesystem: at each level take the longest run of `-`-joined
/// tokens that names an existing directory. Returns None if it can't be resolved
/// to an existing path.
fn slug_to_path(home: &std::path::Path, slug: &str) -> Option<std::path::PathBuf> {
    let body = slug.strip_prefix('-')?;
    let tokens: Vec<&str> = body.split('-').collect();
    let mut cur = home.to_path_buf();
    let mut i = 0;
    while i < tokens.len() {
        let mut advanced = false;
        // Prefer the longest token run that forms an existing directory.
        for j in (i + 1..=tokens.len()).rev() {
            let candidate = tokens[i..j].join("-");
            let p = cur.join(&candidate);
            if p.is_dir() {
                cur = p;
                i = j;
                advanced = true;
                break;
            }
        }
        if !advanced {
            return None;
        }
    }
    Some(cur)
}

/// Newest session file mtime (unix seconds) for a session directory, 0 if none.
fn session_mtime_secs(dir: &std::path::Path) -> u64 {
    newest_session(dir)
        .and_then(|p| std::fs::metadata(&p).ok())
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn api_projects(State(ctx): State<Arc<Ctx>>) -> Json<Vec<serde_json::Value>> {
    let home = &ctx.home;
    let sessions_dir = home.join(".omp/agent/sessions");
    // Dedup by resolved project path (several slugs can map to one repo), keep the
    // newest session mtime, and skip slugs that don't resolve to a real dir or have
    // no session file (junk like a bare parent dir).
    let mut by_path: std::collections::HashMap<std::path::PathBuf, u64> = std::collections::HashMap::new();
    if let Ok(rd) = std::fs::read_dir(&sessions_dir) {
        for e in rd.flatten() {
            let dir = e.path();
            if !dir.is_dir() {
                continue;
            }
            let slug = match e.file_name().to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let proj = match slug_to_path(home, &slug) {
                Some(p) => p,
                None => continue,
            };
            if !proj.is_dir() {
                continue;
            }
            let mtime = session_mtime_secs(&dir);
            if mtime == 0 {
                continue;
            }
            let slot = by_path.entry(proj).or_insert(0);
            if mtime > *slot {
                *slot = mtime;
            }
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let most_recent = by_path.values().copied().max().unwrap_or(0);
    let current = detect_current_project_root();
    let mut out: Vec<serde_json::Value> = by_path
        .into_iter()
        .map(|(path, mtime)| {
            let is_current = current.as_deref() == Some(path.as_path());
            // green dot = the project you're viewing, used within 2h, or the single
            // most-recent. (We can't poll omp for a live PID; this is the best signal.)
            let recent = now.saturating_sub(mtime) <= 2 * 60 * 60;
            let active = is_current || recent || mtime == most_recent;
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            serde_json::json!({
                "name": name,
                "path": path.display().to_string(),
                "current": is_current,
                "active": active,
                "lastModified": mtime,
            })
        })
        .collect();
    // current first, then active, then most-recently-modified.
    out.sort_by(|a, b| {
        let ka = (a["current"].as_bool().unwrap_or(false), a["active"].as_bool().unwrap_or(false));
        let kb = (b["current"].as_bool().unwrap_or(false), b["active"].as_bool().unwrap_or(false));
        kb.0.cmp(&ka.0)
            .then(kb.1.cmp(&ka.1))
            .then_with(|| b["lastModified"].as_u64().unwrap_or(0).cmp(&a["lastModified"].as_u64().unwrap_or(0)))
    });
    Json(out)
}

// ---- Workflow templates (react-flow starter graphs) ----

/// One react-flow node in the `{ name, nodes, edges }` graph shape that
/// `api_workflow_get`/`api_workflow_save` use. `kind` ∈ skill|subagent|tool
/// (consumed by `generate_workflow_extension`); `ref` is the skill/agent/tool id.
fn wf_node(id: &str, label: &str, kind: &str, refv: &str, x: i64, y: i64) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "type": "default",
        "position": { "x": x, "y": y },
        "data": { "label": label, "kind": kind, "ref": refv },
    })
}

fn wf_edge(source: &str, target: &str) -> serde_json::Value {
    serde_json::json!({
        "id": format!("e-{}-{}", source, target),
        "source": source,
        "target": target,
    })
}

async fn api_workflow_templates(State(_ctx): State<Arc<Ctx>>) -> Json<Vec<serde_json::Value>> {
    let research_plan_build = serde_json::json!({
        "name": "research → plan → build",
        "nodes": [
            wf_node("research", "Research the problem space", "subagent", "explore", 0, 0),
            wf_node("plan", "Draft an implementation plan", "subagent", "plan", 240, 0),
            wf_node("build", "Implement the plan", "subagent", "task", 480, 0),
        ],
        "edges": [wf_edge("research", "plan"), wf_edge("plan", "build")],
    });
    let review = serde_json::json!({
        "name": "review",
        "nodes": [
            wf_node("review", "Review the diff for quality + security", "subagent", "reviewer", 0, 0),
            wf_node("report", "Summarize findings + action items", "skill", "", 240, 0),
        ],
        "edges": [wf_edge("review", "report")],
    });
    let qa = serde_json::json!({
        "name": "qa",
        "nodes": [
            wf_node("test", "Run the test suite", "tool", "bash", 0, 0),
            wf_node("fix", "Fix any failing tests", "subagent", "task", 240, 0),
            wf_node("verify", "Re-run tests to confirm green", "tool", "bash", 480, 0),
        ],
        "edges": [wf_edge("test", "fix"), wf_edge("fix", "verify")],
    });
    Json(vec![
        serde_json::json!({ "name": "research → plan → build", "graph": research_plan_build }),
        serde_json::json!({ "name": "review", "graph": review }),
        serde_json::json!({ "name": "qa", "graph": qa }),
    ])
}

#[cfg(test)]
mod tests {
    use super::{session_slug, slug_to_path};
    use std::path::PathBuf;

    /// Unique scratch home under the OS temp dir; cleaned on drop.
    struct TmpHome(PathBuf);
    impl TmpHome {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!("8sync_web_test_{}_{}", std::process::id(), tag));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            TmpHome(p)
        }
        fn mkdirs(&self, rel: &str) -> PathBuf {
            let p = self.0.join(rel);
            std::fs::create_dir_all(&p).unwrap();
            p
        }
    }
    impl Drop for TmpHome {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn slug_roundtrip_recovers_path_with_literal_dash() {
        let home = TmpHome::new("dash");
        // Project dir whose name contains a literal '-' (the lossy case).
        let root = home.mkdirs("Projects/tools/su-code");
        let slug = session_slug(&home.0, Some(root.as_path())).unwrap();
        assert_eq!(slug, "-Projects-tools-su-code");
        assert_eq!(slug_to_path(&home.0, &slug), Some(root));
    }

    #[test]
    fn slug_roundtrip_simple_and_nested() {
        let home = TmpHome::new("nested");
        for rel in ["foo", "a/b/c"] {
            let root = home.mkdirs(rel);
            let slug = session_slug(&home.0, Some(root.as_path())).unwrap();
            assert_eq!(slug_to_path(&home.0, &slug), Some(root), "rel={}", rel);
        }
    }

    #[test]
    fn slug_to_path_unresolvable_is_none() {
        let home = TmpHome::new("missing");
        assert_eq!(slug_to_path(&home.0, "-does-not-exist"), None);
        // Missing leading '-' is not a valid slug.
        assert_eq!(slug_to_path(&home.0, "Projects"), None);
    }
}
