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
    let bundled: [(&str, &str); 18] = [
        ("skills/codegraph",               "codegraph"),
        ("skills/karpathy",                "karpathy-guidelines"),
        ("skills/ponytail",                "ponytail"),
        ("skills/assp-skill",              "assp-skill"),
        ("skills/impeccable",              "impeccable"),
        ("skills/taste-skill",             "taste-skill"),
        ("skills/8sync-cli",               "8sync-cli"),
        ("skills/image-routing",           "image-routing"),
        ("skills/zai-vision",              "zai-vision"),
        ("skills/locate-anything",         "locate-anything"),
        ("skills/code-review-and-quality", "code-review-and-quality"),
        ("skills/senior-security",         "senior-security"),
        ("skills/senior-frontend",         "senior-frontend"),
        ("skills/full-flow",               "full-flow"),
        ("skills/encore-deploy",           "encore-deploy"),
        ("skills/last30days",              "last30days"),
        ("skills/token-bench",             "token-bench"),
        ("skills/feature",                 "feature"),
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
        let _ = std::fs::remove_dir_all(r.join("su-code/skills/gs"));
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
/// `<root>/su-code/skills/<name>/`. Returns the number of skills processed.
pub(crate) fn mirror_global_to_local(home: &Path, root: &Path, force: bool) -> Result<usize> {
    let global_dir = home.join(".omp/skills");
    let local_dir = root.join("su-code/skills");
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
    register_omp_mcp(&env.home, "codebase-memory-mcp", "codebase-memory-mcp", &[], &[])
}

/// Idempotently add an MCP server `name` (stdio `command` + `args`) to omp's user
/// MCP config (`~/.omp/agent/mcp.json`), preserving any servers already there.
fn register_omp_mcp(home: &Path, name: &str, command: &str, args: &[&str], env: &[(&str, &str)]) -> Result<()> {
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
    let mut desired = serde_json::json!({ "type": "stdio", "command": command, "args": args });
    // Only emit an `env` key when there are vars — keeps the stored entry for the
    // env-less servers byte-identical (so the equality self-heal check holds).
    if !env.is_empty() {
        let env_obj: serde_json::Map<String, serde_json::Value> = env
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
            .collect();
        desired
            .as_object_mut()
            .expect("stdio mcp server is an object")
            .insert("env".into(), serde_json::Value::Object(env_obj));
    }
    if smap.get(name) == Some(&desired) {
        ui::skip(name, "already in omp mcp.json");
        return Ok(());
    }
    // Self-heal: update in place when the command/args changed (e.g. serena's
    // executable rename) instead of skipping a stale entry.
    let updating = smap.contains_key(name);
    smap.insert(name.to_string(), desired);
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&root)?)?;
    let verb = if updating { "updated" } else { "registered" };
    ui::ok(&format!("{} {} MCP → {}", verb, name, mcp_path.display()));
    Ok(())
}

/// Best-effort bootstrap of `uv` (Astral's Python tool manager) — the canonical
/// installer for both `headroom-ai[mcp]` and serena (`uvx`). User-level curl
/// install (no sudo); lands in `~/.local/bin` (already on PATH). Idempotent.
/// Returns true if `uv` is available afterwards.
fn ensure_uv() -> bool {
    if which::which("uv").is_ok() {
        return true;
    }
    ui::step("uv (missing — bootstrapping Astral uv: powers headroom + serena)");
    let _ = Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://astral.sh/uv/install.sh | sh")
        .status();
    which::which("uv").is_ok()
}

