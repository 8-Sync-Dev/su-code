// Public, programmatic entrypoint used from setup.rs to register codegraph
// (and any other always-on skill) without going through the CLI.
pub fn add_spec(env: &crate::env_detect::Env, toml_path: &std::path::Path, spec: &str) -> anyhow::Result<()> {
    add_skill(env, toml_path, Some(spec))
}

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
      8sync skill add path:/abs/path#better-name                 …and rename it on install
      8sync skill add builtin:karpathy                           register a builtin skill (already shipped)
      8sync skill add https://...#better-name                    same #newname override works on any spec
      8sync skill sync                                           rewrite ~/.omp/skills/00-force-load.md from registry
      8sync skill gen 1 2                                        FUSE local skill #1 and #2 into one combined SKILL.md
      8sync skill gen karpathy-guidelines codegraph              same, but by name

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
    /// Sub-action: list (default) | help | add <spec> | sync | gen <id> <id> …
    pub sub: Option<String>,
    /// Arguments for the sub-action.
    /// - `add`: source spec (one)
    /// - `gen`: 2+ skill IDs (1-based index from local list, OR skill name)
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

pub fn run(a: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    let skills_toml = env.xdg_config.join("8sync/skills.toml");
    match a.sub.as_deref() {
        None | Some("list") => list_skills(&env, &skills_toml),
        Some("help") => print_help(&env, &skills_toml),
        Some("add") => add_skill(&env, &skills_toml, a.args.first().map(|s| s.as_str())),
        Some("sync") => sync_skills(&env),
        Some("gen") => gen_skill(&env, &a.args),
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
    for (i, p) in dirs.iter().enumerate() {
        let (m, _entry) = meta_for_dir(p);
        let short = truncate(&m.description, 100);
        println!("  {:>2}. {} — {}", i + 1, m.name, short);
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
    // Allow an explicit name override via `<spec>#<name>` suffix.
    // Examples:
    //   path:/abs/foo#better-name
    //   gh:owner/repo#better-name
    //   https://github.com/owner/repo#better-name
    let (core, name_override) = match spec.trim().rsplit_once('#') {
        Some((lhs, rhs)) if !rhs.is_empty() && !rhs.contains('/') => (lhs, Some(rhs.to_string())),
        _ => (spec.trim(), None),
    };
    let with_name = |default: String| name_override.clone().unwrap_or(default);

    if let Some(rest) = core.strip_prefix("builtin:") {
        return Ok(Source::Builtin { name: with_name(rest.to_string()) });
    }
    if let Some(rest) = core.strip_prefix("path:") {
        let p = PathBuf::from(rest);
        let default_name = p
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or_else(|| anyhow!("cannot derive name from path: {}", rest))?
            .to_string();
        return Ok(Source::Path { src: p, name: with_name(default_name) });
    }
    if let Some(rest) = core.strip_prefix("gh:") {
        let default_name = rest
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("bad gh spec: {}", rest))?
            .to_string();
        return Ok(Source::Git {
            url: format!("https://github.com/{}", rest.trim_end_matches(".git")),
            name: with_name(default_name),
        });
    }
    if core.starts_with("https://") || core.starts_with("http://") || core.starts_with("git@") {
        let default_name = core
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("bad git url: {}", core))?
            .to_string();
        return Ok(Source::Git { url: core.to_string(), name: with_name(default_name) });
    }
    Err(anyhow!(
        "unknown spec `{}` — use <https URL> | gh:owner/repo | path:/abs | builtin:name (optional #newname suffix to rename)",
        spec
    ))
}

/// Fetch the README.md of a public GitHub repo via raw.githubusercontent.com.
/// Tries refs in order: HEAD, main, master.
fn fetch_github_readme(owner: &str, repo: &str) -> Result<String> {
    for branch in ["HEAD", "main", "master"] {
        for name in ["README.md", "readme.md", "README.MD", "Readme.md"] {
            let url = format!(
                "https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{name}"
            );
            let out = Command::new("curl")
                .args(["-fsSL", "--max-time", "15", &url])
                .output();
            if let Ok(o) = out {
                if o.status.success() && !o.stdout.is_empty() {
                    return Ok(String::from_utf8_lossy(&o.stdout).into_owned());
                }
            }
        }
    }
    Err(anyhow!("could not fetch README.md from {}/{}", owner, repo))
}

