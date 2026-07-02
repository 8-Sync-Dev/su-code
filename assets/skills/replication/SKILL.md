---
name: replication
description: Plan a replication workflow for a paper, claim, or benchmark; execute only after an explicit environment choice. Use when the user wants to reproduce a paper's result or verify a benchmark claim.
---

# Replication

Ported from `companion-inc/feynman`'s `/replicate` slash-command. Self-contained omp-native version.

## Tool mapping

Extract paper/code details → `task` (role "Researcher") or direct `web_search`/`read`. Execute — see the `docker`, `modal-compute`, and `runpod-compute` skills for the isolated-environment options below.

## Workflow

1. **Extract** — `task` (role "Researcher") to pull implementation details from the target paper and any linked code. If `CHANGELOG.md` exists, read its recent entries first.
2. **Recipe pass** — For ML training/fine-tuning/benchmark/dataset-heavy targets, do a recipe extraction before planning execution (see `ml-training-recipe`): link each claimed result to the exact dataset, method, hyperparameters, compute assumptions, metric, and code path. Mark unchecked details `unverified`.
3. **Plan** — What code, datasets, metrics, environment are needed. Be explicit about verified vs. inferred vs. missing, and what checks/test oracles decide success.
4. **Environment** — `ask` the user before running anything:
   - **Local** — cwd
   - **Virtual environment** — isolated venv/conda first
   - **Docker** — see the `docker` skill
   - **Modal** — see `modal-compute`
   - **RunPod** — see `runpod-compute`
   - **Plan only** — produce the plan, execute nothing
5. **Execute** — If an environment was chosen, implement and run there. Save notes/scripts/raw outputs/results to disk in a reproducible layout. Never call it "replicated" unless the planned checks actually passed.
6. **Log** — For multi-step/resumable work, append concise `CHANGELOG.md` entries after meaningful progress, failed attempts, major verification outcomes, and before stopping: active objective, what changed, what was checked, next step.
7. **Report** — End with a `Sources` section (paper, dataset, docs, repo URLs).

Never install packages, run training, or execute experiments without confirming the execution environment first.
