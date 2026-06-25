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
