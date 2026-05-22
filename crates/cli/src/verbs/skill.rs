use anyhow::{anyhow, Result};
use clap::Args as ClapArgs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{assets, env_detect, ui};

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync skill                                                list installed skills (global + project-local) with descriptions
      8sync skill list                                           same as above
      8sync skill help                                           explain the auto-inject flow and config paths
      8sync skill add https://github.com/colbymchenry/codegraph  clone a skill from a GitHub URL
      8sync skill add gh:owner/repo                              same, short form
      8sync skill add path:/abs/path                             register a skill from a local directory (symlink)
      8sync skill add builtin:karpathy                           register a builtin skill (already shipped)
      8sync skill sync                                           rewrite ~/.omp/skills/00-force-load.md from registry

    SPEC
      Each skill is a directory containing `SKILL.md` at its root (Anthropic Agent
      Skills open standard). YAML frontmatter on SKILL.md MUST set `name:` and
      `description:` so AGENTS.md can inject the right invocation hint.

      Layout (progressive disclosure):
        <name>/
        ├── SKILL.md      (required — frontmatter + concise body)
        ├── scripts/      (optional — deterministic helpers)
        ├── references/   (optional — load on demand)
        └── assets/       (optional — output templates)

    BEHAVIOR
      · Inside a project (root has .git / Cargo.toml / package.json / ...):
          - clone into <root>/agents/skills/<name>/   (project-local, committed)
          - clone into ~/.omp/skills/<name>/          (global, omp loads always)
          - rewrite the `<!-- 8sync:skills:* -->` block in <root>/AGENTS.md
      · Outside any project: only ~/.omp/skills/<name>/.
      · If the target already exists: `git pull --ff-only` (idempotent).

    BUNDLED SKILLS (installed by `8sync setup`)
      karpathy-guidelines  Andrej Karpathy's engineering discipline (read first every session)
      image-routing        decide image vs text reads to save tokens
      8sync-cli            prefer 8sync verbs over raw shell when an equivalent exists

    FILES
      ~/.config/8sync/skills.toml      skill registry (editable TOML)
      ~/.omp/skills/                   global skill directories (one per skill)
      ~/.omp/skills/00-force-load.md   master file — omp reads this first in every session
      <project>/agents/skills/         project-local skills (referenced from AGENTS.md)
"})]
pub struct Args {
    /// Sub-action: list (default) | help | add <spec> | sync
    pub sub: Option<String>,
    /// Argument to the sub-action (e.g. the source spec for `add`).
    pub arg: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    let skills_toml = env.xdg_config.join("8sync/skills.toml");
    match a.sub.as_deref() {
        None | Some("list") => list_skills(&env, &skills_toml),
        Some("help") => print_help(&env, &skills_toml),
        Some("add") => add_skill(&env, &skills_toml, a.arg.as_deref()),
        Some("sync") => sync_skills(&env),
        Some(other) => {
            ui::warn(&format!("unknown subcommand: {}", other));
            ui::info("try: 8sync skill help");
            Ok(())
        }
    }
}

// ─── SKILL.md frontmatter ──────────────────────────────────────────

/// Minimal Agent-Skills frontmatter projection used by 8sync.
struct SkillMeta {
    name: String,
    description: String,
}

