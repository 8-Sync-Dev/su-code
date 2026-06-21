//! Skill source specs (parse), GitHub README fetch / shallow clone, and
//! SKILL.md synthesis for non-spec repos.
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ui;

/// Source spec parsed from `add` argument.
pub(crate) enum Source {
    Git { url: String, name: String, git_ref: Option<String> },
    Path { src: PathBuf, name: String },
    Builtin { name: String },
}

pub(crate) fn parse_spec(spec: &str) -> Result<Source> {
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
        let (rest, git_ref) = split_git_ref(rest);
        let default_name = rest
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("bad gh spec: {}", rest))?
            .to_string();
        return Ok(Source::Git {
            url: format!("https://github.com/{}", rest.trim_end_matches(".git")),
            name: with_name(default_name),
            git_ref,
        });
    }
    if core.starts_with("https://") || core.starts_with("http://") || core.starts_with("git@") {
        // Pin syntax `<url>@<ref>` for http(s)/gh; SSH `git@host:...` keeps its `@`.
        let (base, git_ref) = if core.starts_with("git@") {
            (core, None)
        } else {
            split_git_ref(core)
        };
        let default_name = base
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("bad git url: {}", base))?
            .to_string();
        return Ok(Source::Git { url: base.to_string(), name: with_name(default_name), git_ref });
    }
    Err(anyhow!(
        "unknown spec `{}` — use <https URL> | gh:owner/repo | path:/abs | builtin:name (optional #newname suffix to rename)",
        spec
    ))
}

/// Split a trailing `@<ref>` pin off an http(s)/gh git spec. The ref must be a
/// bare token (no `/`, no `:`) so `git@host:owner/repo` and path-y refs stay
/// intact. Returns `(base, ref)`.
fn split_git_ref(s: &str) -> (&str, Option<String>) {
    match s.rsplit_once('@') {
        Some((base, r)) if !r.is_empty() && !r.contains('/') && !r.contains(':') => {
            (base, Some(r.to_string()))
        }
        _ => (s, None),
    }
}

/// Fetch the README.md of a public GitHub repo via raw.githubusercontent.com.
/// Tries refs in order: HEAD, main, master.
pub(crate) fn fetch_github_readme(owner: &str, repo: &str) -> Result<String> {
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
pub(crate) fn github_owner_repo(url: &str) -> Option<(String, String)> {
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
pub(crate) fn synthesize_skill_md(readme: &str, name: &str, source_url: &str) -> String {
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

pub(crate) fn yaml_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

/// Write a synthesised SKILL.md into `<target_dir>/SKILL.md`. Creates the dir.
pub(crate) fn write_synth_skill(target_dir: &Path, content: &str) -> Result<()> {
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

pub(crate) fn install_path_skill(src: &Path, target: &Path) -> Result<()> {
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

/// `git clone --depth 1 --quiet <url> <dest>`.
pub(crate) fn git_clone_shallow(url: &str, dest: &Path) -> Result<()> {
    let st = Command::new("git")
        .args(["clone", "--depth", "1", "--quiet", url])
        .arg(dest)
        .status()
        .map_err(|e| anyhow!("git unavailable: {}", e))?;
    if !st.success() {
        return Err(anyhow!("git clone exited non-zero for {}", url));
    }
    Ok(())
}

/// Clone `url` into `dest`. `git_ref = None` → fast `--depth 1` default-branch
/// clone; a ref → full clone + checkout so any commit/tag/branch resolves
/// (pins are rare; correctness over speed).
pub(crate) fn git_clone_at(url: &str, dest: &Path, git_ref: Option<&str>) -> Result<()> {
    let Some(r) = git_ref else {
        return git_clone_shallow(url, dest);
    };
    let st = Command::new("git")
        .args(["clone", "--quiet", url])
        .arg(dest)
        .status()
        .map_err(|e| anyhow!("git unavailable: {}", e))?;
    if !st.success() {
        return Err(anyhow!("git clone exited non-zero for {}", url));
    }
    let co = Command::new("git")
        .arg("-C")
        .arg(dest)
        .args(["checkout", "--quiet", r])
        .status()
        .map_err(|e| anyhow!("git unavailable: {}", e))?;
    if !co.success() {
        return Err(anyhow!("git checkout `{}` failed for {}", r, url));
    }
    Ok(())
}

/// `git -C <dir> rev-parse HEAD` → resolved commit SHA (for the registry lock).
pub(crate) fn resolve_head_sha(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Installable skills inside a cloned repo. Single-skill repo (root SKILL.md) →
/// `[(repo_name, root)]`; collection repo (`skills/<name>/SKILL.md`) → one entry
/// per sub-skill. Empty when neither layout matches (caller falls back to README).
pub(crate) fn collect_repo_skills(repo: &Path, repo_name: &str) -> Vec<(String, PathBuf)> {
    if repo.join("SKILL.md").is_file() {
        return vec![(repo_name.to_string(), repo.to_path_buf())];
    }
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(repo.join("skills")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && p.join("SKILL.md").is_file() {
                if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                    out.push((n.to_string(), p));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
