//! `8sync feature` — deterministic ops for the large-scope GSD feature framework.
//!
//! Owns the on-disk planning tree + the cross-feature ACTIVE switch under
//! `<project>/su-code/planning/`. The AI-judgement steps (`plan`/`go`/`ship`)
//! need model reasoning and live in the bundled `feature` skill + the `/feature`
//! omp command; this verb only does the fast, deterministic file ops:
//!   new <slug>    scaffold `su-code/planning/<slug>/` from templates + activate
//!   switch <slug> flip the active feature (rewrite ACTIVE.md + config)
//!   status        print the active feature's STATE position
//!   list          list features (active marked) + archived
//! No-arg → status. `plan`/`go`/`ship` print a pointer to the `/feature` command.
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use std::path::Path;
use std::process::Command;

use crate::{assets, brand, ui, verbs::here};

/// Planning tree location, relative to the project root (matches the bundled
/// `config.json` `paths.planning_root`). Fixed so the switch is deterministic —
/// `config.json` lives *inside* this dir and cannot relocate it.
const PLANNING_REL: &str = "su-code/planning";

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync feature new zalo-group   scaffold su-code/planning/zalo-group/ (+ activate)
          8sync feature list             list features (★ = active) + archived
          8sync feature switch other     flip the active feature
          8sync feature status           show the active feature's STATE position
          8sync feature                  (no arg) same as status

        Large multi-phase scopes only. Small/single-concern work → `/auto`.
        The reasoning steps run in an omp session:
          /feature plan | go | ship      discuss + plan + execute (via the engine) + verify
    "}
)]
pub struct Args {
    /// new | switch | status | list  (empty → status)
    pub sub: Option<String>,
    /// Feature slug (for `new` / `switch`).
    pub slug: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    let cwd = std::env::current_dir().context("no cwd")?;
    let root = here::detect_project_root(&cwd).unwrap_or(cwd);

    match a.sub.as_deref() {
        None | Some("status") => status(&root),
        Some("new") => {
            let slug = a
                .slug
                .as_deref()
                .with_context(|| format!("usage: {} feature new <slug>", brand::CMD))?;
            new(&root, slug)
        }
        Some("switch") => {
            let slug = a
                .slug
                .as_deref()
                .with_context(|| format!("usage: {} feature switch <slug>", brand::CMD))?;
            switch(&root, slug)
        }
        Some("list") => list(&root),
        Some(step @ ("plan" | "go" | "ship")) => {
            ui::warn(&format!("`{} feature {}` needs model judgement — run it in an omp session:", brand::CMD, step));
            ui::info(&format!("  {} .   then   /feature {}", brand::CMD, step));
            Ok(())
        }
        Some(other) => {
            ui::warn(&format!("unknown subcommand: {}", other));
            ui::info(&format!("try: {} feature new <slug> | switch <slug> | status | list", brand::CMD));
            Ok(())
        }
    }
}

// ═════════════════════════════════════════════════════════════════
// subcommands
// ═════════════════════════════════════════════════════════════════

