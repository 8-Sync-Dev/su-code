---
name: autoresearch
description: Bounded research experiment loop - try hypotheses, measure benchmark evidence, keep what works, discard what doesn't, repeat. Use when the user wants an automated try/measure/keep-or-revert optimization loop against a benchmark command.
---

# Autoresearch

Ported from `companion-inc/feynman`'s `/autoresearch` slash-command. Self-contained omp-native version — runs the bounded foreground experiment loop with omp's own tools (`bash`/`eval` for the benchmark, `edit`/`write` for the change, `write` for logging). No `feynman` CLI or its `init_experiment`/`run_experiment`/`log_experiment` tools required.

## Step 1: Gather

If `autoresearch.md` and `autoresearch.jsonl` already exist in the repo, `ask` the user whether to resume or start fresh. If `CHANGELOG.md` exists, read its most recent relevant entries first.

Otherwise collect from the user before doing anything else:
- What to optimize (accuracy, retrieval quality, loss, latency, …)
- The benchmark command to run
- The metric name, unit, and direction (lower/higher is better)
- Files in scope for changes
- Max iterations (default 20)

## Step 2: Environment

`ask` where to run: **Local** (cwd) / **New git branch** (keep main clean) / **Virtual environment** / **Docker** (see the `docker` skill) / **Modal** (see `modal-compute`) / **RunPod** (see `runpod-compute`). Do not proceed without a clear answer.

## Step 3: Confirm

Present the full plan and `ask` for explicit approval before starting:
```
Optimization target: [metric] ([direction])
Benchmark command:   [command]
Files in scope:      [files]
Environment:         [chosen environment]
Max iterations:      [N]
```

## Step 4: Run

Initialize `autoresearch.md` (human-readable log), `autoresearch.jsonl` (one JSON record per iteration: `{iteration, change, metric_value, baseline_value, decision, evidence}`), and `autoresearch.sh` (re-runnable benchmark invocation). Run the baseline with `bash`/`eval` and record it as iteration 0.

Each iteration:
1. `edit` the change for this hypothesis.
2. Run the benchmark command (`bash`/`eval`), capture output.
3. `write`/append the result + evidence + decision to `autoresearch.jsonl` and `autoresearch.md`.
4. Compare against the baseline: keep the change, revert it (`edit` back), or record the failed hypothesis and move to the next.
5. Repeat until interrupted or `maxIterations` reached.

After the baseline and after meaningful milestones, append a concise `CHANGELOG.md` entry: what changed, what metric result was observed, what failed, next step.

## Subcommands (informal — respond to these phrasings)

- "resume autoresearch" — read `autoresearch.md`/`.jsonl`, continue the loop
- "stop autoresearch" — stop the loop, keep the data files
- "clear autoresearch" — delete `autoresearch.md`/`.jsonl`/`.sh` and start fresh (confirm with the user first — destructive)
