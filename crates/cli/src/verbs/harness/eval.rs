//! `8sync harness eval` — quality probe for the engineering loop. Runs a fixed
//! task-suite through omp non-interactively, scores each task with a
//! deterministic self-check (`verify.sh` owns the assertion so the agent can't
//! game it), and writes a JSON scorecard (+ optional baseline diff). Model +
//! network, non-deterministic — a periodic quality SIGNAL, not a CI gate.
use std::collections::BTreeSet;
use std::process::Command;
use std::time::Instant;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::assets;
use crate::{env_detect, ui};

/// Per-task omp wall-clock cap (seconds). A stuck task fails on its verifier.
const MAX_TIME_SECS: &str = "300";

#[derive(Serialize, Deserialize)]
struct TaskResult {
    name: String,
    pass: bool,
    secs: u64,
}

#[derive(Serialize, Deserialize)]
struct EvalReport {
    stamp: String,
    passed: usize,
    total: usize,
    results: Vec<TaskResult>,
}

/// Unique fixture names = first path segment under the embedded `eval/` tree.
fn fixture_names() -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for p in assets::iter_under("eval/") {
        if let Some(rest) = p.strip_prefix("eval/") {
            if let Some(name) = rest.split('/').next() {
                if !name.is_empty() {
                    set.insert(name.to_string());
                }
            }
        }
    }
    set.into_iter().collect()
}

pub(crate) fn harness_eval(env: &env_detect::Env, baseline: bool) -> Result<()> {
    ui::header("8sync harness eval — loop quality probe");
    if which::which("omp").is_err() {
        ui::warn("omp not found — eval needs omp on PATH");
        return Ok(());
    }
    let names = fixture_names();
    if names.is_empty() {
        ui::warn("no eval fixtures bundled");
        return Ok(());
    }
    let cache = env.home.join(".cache/8sync/eval");
    ui::info(&format!(
        "running {} task(s) through omp (model + network; non-deterministic)…",
        names.len()
    ));
    println!();

    let mut results: Vec<TaskResult> = Vec::new();
    for name in &names {
        let dir = cache.join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        // Materialise the fixture's setup/ tree into the run dir.
        assets::install_tree(&format!("eval/{}/setup", name), &dir)?;
        // verify.sh sits beside setup/ → drop it into the run dir (hidden).
        if let Some(v) = assets::read(&format!("eval/{}/verify.sh", name)) {
            let vp = dir.join(".eval-verify.sh");
            std::fs::write(&vp, v)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&vp, std::fs::Permissions::from_mode(0o755));
            }
        }
        let task = assets::read(&format!("eval/{}/task.md", name)).unwrap_or_default();

        let t0 = Instant::now();
        let out = Command::new("omp")
            .args(["-p", "--no-session", "--auto-approve", "--max-time", MAX_TIME_SECS])
            .arg(&task)
            .current_dir(&dir)
            .output();
        let secs = t0.elapsed().as_secs();

        let pass = match out {
            Ok(o) => {
                let _ = std::fs::write(dir.join(".omp-out.txt"), &o.stdout);
                Command::new("sh")
                    .arg(".eval-verify.sh")
                    .arg(".omp-out.txt")
                    .current_dir(&dir)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            }
            Err(_) => false,
        };
        println!("   {} {:<22} {:>4}s", if pass { "✓" } else { "✗" }, name, secs);
        results.push(TaskResult { name: name.clone(), pass, secs });
    }

    let passed = results.iter().filter(|r| r.pass).count();
    let total = results.len();
    println!();
    let pct = if total > 0 { passed * 100 / total } else { 0 };
    ui::info(&format!("score: {}/{} passed ({}%)", passed, total, pct));

    let report = EvalReport { stamp: super::memory::now_stamp(), passed, total, results };
    let json = serde_json::to_string_pretty(&report).unwrap_or_default();

    // Scorecards live in the gitignored cache (machine-local + model-dependent;
    // not committed repo state). The baseline is the reference future runs diff.
    let out_dir = cache.clone();
    let _ = std::fs::create_dir_all(&out_dir);
    let run_path = out_dir.join(format!("eval-{}.json", report.stamp.replace(':', "-")));
    if std::fs::write(&run_path, &json).is_ok() {
        ui::ok(&format!("scorecard → {}", run_path.display()));
    }

    let baseline_path = out_dir.join("eval-baseline.json");
    if baseline {
        if std::fs::write(&baseline_path, &json).is_ok() {
            ui::ok(&format!("baseline saved → {}", baseline_path.display()));
        }
    } else if let Ok(prev) = std::fs::read_to_string(&baseline_path) {
        if let Ok(base) = serde_json::from_str::<EvalReport>(&prev) {
            let delta = passed as i64 - base.passed as i64;
            ui::info(&format!(
                "vs baseline ({}): {}/{} → {}/{} ({}{})",
                base.stamp,
                base.passed,
                base.total,
                passed,
                total,
                if delta >= 0 { "+" } else { "" },
                delta
            ));
        }
    }
    Ok(())
}