/// Parse `owner/repo` from any flavour of GitHub URL we accept.
fn github_owner_repo(url: &str) -> Option<(String, String)> {
    let u = url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .trim_start_matches("git+");
    let rest = u
        .strip_prefix("https://github.com/")
        .or_else(|| u.strip_prefix("http://github.com/"))
        .or_else(|| u.strip_prefix("git@github.com:"))?;
    let mut it = rest.splitn(3, '/');
    let owner = it.next()?.to_string();
    let repo = it.next()?.to_string();
    if owner.is_empty() || repo.is_empty() { return None; }
    Some((owner, repo))
}

/// Wrap README content with YAML frontmatter so it becomes a valid SKILL.md.
/// If the README already has frontmatter (rare — author already shipped it as
/// a skill), pass it through unchanged.
fn synthesize_skill_md(readme: &str, name: &str, source_url: &str) -> String {
    let trimmed = readme.trim_start_matches('\u{feff}');
    if trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n") {
        return readme.to_string();
    }
    let desc = extract_description(readme, name);
    let q = yaml_quote(&desc);
    format!(
        "---\n\
name: {name}\n\
description: {q}\n\
source: {source_url}\n\
---\n\
\n\
> Skill synthesised from `{source_url}/README.md` by `8sync skill add`.\n\
> Install / setup / usage commands are in the body below (verbatim README).\n\
> When this skill applies (see description above), follow the commands here.\n\
\n\
{readme}"
    )
}

/// Pick a one-line description from a README: first non-empty, non-heading,
/// non-badge prose line. Falls back to a generic "use when user mentions <name>".
fn extract_description(readme: &str, fallback_name: &str) -> String {
    for line in readme.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with('#') { continue; }
        if t.starts_with("![") || t.starts_with("[![") { continue; }
        if t.starts_with('<') { continue; }
        if t.starts_with("---") { continue; }
        let cleaned = t
            .trim_start_matches('>')
            .trim()
            .trim_start_matches('*')
            .trim_end_matches('*')
            .trim_start_matches('_')
            .trim_end_matches('_')
            .trim();
        if cleaned.len() < 10 { continue; }
        // Single-line YAML value: collapse internal whitespace, clamp length.
        let single: String = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut chars: String = single.chars().take(400).collect();
        if single.chars().count() > 400 { chars.push('…'); }
        return format!(
            "Use this skill when the user mentions {fallback_name} or related concepts. {chars}"
        );
    }
    format!(
        "Use this skill when the user mentions {fallback_name}. See body for install/setup/usage commands fetched from the upstream README."
    )
}

fn yaml_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

/// Write a synthesised SKILL.md into `<target_dir>/SKILL.md`. Creates the dir.
fn write_synth_skill(target_dir: &Path, content: &str) -> Result<()> {
    std::fs::create_dir_all(target_dir)?;
    let target = target_dir.join("SKILL.md");
    let prev = std::fs::read_to_string(&target).unwrap_or_default();
    if prev != content {
        std::fs::write(&target, content)?;
        ui::ok(&format!("wrote {}", target.display()));
    } else {
        ui::skip(&target.display().to_string(), "unchanged");
    }
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
            // README-as-skill model: fetch upstream README.md and synthesise a
            // single SKILL.md with YAML frontmatter. No cloning — the README
            // already carries install/setup/usage commands.
            let (owner, repo) = github_owner_repo(url)
                .ok_or_else(|| anyhow!("only github.com URLs supported (got `{}`)", url))?;
            ui::info(&format!("fetching README from {}/{}", owner, repo));
            let readme = fetch_github_readme(&owner, &repo)?;
            let body = synthesize_skill_md(&readme, &name, url);
            write_synth_skill(&global_target, &body)?;
            audit_skill_layout(&global_target);
            if let Some(root) = project_root.as_ref() {
                let local_target = root.join("agents/skills").join(&name);
                write_synth_skill(&local_target, &body)?;
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

        // Self-mirror guard: if the global skill is a symlink that resolves to
        // local_target (e.g. `path:` install with cwd == project root), refusing
        // to remove+copy would otherwise WIPE the source. Skip cleanly.
        let g_canon = std::fs::canonicalize(g).ok();
        let l_canon = std::fs::canonicalize(&local_target).ok();
        if let (Some(gc), Some(lc)) = (g_canon.as_ref(), l_canon.as_ref()) {
            if gc == lc {
                ui::skip(
                    &local_target.display().to_string(),
                    "global symlink resolves here (skipped — already source-of-truth)",
                );
                count += 1;
                continue;
            }
        }

        // Always vendor-copy (no nested .git/) so the local tree is committable.
        // For git-backed skills we still call `git pull --ff-only` on the GLOBAL
        // copy elsewhere; here we just refresh local files from whatever global has.
        let existed = local_target.exists();
        if existed {
            let _ = std::fs::remove_dir_all(&local_target);
        }
        copy_dir_recursive(g, &local_target)?;
        ui::ok(&format!(
            "{} → {}",
            if existed { "refreshed" } else { "vendored " },
            local_target.display()
        ));
        count += 1;
    }
    Ok(count)
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

// ─── gen (fuse N skills into one) ──────────────────────────────────

/// Resolve a CLI arg into a local skill dir under `<root>/agents/skills/`.
/// Accepts either a 1-based index (matching the order shown in `8sync skill`
/// list and AGENTS.md), or a skill name (the directory name OR the frontmatter
/// `name`).
fn resolve_skill_arg(arg: &str, locals: &[PathBuf]) -> Result<PathBuf> {
    if let Ok(idx) = arg.parse::<usize>() {
        if idx == 0 || idx > locals.len() {
            return Err(anyhow!(
                "index {} out of range (1..={})",
                idx,
                locals.len()
            ));
        }
        return Ok(locals[idx - 1].clone());
    }
    for p in locals {
        let dir_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if dir_name == arg {
            return Ok(p.clone());
        }
        let (m, _) = meta_for_dir(p);
        if m.name == arg {
            return Ok(p.clone());
        }
    }
    Err(anyhow!(
        "no local skill matches `{}` (try `8sync skill` to see the numbered list)",
        arg
    ))
}

/// Strip YAML frontmatter (the leading `---` … `---` block) from a SKILL.md
/// body so we can splice multiple bodies together without dragging duplicate
/// frontmatter into the fused file.
fn strip_frontmatter(s: &str) -> &str {
    let trimmed = s.trim_start_matches('\u{feff}');
    let Some(rest) = trimmed.strip_prefix("---\n").or_else(|| trimmed.strip_prefix("---\r\n")) else {
        return s;
    };
    // Find the closing `---` on its own line.
    let mut idx = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            idx += line.len();
            return rest[idx..].trim_start_matches('\n');
        }
        idx += line.len();
    }
    s
}