/// Remove a stale MCP server from omp's `mcp.json` (e.g. a tool whose binary
/// failed to install) so omp never fails at startup spawning a missing
/// executable. No-op when absent or the file is unreadable.
fn deregister_omp_mcp(home: &Path, name: &str) -> Result<()> {
    let mcp_path = home.join(".omp/agent/mcp.json");
    let Ok(s) = std::fs::read_to_string(&mcp_path) else {
        return Ok(());
    };
    let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&s) else {
        return Ok(());
    };
    let removed = root
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .is_some_and(|m| m.remove(name).is_some());
    if removed {
        std::fs::write(&mcp_path, serde_json::to_string_pretty(&root)?)?;
        ui::warn(&format!(
            "{} not installed — removed its stale MCP entry (omp won't error at startup)",
            name
        ));
    }
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
        ui::step("headroom (missing — installing headroom-ai[mcp] via uv)");
        if ensure_uv() {
            let _ = Command::new("uv")
                .args(["tool", "install", "headroom-ai[mcp]"])
                .status();
        }
        // Fallback for boxes with pipx/pip but no uv (e.g. curl bootstrap blocked).
        if which::which("headroom").is_err() {
            let cmd = "if command -v pipx >/dev/null 2>&1; then pipx install 'headroom-ai[mcp]'; \
elif command -v pip >/dev/null 2>&1; then pip install --user 'headroom-ai[mcp]' \
|| pip install --user --break-system-packages 'headroom-ai[mcp]'; fi";
            let _ = Command::new("sh").arg("-c").arg(cmd).status();
        }
    }
    // Register ONLY when the binary exists — never leave a broken MCP entry that
    // makes omp fail at startup. If still missing, purge any stale entry.
    if which::which("headroom").is_ok() {
        let v = env_detect::cmd_version("headroom", &["--version"]).unwrap_or_default();
        ui::ok(&format!("headroom present ({})", v.trim()));
        register_omp_mcp(&env.home, "headroom", "headroom", &["mcp", "serve"], &[])
    } else {
        ui::warn("headroom unavailable — skipped MCP (install `uv`: https://astral.sh/uv, then re-run `8sync harness`)");
        deregister_omp_mcp(&env.home, "headroom")
    }
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
    let target = dir.join(crate::brand::ns_file("recall.ts"));
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
    let body = crate::brand::render(&body).into_owned();
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

/// Deploy the bundled MCP `server.json` standard spec to `~/.omp/specs/` so it's
/// present on the machine by default — the on-disk ground truth every omp session
/// follows when writing/reasoning about `mcp.json`. APPEND_SYSTEM points here.
/// Idempotent (byte-identical skip).
pub(crate) fn ensure_mcp_spec(home: &Path) -> Result<()> {
    let Some(body) = assets::read("specs/mcp-server.md") else {
        return Ok(());
    };
    let body = crate::brand::render(&body).into_owned();
    let target = home.join(".omp/specs/mcp-server.md");
    if let Some(p) = target.parent() {
        std::fs::create_dir_all(p)?;
    }
    if std::fs::read_to_string(&target).ok().as_deref() == Some(body.as_str()) {
        ui::skip("mcp-server.md", "spec already deployed");
        return Ok(());
    }
    std::fs::write(&target, &body)?;
    ui::ok(&format!("MCP standard spec → {}", target.display()));
    Ok(())
}

/// Register serena (LSP-based semantic code toolkit) as an omp MCP server, giving
/// the agent symbol-level find + precise edits — token-cheaper than blind file
/// reads/rewrites. Launched via `uvx` (always-latest, no install); bootstraps
/// `uv` if absent. Skipped (and any stale entry purged) only if uv can't install.
pub(crate) fn ensure_serena_mcp(env: &env_detect::Env) -> Result<()> {
    if which::which("uvx").is_err() && which::which("uv").is_err() {
        ensure_uv();
    }
    if which::which("uvx").is_ok() || which::which("uv").is_ok() {
        register_omp_mcp(
            &env.home,
            "serena",
            "uvx",
            &[
                "--from",
                "git+https://github.com/oraios/serena",
                "serena",
                "start-mcp-server",
                "--context",
                "claude-code",
            ],
            &[],
        )
    } else {
        ui::skip("serena MCP", "needs `uv` (https://astral.sh/uv) — install failed, skipped");
        deregister_omp_mcp(&env.home, "serena")
    }
}

/// Resolve the Z.AI API key for the vision MCP. Prefer an explicit env var
/// (`Z_AI_API_KEY` / `ZAI_API_KEY` / `ZHIPUAI_API_KEY`); otherwise pull it from
/// omp's auth-broker via `omp token zai` — the SAME key that auths `zai/glm-5.2`.
/// Returns None only when neither source yields a plausible key; the caller then
/// still registers the server (tools discovered) but without auth.
fn resolve_zai_api_key() -> Option<String> {
    for var in ["Z_AI_API_KEY", "ZAI_API_KEY", "ZHIPUAI_API_KEY"] {
        if let Ok(v) = std::env::var(var) {
            if v.len() >= 12 {
                return Some(v);
            }
        }
    }
    // omp auth-broker holds the provider key (provider id `zai`, matching the
    // `zai/glm-5.2` model role). `omp token zai` prints just the key on stdout.
    if let Ok(out) = Command::new("omp").args(["token", "zai"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if s.len() >= 12 && !s.contains(' ') && !s.contains('\n') {
                return Some(s);
            }
        }
    }
    None
}