/// `8sync harness eval --project` — agent-team READINESS scorecard for the
/// current repo: per-role capability coverage (%), computed from real checks
/// (engines on PATH, skills present, memory spine, stack signals). This is
/// "what is the team equipped with HERE", NOT output quality — that is the
/// model+network `harness eval` loop probe. Honest + deterministic + offline.
pub(crate) fn harness_eval_project(env: &env_detect::Env) -> Result<()> {
    use crate::verbs::skill::discover::detect_current_project_root;
    ui::header("8sync harness eval --project — agent-team readiness scorecard");
    let Some(root) = detect_current_project_root() else {
        ui::warn("not inside a project — cd into a repo first");
        return Ok(());
    };
    ui::info(&format!("project: {}", root.display()));
    ui::info("readiness = capabilities the team HAS here (not output quality — that's `harness eval`)");
    println!();

    let home = &env.home;
    let skill = |n: &str| home.join(".omp/skills").join(n).exists() || root.join("agents/skills").join(n).exists();
    let bin = |n: &str| which::which(n).is_ok();
    let has = |p: &str| root.join(p).exists();
    let pkg = std::fs::read_to_string(root.join("package.json")).unwrap_or_default();
    let cfg = std::fs::read_to_string(home.join(".omp/agent/config.yml")).unwrap_or_default();
    let dep = |k: &str| pkg.contains(k);
    let frontend = dep("react") || dep("vue") || dep("next") || dep("svelte") || dep("\"vite\"");
    let backend = dep("encore.dev") || dep("express") || dep("fastify") || dep("@nestjs")
        || has("Cargo.toml") || has("go.mod") || has("requirements.txt") || has("pyproject.toml");
    let build_cmd = pkg.contains("\"build\"") || has("Cargo.toml") || has("Makefile") || has("go.mod");
    let test_cmd = pkg.contains("\"test\"") || has("Cargo.toml") || pkg.contains("vitest") || pkg.contains("jest");
    let cbm = bin("codebase-memory-mcp");

    let roles: Vec<(&str, Vec<(&str, bool)>)> = vec![
        ("dev", vec![("codegraph", has(".codegraph")), ("cbm-graph", cbm), ("build", build_cmd), ("karpathy+ponytail", skill("karpathy-guidelines") && skill("ponytail"))]),
        ("qa/testing", vec![("test", test_cmd), ("full-flow", skill("full-flow")), ("browser-testing", skill("browser-testing-with-devtools")), ("headroom", bin("headroom"))]),
        ("research", vec![("omp/web_search", bin("omp")), ("agent-reach|deep-research", skill("agent-reach") || skill("deep-research")), ("last30days", skill("last30days"))]),
        ("ba/po", vec![("planning", skill("planning-and-task-breakdown")), ("spec-driven", skill("spec-driven-development")), ("STATE+DECISIONS", has("agents/STATE.md") && has("agents/DECISIONS.md"))]),
        ("fe", vec![("frontend-stack", frontend), ("impeccable+taste", skill("impeccable") && skill("taste-skill")), ("senior-frontend", skill("senior-frontend"))]),
        ("be", vec![("backend-stack", backend), ("api-design", skill("api-and-interface-design")), ("security", skill("senior-security") || skill("security-and-hardening"))]),
        ("docs", vec![("docs-skill", skill("documentation-and-adrs")), ("AGENTS.md", has("AGENTS.md")), ("CHANGELOG", has("CHANGELOG.md"))]),
        ("memory/learn", vec![("Mnemopi-ON", cfg.contains("backend: mnemopi")), ("KNOWLEDGE+PLAYBOOKS", has("agents/KNOWLEDGE.md") && has("agents/PLAYBOOKS.md")), ("cbm-graph", cbm)]),
        ("token-opt", vec![("codegraph", bin("codegraph")), ("cbm", cbm), ("headroom", bin("headroom"))]),
    ];

    let (mut tp, mut tn) = (0usize, 0usize);
    for (role, checks) in &roles {
        let p = checks.iter().filter(|(_, ok)| *ok).count();
        let n = checks.len();
        tp += p; tn += n;
        let pct = 100 * p / n;
        let detail: Vec<String> = checks.iter().map(|(l, ok)| format!("{}{}", if *ok { "✓" } else { "·" }, l)).collect();
        let line = format!("  {:<13} {:>3}%  {}", role, pct, detail.join("  "));
        if pct == 100 { ui::ok(&line); } else if pct >= 50 { ui::info(&line); } else { ui::warn(&line); }
    }
    let overall = if tn > 0 { 100 * tp / tn } else { 0 };
    println!();
    ui::info(&format!("OVERALL team readiness: {}%  ({}/{} capabilities present)", overall, tp, tn));
    ui::info("close gaps (·): `8sync harness` (engines+skills) · enable Mnemopi · add a stack skill · seed agents/*");
    Ok(())
}
