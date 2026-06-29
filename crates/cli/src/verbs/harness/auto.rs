//! `8sync harness` (bare, no subcommand) — the ONE command that makes a project
//! fully agent-ready and current, in a single idempotent pass:
//!   skills (deploy bundled + external) → update (pull registered from source) →
//!   mirror into the project (additive) → inject force-load → seed memory +
//!   gitleaks hook → consolidate learnings → re-index codegraph.
//! Re-run anytime; safe + cheap. `harness init` = explicit full bootstrap with
//! progress UI; `harness up` = light refresh; `harness up --timer` = background loop.
use std::process::Command;

use anyhow::Result;

use super::external::install_external_skill_packs;
use super::memory::{consolidate_learnings, seed_gitleaks_hook, seed_harness_memory};
use crate::verbs::skill::{deploy, discover, inject_agents_md, inject_subfolder_indexes, update};
use crate::{assets, env_detect, ui};

pub(crate) fn harness_auto(env: &env_detect::Env, force: bool) -> Result<()> {
    ui::header("8sync harness");

    // 1. Global skill library — idempotent. Re-deploys bundled skills new in this
    //    binary (after `8sync up`) and the master force-load file.
    let force_load = env.home.join(".omp/skills/00-force-load.md");
    if let Some(p) = force_load.parent() {
        std::fs::create_dir_all(p)?;
    }
    if let Some(c) = assets::read("skills/00-force-load.md") {
        std::fs::write(&force_load, c)?;
    }
    deploy::install_bundled_global(env)?;
    deploy::ensure_codegraph(env)?;
    deploy::ensure_codebase_memory_mcp(env)?;
    deploy::ensure_headroom_mcp(env)?;
    let _ = deploy::ensure_omp_memory_config(&env.home);
    let _ = deploy::ensure_recall_hook(&env.home);
    let _ = deploy::ensure_append_system(&env.home);
    let _ = deploy::ensure_serena_mcp(env);
    deploy::ensure_feynman_cli();
    let _ = install_external_skill_packs(env); // best-effort; skips packs already present
    let global_dir = env.home.join(".omp/skills");
    for d in discover::list_installed_skill_dirs(&global_dir).unwrap_or_default() {
        deploy::ensure_skill_layout(&d);
    }

    let Some(root) = discover::detect_current_project_root() else {
        ui::ok("global skills ready — `cd` into a project and re-run `8sync harness`");
        let _ = deploy::ensure_workflow_extension(&env.home, None);
        let _ = deploy::ensure_engine(&env.home, None);
        let _ = deploy::cleanup_legacy_gs(&env.home, None);
        return Ok(());
    };

    // 2. Update registered skills from their sources, then mirror the rest in
    //    (additive: never clobber an edited local skill — pass `--force` to refresh).
    let _ = update::update_skills(env, &env.xdg_config.join("8sync/skills.toml"), None);
    let count = deploy::mirror_global_to_local(&env.home, &root, force)?;
    if count > 0 {
        ui::ok(&format!("skills vendored → {}", root.join("agents/skills").display()));
    }
    for d in discover::list_installed_skill_dirs(&root.join("agents/skills")).unwrap_or_default() {
        deploy::ensure_skill_layout(&d);
    }

    // 3. The self-learning loop, one pass: codegraph + memory + safety + inject.
    deploy::ensure_codegraph_init(&root);
    seed_harness_memory(&root)?;
    seed_gitleaks_hook(&root);
    inject_agents_md(&env.home, &root)?;
    inject_subfolder_indexes(&root)?;
    let _ = deploy::ensure_workflow_extension(&env.home, Some(&root));
    let _ = deploy::ensure_engine(&env.home, Some(&root));
    let _ = deploy::cleanup_legacy_gs(&env.home, Some(&root));
    let _ = consolidate_learnings(&root);

    // 4. Re-index so the agent learns the current tree.
    if which::which("codegraph").is_ok() {
        ui::step("codegraph index (re-learn current state)");
        let _ = Command::new("codegraph").arg("index").arg(&root).status();
    }
    deploy::index_codebase_memory(&root);

    ui::ok(&format!("harness ready → {}", root.display()));
    ui::info("background loop: `8sync harness up --timer 30m` · full rebuild: `8sync harness init`");
    Ok(())
}