/// Parse YAML frontmatter at the top of `skill_md`. Supports the two fields
/// 8sync cares about (`name`, `description`), single-line or double-quoted.
/// Returns Ok(None) when the file has no frontmatter (i.e. doesn't start with
/// `---`) — caller decides whether that's fatal.
fn read_skill_meta(skill_md: &Path) -> Result<Option<SkillMeta>> {
    if !skill_md.exists() {
        return Ok(None);
    }
    let s = std::fs::read_to_string(skill_md)?;
    let body = s.strip_prefix("---\n").or_else(|| s.strip_prefix("---\r\n"));
    let Some(body) = body else { return Ok(None) };
    let mut name = String::new();
    let mut desc = String::new();
    for line in body.lines() {
        if line.trim() == "---" {
            break;
        }
        if let Some(rest) = line.strip_prefix("name:") {
            name = unquote(rest.trim()).to_string();
        } else if let Some(rest) = line.strip_prefix("description:") {
            desc = unquote(rest.trim()).to_string();
        }
    }
    if name.is_empty() {
        return Ok(None);
    }
    Ok(Some(SkillMeta { name, description: desc }))
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 {
        let bs = s.as_bytes();
        let first = bs[0];
        let last = bs[s.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

/// Best-effort metadata + entry-point filename for a skill directory.
/// Preference order:
///   1. `SKILL.md` with valid YAML frontmatter (Agent Skills open standard).
///   2. `SKILL.md` present but no frontmatter — flag, use dir name.
///   3. `CLAUDE.md` (Claude Code convention) — recommend as entrypoint.
///   4. `README.md` — recommend as entrypoint.
///   5. Nothing — warn that AI cannot auto-discover.
fn meta_for_dir(dir: &Path) -> (SkillMeta, &'static str) {
    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string();
    let skill_md = dir.join("SKILL.md");
    if let Ok(Some(m)) = read_skill_meta(&skill_md) {
        return (m, "SKILL.md");
    }
    if skill_md.exists() {
        return (
            SkillMeta {
                name,
                description: "(SKILL.md present but missing YAML frontmatter — non-standard)".to_string(),
            },
            "SKILL.md",
        );
    }
    for (file, label) in [
        ("CLAUDE.md", "Claude-Code-style skill — entrypoint: CLAUDE.md (no Agent-Skills SKILL.md)"),
        ("README.md", "Tool/repo with no SKILL.md — entrypoint: README.md (read it for usage)"),
        ("AGENTS.md", "AGENTS.md-style skill — entrypoint: AGENTS.md (no Agent-Skills SKILL.md)"),
    ] {
        if dir.join(file).exists() {
            return (SkillMeta { name, description: label.to_string() }, file);
        }
    }
    (
        SkillMeta {
            name,
            description: "(no SKILL.md / CLAUDE.md / README.md — AI cannot auto-discover)".to_string(),
        },
        "SKILL.md",
    )
}

// ─── list ──────────────────────────────────────────────────────────

fn list_skills(env: &env_detect::Env, toml_path: &Path) -> Result<()> {
    ui::header("8sync skill");

    println!("[config] {}", toml_path.display());
    if toml_path.exists() {
        let s = std::fs::read_to_string(toml_path)?;
        if s.trim().is_empty() {
            println!("  (empty)");
        } else {
            for line in s.lines() {
                println!("  {}", line);
            }
        }
    } else {
        println!("  (missing) — run `8sync setup` first");
    }

    let omp_skills = env.home.join(".omp/skills");
    println!("\n[global installed skills] {}", omp_skills.display());
    print_skill_list(&omp_skills);

    let force_load = omp_skills.join("00-force-load.md");
    println!("\n[rules: force-load] {}", force_load.display());
    if force_load.exists() {
        println!("  status: present (global auto-inject entrypoint)");
    } else {
        println!("  status: missing (run `8sync skill sync`)");
    }

    if let Some(root) = detect_current_project_root() {
        println!("\n[current project context]");
        println!("  root   : {}", root.display());
        println!("  agents : {}", root.join("agents").display());
        println!("  anchor : {}", root.join("AGENTS.md").display());
        println!("  skills : {}", root.join("agents/skills").display());
        print_skill_list(&root.join("agents/skills"));
    } else {
        println!("\n[current project context]");
        println!("  not inside a detected project root");
    }

    println!("\n[injection model]");
    println!("  global : ~/.omp/skills/00-force-load.md loads on every omp session");
    println!("  local  : <repo>/AGENTS.md has a `<!-- 8sync:skills:* -->` block listing local skills");
    println!("  spec   : Agent Skills open standard — each skill dir has `SKILL.md` with YAML frontmatter");

    println!("\nUse `8sync skill help` for add/sync details.");
    Ok(())
}

fn print_skill_list(dir: &Path) {
    let dirs = list_installed_skill_dirs(dir).unwrap_or_default();
    if dirs.is_empty() {
        println!("  (none)");
        return;
    }
    for p in &dirs {
        let (m, _entry) = meta_for_dir(p);
        let short = truncate(&m.description, 100);
        println!("  - {} — {}", m.name, short);
        println!("      {}", p.display());
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

// ─── add ───────────────────────────────────────────────────────────

/// Source spec parsed from `add` argument.
enum Source {
    Git { url: String, name: String },
    Path { src: PathBuf, name: String },
    Builtin { name: String },
}

fn parse_spec(spec: &str) -> Result<Source> {
    let s = spec.trim();
    if let Some(rest) = s.strip_prefix("builtin:") {
        return Ok(Source::Builtin { name: rest.to_string() });
    }
    if let Some(rest) = s.strip_prefix("path:") {
        let p = PathBuf::from(rest);
        let name = p
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or_else(|| anyhow!("cannot derive name from path: {}", rest))?
            .to_string();
        return Ok(Source::Path { src: p, name });
    }
    if let Some(rest) = s.strip_prefix("gh:") {
        let name = rest
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("bad gh spec: {}", rest))?
            .to_string();
        return Ok(Source::Git {
            url: format!("https://github.com/{}", rest.trim_end_matches(".git")),
            name,
        });
    }
    if s.starts_with("https://") || s.starts_with("http://") || s.starts_with("git@") {
        let name = s
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("bad git url: {}", s))?
            .to_string();
        return Ok(Source::Git { url: s.to_string(), name });
    }
    Err(anyhow!(
        "unknown spec `{}` — use <https URL> | gh:owner/repo | path:/abs | builtin:name",
        s
    ))
}

fn clone_or_pull_git(url: &str, target: &Path) -> Result<()> {
    if target.exists() {
        let st = Command::new("git")
            .args(["-C", target.to_str().unwrap(), "pull", "--ff-only"])
            .status();
        match st {
            Ok(s) if s.success() => {
                ui::ok(&format!("updated {}", target.display()));
            }
            _ => {
                ui::warn(&format!("git pull failed at {}, leaving as-is", target.display()));
            }
        }
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let st = Command::new("git")
        .args(["clone", "--depth", "1", url, target.to_str().unwrap()])
        .status()?;
    if !st.success() {
        return Err(anyhow!("git clone failed: {} → {}", url, target.display()));
    }
    ui::ok(&format!("cloned → {}", target.display()));
    Ok(())
}

fn install_path_skill(src: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        ui::skip(&target.display().to_string(), "exists");
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(src, target)?;
    #[cfg(not(unix))]
    {
        let _ = src;
        return Err(anyhow!("path: spec requires unix symlinks"));
    }
    ui::ok(&format!("linked {} → {}", target.display(), src.display()));
    Ok(())
}

/// After install, audit that the skill dir has a `SKILL.md` with frontmatter.
/// Emit a warning if not — the AI will be unable to auto-load it via the
/// open-standard discovery mechanism.
fn audit_skill_layout(dir: &Path) {
    let skill_md = dir.join("SKILL.md");
    if !skill_md.exists() {
        ui::warn(&format!(
            "{} has no SKILL.md — non-standard layout, AI auto-discovery may not work",
            dir.display()
        ));
        return;
    }
    match read_skill_meta(&skill_md) {
        Ok(Some(m)) => {
            ui::ok(&format!("SKILL.md valid: name={}", m.name));
            if m.description.is_empty() {
                ui::warn("SKILL.md frontmatter has empty `description` — AI won't know when to use it");
            }
        }
        Ok(None) => {
            ui::warn(&format!(
                "{} exists but has no YAML frontmatter (`---` header) — required by Agent Skills spec",
                skill_md.display()
            ));
        }
        Err(e) => ui::warn(&format!("could not read {}: {}", skill_md.display(), e)),
    }
}

fn add_skill(env: &env_detect::Env, toml_path: &Path, spec: Option<&str>) -> Result<()> {
    let Some(spec) = spec else {
        ui::err("usage: 8sync skill add <https URL|gh:owner/repo|path:/abs|builtin:name>");
        return Ok(());
    };
    let src = parse_spec(spec)?;
    let name = match &src {
        Source::Git { name, .. } => name.clone(),
        Source::Path { name, .. } => name.clone(),
        Source::Builtin { name } => name.clone(),
    };
    if name.is_empty() {
        return Err(anyhow!("empty skill name from `{}`", spec));
    }

    let project_root = detect_current_project_root();
    let global_target = env.home.join(".omp/skills").join(&name);

    match &src {
        Source::Git { url, .. } => {
            clone_or_pull_git(url, &global_target)?;
            audit_skill_layout(&global_target);
            if let Some(root) = project_root.as_ref() {
                let local_target = root.join("agents/skills").join(&name);
                clone_or_pull_git(url, &local_target)?;
                audit_skill_layout(&local_target);
            }
        }
        Source::Path { src, .. } => {
            install_path_skill(src, &global_target)?;
            audit_skill_layout(&global_target);
            if let Some(root) = project_root.as_ref() {
                let local_target = root.join("agents/skills").join(&name);
                install_path_skill(src, &local_target)?;
                audit_skill_layout(&local_target);
            }
        }
        Source::Builtin { .. } => {
            ui::info(&format!(
                "builtin:{} — already shipped under {}",
                name,
                global_target.display()
            ));
        }
    }

    // Update skills.toml registry (idempotent append).
    if let Some(parent) = toml_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = std::fs::read_to_string(toml_path).unwrap_or_default();
    let header = format!("[{}]", name);
    if !existing.contains(&header) {
        let src_val = match &src {
            Source::Git { url, .. } => url.clone(),
            Source::Path { src, .. } => format!("path:{}", src.display()),
            Source::Builtin { name } => format!("builtin:{}", name),
        };
        let mut s = existing;
        if !s.ends_with('\n') && !s.is_empty() {
            s.push('\n');
        }
        s.push_str(&format!("\n[{}]\nsrc = \"{}\"\nwhen = \"always\"\n", name, src_val));
        std::fs::write(toml_path, s)?;
        ui::ok(&format!("registered '{}' → {}", name, toml_path.display()));
    }

    if let Some(root) = project_root.as_ref() {
        inject_agents_md(&env.home, root)?;
    }

    ui::info("omp will pick this up on the next `omp --continue` session.");
    Ok(())
}

// ─── sync ──────────────────────────────────────────────────────────

fn sync_skills(env: &env_detect::Env) -> Result<()> {
    // 1. Refresh master force-load file
    let target = env.home.join(".omp/skills/00-force-load.md");
    std::fs::create_dir_all(target.parent().unwrap())?;
    let content = assets::read("skills/00-force-load.md").unwrap_or_default();
    std::fs::write(&target, content)?;
    ui::ok(&format!("synced master skill list → {}", target.display()));
    ui::info("global: every new omp session will read this file first");

    // 2. Re-deploy the 3 bundled skills into ~/.omp/skills/ from embedded assets,
    //    so users who only ran `8sync skill sync` (without `8sync setup`) still
    //    get the built-ins installed globally.
    install_bundled_global(env)?;

    // 3. If we're inside a project, mirror every global skill into
    //    <root>/agents/skills/<name>/ so the project gets a committable,
    //    vendored copy that AI loads via AGENTS.md. Mirroring strategy:
    //      - If the global dir is a git clone: `git clone <origin>` locally
    //        (kept as an independent clone, `git pull --ff-only` on re-sync).
    //      - Otherwise (builtin / path:): plain recursive copy.
    if let Some(root) = detect_current_project_root() {
        let count = mirror_global_to_local(&env.home, &root)?;
        if count > 0 {
            ui::ok(&format!("mirrored {} skill(s) into {}", count, root.join("agents/skills").display()));
        }
        inject_agents_md(&env.home, &root)?;
    } else {
        ui::info("not inside a project — skipped local agents/skills/ mirror");
    }
    Ok(())
}

/// Write the 3 bundled skills (karpathy, image-routing, 8sync-cli) to
/// ~/.omp/skills/<name>/SKILL.md from the embedded asset bundle. Overwrites
/// existing SKILL.md to keep frontmatter up to date.
fn install_bundled_global(env: &env_detect::Env) -> Result<()> {
    let skills_dir = env.home.join(".omp/skills");
    let trio: [(&str, &str); 3] = [
        ("skills/karpathy/SKILL.md",      "karpathy-guidelines"),
        ("skills/image-routing/SKILL.md", "image-routing"),
        ("skills/8sync-cli/SKILL.md",     "8sync-cli"),
    ];
    for (asset_path, name) in trio {
        let Some(body) = assets::read(asset_path) else {
            ui::warn(&format!("asset missing: {}", asset_path));
            continue;
        };
        let target_dir = skills_dir.join(name);
        std::fs::create_dir_all(&target_dir)?;
        let target = target_dir.join("SKILL.md");
        let prev = std::fs::read_to_string(&target).unwrap_or_default();
        if prev != body {
            std::fs::write(&target, body)?;
            ui::ok(&format!("wrote {}", target.display()));
        }
    }
    Ok(())
}

/// For every skill dir under `~/.omp/skills/`, create or refresh a copy under
/// `<root>/agents/skills/<name>/`. Returns the number of skills processed.
fn mirror_global_to_local(home: &Path, root: &Path) -> Result<usize> {
    let global_dir = home.join(".omp/skills");
    let local_dir = root.join("agents/skills");
    std::fs::create_dir_all(&local_dir)?;
    let globals = list_installed_skill_dirs(&global_dir).unwrap_or_default();
    let mut count = 0usize;
    for g in &globals {
        let name = match g.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let local_target = local_dir.join(name);
        let origin = git_origin_url(g);
        if let Some(url) = origin {
            // Git-backed skill → keep local as an independent clone.
            clone_or_pull_git(&url, &local_target)?;
        } else if local_target.exists() {
            // Refresh in place from the global vendor copy.
            copy_dir_recursive(g, &local_target)?;
            ui::ok(&format!("refreshed {}", local_target.display()));
        } else {
            copy_dir_recursive(g, &local_target)?;
            ui::ok(&format!("copied  → {}", local_target.display()));
        }
        count += 1;
    }
    Ok(count)
}

/// Return the `origin` remote URL of a git repo at `dir`, or None if `dir` is
/// not a git working copy.
fn git_origin_url(dir: &Path) -> Option<String> {
    if !dir.join(".git").exists() {
        return None;
    }
    let out = Command::new("git")
        .args(["-C", dir.to_str()?, "config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if url.is_empty() { None } else { Some(url) }
}

/// Recursively copy `src` into `dst`. Skips `.git/` (vendor copies should not
/// carry the git history of an unrelated repo). Overwrites existing files.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" { continue; }
        let from = entry.path();
        let to = dst.join(&name);
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ft.is_symlink() {
            // Resolve and copy the target as a regular file (keeps vendor copy self-contained).
            if let Ok(target) = std::fs::read_link(&from) {
                let resolved = if target.is_absolute() { target } else { from.parent().map(|p| p.join(&target)).unwrap_or(target) };
                if resolved.is_file() {
                    std::fs::copy(&resolved, &to)?;
                }
            }
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

// ─── help ──────────────────────────────────────────────────────────

fn print_help(env: &env_detect::Env, toml_path: &Path) -> Result<()> {
    ui::header("8sync skill help");
    println!("SYNTAX");
    println!("  8sync skill");
    println!("  8sync skill list");
    println!("  8sync skill help");
    println!("  8sync skill add <https URL|gh:owner/repo|path:/abs|builtin:name>");
    println!("  8sync skill sync");

    println!("\nSPEC (Agent Skills open standard)");
    println!("  Each skill is a directory containing `SKILL.md` at its root.");
    println!("  SKILL.md MUST start with YAML frontmatter:");
    println!("    ---");
    println!("    name: skill-name          # lowercase, digits, hyphens, ≤64 chars");
    println!("    description: pushy 3rd-person sentence describing what + WHEN to use");
    println!("    ---");
    println!("  Optional subdirs: scripts/, references/, assets/ (progressive disclosure).");

    println!("\nAUTO-INJECT FLOW");
    println!("  1) `8sync skill add <url>` clones into ~/.omp/skills/<name>/  (global)");
    println!("     and (if inside a project) <root>/agents/skills/<name>/    (local)");
    println!("  2) <root>/AGENTS.md is rewritten between `<!-- 8sync:skills:* -->` sentinels");
    println!("     to list every global + local skill with its frontmatter description.");
    println!("  3) omp reads ~/.omp/skills/00-force-load.md + AGENTS.md every session.");

    println!("\nPATHS");
    println!("  global skills dir : {}", env.home.join(".omp/skills").display());
    println!("  global rules file : {}", env.home.join(".omp/skills/00-force-load.md").display());
    println!("  config registry   : {}", toml_path.display());

    if let Some(root) = detect_current_project_root() {
        println!("  local project root: {}", root.display());
        println!("  local anchor file : {}", root.join("AGENTS.md").display());
        println!("  local skills dir  : {}", root.join("agents/skills").display());
    }
    Ok(())
}

// ─── shared helpers ────────────────────────────────────────────────

fn list_installed_skill_dirs(skills_dir: &Path) -> Result<Vec<PathBuf>> {
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

fn detect_current_project_root() -> Option<PathBuf> {
    let markers = [
        ".git",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "deno.json",
        "go.mod",
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

// ─── AGENTS.md injection ───────────────────────────────────────────

const BEGIN: &str = "<!-- 8sync:skills:begin -->";
const END: &str = "<!-- 8sync:skills:end -->";

/// Find the byte offset of `needle` only when it occurs as the start of a line
/// (offset 0, or the byte immediately before is '\n'). Returns the offset of
/// the needle itself (not the newline).
fn find_line(hay: &str, needle: &str) -> Option<usize> {
    let bytes = hay.as_bytes();
    let mut start = 0;
    while let Some(rel) = hay[start..].find(needle) {
        let pos = start + rel;
        if pos == 0 || bytes[pos - 1] == b'\n' {
            return Some(pos);
        }
        start = pos + needle.len();
    }
    None
}

/// Rewrite (or insert) the force-load block in `<root>/AGENTS.md`.
///
/// The block lists **both** global skills under `~/.omp/skills/` and project-local
/// skills under `<root>/agents/skills/`, each annotated with the description
/// extracted from its `SKILL.md` YAML frontmatter. The wording is intentionally
/// strong ("MUST", "BEFORE any code touch") so the AI cannot reasonably skip.
pub fn inject_agents_md(home: &Path, root: &Path) -> Result<()> {
    let agents = root.join("AGENTS.md");

    let global_dir = home.join(".omp/skills");
    let local_dir = root.join("agents/skills");

    let globals = list_installed_skill_dirs(&global_dir).unwrap_or_default();
    let locals = list_installed_skill_dirs(&local_dir).unwrap_or_default();

    let mut g_lines = String::new();
    if globals.is_empty() {
        g_lines.push_str("- _(none — run `8sync skill sync` after `8sync setup`)_\n");
    } else {
        for (i, p) in globals.iter().enumerate() {
            let (m, entry) = meta_for_dir(p);
            let desc = if m.description.is_empty() { "(no description)".to_string() } else { m.description };
            let dirname = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            g_lines.push_str(&format!(
                "{}. **`{}`** — `~/.omp/skills/{}/{}`\n     _{}_\n",
                i + 1,
                m.name,
                dirname,
                entry,
                desc,
            ));
        }
    }

    let mut l_lines = String::new();
    if locals.is_empty() {
        l_lines.push_str("- _(none yet — run `8sync skill add <github-url>`)_\n");
    } else {
        for (i, p) in locals.iter().enumerate() {
            let (m, entry) = meta_for_dir(p);
            let desc = if m.description.is_empty() { "(no description)".to_string() } else { m.description };
            let dirname = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            l_lines.push_str(&format!(
                "{}. **`{}`** — `agents/skills/{}/{}`\n     _{}_\n",
                i + 1,
                m.name,
                dirname,
                entry,
                desc,
            ));
        }
    }

    let block = format!(
        "{BEGIN}\n\
## 🚨 MANDATORY — đọc TRƯỚC khi làm bất cứ task gì\n\
\n\
Bạn (AI) **PHẢI** đọc đầy đủ các skill liệt kê dưới đây **trước dòng code đầu tiên** trong session này. Không skip, không suy đoán, không viết tắt.\n\
\n\
Mỗi skill là 1 directory theo [Agent Skills open standard](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview): `SKILL.md` ở root có YAML frontmatter (`name`, `description`). Description đã liệt kê dưới — nó cho biết **khi nào** dùng skill.\n\
\n\
### Global skills (always-on — `~/.omp/skills/`)\n\
{g_lines}\n\
### Project-local skills (BẮT BUỘC dùng cho repo này — `agents/skills/`)\n\
{l_lines}\n\
### Quy tắc bất biến\n\
\n\
- Đọc tất cả `SKILL.md` ở 2 list trên **TRƯỚC** khi gọi tool đầu tiên.\n\
- Nếu skill có thư mục `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.\n\
- Nếu skill có `references/` → đọc on-demand khi task chạm vào chủ đề tương ứng.\n\
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.\n\
- Nếu một skill local có vẻ liên quan đến task hiện tại (theo description), bạn **MUST** đọc nó trước khi sửa code — không được \"chắc là không cần\".\n\
{END}"
    );

    let existing = std::fs::read_to_string(&agents).unwrap_or_default();

    // Match sentinels only when they appear as a standalone line (start of file
    // or preceded by '\n'). Inline mentions of the sentinel strings inside prose
    // — e.g. documentation describing this very feature — must not trigger a
    // mid-paragraph rewrite.
    let new_contents = if let (Some(b), Some(e)) = (find_line(&existing, BEGIN), find_line(&existing, END)) {
        if b < e {
            let mut s = String::with_capacity(existing.len() + block.len());
            s.push_str(&existing[..b]);
            s.push_str(&block);
            s.push_str(&existing[e + END.len()..]);
            s
        } else {
            insert_block_after_h1(&existing, &block)
        }
    } else {
        insert_block_after_h1(&existing, &block)
    };

    std::fs::write(&agents, new_contents)?;
    ui::ok(&format!(
        "injected force-load block into {} ({} global, {} local)",
        agents.display(),
        globals.len(),
        locals.len(),
    ));
    Ok(())
}

fn insert_block_after_h1(existing: &str, block: &str) -> String {
    if existing.is_empty() {
        return format!("# AGENTS.md\n\n{block}\n");
    }
    let mut out = String::with_capacity(existing.len() + block.len() + 8);
    let mut inserted = false;
    let mut prev_was_h1 = false;
    for line in existing.lines() {
        out.push_str(line);
        out.push('\n');
        if !inserted {
            if prev_was_h1 {
                out.push('\n');
                out.push_str(block);
                out.push_str("\n\n");
                inserted = true;
                prev_was_h1 = false;
            } else if line.starts_with("# ") {
                prev_was_h1 = true;
            }
        }
    }
    if !inserted {
        let mut s = String::with_capacity(existing.len() + block.len() + 4);
        s.push_str(block);
        s.push_str("\n\n");
        s.push_str(existing);
        return s;
    }
    out
}