fn new(root: &Path, slug: &str) -> Result<()> {
    if !is_kebab(slug) {
        bail!("invalid slug '{}' — use kebab-case (a-z, 0-9, '-'; e.g. zalo-group)", slug);
    }
    let planning = root.join(PLANNING_REL);
    let feat_dir = planning.join(slug);
    if feat_dir.exists() {
        bail!(
            "feature '{}' already exists at {} — use `{} feature switch {}` or remove it first",
            slug,
            feat_dir.display(),
            brand::CMD,
            slug
        );
    }

    ui::header(&format!("{} feature new {}", brand::CMD, slug));
    std::fs::create_dir_all(&feat_dir).with_context(|| format!("create {}", feat_dir.display()))?;
    std::fs::create_dir_all(planning.join("_archive"))?;

    let name = title_case(slug);
    let date = today();
    // Scaffold the four per-feature docs from the bundled templates.
    for t in ["PROJECT", "REQUIREMENTS", "ROADMAP", "STATE"] {
        let asset = format!("skills/feature/templates/{}.md", t);
        let body = assets::read(&asset)
            .with_context(|| format!("bundled template missing: {}", asset))?;
        let body = substitute(&body, slug, &name, &date);
        std::fs::write(feat_dir.join(format!("{}.md", t)), body)?;
    }
    ui::ok(&format!("scaffolded {}/{{PROJECT,REQUIREMENTS,ROADMAP,STATE}}.md", feat_dir.display()));

    // planning-root config.json: create from template or merge active_feature
    // into an existing one (preserve a user's workflow settings).
    let mut config = load_config(&planning);
    if let Some(obj) = config.as_object_mut() {
        obj.insert("active_feature".into(), serde_json::Value::String(slug.to_string()));
    }
    std::fs::write(planning.join("config.json"), serde_json::to_string_pretty(&config)? + "\n")?;

    set_active(&planning, slug)?;
    write_active_pointer(root, slug);

    ui::ok(&format!("active feature → {}", slug));
    ui::info(&format!("next: run `{} .` then `/feature plan`", brand::CMD));
    Ok(())
}

fn switch(root: &Path, slug: &str) -> Result<()> {
    let planning = root.join(PLANNING_REL);
    let feat_dir = planning.join(slug);
    if !feat_dir.is_dir() {
        bail!(
            "no feature '{}' at {} — run `{} feature new {}` or `{} feature list`",
            slug,
            feat_dir.display(),
            brand::CMD,
            slug,
            brand::CMD
        );
    }
    let mut config = load_config(&planning);
    if let Some(obj) = config.as_object_mut() {
        obj.insert("active_feature".into(), serde_json::Value::String(slug.to_string()));
    }
    std::fs::write(planning.join("config.json"), serde_json::to_string_pretty(&config)? + "\n")?;
    set_active(&planning, slug)?;
    write_active_pointer(root, slug);
    ui::ok(&format!("active feature → {}", slug));
    Ok(())
}

fn status(root: &Path) -> Result<()> {
    let planning = root.join(PLANNING_REL);
    let Some(slug) = active_slug(&planning) else {
        ui::info(&format!("no active feature — run `{} feature new <slug>` to start one", brand::CMD));
        return Ok(());
    };
    ui::header(&format!("feature: {}", slug));
    let state = planning.join(&slug).join("STATE.md");
    match std::fs::read_to_string(&state) {
        Ok(s) => {
            let fm = frontmatter(&s);
            let get = |k: &str| fm.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.as_str()).unwrap_or("?");
            ui::info(&format!("status:       {}", get("status")));
            ui::info(&format!("active_phase: {}", get("active_phase")));
            ui::info(&format!("next_action:  {}", get("next_action")));
            ui::info(&format!("STATE:        {}", state.display()));
        }
        Err(_) => ui::warn(&format!("active feature '{}' has no STATE.md at {}", slug, state.display())),
    }
    Ok(())
}

fn list(root: &Path) -> Result<()> {
    let planning = root.join(PLANNING_REL);
    if !planning.is_dir() {
        ui::info(&format!("no features yet — run `{} feature new <slug>`", brand::CMD));
        return Ok(());
    }
    let active = active_slug(&planning);
    ui::header("features");
    let mut any = false;
    if let Ok(rd) = std::fs::read_dir(&planning) {
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n != "_archive")
            .collect();
        names.sort();
        for n in names {
            any = true;
            if active.as_deref() == Some(n.as_str()) {
                ui::ok(&format!("★ {} (active)", n));
            } else {
                ui::info(&format!("  {}", n));
            }
        }
    }
    if !any {
        ui::info("  (none)");
    }
    // archived
    let archive = planning.join("_archive");
    if let Ok(rd) = std::fs::read_dir(&archive) {
        let mut arch: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        arch.sort();
        if !arch.is_empty() {
            ui::info("archived:");
            for n in arch {
                ui::info(&format!("  {}", n));
            }
        }
    }
    Ok(())
}

// ═════════════════════════════════════════════════════════════════
// helpers
// ═════════════════════════════════════════════════════════════════

