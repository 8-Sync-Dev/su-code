//! `8sync harness init` — one command to stand up a maximal agent harness:
//! deploy every bundled skill (+ codegraph binary + external skill packs),
//! mirror them into the project, init codegraph, seed agent memory + CHANGELOG,
//! and inject force-load rules into the root AGENTS.md/CLAUDE.md plus a compact
//! index into every significant sub-folder. Progress is tracked step by step.
use std::time::Instant;

use anyhow::Result;

use super::external::install_external_skill_packs;
use super::memory::{seed_gitleaks_hook, seed_harness_memory};
use crate::verbs::skill::{deploy, discover, inject_agents_md, inject_subfolder_indexes};
use crate::{assets, env_detect, ui};

/// Lightweight stepped progress indicator (no TUI dep): `▸ [i/N] label`.
struct Progress {
    total: usize,
    cur: usize,
    start: Instant,
}

impl Progress {
    fn new(total: usize) -> Self {
        Progress { total, cur: 0, start: Instant::now() }
    }
    fn step(&mut self, label: &str) {
        self.cur += 1;
        ui::step(&format!("[{}/{}] {}", self.cur, self.total, label));
    }
    fn done(&self) {
        ui::ok(&format!(
            "harness ready in {:.1}s ({} steps)",
            self.start.elapsed().as_secs_f32(),
            self.total
        ));
    }
}

pub(crate) fn harness_init(env: &env_detect::Env, force: bool) -> Result<()> {
    ui::header("8sync harness init");
    let in_project = discover::detect_current_project_root().is_some();
    let total = if in_project { 8 } else { 4 };
    let mut p = Progress::new(total);

    // 1. Master force-load file (omp reads this first every session).
    p.step("master skill list → ~/.omp/skills/00-force-load.md");
    let target = env.home.join(".omp/skills/00-force-load.md");
    std::fs::create_dir_all(target.parent().unwrap())?;
    let content = assets::read("skills/00-force-load.md").unwrap_or_default();
    std::fs::write(&target, content)?;

    // 2. Deploy bundled skills (embedded assets → ~/.omp/skills).
    p.step("deploy bundled skills (codegraph · karpathy · ponytail · assp · impeccable · taste · …)");
    deploy::install_bundled_global(env)?;

    // 3. codegraph binary (auto curl installer if missing).
    p.step("ensure codegraph binary");
    deploy::ensure_codegraph(env)?;
    deploy::ensure_codebase_memory_mcp(env)?;

    // 4. External skill packs (ponytail full + addyosmani) — best-effort/network.
    p.step("download external skill packs (ponytail · addyosmani)");
    let _ = install_external_skill_packs(env);

    // Normalise every global skill dir to the 3-folder layout.
    let global_dir = env.home.join(".omp/skills");
    for d in discover::list_installed_skill_dirs(&global_dir).unwrap_or_default() {
        deploy::ensure_skill_layout(&d);
    }

    // 5-8. Project-scoped scaffolding.
    if let Some(root) = discover::detect_current_project_root() {
        p.step("mirror skills → agents/skills/");
        let count = deploy::mirror_global_to_local(&env.home, &root, force)?;
        if count > 0 {
            ui::ok(&format!("mirrored {} skill(s) into {}", count, root.join("agents/skills").display()));
        }
        let local_dir = root.join("agents/skills");
        for d in discover::list_installed_skill_dirs(&local_dir).unwrap_or_default() {
            deploy::ensure_skill_layout(&d);
        }

        p.step("codegraph init + seed memory/CHANGELOG");
        deploy::ensure_codegraph_init(&root);
        seed_harness_memory(&root)?;
        seed_gitleaks_hook(&root);

        p.step("inject force-load → AGENTS.md / CLAUDE.md");
        inject_agents_md(&env.home, &root)?;

        p.step("inject sub-folder skill indexes");
        let n = inject_subfolder_indexes(&root)?;
        if n > 0 {
            ui::ok(&format!("dropped skill-index AGENTS.md into {} sub-folder(s)", n));
        }
        p.done();
        ui::info("start a session: `8sync .` or `omp --continue`");
        ui::info("refresh later: `8sync harness up`  (auto: `8sync harness up --timer 30m`)");
        ui::info("opt-in skill: `8sync skill add builtin:social-growth` (social/branding/leads — không auto-bật)");
    } else {
        p.done();
        ui::warn("not inside a project (no AGENTS.md/.git/Cargo.toml/package.json/... in cwd or ancestors)");
        ui::info("  → `cd` into a project root, then re-run `8sync harness init`");
    }
    Ok(())
}