/// Ensure the **Z.AI vision MCP** (`@z_ai/mcp-server`) is installed + registered.
/// GLM-5.2 is text-only; this MCP exposes GLM-5V-Turbo as model-callable tools
/// (`ui_to_artifact`, `extract_text_from_screenshot`, `diagnose_error_screenshot`,
/// `understand_technical_diagram`, `analyze_data_visualization`, `ui_diff_check`,
/// `analyze_image`, `analyze_video`) authed by the SAME Z.AI key. Closing the loop:
/// `8sync shot <url>` (browser capture) → zai-vision tool → text → GLM-5.2 acts.
/// Defaults `Z_AI_VISION_MODEL` to `glm-4.6v-flash` — the ONLY vision model this
/// setup verified working end-to-end through the real MCP tool call on a stock
/// Z.AI account with no vision resource package (it's the free-tier vision model
/// per Z.AI's pricing page; paid ones like glm-4.6v/glm-5v-turbo 400 with
/// "1113 insufficient balance" until a vision package is purchased). Installs via
/// `bun add -g` (fast stdio binary on PATH); falls back to `bunx`. Never bails.
pub(crate) fn ensure_zai_vision_mcp(env: &env_detect::Env) -> Result<()> {
    // 1. Install the package so `zai-mcp-server` is on PATH (preferred over a
    //    per-connect `bunx` cold-start). bun is omnipresent in the omp stack.
    if which::which("zai-mcp-server").is_err() && which::which("bun").is_ok() {
        ui::step("z.ai vision MCP (missing — installing @z_ai/mcp-server via bun)");
        let _ = Command::new("bun").args(["add", "-g", "@z_ai/mcp-server"]).status();
    }
    let (command, args): (String, Vec<String>) = if which::which("zai-mcp-server").is_ok() {
        ("zai-mcp-server".to_string(), Vec::new())
    } else if which::which("bunx").is_ok() {
        ("bunx".to_string(), vec!["@z_ai/mcp-server".to_string()])
    } else {
        ui::warn("z.ai vision MCP: needs `bun` (https://bun.sh) — skipped; GLM-5.2 stays text-only");
        return deregister_omp_mcp(&env.home, "zai-vision");
    };
    // 2. Auth: same Z.AI key that auths `zai/glm-5.2`. Declared at fn scope so the
    //    borrow in env_vars outlives the register_omp_mcp call.
    let key = resolve_zai_api_key();
    let key_str = key.clone().unwrap_or_default();
    let mut env_vars: Vec<(&str, &str)> = vec![("Z_AI_MODE", "ZAI"), ("Z_AI_VISION_MODEL", "glm-4.6v-flash")];
    if key.is_some() {
        env_vars.push(("Z_AI_API_KEY", key_str.as_str()));
    } else {
        ui::warn("z.ai vision MCP: no Z_AI_API_KEY (env nor `omp token zai`) — registered WITHOUT auth; set it in ~/.omp/agent/mcp.json");
    }
    // 3. Register. omp's stringMap takes no ${VAR} expansion, so the key is
    //    inlined into mcp.json (user-private, never committed; gitignored).
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    register_omp_mcp(&env.home, "zai-vision", &command, &args_ref, &env_vars)?;
    ui::ok("z.ai vision MCP (GLM-5V) bridges GLM-5.2's text-only gap — ui_to_artifact · extract_text_from_screenshot · diagnose_error_screenshot · ui_diff_check · analyze_image");
    Ok(())
}