/// Read the planning-root `config.json` if present; else the bundled template;
/// else a hardcoded default matching the template shape.
fn load_config(planning: &Path) -> serde_json::Value {
    if let Ok(s) = std::fs::read_to_string(planning.join("config.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            return v;
        }
    }
    if let Some(s) = assets::read("skills/feature/templates/config.json") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            return v;
        }
    }
    serde_json::json!({
        "active_feature": "",
        "workflow": {
            "parallelization": true,
            "min_parallel_tasks": 3,
            "plan_review": "complex",
            "review_dimensions": ["security", "correctness", "convention"],
            "code_review": true,
            "verifier": true
        },
        "paths": { "planning_root": "su-code/planning", "archive": "su-code/planning/_archive" }
    })
}

/// Rewrite `ACTIVE.md` from the bundled template (clean header + slug). Falls
/// back to a minimal file if the template asset is unavailable.
fn set_active(planning: &Path, slug: &str) -> Result<()> {
    std::fs::create_dir_all(planning)?;
    let body = match assets::read("skills/feature/templates/ACTIVE.md") {
        Some(t) => t.replace("SLUG", slug),
        None => format!("<!-- active feature: first non-comment line = slug -->\n{}\n", slug),
    };
    std::fs::write(planning.join("ACTIVE.md"), body)?;
    Ok(())
}

/// The active slug = first non-blank line of `ACTIVE.md` after stripping HTML
/// comment blocks (`<!-- ... -->`, possibly multi-line). None if empty/missing.
fn active_slug(planning: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(planning.join("ACTIVE.md")).ok()?;
    let stripped = strip_html_comments(&raw);
    stripped
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
}

/// Best-effort: keep a one-line pointer at the top of the project's session
/// `su-code/STATE.md` so `/auto` + the recall hook see which feature is active.
/// No-op when STATE.md is absent (only seeded projects have it); never clobbers
/// other content — replaces its own marker line idempotently.
fn write_active_pointer(root: &Path, slug: &str) {
    let state = root.join("su-code/STATE.md");
    let Ok(content) = std::fs::read_to_string(&state) else {
        return;
    };
    const MARKER: &str = "> **Active feature:**";
    let pointer = format!(
        "{} `su-code/planning/{}/STATE.md` — `{} feature status`",
        MARKER, slug, brand::CMD
    );
    let mut lines: Vec<String> = content
        .lines()
        .filter(|l| !l.trim_start().starts_with(MARKER))
        .map(|l| l.to_string())
        .collect();
    let pos = lines
        .iter()
        .position(|l| l.starts_with('#'))
        .map(|i| i + 1)
        .unwrap_or(0);
    lines.insert(pos, pointer);
    let _ = std::fs::write(&state, lines.join("\n") + "\n");
}

/// Substitute the template placeholders. Order: FEATURE_NAME before SLUG is
/// irrelevant (distinct tokens); DATE last.
fn substitute(body: &str, slug: &str, name: &str, date: &str) -> String {
    body.replace("FEATURE_NAME", name)
        .replace("SLUG", slug)
        .replace("DATE", date)
}

fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
}

fn title_case(slug: &str) -> String {
    slug.split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Today as `YYYY-MM-DD` via `date` (Linux target). "unknown" if unavailable.
fn today() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Parse a leading `---\n…\n---` YAML-ish frontmatter block into (key, value)
/// pairs. Values are trimmed and unquoted. Nested keys are ignored (only the
/// top-level scalar lines we need: status/active_phase/next_action).
fn frontmatter(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut lines = s.lines();
    if lines.next().map(str::trim) != Some("---") {
        return out;
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        // top-level `key: value` only (skip indented / nested)
        if line.starts_with([' ', '\t']) {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let v = v.trim().trim_matches('"').trim_matches('\'').trim();
            out.push((k.trim().to_string(), v.to_string()));
        }
    }
    out
}

/// Remove `<!-- ... -->` blocks (handles multi-line) so `ACTIVE.md` parsing sees
/// only real content.
fn strip_html_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}
