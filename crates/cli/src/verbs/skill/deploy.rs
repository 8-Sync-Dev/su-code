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

/// Sentinel markers that tag a deployed file as 8sync-managed. A file carrying
/// one is provably ours and safe to retire regardless of exact content. The
/// released `/auto` + `8sync-engine.ts` predate these markers, so they are
/// provenance-matched by content instead (see [`RetiredAsset`]).
const MANAGED_SENTINELS: &[&str] = &["<!-- 8sync-managed -->", "// 8sync-managed"];

/// A retired managed artifact retained read-only under `assets/retired/` so a
/// managed-provenance deletion can PROVE an on-disk file is ours before removing
/// it. `rendered` flags prose assets that were brand-rendered on deploy (`.md`)
/// so the match also accepts the rendered form; `.ts`/binary assets deploy raw.
struct RetiredAsset {
    asset: &'static str,
    rendered: bool,
}

/// Retired `/auto` command (released v0.28–0.45 managed surface this cutover
/// replaces) — prose, brand-rendered on deploy.
const RETIRED_AUTO_MD: RetiredAsset = RetiredAsset { asset: "retired/auto.md", rendered: true };
/// Retired `8sync-engine.ts` extension — `.ts`, deployed raw (never rendered).
const RETIRED_ENGINE_TS: RetiredAsset =
    RetiredAsset { asset: "retired/8sync-engine.ts", rendered: false };
/// Rebrand-only: stale old-`8sync-`-named copies of assets this binary still
/// ships (now deployed under `<NS>-`). Matched against current asset content so a
/// user file by the same name is never destroyed on a rebrand.
const STALE_RECALL_TS: RetiredAsset = RetiredAsset { asset: "hooks/8sync-recall.ts", rendered: false };
const STALE_WORKFLOW_TS: RetiredAsset =
    RetiredAsset { asset: "extensions/8sync-workflow.ts", rendered: false };

/// True iff `path` is provably an 8sync-managed artifact safe to delete: it
/// carries a managed sentinel, OR byte-matches a known retired managed asset
/// (raw, and for prose also the brand-rendered form). An unknown / user-edited
/// file never matches — it survives with a diagnostic instead.
fn is_managed_artifact(path: &Path, retired: &RetiredAsset) -> bool {
    let Ok(bytes) = std::fs::read(path) else { return false };
    if has_managed_sentinel(&bytes) {
        return true;
    }
    let Some(managed) = assets::read_bytes(retired.asset) else { return false };
    if bytes == managed {
        return true;
    }
    if retired.rendered {
        if let (Ok(disk), Ok(raw)) = (std::str::from_utf8(&bytes), std::str::from_utf8(&managed)) {
            let rendered = crate::brand::render(raw);
            return &*rendered == disk;
        }
    }
    false
}

/// True iff `bytes` decode as UTF-8 and contain a known 8sync-managed sentinel.
fn has_managed_sentinel(bytes: &[u8]) -> bool {
    let Ok(s) = std::str::from_utf8(bytes) else { return false };
    MANAGED_SENTINELS.iter().any(|m| s.contains(m))
}

/// Remove `path` ONLY if it is provably 8sync-managed (sentinel or retired
/// byte-match). A user-owned / unknown file by the same name SURVIVES and emits a
/// diagnostic so the user can act. Best-effort: never bails.
fn remove_managed_file(path: &Path, retired: &RetiredAsset, label: &str) {
    if !path.exists() {
        return;
    }
    if is_managed_artifact(path, retired) {
        if std::fs::remove_file(path).is_ok() {
            ui::ok(&format!("removed retired managed {} → {}", label, path.display()));
        }
    } else {
        ui::warn(&format!(
            "kept user-owned `{}` — not a known 8sync-managed asset: {} (remove manually if intended)",
            label,
            path.display(),
        ));
    }
}