fn gen_skill(env: &env_detect::Env, args: &[String]) -> Result<()> {
    if args.len() < 2 {
        ui::err("usage: 8sync skill gen <id1> <id2> [id3 …]");
        ui::info("  ids: 1-based index from `8sync skill` local list, or skill name");
        ui::info("  example: 8sync skill gen 1 2");
        ui::info("           8sync skill gen karpathy-guidelines codegraph");
        return Ok(());
    }
    let root = detect_current_project_root()
        .ok_or_else(|| anyhow!("`skill gen` must run inside a project (no .git / Cargo.toml / package.json found)"))?;
    let local_dir = root.join("agents/skills");
    let locals = list_installed_skill_dirs(&local_dir).unwrap_or_default();
    if locals.is_empty() {
        return Err(anyhow!(
            "no local skills under {} — run `8sync skill sync` first",
            local_dir.display()
        ));
    }

    // Resolve each arg → (path, meta, body).
    let mut parts: Vec<(PathBuf, SkillMeta, String)> = Vec::with_capacity(args.len());
    for arg in args {
        let p = resolve_skill_arg(arg, &locals)?;
        let (m, _entry) = meta_for_dir(&p);
        let skill_md = p.join("SKILL.md");
        let body = std::fs::read_to_string(&skill_md)
            .map_err(|e| anyhow!("could not read {}: {}", skill_md.display(), e))?;
        parts.push((p, m, body));
    }

    // Synthesize fused name + description + body.
    let fused_name: String = parts
        .iter()
        .map(|(_, m, _)| m.name.clone())
        .collect::<Vec<_>>()
        .join("+");
    if fused_name.len() > 64 {
        ui::warn(&format!(
            "fused name `{}` is {} chars (>64 — Agent Skills spec recommends ≤64)",
            fused_name,
            fused_name.len()
        ));
    }
    let fused_desc = format!(
        "Combined skill fusing {}. Use this when the user's task spans the concerns of all listed component skills. The AI MUST apply every component's rules together — read each section below before acting. Components: {}.",
        parts
            .iter()
            .map(|(_, m, _)| format!("`{}`", m.name))
            .collect::<Vec<_>>()
            .join(" + "),
        parts
            .iter()
            .map(|(_, m, _)| m.description.chars().take(140).collect::<String>())
            .collect::<Vec<_>>()
            .join(" || "),
    );

    let mut body = String::with_capacity(4096);
    body.push_str("---\n");
    body.push_str(&format!("name: {}\n", fused_name));
    body.push_str(&format!("description: {}\n", yaml_quote(&fused_desc)));
    body.push_str("sources:\n");
    for (p, _, _) in &parts {
        body.push_str(&format!("  - {}\n", p.display()));
    }
    body.push_str("---\n\n");
    body.push_str(&format!(
        "> Fused skill generated by `8sync skill gen` from {} components.\n",
        parts.len()
    ));
    body.push_str("> Read every section below — the AI MUST apply ALL component rules together.\n\n");
    for (i, (_, m, raw)) in parts.iter().enumerate() {
        body.push_str(&format!("---\n\n## Component {}: `{}`\n\n", i + 1, m.name));
        if !m.description.is_empty() {
            body.push_str(&format!("_{}_\n\n", m.description));
        }
        body.push_str(strip_frontmatter(raw).trim_start());
        if !body.ends_with('\n') {
            body.push('\n');
        }
        body.push('\n');
    }

    // Write to <root>/agents/skills/<fused_name>/SKILL.md (and global mirror).
    let local_target = local_dir.join(&fused_name);
    let global_target = env.home.join(".omp/skills").join(&fused_name);
    std::fs::create_dir_all(&local_target)?;
    std::fs::create_dir_all(&global_target)?;
    std::fs::write(local_target.join("SKILL.md"), &body)?;
    std::fs::write(global_target.join("SKILL.md"), &body)?;
    ui::ok(&format!("wrote {}/SKILL.md", local_target.display()));
    ui::ok(&format!("wrote {}/SKILL.md", global_target.display()));

    // Re-inject AGENTS.md so the fused skill appears in the force-load block.
    inject_agents_md(&env.home, &root)?;
    ui::info(&format!(
        "fused skill `{}` ready — omp will load it on next session",
        fused_name
    ));
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
    println!("  8sync skill gen <id1> <id2> [id3 …]   # fuse N local skills into one combined SKILL.md");

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

/// Rewrite (or insert) the force-load block in **every** agent entry file at
/// the project root: AGENTS.md, CLAUDE.md, GEMINI.md, .cursorrules, .windsurfrules.
/// Each existing file gets the block injected. Each missing well-known file gets
/// stub-created so future agents can't avoid the block.
///
/// The block leads with explicit `READ NOW: <absolute SKILL.md path>` lines so
/// agents that index headings see the exact files to open before anything else.
pub fn inject_agents_md(home: &Path, root: &Path) -> Result<()> {
    let global_dir = home.join(".omp/skills");
    let local_dir = root.join("agents/skills");

    let mut globals = list_installed_skill_dirs(&global_dir).unwrap_or_default();
    let mut locals = list_installed_skill_dirs(&local_dir).unwrap_or_default();
    // Force-rank: codegraph is always first (rule #0 of force-load).
    pin_skill_first(&mut globals, "codegraph");
    pin_skill_first(&mut locals, "codegraph");

    // --- 1. READ NOW header: explicit absolute paths agents must open right now.
    let mut read_now = String::new();
    read_now.push_str("**READ NOW (in order). Do NOT skip. Open each file BEFORE the first tool call:**\n\n");
    let mut idx = 1usize;
    for p in globals.iter().chain(locals.iter()) {
        let (_, entry) = meta_for_dir(p);
        let file = p.join(entry);
        read_now.push_str(&format!("  {}. `{}`\n", idx, file.display()));
        idx += 1;
    }
    if idx == 1 {
        read_now.push_str("  _(no skills installed yet — run `8sync skill sync`)_\n");
    }

    // --- 2. Annotated list with descriptions (for humans + indexing agents).
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
                i + 1, m.name, dirname, entry, desc,
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
                i + 1, m.name, dirname, entry, desc,
            ));
        }
    }

    let block = format!(
        "{BEGIN}\n\
## 🚨 MANDATORY — đọc TRƯỚC khi làm bất cứ task gì\n\
\n\
Bạn (AI) **PHẢI** đọc đầy đủ các skill liệt kê dưới đây **trước dòng code đầu tiên** trong session này. Không skip, không suy đoán, không viết tắt.\n\
\n\
{read_now}\n\
Mỗi skill là 1 directory theo [Agent Skills open standard](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview): `SKILL.md` ở root có YAML frontmatter (`name`, `description`). Description cho biết **khi nào** dùng skill.\n\
\n\
### Global skills (always-on — `~/.omp/skills/`)\n\
{g_lines}\n\
### Project-local skills (BẮT BUỘC dùng cho repo này — `agents/skills/`)\n\
{l_lines}\n\
### Quy tắc bất biến\n\
\n\
- Đọc TẤT CẢ `SKILL.md` / `CLAUDE.md` ở 2 list trên **TRƯỚC** khi gọi tool đầu tiên.\n\
- **Codegraph FIRST** cho mọi câu hỏi explore code: `codegraph` thay vì grep/find/Read.\n\
- Nếu skill có thư mục `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.\n\
- Nếu skill có `references/` → đọc on-demand khi task chạm vào chủ đề tương ứng.\n\
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.\n\
- Nếu một skill local có vẻ liên quan đến task hiện tại (theo description), bạn **MUST** đọc nó trước khi sửa code.\n\
{END}"
    );

    // --- 3. Inject into every known agent entry file.
    //   *.md       → markdown skeleton with H1 + block after it (uses sentinel rewrite)
    //   .cursorrules / .windsurfrules → plain-text rule files; the sentinel HTML
    //     comments are tolerated as inert text by Cursor/Windsurf
    let targets: &[(&str, EntryKind)] = &[
        ("AGENTS.md",       EntryKind::Markdown { h1: "AGENTS.md — guidance for AI" }),
        ("CLAUDE.md",       EntryKind::Markdown { h1: "CLAUDE.md — guidance for Claude Code" }),
        ("GEMINI.md",       EntryKind::Markdown { h1: "GEMINI.md — guidance for Gemini" }),
        (".cursorrules",    EntryKind::Plain),
        (".windsurfrules",  EntryKind::Plain),
    ];

    let mut written = 0usize;
    for (name, kind) in targets {
        let path = root.join(name);
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        // Only stub-create AGENTS.md + CLAUDE.md by default — the others stay
        // opt-in (created only if they already exist). This avoids polluting
        // every repo with .cursorrules / GEMINI.md the user never asked for.
        let stub_create = matches!(*name, "AGENTS.md" | "CLAUDE.md");
        if existing.is_empty() && !stub_create { continue; }

        let new_contents = match kind {
            EntryKind::Markdown { h1 } => rewrite_md_with_block(&existing, &block, h1),
            EntryKind::Plain => rewrite_plain_with_block(&existing, &block),
        };
        if new_contents != existing {
            std::fs::write(&path, &new_contents)?;
            ui::ok(&format!("injected force-load → {}", path.display()));
            written += 1;
        }
    }

    ui::info(&format!(
        "force-load: {} global skills, {} local skills, written into {} file(s)",
        globals.len(), locals.len(), written,
    ));
    Ok(())
}