/// Exact tool catalogs for the MCP servers `8sync harness` auto-registers.
/// Static (spawning each server just to list tools would slow every `harness`
/// run) but kept in sync with the pinned tool sets this harness installs —
/// this is what `ensure_omp_capabilities_snapshot` embeds verbatim so the
/// agent gets EXACT tool names instead of guessing/hallucinating them (the
/// codegraph-verb hallucination bug in KNOWLEDGE.md is exactly what this
/// prevents). Unknown/user-added servers get no catalog — the snapshot says so.
fn known_mcp_tool_catalog(server: &str) -> &'static [(&'static str, &'static str)] {
    match server {
        "codebase-memory-mcp" => &[
            ("search_graph", "BM25 / name-pattern / semantic search over functions, classes, routes"),
            ("query_graph", "raw Cypher against the knowledge graph (complex joins, aggregations)"),
            ("trace_path", "callers/callees, data-flow with args, or cross-service (HTTP/async) trace"),
            ("get_architecture", "packages/services/deps + Leiden community clusters overview"),
            ("get_code_snippet", "read a symbol's source by qualified_name (from search_graph first)"),
            ("get_graph_schema", "node labels + edge types available to query"),
            ("search_code", "grep enriched with graph context, deduped into containing functions"),
            ("detect_changes", "diff-based impact analysis vs a base ref/branch"),
            ("index_repository", "(re)index a repo; `cross-repo-intelligence` mode links routes across repos"),
            ("index_status", "indexing progress/state for a project"),
            ("list_projects", "every project currently indexed"),
            ("delete_project", "drop a project's index"),
            ("manage_adr", "get/update/list-sections of the Architecture Decision Record"),
            ("ingest_traces", "feed runtime traces into the graph to enrich edges"),
        ],
        "headroom" => &[
            ("headroom_compress", "compress >~50-line output BEFORE it enters context (60-95% fewer tokens)"),
            ("headroom_retrieve", "fetch the original uncompressed content back by its hash"),
            ("headroom_stats", "this session's compression stats (tokens/cost saved)"),
        ],
        "serena" => &[
            ("find_symbol", "locate classes/functions/methods by name path (supports include_body)"),
            ("find_referencing_symbols", "who calls/uses a symbol — run before editing an exported symbol"),
            ("find_declaration", "declaration of a symbol via a regex-captured call-site context"),
            ("find_implementations", "implementations of an interface/abstract symbol"),
            ("get_symbols_overview", "structural summary of a file (first call when opening it)"),
            ("replace_symbol_body", "precise symbol-level rewrite (MUST have read include_body=True first)"),
            ("insert_after_symbol", "insert code right after a def/class/method"),
            ("insert_before_symbol", "insert code right before a def/class (e.g. a new import)"),
            ("rename_symbol", "project-wide rename via LSP — use instead of text search/replace"),
            ("rename_file", "move/rename a file AND rewrite every import/reference"),
            ("safe_delete_symbol", "delete only if no references remain, else lists them"),
            ("replace_content", "regex/literal replace within one file (large wildcard ranges OK)"),
            ("replace_in_files", "bulk regex/literal replace across many files (dry_run previews first)"),
            ("get_diagnostics_for_file", "LSP errors/warnings grouped by symbol"),
            ("get_current_config", "active project/tools/contexts/modes"),
            ("activate_project", "switch the active project by name or path"),
            ("list_memories", "serena's own project memory notes (topic-filterable)"),
            ("read_memory", "read one serena memory by name"),
            ("write_memory", "write/update a serena memory"),
            ("edit_memory", "regex-edit a serena memory"),
            ("rename_memory", "rename/move a serena memory"),
            ("delete_memory", "delete a serena memory (only when explicitly asked)"),
            ("onboarding", "first-run project onboarding instructions"),
        ],
        "zai-vision" => &[
            ("ui_to_artifact", "UI screenshot -> frontend code / AI prompt / design spec / description"),
            ("extract_text_from_screenshot", "OCR: code, terminal output, logs, docs (language hint optional)"),
            ("diagnose_error_screenshot", "root-cause an error/stack-trace screenshot -> fix"),
            ("understand_technical_diagram", "architecture/flowchart/UML/ER/sequence diagrams -> text"),
            ("analyze_data_visualization", "charts/graphs -> trends, anomalies, business read"),
            ("ui_diff_check", "visual regression: compare expected vs actual UI screenshots"),
            ("analyze_image", "general-purpose FALLBACK when no specialized tool above fits"),
            ("analyze_video", "video content understanding (uses `video_source` not `image_source`)"),
        ],
        _ => &[],
    }
}