/// Remove a legacy single file for which we hold NO retired fingerprint (e.g. the
/// pre-engine `/gs` command) — deleted ONLY if it carries a managed sentinel;
/// otherwise preserved with a diagnostic. Never destroys an unknown file.
fn remove_legacy_file_if_managed(path: &Path, label: &str) {
    if !path.exists() {
        return;
    }
    let Ok(bytes) = std::fs::read(path) else { return };
    if has_managed_sentinel(&bytes) {
        if std::fs::remove_file(path).is_ok() {
            ui::ok(&format!("removed managed {} → {}", label, path.display()));
        }
    } else {
        ui::warn(&format!(
            "kept user-owned `{}` — not provenance-matched: {} (remove manually if intended)",
            label,
            path.display(),
        ));
    }
}

/// Remove a legacy directory for which we hold NO fingerprint (e.g. the
/// pre-engine `skills/gs` dir) — deleted ONLY when empty or every member carries
/// a managed sentinel; a directory holding unknown content SURVIVES with a
/// diagnostic. Never recursively destroys unknown user files.
fn remove_legacy_dir_if_managed(path: &Path, label: &str) {
    if !path.exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else { return };
    let members: Vec<_> = entries.flatten().collect();
    if members.is_empty() {
        let _ = std::fs::remove_dir(path);
        return;
    }
    let all_managed = members.iter().all(|e| {
        if e.path().is_dir() {
            return false;
        }
        std::fs::read(e.path())
            .map(|b| has_managed_sentinel(&b))
            .unwrap_or(false)
    });
    if all_managed {
        if std::fs::remove_dir_all(path).is_ok() {
            ui::ok(&format!("removed managed {} → {}", label, path.display()));
        }
    } else {
        ui::warn(&format!(
            "kept user-owned `{}` — contains unknown files: {} (remove manually if intended)",
            label,
            path.display(),
        ));
    }
}