enum EntryKind {
    Markdown { h1: &'static str },
    Plain,
}

/// Markdown injection: replace existing sentinel block, or insert after the first
/// H1, or create a minimal skeleton if the file is empty.
fn rewrite_md_with_block(existing: &str, block: &str, h1_title: &str) -> String {
    if let (Some(b), Some(e)) = (find_line(existing, BEGIN), find_line(existing, END)) {
        if b < e {
            let mut s = String::with_capacity(existing.len() + block.len());
            s.push_str(&existing[..b]);
            s.push_str(block);
            s.push_str(&existing[e + END.len()..]);
            return s;
        }
    }
    if existing.is_empty() {
        return format!("# {h1_title}\n\n{block}\n");
    }
    insert_block_after_h1(existing, block)
}

/// Plain-text injection (.cursorrules / .windsurfrules): replace existing
/// sentinel block, or prepend the block at the top of the file.
fn rewrite_plain_with_block(existing: &str, block: &str) -> String {
    if let (Some(b), Some(e)) = (find_line(existing, BEGIN), find_line(existing, END)) {
        if b < e {
            let mut s = String::with_capacity(existing.len() + block.len());
            s.push_str(&existing[..b]);
            s.push_str(block);
            s.push_str(&existing[e + END.len()..]);
            return s;
        }
    }
    let sep = if existing.is_empty() { "" } else { "\n\n" };
    format!("{block}{sep}{existing}")
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

/// Reorder `dirs` so the directory whose basename equals `name` is first.
/// Used to force codegraph (and any future always-on skill) to the top of
/// the AGENTS.md force-load list — agents read top-down, so position == priority.
fn pin_skill_first(dirs: &mut Vec<PathBuf>, name: &str) {
    if let Some(idx) = dirs.iter().position(|p| {
        p.file_name().and_then(|s| s.to_str()) == Some(name)
    }) {
        if idx != 0 {
            let p = dirs.remove(idx);
            dirs.insert(0, p);
        }
    }
}