/// Capture a manifest of omp's LIVE capability surface (version + key flags +
/// registered MCP servers + installed skills) to `~/.omp/capabilities.md` so the
/// agent (and `doctor`) know what omp actually offers this session — refreshed
/// every `8sync harness` run. This is the "read omp's README on every update"
/// step: omp is a binary, so we discover its surface from `omp --help` + the
/// config dirs. Surfaces the high-value flags the harness wants maximised:
/// `--advisor`, `--thinking`, `inspect_image`, the `--smol`/`--slow`/`--plan`
/// model roles, and retain/recall (Mnemopi).
pub(crate) fn ensure_omp_capabilities_snapshot(home: &Path) -> Result<()> {
    let omp_ver = env_detect::cmd_version("omp", &["--version"]).unwrap_or_default();
    let help = Command::new("omp")
        .arg("--help")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let has = |flag: &str| help.contains(flag);
    let flags: [(&str, bool); 5] = [
        ("--advisor (passive turn reviewer)", has("--advisor")),
        ("--thinking (reasoning effort)", has("--thinking")),
        ("inspect_image (built-in vision tool)", help.contains("inspect_image")),
        ("--smol / --slow / --plan (adaptive models)", has("--smol")),
        ("--skills (force-load discovery)", has("--skills")),
    ];
    // Parse the "Available Tools" block straight out of `omp --help` — this is
    // omp's OWN base tool set (read/bash/edit/write/grep/glob/lsp/browser/…),
    // distinct from the MCP servers below. Parsed (not hardcoded) so it tracks
    // whatever this installed omp version actually ships.
    let builtin_tools: Vec<(String, String)> = {
        let mut out = Vec::new();
        let mut in_section = false;
        for line in help.lines() {
            if line.trim_start().starts_with("Available Tools") {
                in_section = true;
                continue;
            }
            if !in_section {
                continue;
            }
            if line.trim().is_empty() || !line.starts_with("  ") {
                break;
            }
            if let Some((name, desc)) = line.trim().split_once('-') {
                out.push((name.trim().to_string(), desc.trim().to_string()));
            }
        }
        out
    };
    let mem_on = std::fs::read_to_string(home.join(".omp/agent/config.yml"))
        .unwrap_or_default()
        .contains("backend: mnemopi");
    // Mnemopi's memory tools are added to the agent's tool set dynamically when
    // `memory.backend: mnemopi` is configured — they don't show up in the
    // static `omp --help` (which reflects the tool-less default), so they're
    // pinned here instead, gated on `mem_on`.
    let memory_tools: &[(&str, &str)] = &[
        ("recall", "search long-term memory for specific facts/entries (ranked, raw)"),
        ("reflect", "synthesize an answer across many memories (open-ended questions)"),
        ("retain", "store durable facts (decisions, prefs, project context) for future sessions"),
        ("memory_edit", "update/forget/invalidate a specific stored memory by id (from recall)"),
    ];
    let mcp_names: Vec<String> = std::fs::read_to_string(home.join(".omp/agent/mcp.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("mcpServers")
                .and_then(|m| m.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()))
        })
        .unwrap_or_default();
    let mut mcp_names_sorted = mcp_names.clone();
    mcp_names_sorted.sort();
    let skill_count = std::fs::read_dir(home.join(".omp/skills"))
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .count()
        })
        .unwrap_or(0);
    let mut out = String::new();
    out.push_str("# omp capabilities snapshot\n\n");
    out.push_str(&format!(
        "Captured by `8sync harness`. omp version: **{}**\n\n",
        omp_ver.trim()
    ));
    out.push_str(
        "Refreshed every `8sync harness` run (omp self-updates via `omp update`). \
         This file is the GROUND TRUTH for exact tool names/params — call these, \
         never guess or invent a tool name.\n\n",
    );
    out.push_str("## Maximise these features\n\n");
    for (label, on) in flags.iter() {
        out.push_str(&format!(
            "- [{}] {} — {}\n",
            if *on { 'x' } else { ' ' },
            label,
            if *on { "available" } else { "not detected" }
        ));
    }
    out.push_str(&format!(
        "- [{}] retain/recall/reflect (Mnemopi long-term memory) — {}\n",
        if mem_on { 'x' } else { ' ' },
        if mem_on { "ON" } else { "OFF" }
    ));
    out.push_str(
        "\n## Modality routing (token discipline)\n\n\
         Read STRUCTURE as an image, PRECISE things as text. Vision models (Opus-class): \
         render a codegraph / diagram / dashboard / big PDF with `8sync shot`/`pdf-img` and \
         read the image (modality-fit — structure beats its adjacency-list text). NEVER \
         image-ify source code / exact config / line-numbered data — text is cheaper AND \
         lossless (Claude bills images per 28x28 patch, pay-per-pixel; the 10x/90% figure \
         needs a dedicated OCR encoder, not a screenshot). GLM-5.2 is text-only → images \
         via zai-vision. Full table: `~/.omp/skills/image-routing/SKILL.md`.\n",
    );
    out.push_str("\n## omp built-in tools (from `omp --help`)\n\n");
    if builtin_tools.is_empty() {
        out.push_str("_(could not parse — run `omp --help` manually)_\n");
    } else {
        for (name, desc) in &builtin_tools {
            out.push_str(&format!("- `{}` — {}\n", name, desc));
        }
    }
    if mem_on {
        out.push_str("\n## Memory tools (Mnemopi — ON)\n\n");
        out.push_str("`recall`/`reflect` BEFORE answering about past sessions/decisions/prefs; `retain` durable facts AFTER. Never re-derive what's already retained.\n\n");
        for (name, desc) in memory_tools {
            out.push_str(&format!("- `{}` — {}\n", name, desc));
        }
    }
    out.push_str("\n## Registered MCP servers — EXACT tool catalog\n\n");
    out.push_str(&format!(
        "`{}` server(s) in `~/.omp/agent/mcp.json`. Use these BEFORE raw grep/read (STEP 0). Tool names are prefixed by the client (e.g. `mcp__<server>_<tool>`); the base name below is what matters.\n\n",
        mcp_names_sorted.len()
    ));
    for name in &mcp_names_sorted {
        let tools = known_mcp_tool_catalog(name);
        out.push_str(&format!("### {}\n\n", name));
        if tools.is_empty() {
            out.push_str("_(not a pinned harness server — no static catalog; check its own docs/`--help`)_\n\n");
        } else {
            for (tool, desc) in tools {
                out.push_str(&format!("- `{}` — {}\n", tool, desc));
            }
            out.push('\n');
        }
    }
    // Local GGUF models (mistral.rs → omp providers), if any are registered.
    let reg_raw =
        std::fs::read_to_string(home.join(".config/8sync/local-models.tsv")).unwrap_or_default();
    let locals: Vec<&str> = reg_raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if !locals.is_empty() {
        out.push_str("\n## Local GGUF models (mistral.rs → omp)\n\n");
        out.push_str("On-device GGUF models served by mistral.rs (Rust, memory-safe) and registered as omp providers. Use like any model: `8sync ai --model local/<name>`. Manage: `8sync harness add-local-model list|rm`.\n\n");
        for l in &locals {
            let mut it = l.splitn(3, '\t');
            let name = it.next().unwrap_or("").trim();
            let port = it.next().unwrap_or("").trim();
            if !name.is_empty() {
                out.push_str(&format!("- `local/{}` — mistral.rs on port {}\n", name, port));
            }
        }
    }
    out.push_str(&format!(
        "## Installed skills\n\n`{}` skill dir(s) in `~/.omp/skills/`.\n",
        skill_count
    ));
    let mcp_servers = mcp_names_sorted.len();
    let target = home.join(".omp/capabilities.md");
    let changed = std::fs::read_to_string(&target).ok().as_deref() != Some(out.as_str());
    std::fs::write(&target, out)?;
    if changed {
        ui::ok(&format!(
            "omp capabilities snapshot → {} ({} · {} MCP · {} skills)",
            target.display(),
            omp_ver.trim(),
            mcp_servers,
            skill_count
        ));
    } else {
        ui::skip("omp capabilities snapshot", "unchanged");
    }
    Ok(())
}
/// Best-effort: ensure the `feynman` research CLI (companion-inc/feynman) is
/// available so the 20 feynman research skills registered in su-code/skills.toml
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
    let global = home.join(".omp/agent/extensions").join(crate::brand::ns_file("workflow.ts"));
    if let Some(p) = global.parent() {
        std::fs::create_dir_all(p)?;
    }
    let changed = std::fs::read_to_string(&global).map(|s| s != body).unwrap_or(true);
    std::fs::write(&global, &body)?;
    if changed {
        ui::ok(&format!("8sync-workflow extension → {}", global.display()));
    }
    if let Some(r) = root {
        let proj = r.join(".omp/extensions").join(crate::brand::ns_file("workflow.ts"));
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
    let body = if asset.ends_with(".md") { crate::brand::render(&body).into_owned() } else { body };
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
    let eng = crate::brand::ns_file("engine.ts");
    deploy_omp_pair(
        home,
        root,
        "extensions/8sync-engine.ts",
        &format!(".omp/agent/extensions/{eng}"),
        &format!(".omp/extensions/{eng}"),
        "8sync-engine extension",
    )?;
    deploy_omp_pair(
        home,
        root,
        "commands/auto.md",
        ".omp/agent/commands/auto.md",
        ".omp/commands/auto.md",
        "/auto command",
    )?;
    deploy_omp_pair(
        home,
        root,
        "commands/feature.md",
        ".omp/agent/commands/feature.md",
        ".omp/commands/feature.md",
        "/feature command",
    )?;
    deploy_omp_pair(
        home,
        root,
        "commands/push-now.md",
        ".omp/agent/commands/push-now.md",
        ".omp/commands/push-now.md",
        "/push-now command",
    )?;
    deploy_omp_pair(
        home,
        root,
        "commands/pull-now.md",
        ".omp/agent/commands/pull-now.md",
        ".omp/commands/pull-now.md",
        "/pull-now command",
    )
}

