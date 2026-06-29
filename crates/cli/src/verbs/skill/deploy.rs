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
    let bundled: [(&str, &str); 15] = [
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

/// Clean cutover for machines that installed an earlier 8sync: remove the retired
/// `/gs` command + skill (global + project). Idempotent no-op when absent — `/auto`
/// is the single automation entry now.
pub(crate) fn cleanup_legacy_gs(home: &Path, root: Option<&Path>) {
    let _ = std::fs::remove_file(home.join(".omp/agent/commands/gs.md"));
    let _ = std::fs::remove_dir_all(home.join(".omp/skills/gs"));
    if let Some(r) = root {
        let _ = std::fs::remove_file(r.join(".omp/commands/gs.md"));
        let _ = std::fs::remove_dir_all(r.join("agents/skills/gs"));
    }
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
/// Ensure omp's anti-forget stack in the user's settings (`~/.omp/agent/config.yml`):
/// (1) Mnemopi long-term memory (API-only — no local model), and (2) compaction
/// tuned to fire at 50% context + when idle (snapcompact strategy stays the omp
/// default), so the agent stops forgetting skills/rules/workflow past ~50%.
/// Idempotent sentinel-block; never clobbers a user-authored `memory:` block.
pub(crate) fn ensure_omp_memory_config(home: &Path) -> Result<()> {
    let cfg = home.join(".omp/agent/config.yml");
    if let Some(p) = cfg.parent() { std::fs::create_dir_all(p)?; }
    // omp rewrites/normalizes config.yml and strips comments, so detect by KEY
    // presence (not sentinel markers) and only append top-level keys when absent.
    let mut s = std::fs::read_to_string(&cfg).unwrap_or_default();
    let mut changed = false;
    let has_mnemopi = s.contains("backend: mnemopi");
    let has_memory_key = s.lines().any(|l| l.starts_with("memory:"));
    if has_mnemopi {
        ui::skip("mnemopi memory", "backend already set");
    } else if has_memory_key {
        ui::warn("config.yml has its own `memory:` — left as-is");
    } else {
        s.push_str("\nmemory:\n  backend: mnemopi\nmnemopi:\n  scoping: per-project-tagged\n  llmMode: smol\n  noEmbeddings: true\n  polyphonicRecall: true\n");
        changed = true;
        ui::ok("mnemopi memory enabled (API-only)");
    }
    if s.lines().any(|l| l.starts_with("compaction:")) {
        ui::skip("compaction@50%", "key already present (user-configured)");
    } else {
        s.push_str("\ncompaction:\n  thresholdPercent: 50\n  idleEnabled: true\n");
        changed = true;
        ui::ok("compaction@50% + idle enabled (anti-forget)");
    }
    if changed { std::fs::write(&cfg, s)?; }
    Ok(())
}

/// Deploy the anti-forget recall hook to `~/.omp/hooks/pre/8sync-recall.ts`.
/// The hook injects a lean ref bundle (skill index + live STATE) at every
/// `before_agent_start` and into every compaction summary, so the agent keeps
/// the skill/rule/workflow index fresh even past 50% context / compaction.
/// Idempotent: skipped if the deployed file is byte-identical to the asset.
pub(crate) fn ensure_recall_hook(home: &Path) -> Result<()> {
    let dir = home.join(".omp/hooks/pre");
    std::fs::create_dir_all(&dir)?;
    let target = dir.join("8sync-recall.ts");
    let Some(body) = assets::read("hooks/8sync-recall.ts") else { return Ok(()); };
    if std::fs::read(&target).ok().as_deref() == Some(body.as_bytes()) {
        ui::skip("recall hook", "already deployed");
        return Ok(());
    }
    std::fs::write(&target, body.as_bytes())?;
    ui::ok(&format!("recall hook → {}", target.display()));
    Ok(())
}

/// Deploy the always-apply operating directives to `~/.omp/agent/APPEND_SYSTEM.md`.
/// omp appends this verbatim to EVERY system prompt (never compacts away), so the
/// code-intel-first rule + always-on skill manifest are read on every turn — the
/// fix for "skills/rules are defined but the agent ignores them past ~50% context".
/// Idempotent (byte-identical skip); appended, so omp's base prompt is preserved.
pub(crate) fn ensure_append_system(home: &Path) -> Result<()> {
    let Some(body) = assets::read("configs/omp/APPEND_SYSTEM.md") else {
        return Ok(());
    };
    let target = home.join(".omp/agent/APPEND_SYSTEM.md");
    if let Some(p) = target.parent() {
        std::fs::create_dir_all(p)?;
    }
    if std::fs::read_to_string(&target).ok().as_deref() == Some(body.as_str()) {
        ui::skip("APPEND_SYSTEM.md", "already deployed");
        return Ok(());
    }
    std::fs::write(&target, &body)?;
    ui::ok(&format!("always-on directives → {}", target.display()));
    Ok(())
}

/// Register serena (LSP-based semantic code toolkit) as an omp MCP server, giving
/// the agent symbol-level find + precise edits — token-cheaper than blind file
/// reads/rewrites. Launched via `uvx` (always-latest, no install) when `uv` is
/// present; otherwise skipped with a hint (the launcher must be on PATH).
pub(crate) fn ensure_serena_mcp(env: &env_detect::Env) -> Result<()> {
    if which::which("uvx").is_err() && which::which("uv").is_err() {
        ui::skip(
            "serena MCP",
            "needs `uv` (https://docs.astral.sh/uv) — install it then re-run `8sync harness`",
        );
        return Ok(());
    }
    register_omp_mcp(
        &env.home,
        "serena",
        "uvx",
        &[
            "--from",
            "git+https://github.com/oraios/serena",
            "serena-mcp-server",
            "--context",
            "ide-assistant",
        ],
    )
}
/// Best-effort: ensure the `feynman` research CLI (companion-inc/feynman) is
/// available so the 20 feynman research skills registered in agents/skills.toml
/// (deep-research, alpha-research, literature-review, …) are functional rather
/// than inert — they shell out to `feynman`/`alpha`. A failed install is
/// non-fatal (skills still list; the user can `npx @companion-ai/feynman`
/// later). Never bails the harness run.
pub(crate) fn ensure_feynman_cli() {
    if which::which("feynman").is_ok() {
        let v = env_detect::cmd_version("feynman", &["--version"]).unwrap_or_default();
        ui::skip("feynman CLI", &format!("present ({})", v));
        return;
    }
    ui::step("feynman CLI (missing — installing @companion-ai/feynman)");
    // Global install so skills resolve `feynman` directly on PATH. `npx` remains
    // the zero-install fallback, so a non-zero exit is only a soft failure.
    let cmd = "npm install -g @companion-ai/feynman 2>/dev/null || true";
    match Command::new("sh").arg("-c").arg(cmd).status() {
        Ok(s) if s.success() && which::which("feynman").is_ok() => {
            ui::ok("feynman CLI installed (research skills functional)");
        }
        _ => ui::warn(
            "feynman global install skipped/failed — skills still list (run via `npx @companion-ai/feynman`)",
        ),
    }
}

/// Deploy the `8sync-workflow` omp extension — a gsd-pi-grade surface that
/// registers model-callable workflow tools (wf_state_get/set, persisted across
/// compaction via a custom session entry) + a `/wf` status command + a
/// session_start state-restore handler. Lives in omp's config dir
/// (`~/.omp/agent/extensions/` global + `<root>/.omp/extensions/` project) so it
/// NEVER patches omp core → omp updates stay safe. The Workflow viz page
/// (`8sync harness web`) appends exported-workflow `registerTool` blocks to the
/// project copy. Idempotent (byte-identical skip), mirrors `ensure_gs_command`.
pub(crate) fn ensure_workflow_extension(home: &Path, root: Option<&Path>) -> Result<()> {
    let Some(body) = assets::read("extensions/8sync-workflow.ts") else {
        return Ok(());
    };
    let global = home.join(".omp/agent/extensions/8sync-workflow.ts");
    if let Some(p) = global.parent() {
        std::fs::create_dir_all(p)?;
    }
    let changed = std::fs::read_to_string(&global).map(|s| s != body).unwrap_or(true);
    std::fs::write(&global, &body)?;
    if changed {
        ui::ok(&format!("8sync-workflow extension → {}", global.display()));
    }
    if let Some(r) = root {
        let proj = r.join(".omp/extensions/8sync-workflow.ts");
        if let Some(p) = proj.parent() {
            std::fs::create_dir_all(p)?;
        }
        let changed = std::fs::read_to_string(&proj).map(|s| s != body).unwrap_or(true);
        std::fs::write(&proj, &body)?;
        if changed {
            ui::ok(&format!("8sync-workflow extension → {}", proj.display()));
        }
    }
    Ok(())
}

/// Deploy an omp artifact (command/extension) to the global config dir and, when
/// inside a project, the project config dir too. Byte-identical writes are quiet.
fn deploy_omp_pair(
    home: &Path,
    root: Option<&Path>,
    asset: &str,
    global_rel: &str,
    proj_rel: &str,
    label: &str,
) -> Result<()> {
    let Some(body) = assets::read(asset) else {
        return Ok(());
    };
    let global = home.join(global_rel);
    if let Some(p) = global.parent() {
        std::fs::create_dir_all(p)?;
    }
    let changed = std::fs::read_to_string(&global).map(|s| s != body).unwrap_or(true);
    std::fs::write(&global, &body)?;
    if changed {
        ui::ok(&format!("{} → {}", label, global.display()));
    }
    if let Some(r) = root {
        let proj = r.join(proj_rel);
        if let Some(p) = proj.parent() {
            std::fs::create_dir_all(p)?;
        }
        let changed = std::fs::read_to_string(&proj).map(|s| s != body).unwrap_or(true);
        std::fs::write(&proj, &body)?;
        if changed {
            ui::ok(&format!("{} → {}", label, proj.display()));
        }
    }
    Ok(())
}

/// Deploy the gsd-pi-style automation engine — the `8sync-engine` omp extension
/// (durable slice/task state machine + code-enforced verify-retry gate + git
/// worktree tools) and its `/auto` orchestration command. 100% on omp core (config
/// dirs only, never patches omp) so updates stay safe. Mirrors the workflow ext.
pub(crate) fn ensure_engine(home: &Path, root: Option<&Path>) -> Result<()> {
    deploy_omp_pair(
        home,
        root,
        "extensions/8sync-engine.ts",
        ".omp/agent/extensions/8sync-engine.ts",
        ".omp/extensions/8sync-engine.ts",
        "8sync-engine extension",
    )?;
    deploy_omp_pair(
        home,
        root,
        "commands/auto.md",
        ".omp/agent/commands/auto.md",
        ".omp/commands/auto.md",
        "/auto command",
    )
}
