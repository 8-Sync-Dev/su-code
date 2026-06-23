//! `8sync harness bench` — deterministic benchmark of the loop-engineering
//! context budget. Measures the token cost an agent pays EVERY session
//! (force-load prefix + CORE skill bodies) versus what Phase A defers
//! (SPECIALIST + on-demand bodies), plus a KV-cache stable-prefix gate. No model
//! calls → fully reproducible, safe to gate a phase transition on.
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::verbs::skill::discover::detect_current_project_root;
use crate::verbs::skill::inject::build_force_load;
use crate::verbs::skill::meta::meta_for_dir;
use crate::{env_detect, ui};

/// Rough token estimate (~chars/4, the standard heuristic). Deterministic; for
/// RELATIVE comparison across phases, not billing accuracy.
fn tok(chars: usize) -> usize {
    chars.div_ceil(4)
}

/// Char count of a skill's SKILL.md body — what the model reads when it loads
/// the skill (the unit progressive disclosure defers).
fn body_chars(dir: &Path) -> usize {
    let (_m, entry) = meta_for_dir(dir);
    std::fs::read_to_string(dir.join(entry))
        .map(|s| s.chars().count())
        .unwrap_or(0)
}

/// Total bytes of every file under `dir` — the full footprint that becomes
/// reachable once a skill is triggered (SKILL.md + references/ + scripts/).
fn dir_bytes(dir: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0u64;
    for e in rd.flatten() {
        match e.metadata() {
            Ok(md) if md.is_dir() => total += dir_bytes(&e.path()),
            Ok(md) => total += md.len(),
            Err(_) => {}
        }
    }
    total
}

fn sum_body(dirs: &[PathBuf]) -> usize {
    dirs.iter().map(|d| body_chars(d)).sum()
}
fn sum_dir(dirs: &[PathBuf]) -> u64 {
    dirs.iter().map(|d| dir_bytes(d)).sum()
}
fn kb(bytes: u64) -> String {
    format!("{:.0} KB", bytes as f64 / 1024.0)
}
fn names(dirs: &[PathBuf]) -> String {
    dirs.iter()
        .filter_map(|d| d.file_name().and_then(|s| s.to_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Char count of the memory-spine files the agent reads at session start
/// (part of the per-session upfront budget alongside prefix + CORE bodies).
fn spine_chars(root: &Path) -> usize {
    ["PROJECT.md", "KNOWLEDGE.md", "STATE.md", "DECISIONS.md", "PREFERENCES.md", "PLAYBOOKS.md"]
        .iter()
        .map(|f| {
            std::fs::read_to_string(root.join("agents").join(f))
                .map(|s| s.chars().count())
                .unwrap_or(0)
        })
        .sum()
}

pub(crate) fn harness_bench(env: &env_detect::Env) -> Result<()> {
    ui::header("8sync harness bench — loop-engineering context budget");
    let Some(root) = detect_current_project_root() else {
        ui::warn("not inside a project — cd into a repo root and re-run");
        return Ok(());
    };

    let st = build_force_load(&env.home, &root);

    // Force-load prefix (always injected into AGENTS.md / CLAUDE.md / …).
    let block_tok = tok(st.block.chars().count());

    // Skill-body budget (SKILL.md bodies the model actually loads).
    let core_tok = tok(sum_body(&st.core));
    let spec_tok = tok(sum_body(&st.specialist));
    let on_tok = tok(sum_body(&st.ondemand));

    // Memory spine (PROJECT/KNOWLEDGE/STATE/DECISIONS/PREFERENCES) — read at
    // session start, so it counts toward the per-session upfront budget.
    let spine_tok = tok(spine_chars(&root));
    // Paid every session (Phase A/B) = prefix + CORE bodies + memory spine.
    let upfront = block_tok + core_tok + spine_tok;
    let deferred = spec_tok + on_tok;
    // Naive baseline (pre-A2): every always-on body (CORE + SPECIALIST) upfront.
    let naive = block_tok + core_tok + spec_tok + spine_tok;
    let saved = naive.saturating_sub(upfront);
    let saved_pct = if naive > 0 { saved * 100 / naive } else { 0 };

    // Full on-disk footprint reachable on trigger.
    let (core_fp, spec_fp, on_fp) = (sum_dir(&st.core), sum_dir(&st.specialist), sum_dir(&st.ondemand));

    // A1 gate: rebuilding the block must be byte-identical (no volatile content
    // → KV-cache safe). Regression guard if a timestamp ever leaks back in.
    let stable = build_force_load(&env.home, &root).block == st.block;

    println!();
    println!("  project: {}", root.display());
    println!();
    println!("  ── UPFRONT — context paid EVERY session ─────────────────────");
    println!("   force-load prefix        ~{:>6} tok", block_tok);
    println!("   CORE bodies ({:>2})         ~{:>6} tok   [{}]", st.core.len(), core_tok, names(&st.core));
    println!("   memory spine (6 files)   ~{:>6} tok", spine_tok);
    println!("   ─────────────────────────────────────────");
    println!("   UPFRONT TOTAL            ~{:>6} tok", upfront);
    println!();
    println!("  ── DEFERRED — read only when a task triggers it ─────────────");
    println!("   SPECIALIST bodies ({:>2})   ~{:>6} tok   [{}]", st.specialist.len(), spec_tok, names(&st.specialist));
    println!("   on-demand bodies  ({:>2})   ~{:>6} tok", st.ondemand.len(), on_tok);
    println!("   DEFERRED TOTAL           ~{:>6} tok", deferred);
    println!();
    println!("  ── A2 progressive disclosure ────────────────────────────────");
    println!("   naive  (all always-on upfront) ~{:>6} tok", naive);
    println!("   Phase A (CORE only upfront)    ~{:>6} tok", upfront);
    println!("   SAVED upfront                  ~{:>6} tok   ({}%)", saved, saved_pct);
    println!();
    println!("  ── footprint reachable on trigger (files on disk) ───────────");
    println!("   CORE {} · SPECIALIST {} · on-demand {}", kb(core_fp), kb(spec_fp), kb(on_fp));
    println!();

    if stable {
        ui::ok("A1 stable-prefix: force-load block byte-identical on rebuild (KV-cache safe)");
    } else {
        ui::err("A1 stable-prefix: block DIFFERS on rebuild — volatile content in the prefix");
    }
    ui::info("A3 headroom: route tool output > ~50 lines through headroom_compress (STEP 0)");
    ui::info(&format!(
        "scorecard: upfront ~{} tok · deferred ~{} tok · A2 saved {}% · A1 {}",
        upfront, deferred, saved_pct, if stable { "PASS" } else { "FAIL" }
    ));
    Ok(())
}