/// One-time rebrand migration: when the binary is rebranded (`brand::NS` differs
/// from the historical `8sync`), move the `8sync`-namespaced persistent config to
/// the new namespace and remove stale deployed artifacts left under the old
/// `8sync-` filenames (the new ones deploy under `<NS>-`, so a leftover
/// `8sync-engine.ts` would make omp load the engine tools twice). AGENTS.md
/// sentinels self-heal via `skill::inject`'s legacy-aware block finder, and the
/// `.cache/` namespace is intentionally left literal (see `brand.rs`). No-op on
/// the default build and idempotent once migrated. Best-effort: never bails.
pub(crate) fn migrate_namespace(home: &Path) {
    if crate::brand::NS == "8sync" {
        return;
    }
    // 1. Config namespace: ~/.config/8sync → ~/.config/<NS>, kitty conf filename.
    if let Some(cfg) = dirs::config_dir() {
        rename_if_new_absent(&cfg.join("8sync"), &cfg.join(crate::brand::NS));
        rename_if_new_absent(
            &cfg.join("kitty").join("8sync.conf"),
            &cfg.join("kitty").join(format!("{}.conf", crate::brand::NS)),
        );
        // 3. Old systemd user timer (the NS-named unit installs on next `up --timer`).
        let unit_dir = cfg.join("systemd/user");
        if unit_dir.join("8sync-harness-up.timer").exists() {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "disable", "--now", "8sync-harness-up.timer"])
                .status();
            let _ = std::fs::remove_file(unit_dir.join("8sync-harness-up.service"));
            let _ = std::fs::remove_file(unit_dir.join("8sync-harness-up.timer"));
        }
    }
    // 2. Stale global deployed artifacts under the old `8sync-` names.
    for stale in [
        home.join(".omp/hooks/pre/8sync-recall.ts"),
        home.join(".omp/agent/extensions/8sync-engine.ts"),
        home.join(".omp/agent/extensions/8sync-workflow.ts"),
    ] {
        let _ = std::fs::remove_file(&stale);
    }
}

/// `rename(old → new)` only when the old path exists and the new one does not —
/// so a rebrand migrates once and never clobbers freshly-written state.
fn rename_if_new_absent(old: &Path, new: &Path) {
    if old.exists() && !new.exists() {
        let _ = std::fs::rename(old, new);
    }
}
