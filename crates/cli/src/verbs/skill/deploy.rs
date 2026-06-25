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
    let bundled: [(&str, &str); 16] = [
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
        ("skills/gs",                      "gs"),
        ("skills/token-bench",             "token-bench"),
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

/// Deploy the `/gs` orchestrator slash command into omp's command dirs so it is
/// available as `/gs` in every session. Always writes the global user command
/// (`~/.omp/agent/commands/gs.md`); inside a project also writes the repo copy
/// (`<root>/.omp/commands/gs.md`) so the whole team gets `/gs` on clone.
pub(crate) fn ensure_gs_command(home: &Path, root: Option<&Path>) -> Result<()> {
    let Some(body) = assets::read("commands/gs.md") else {
        return Ok(());
    };
    let global = home.join(".omp/agent/commands/gs.md");
    if let Some(p) = global.parent() {
        std::fs::create_dir_all(p)?;
    }
    let changed = std::fs::read_to_string(&global).map(|s| s != body).unwrap_or(true);
    std::fs::write(&global, &body)?;
    if changed {
        ui::ok(&format!("/gs command → {}", global.display()));
    }
    if let Some(r) = root {
        let proj = r.join(".omp/commands/gs.md");
        if let Some(p) = proj.parent() {
            std::fs::create_dir_all(p)?;
        }
        let changed = std::fs::read_to_string(&proj).map(|s| s != body).unwrap_or(true);
        std::fs::write(&proj, &body)?;
        if changed {
            ui::ok(&format!("/gs command → {}", proj.display()));
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
pub(crate) fn mirror_global_to_local(home: &Path, root: &Path, force: bool) -> Result<usize> {
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

        // Additive by default: never clobber an existing (maybe customized) local
        // skill — only vendor missing ones. `--force` re-mirrors everything.
        let existed = local_target.exists();
        if existed && !force {
            ui::skip(&local_target.display().to_string(), "exists (use --force to refresh)");
            count += 1;
            continue;
        }
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

/// Ensure the `codebase-memory-mcp` binary is installed (upstream installer,
/// binary-only) and registered as an omp MCP server. Mirrors `ensure_codegraph`:
/// `8sync harness` auto-sets-up code intelligence so the agent gets the graph
/// tools (search_graph/semantic_query/trace_path/…) with zero manual config.
pub(crate) fn ensure_codebase_memory_mcp(env: &env_detect::Env) -> Result<()> {
    if which::which("codebase-memory-mcp").is_err() {
        ui::step("codebase-memory-mcp (binary missing — upstream installer, binary-only)");
        let url = "https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh";
        let st = Command::new("sh")
            .arg("-c")
            .arg(format!("curl -fsSL {} | bash -s -- --skip-config", url))
            .status();
        match st {
            Ok(s) if s.success() => ui::ok("codebase-memory-mcp installed"),
            Ok(s) => ui::warn(&format!("codebase-memory-mcp installer exited {} — continuing", s)),
            Err(e) => ui::warn(&format!("could not run installer: {} — continuing", e)),
        }
    } else {
        let v = env_detect::cmd_version("codebase-memory-mcp", &["--version"]).unwrap_or_default();
        ui::skip("codebase-memory-mcp", &format!("present ({})", v));
    }
    if which::which("codebase-memory-mcp").is_ok() {
        // Self-index on every MCP connect — no manual reindex needed thereafter.
        let _ = Command::new("codebase-memory-mcp")
            .args(["config", "set", "auto_index", "true"])
            .status();
    }
    register_omp_mcp(&env.home, "codebase-memory-mcp", "codebase-memory-mcp", &[])
}

/// Idempotently add an MCP server `name` (stdio `command` + `args`) to omp's user
/// MCP config (`~/.omp/agent/mcp.json`), preserving any servers already there.
fn register_omp_mcp(home: &Path, name: &str, command: &str, args: &[&str]) -> Result<()> {
    let mcp_path = home.join(".omp/agent/mcp.json");
    if let Some(p) = mcp_path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut root: serde_json::Value = std::fs::read_to_string(&mcp_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if !root.is_object() {
        root = serde_json::json!({});
    }
    let obj = root.as_object_mut().unwrap();
    obj.entry("$schema").or_insert_with(|| {
        serde_json::Value::String(
            "https://raw.githubusercontent.com/can1357/oh-my-pi/main/packages/coding-agent/src/config/mcp-schema.json"
                .to_string(),
        )
    });
    let servers = obj.entry("mcpServers").or_insert_with(|| serde_json::json!({}));
    if !servers.is_object() {
        *servers = serde_json::json!({});
    }
    let smap = servers.as_object_mut().unwrap();
    if smap.contains_key(name) {
        ui::skip(name, "already in omp mcp.json");
        return Ok(());
    }
    smap.insert(
        name.to_string(),
        serde_json::json!({ "type": "stdio", "command": command, "args": args }),
    );
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&root)?)?;
    ui::ok(&format!("registered {} MCP → {}", name, mcp_path.display()));
    Ok(())
}

/// Best-effort: build/refresh the codebase-memory-mcp knowledge graph for `root`.
pub(crate) fn index_codebase_memory(root: &Path) {
    if which::which("codebase-memory-mcp").is_err() {
        return;
    }
    ui::step("codebase-memory-mcp index (knowledge graph)");
    let arg = serde_json::json!({ "repo_path": root.display().to_string() }).to_string();
    let _ = Command::new("codebase-memory-mcp")
        .args(["cli", "index_repository"])
        .arg(arg)
        .status();
}

/// Ensure `headroom` (context-compression MCP) is installed + registered as an
/// omp MCP server. Headroom compresses long tool outputs / logs / diffs before
/// they reach the model (60–95% fewer tokens) — complements codegraph/cbm.
pub(crate) fn ensure_headroom_mcp(env: &env_detect::Env) -> Result<()> {
    if which::which("headroom").is_err() {
        ui::step("headroom (missing — installing headroom-ai[mcp])");
        // Isolated installs first (Arch is PEP-668 externally-managed).
        let cmd = "if command -v uv >/dev/null 2>&1; then uv tool install 'headroom-ai[mcp]'; \
elif command -v pipx >/dev/null 2>&1; then pipx install 'headroom-ai[mcp]'; \
else pip install --user 'headroom-ai[mcp]' || pip install --user --break-system-packages 'headroom-ai[mcp]'; fi";
        let st = Command::new("sh").arg("-c").arg(cmd).status();
        match st {
            Ok(s) if s.success() => ui::ok("headroom installed"),
            Ok(s) => ui::warn(&format!("headroom install exited {} — registering anyway (manual: pipx install 'headroom-ai[mcp]')", s)),
            Err(e) => ui::warn(&format!("could not run installer: {} — continuing", e)),
        }
    } else {
        let v = env_detect::cmd_version("headroom", &["--version"]).unwrap_or_default();
        ui::skip("headroom", &format!("present ({})", v));
    }
    register_omp_mcp(&env.home, "headroom", "headroom", &["mcp", "serve"])
}

/// Enable omp's local long-term memory (Mnemopi) in the user's omp settings
/// (`~/.omp/agent/config.yml`) so the agent recalls + retains durable project
/// memory across sessions — "deep awareness that never forgets". API-only by
/// design: `llmMode: smol` reuses the configured online model and
/// `noEmbeddings: true` uses full-text recall, so there are NO local model
/// downloads (runs on any machine). Idempotent + non-clobbering: skips if
/// Mnemopi is already configured or the user authored their own `memory:` block.
pub(crate) fn ensure_mnemopi_memory(home: &Path) -> Result<()> {
    let cfg = home.join(".omp/agent/config.yml");
    if let Some(p) = cfg.parent() {
        std::fs::create_dir_all(p)?;
    }
    let existing = std::fs::read_to_string(&cfg).unwrap_or_default();
    const SENTINEL: &str = "# >>> 8sync mnemopi (managed) >>>";
    if existing.contains("backend: mnemopi") || existing.contains(SENTINEL) {
        ui::skip("mnemopi memory", "already enabled in config.yml");
        return Ok(());
    }
    // Never clobber a user-authored memory backend — they made that choice.
    if existing.lines().any(|l| l.starts_with("memory:")) {
        ui::warn("config.yml has its own `memory:` — left as-is; set `backend: mnemopi` manually for recall");
        return Ok(());
    }
    let block = concat!(
        "# >>> 8sync mnemopi (managed) >>>\n",
        "# Local long-term memory: recall + retain durable project memory across sessions.\n",
        "# API-only (no local model): llmMode smol reuses the online model; noEmbeddings = FTS recall.\n",
        "memory:\n",
        "  backend: mnemopi\n",
        "mnemopi:\n",
        "  scoping: per-project-tagged\n",
        "  llmMode: smol\n",
        "  noEmbeddings: true\n",
        "  polyphonicRecall: true\n",
        "# <<< 8sync mnemopi <<<\n",
    );
    let sep = if existing.is_empty() || existing.ends_with('\n') { "" } else { "\n" };
    std::fs::write(&cfg, format!("{existing}{sep}{block}"))?;
    ui::ok(&format!("mnemopi memory enabled (API-only) → {}", cfg.display()));
    Ok(())
}
