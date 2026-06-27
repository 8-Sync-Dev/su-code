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

fn api_routes() -> Router<Arc<Ctx>> {
    Router::new()
        .route("/api/state", get(api_state))
        .route("/api/skills", get(api_skills))
        .route("/api/skills/toggle", post(api_skill_toggle))
        .route("/api/engines", get(api_engines))
        .route("/api/bench", get(api_bench))
        .route("/api/eval", get(api_eval))
        .route("/api/memory/:file", get(api_memory_get).post(api_memory_set))
}

type ApiErr = (StatusCode, String);

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
    Json(serde_json::json!({
        "codegraph": eng("codegraph"),
        "cbm": eng("codebase-memory-mcp"),
        "headroom": eng("headroom"),
        "serena": eng("serena"),
        "mnemopi_on": cfg.contains("backend: mnemopi"),
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