/// Managed-provenance clean cutover: retire the `/auto` command +
/// `8sync-engine.ts` extension (global + project) and any even-older `/gs`
/// command/skill remnants. Called ONLY after [`ensure_gs`] succeeds, so a failed
/// GS install never strands the user with no automation surface.
///
/// A file/dir is removed ONLY if we can PROVE 8sync authored it — it byte-matches
/// a known retired managed asset (see `assets/retired/`) or carries an explicit
/// 8sync-managed sentinel. An unknown or user-owned file by the same name
/// SURVIVES with a diagnostic; nothing is destroyed without managed provenance.
/// For the pre-engine `/gs` command + `skills/gs` dir we hold no retired
/// fingerprint, so those are removed only when sentinel-tagged or empty.
pub(crate) fn cleanup_legacy_auto(home: &Path, root: Option<&Path>) {
    // Retired /auto engine (this cutover) — provenance-matched, never blind.
    remove_managed_file(
        &home.join(".omp/agent/commands/auto.md"),
        &RETIRED_AUTO_MD,
        "/auto command",
    );
    remove_managed_file(
        &home.join(".omp/agent/extensions/8sync-engine.ts"),
        &RETIRED_ENGINE_TS,
        "8sync-engine extension",
    );
    // Even older /gs command + skill remnants (pre-engine era): we hold no
    // retired fingerprint, so these are NEVER blindly deleted.
    remove_legacy_file_if_managed(&home.join(".omp/agent/commands/gs.md"), "/gs command (legacy)");
    remove_legacy_dir_if_managed(&home.join(".omp/skills/gs"), "/gs skill (legacy)");
    if let Some(r) = root {
        remove_managed_file(
            &r.join(".omp/commands/auto.md"),
            &RETIRED_AUTO_MD,
            "/auto command (project)",
        );
        remove_managed_file(
            &r.join(".omp/extensions/8sync-engine.ts"),
            &RETIRED_ENGINE_TS,
            "8sync-engine extension (project)",
        );
        remove_legacy_file_if_managed(&r.join(".omp/commands/gs.md"), "/gs command (project, legacy)");
        remove_legacy_dir_if_managed(&r.join("su-code/skills/gs"), "/gs skill (project, legacy)");
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
/// tools (search_graph/trace_path/get_architecture/…) with zero manual config.
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

/// Keep the STEP-0 MCP servers' tools ALWAYS VISIBLE via `mcp.discoveryDefaultServers`
/// in `~/.omp/agent/config.yml`. omp's default `tools.discoveryMode: auto` hides ALL
/// MCP tools behind a `search_tool_bm25` discovery hop once the registry exceeds 40
/// tools — measured effect: serena/headroom 0 calls across 29 sessions. Listing the
/// four harness servers keeps their full catalogs in the active tool set (verified in
/// omp 16.4.8: the setting filters discoverable MCP tools by `serverName` and merges
/// them into the session baseline). `tools.essentialOverride` does NOT work for this —
/// omp filters its entries to BUILT-IN tool names only. Key-presence idempotent:
/// never overrides a user-set `discoveryDefaultServers`; migrates away the inert
/// essentialOverride block earlier 8sync builds wrote (exact-match removal only).
pub(crate) fn ensure_mcp_tools_visible(home: &Path) -> Result<()> {
    // omp ≥17 replaced the pre-17 bm25 discovery hop (+ `mcp.discoveryDefaultServers`)
    // with `tools.xdev` (default on): MCP tools mount as `xd://` device URLs, callable
    // via read/write without shipping schemas every request. The old key is obsolete
    // (absent from omp's schema) — writing it is dead weight omp strips on rewrite,
    // which is exactly the churn that made STEP-0 look like it kept "regressing".
    if env_detect::omp_major().is_some_and(|m| m >= 17) {
        ui::ok("STEP-0 MCP tools mounted as xd:// devices (omp ≥17 tools.xdev) — serena/cbm/headroom/zai callable, no config key needed");
        return Ok(());
    }
    const SERVERS: &[&str] = &["codebase-memory-mcp", "headroom", "serena", "zai-vision"];
    // The exact block written by the earlier essentialOverride approach. MCP names
    // in essentialOverride are filtered out by omp (builtins only) AND clobber the
    // builtin essential defaults — remove it, but ONLY this byte-exact 8sync block.
    const LEGACY_PIN: &str = "tools:\n  essentialOverride:\n    - mcp__codebase_memory_mcp_search_graph\n    - mcp__codebase_memory_mcp_trace_path\n    - mcp__codebase_memory_mcp_get_architecture\n    - mcp__codebase_memory_mcp_get_code_snippet\n    - mcp__serena_find_symbol\n    - mcp__serena_find_referencing_symbols\n    - mcp__serena_get_symbols_overview\n    - mcp__headroom_compress\n    - mcp__zai_vision_extract_text_from_screenshot\n    - mcp__zai_vision_analyze_image\n";
    let cfg = home.join(".omp/agent/config.yml");
    if let Some(p) = cfg.parent() { std::fs::create_dir_all(p)?; }
    let mut s = std::fs::read_to_string(&cfg).unwrap_or_default();
    let mut changed = false;
    if s.contains(LEGACY_PIN) {
        s = s.replace(LEGACY_PIN, "");
        changed = true;
        ui::info("migrated: dropped inert tools.essentialOverride MCP pin (builtins-only setting)");
    }
    if s.contains("discoveryDefaultServers") {
        ui::skip("STEP-0 MCP visibility", "mcp.discoveryDefaultServers already set (user-configured)");
        if changed { std::fs::write(&cfg, s)?; }
        return Ok(());
    }
    let list: String = SERVERS.iter().map(|t| format!("    - {t}\n")).collect();
    if s.lines().any(|l| l.starts_with("mcp:")) {
        // Insert under the existing top-level `mcp:` block (same approach as
        // compaction::set_threshold).
        s = s
            .lines()
            .map(|l| {
                if l.starts_with("mcp:") {
                    format!("{l}\n  discoveryDefaultServers:\n{}", list.trim_end())
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        if !s.ends_with('\n') {
            s.push('\n');
        }
    } else {
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&format!("\nmcp:\n  discoveryDefaultServers:\n{list}"));
    }
    std::fs::write(&cfg, s)?;
    ui::ok("STEP-0 MCP servers always visible (mcp.discoveryDefaultServers) — serena/cbm/headroom/zai callable, no search_tool_bm25 hop");
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
        "`{}` server(s) in `~/.omp/agent/mcp.json`. Use these BEFORE raw grep/read (STEP 0). Callable names are the REGISTERED forms: `mcp__<server-with-underscores>_<tool>` (e.g. `mcp__codebase_memory_mcp_search_graph`, `mcp__serena_find_symbol`; exception: `mcp__headroom_compress` — omp collapses a duplicated server prefix). The four harness servers are kept ALWAYS VISIBLE by `8sync harness` (`mcp.discoveryDefaultServers`) — call their tools directly; only other/newly-added servers' tools need one `search_tool_bm25` call first.\n\n",
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

/// Deploy the native `/gs` agent-team engine — the `8sync-gs` omp extension
/// (directory: durable stage machine + gates + model routing + eight gs_* tools),
/// the seven `gs-*` task agents, and the seed `gs.json` config. 100% on omp core
/// (config dirs only, never patches omp). Also deploys the `/feature`, `/push-now`,
/// and `/pull-now` commands. NOT `/auto` — that surface is retired (see
/// `cleanup_legacy_auto`). Install-new-BEFORE-clean-old: a failed install here
/// leaves the old surface intact because cleanup runs only after this returns Ok.
pub(crate) fn ensure_gs(home: &Path, root: Option<&Path>) -> Result<()> {
    // 1. GS extension tree — global (auto-loaded) + project mirror.
    let global_ext = home.join(".omp/agent/extensions/8sync-gs");
    std::fs::create_dir_all(&global_ext)?;
    let (w, _) = assets::install_tree("extensions/8sync-gs", &global_ext)?;
    if w > 0 {
        ui::ok(&format!("8sync-gs extension ({} file(s)) → {}", w, global_ext.display()));
    }
    // 2. The seven gs-* task agents — user root `~/.omp/agent/agents/`.
    let global_agents = home.join(".omp/agent/agents");
    std::fs::create_dir_all(&global_agents)?;
    let (wa, _) = assets::install_tree("agents", &global_agents)?;
    if wa > 0 {
        ui::ok(&format!("gs agents ({} file(s)) → {}", wa, global_agents.display()));
    }
    // 3. Seed ~/.config/8sync/gs.json (never clobber a user-owned config).
    seed_gs_config()?;
    // 4. Project mirror: extension + agents.
    if let Some(r) = root {
        let proj_ext = r.join(".omp/extensions/8sync-gs");
        std::fs::create_dir_all(&proj_ext)?;
        let _ = assets::install_tree("extensions/8sync-gs", &proj_ext)?;
        let proj_agents = r.join(".omp/agents");
        std::fs::create_dir_all(&proj_agents)?;
        let _ = assets::install_tree("agents", &proj_agents)?;
    }
    // 5. Commands that ride alongside GS (NOT /auto).
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

/// Seed `~/.config/8sync/gs.json` from the bundled default only when absent.
fn seed_gs_config() -> Result<()> {
    let Some(body) = assets::read("configs/8sync/gs.json") else {
        return Ok(());
    };
    let Some(cfg_dir) = dirs::config_dir() else {
        return Ok(());
    };
    let target = cfg_dir.join("8sync").join("gs.json");
    if target.exists() {
        return Ok(());
    }
    if let Some(p) = target.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(&target, body)?;
    ui::ok(&format!("seeded gs config → {}", target.display()));
    Ok(())
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
    // 2. Stale global deployed artifacts under the old `8sync-` names (the new
    //    <NS>-named copies deploy above). Provenance-matched against the asset
    //    content this binary ships (or the retired engine) so a user file by the
    //    same name is never destroyed on a rebrand — only provably-managed copies.
    remove_managed_file(
        &home.join(".omp/hooks/pre/8sync-recall.ts"),
        &STALE_RECALL_TS,
        "old-namespace recall hook",
    );
    remove_managed_file(
        &home.join(".omp/agent/extensions/8sync-engine.ts"),
        &RETIRED_ENGINE_TS,
        "old-namespace engine",
    );
    remove_managed_file(
        &home.join(".omp/agent/extensions/8sync-workflow.ts"),
        &STALE_WORKFLOW_TS,
        "old-namespace workflow",
    );
}

/// `rename(old → new)` only when the old path exists and the new one does not —
/// so a rebrand migrates once and never clobbers freshly-written state.
fn rename_if_new_absent(old: &Path, new: &Path) {
    if old.exists() && !new.exists() {
        let _ = std::fs::rename(old, new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Unique scratch dir under the system temp dir; removed when dropped so tests
    /// never touch the real `~/.omp` or `~/.config`.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(label: &str) -> Self {
            static SEQ: AtomicU64 = AtomicU64::new(0);
            let n = SEQ.fetch_add(1, Ordering::SeqCst);
            let p = std::env::temp_dir().join(format!(
                "8sync-deploy-test-{}-{}-{}",
                std::process::id(),
                n,
                label
            ));
            std::fs::create_dir_all(&p).unwrap();
            Scratch(p)
        }
        fn path(&self) -> &Path { &self.0 }
        fn join(&self, rel: &str) -> PathBuf { self.0.join(rel) }
    }
    impl Drop for Scratch {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    fn write(path: &Path, bytes: &[u8]) {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p).unwrap(); }
        std::fs::write(path, bytes).unwrap();
    }

    /// Unknown / user-owned files at the retired paths must survive cleanup — no
    /// blind deletion, no data loss.
    #[test]
    fn user_owned_legacy_files_survive() {
        let home = Scratch::new("user-survive");
        write(&home.join(".omp/agent/commands/auto.md"), b"# my own auto command\n");
        write(&home.join(".omp/agent/extensions/8sync-engine.ts"), b"// my custom engine\n");
        write(&home.join(".omp/agent/commands/gs.md"), b"# my own gs command\n");
        write(&home.join(".omp/skills/gs/SKILL.md"), b"# my own gs skill\n");

        cleanup_legacy_auto(home.path(), None);

        assert!(home.join(".omp/agent/commands/auto.md").exists(), "user auto.md must survive");
        assert!(home.join(".omp/agent/extensions/8sync-engine.ts").exists(), "user engine.ts must survive");
        assert!(home.join(".omp/agent/commands/gs.md").exists(), "user gs.md must survive");
        assert!(home.join(".omp/skills/gs/SKILL.md").exists(), "user skills/gs must survive");
    }

    /// Managed retired copies (byte-identical to the released managed asset) ARE
    /// removed — managed obsolete copies go, provenance-matched.
    #[test]
    fn managed_retired_copies_are_removed() {
        let home = Scratch::new("managed-remove");
        let auto = assets::read_bytes("retired/auto.md").expect("retired/auto.md embedded");
        let engine = assets::read_bytes("retired/8sync-engine.ts").expect("retired engine embedded");
        write(&home.join(".omp/agent/commands/auto.md"), &auto);
        write(&home.join(".omp/agent/extensions/8sync-engine.ts"), &engine);

        cleanup_legacy_auto(home.path(), None);

        assert!(!home.join(".omp/agent/commands/auto.md").exists(), "managed auto.md must be removed");
        assert!(!home.join(".omp/agent/extensions/8sync-engine.ts").exists(), "managed engine.ts must be removed");
    }

    /// Pre-engine `/gs` remnants (no fingerprint held): sentinel-tagged removed,
    /// empty dir removed, unknown content preserved.
    #[test]
    fn legacy_gs_remnants_handled_safely() {
        // sentinel-tagged gs.md → removed
        let a = Scratch::new("gs-sentinel");
        write(&a.join(".omp/agent/commands/gs.md"), b"<!-- 8sync-managed -->\nmanaged gs\n");
        cleanup_legacy_auto(a.path(), None);
        assert!(!a.join(".omp/agent/commands/gs.md").exists(), "sentinel gs.md must be removed");

        // empty skills/gs dir → removed
        let b = Scratch::new("gs-empty");
        std::fs::create_dir_all(b.join(".omp/skills/gs")).unwrap();
        cleanup_legacy_auto(b.path(), None);
        assert!(!b.join(".omp/skills/gs").exists(), "empty skills/gs must be removed");

        // skills/gs holding only a sentinel-tagged file → removed
        let c = Scratch::new("gs-sentinel-dir");
        write(&c.join(".omp/skills/gs/SKILL.md"), b"<!-- 8sync-managed -->\nx\n");
        cleanup_legacy_auto(c.path(), None);
        assert!(!c.join(".omp/skills/gs").exists(), "sentinel-only skills/gs must be removed");

        // skills/gs holding an unknown file → preserved
        let d = Scratch::new("gs-unknown-dir");
        write(&d.join(".omp/skills/gs/SKILL.md"), b"# user skill\n");
        cleanup_legacy_auto(d.path(), None);
        assert!(d.join(".omp/skills/gs/SKILL.md").exists(), "unknown skills/gs content must survive");
    }

    /// The provenance predicate is exact: managed bytes match, a one-byte edit
    /// does not, a sentinel matches, and a missing path is a safe non-match.
    #[test]
    fn provenance_predicate_exactness() {
        let home = Scratch::new("predicate");
        let managed = assets::read_bytes("retired/8sync-engine.ts").unwrap();

        let exact = home.join("engine.ts");
        write(&exact, &managed);
        assert!(is_managed_artifact(&exact, &RETIRED_ENGINE_TS), "exact managed bytes match");

        let mut edited = managed.clone();
        edited[0] ^= 0xFF; // flip one byte → no longer the managed asset
        let edit = home.join("engine-edited.ts");
        write(&edit, &edited);
        assert!(!is_managed_artifact(&edit, &RETIRED_ENGINE_TS), "edited bytes must not match");

        let sent = home.join("engine-sent.ts");
        write(&sent, b"totally custom body\n// 8sync-managed\nmore\n");
        assert!(is_managed_artifact(&sent, &RETIRED_ENGINE_TS), "sentinel tags managed");

        assert!(!is_managed_artifact(&home.join("missing.ts"), &RETIRED_ENGINE_TS), "missing → not managed");
    }

    /// First GS deployment on a clean home installs the full surface even though
    /// legacy cleanup finds nothing, and cleanup never self-clobbers the new tree.
    /// `XDG_CONFIG_HOME` is redirected so the real user config is untouched.
    #[test]
    fn clean_home_install_is_complete() {
        let home = Scratch::new("clean-install");
        let xdg = home.join(".config");
        std::fs::create_dir_all(&xdg).unwrap();
        let prev_xdg = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &xdg);

        let res = ensure_gs(home.path(), None);
        // cleanup runs after a successful install and finds nothing on a clean home.
        cleanup_legacy_auto(home.path(), None);

        std::env::remove_var("XDG_CONFIG_HOME");
        if let Some(v) = prev_xdg { std::env::set_var("XDG_CONFIG_HOME", v); }

        assert!(res.is_ok(), "ensure_gs must succeed on a clean home: {:?}", res.err());

        // 1. GS extension tree deployed and non-empty.
        let ext = home.join(".omp/agent/extensions/8sync-gs");
        assert!(ext.is_dir(), "8sync-gs extension dir must exist");
        assert!(ext.join("index.ts").exists(), "extension entrypoint must deploy");

        // 2. gs-* agents deployed; gs-worker carries NO shell/bash (security fix).
        let worker = home.join(".omp/agent/agents/gs-worker.md");
        assert!(worker.exists(), "gs-worker.md must deploy");
        let body = std::fs::read_to_string(&worker).unwrap();
        let tools_line = body.lines().find(|l| l.starts_with("tools:")).expect("tools: frontmatter");
        assert!(!tools_line.contains("bash"), "gs-worker must NOT have bash: {tools_line}");
        assert!(!tools_line.contains("glob"), "gs-worker must NOT have glob: {tools_line}");
        assert!(home.join(".omp/agent/agents/gs-planner.md").exists(), "gs-planner deploys");

        // 3. Commands ride alongside GS.
        assert!(home.join(".omp/agent/commands/feature.md").exists(), "/feature must deploy");

        // 4. Seed config lands in the redirected XDG dir, never the real one.
        assert!(xdg.join("8sync/gs.json").exists(), "gs.json must be seeded under XDG_CONFIG_HOME");

        // 5. Legacy cleanup left the freshly-installed surface intact.
        assert!(ext.join("index.ts").exists(), "cleanup must not remove GS artifacts");
        assert!(worker.exists(), "cleanup must not remove GS agents");
    }
}
