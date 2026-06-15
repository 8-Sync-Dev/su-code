//! Bundled-skill deployment (embedded assets → ~/.omp/skills), project mirror,
//! and codegraph bootstrap. The building blocks `8sync harness init` composes.
use anyhow::Result;
use std::path::Path;
use std::process::Command;

use super::discover::list_installed_skill_dirs;
use crate::{assets, env_detect, ui};

/// Deploy every bundled skill tree under `assets/skills/<name>/` into
/// `~/.omp/skills/<name>/`. Each tree is deployed verbatim including any
/// `references/` or `scripts/` subdirs. Shell scripts get mode 0755.
pub(crate) fn install_bundled_global(env: &env_detect::Env) -> Result<()> {
    let skills_dir = env.home.join(".omp/skills");
    // (asset prefix, target subdir name). always-on first (read order), then
    // on-demand specialists. Encore/full-flow are on-demand + tech-gated.
    let bundled: [(&str, &str); 14] = [
        ("skills/codegraph",               "codegraph"),
        ("skills/karpathy",                "karpathy-guidelines"),
        ("skills/ponytail",                "ponytail"),
        ("skills/assp-skill",              "assp-skill"),
        ("skills/impeccable",              "impeccable"),
        ("skills/taste-skill",             "taste-skill"),
        ("skills/8sync-cli",               "8sync-cli"),
        ("skills/image-routing",           "image-routing"),
        ("skills/code-review-and-quality", "code-review-and-quality"),
        ("skills/senior-security",         "senior-security"),
        ("skills/senior-frontend",         "senior-frontend"),
        ("skills/full-flow",               "full-flow"),
        ("skills/encore-deploy",           "encore-deploy"),
        ("skills/last30days",              "last30days"),
    ];
    for (asset_prefix, name) in bundled {
        let target_dir = skills_dir.join(name);
        std::fs::create_dir_all(&target_dir)?;
        let (written, _unchanged) = assets::install_tree(asset_prefix, &target_dir)?;
        if written > 0 {
            ui::ok(&format!("synced {} ({} file(s) written) → {}", name, written, target_dir.display()));
        }
    }
    Ok(())
}

/// Ensure a skill directory follows the Agent Skills 3-folder layout:
///   <name>/ ├── SKILL.md  ├── scripts/  └── references/
/// Idempotent. Empty dirs are intentional.
pub(crate) fn ensure_skill_layout(dir: &Path) {
    for sub in ["scripts", "references"] {
        let p = dir.join(sub);
        if !p.exists() {
            let _ = std::fs::create_dir_all(&p);
        }
    }
}

/// For every skill dir under `~/.omp/skills/`, create or refresh a copy under
/// `<root>/agents/skills/<name>/`. Returns the number of skills processed.
pub(crate) fn mirror_global_to_local(home: &Path, root: &Path) -> Result<usize> {
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
pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
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

/// Make sure the `codegraph` binary is installed (upstream curl installer) and
/// registered in the skills.toml registry. The SKILL.md tree is deployed
/// separately from embedded assets.
pub(crate) fn ensure_codegraph(env: &env_detect::Env) -> Result<()> {
    if which::which("codegraph").is_err() {
        ui::step("codegraph (binary missing — running upstream curl installer)");
        let url = "https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.sh";
        let st = Command::new("sh")
            .arg("-c")
            .arg(format!("curl -fsSL {} | sh", url))
            .status();
        match st {
            Ok(s) if s.success() => ui::ok("codegraph installed"),
            Ok(s) => ui::warn(&format!("codegraph installer exited {} — skill SKILL.md was still deployed", s)),
            Err(e) => ui::warn(&format!("could not run installer: {} — continuing", e)),
        }
    } else {
        let v = env_detect::cmd_version("codegraph", &["--version"]).unwrap_or_default();
        ui::skip("codegraph", &format!("present ({})", v));
    }

    let toml_path = env.xdg_config.join("8sync/skills.toml");
    if let Some(parent) = toml_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = std::fs::read_to_string(&toml_path).unwrap_or_default();
    if !existing.contains("[codegraph]") {
        let mut s = existing;
        if !s.ends_with('\n') && !s.is_empty() {
            s.push('\n');
        }
        s.push_str("\n[codegraph]\nsrc  = \"builtin:codegraph\"\nwhen = \"always\"\n");
        std::fs::write(&toml_path, s)?;
        ui::ok(&format!("registered 'codegraph' → {}", toml_path.display()));
    }
    Ok(())
}

/// If `<root>/.codegraph/` is missing and the `codegraph` binary is on PATH,
/// run `codegraph init <root>`. Best-effort: warns on failure, never bails.
pub(crate) fn ensure_codegraph_init(root: &Path) {
    let marker = root.join(".codegraph");
    if marker.exists() {
        ui::skip(&marker.display().to_string(), "codegraph already initialised");
        return;
    }
    if which::which("codegraph").is_err() {
        ui::warn("codegraph binary not on PATH — skipping `codegraph init`");
        return;
    }
    ui::step(&format!("codegraph init {}", root.display()));
    let st = Command::new("codegraph").arg("init").arg(root).status();
    match st {
        Ok(s) if s.success() => ui::ok(&format!("initialised {}", marker.display())),
        Ok(s) => ui::warn(&format!("`codegraph init` exited {} — run manually", s)),
        Err(e) => ui::warn(&format!("could not invoke codegraph: {}", e)),
    }
}
