---
name: feature
argument-hint: '[new <slug> | plan | go | ship | status | switch <slug> | list] [--auto]'
description: Large multi-phase feature scopes (GSD). Drives the `feature` skill — scaffold a planning tree (su-code/planning/<slug>/), plan a phase (Goal+AC), execute it by feeding the phase into a GS run (gs_* tools), and verify against the AC matrix. Cross-session, switchable. Small/single-concern work → /gs.
---

# /feature — large-scope GSD framework

`$ARGUMENTS` first word selects the subcommand; `--auto` anywhere = autonomous full-phase.

Use this for **large, multi-phase features** (>10 files, multiple milestones, spanning
sessions). Small/single-concern work → **`/gs`** (drive the GS engine directly). This layer
owns the multi-feature ROADMAP + per-phase Acceptance-Criteria contract *above* the
`gs_*` loop.

## 0. Load the skill + ground (do this first, every subcommand)
Read the bundled skill and its rules BEFORE acting:
- `~/.omp/skills/feature/SKILL.md` + `~/.omp/skills/feature/references/feature-rules.md`
  (the always-applied rules), then the reference for the specific subcommand
  (`new`/`plan`/`execute`/`ship`/`auto`).
Then ground on state (except for `new`, which creates it):
- `su-code/planning/ACTIVE.md` line 1 → active slug; `su-code/planning/<slug>/STATE.md`
  frontmatter (`status`/`active_phase`/`next_action`); `su-code/planning/config.json`
  (`workflow.*`, `paths.*`). Obey `~/.omp/agent/APPEND_SYSTEM.md` (code-intel first;
  always-on skills). Explore with **codegraph / codebase-memory-mcp / serena** — never
  grep/Read-all; `headroom_compress` any tool output > ~50 lines.

## 1. Dispatch (`$ARGUMENTS` first word)
| word | do |
|------|----|
| `new <slug>` | `references/new.md` — the deterministic scaffold is also `8sync feature new <slug>`; then fill PROJECT/REQUIREMENTS/ROADMAP with the user, cut phases by dependency. Gate 1: user approves the architecture. |
| `plan` | `references/plan.md` — discuss + write `M<x>-CONTEXT.md` (📌 Requirement scope + 🎯 Goal + ✅ AC table) + `M<x>-NN-PLAN.md` (tasks ↔ UC ↔ AC, waves). Plan-review per `config.workflow.plan_review`. Gate 2: user approves the AC + plan. |
| `go` | `references/execute.md` — **start/resume a GS run first** (`/gs <phase goal>` or `/gs --auto <phase goal>`), then `gs_define` (UC/AC), traverse to plan, `gs_plan`, and obey each exact `gs_next` lease. Workers return evidence; `gs_verify` gates each task; parameterless `gs_advance` crosses stages through verifier → independent review/security → user UAT → closeout. |
| `ship` | `references/ship.md` — import the completed native GS run's canonical AC evidence into `M<x>-VERIFICATION.md`, tick ROADMAP, and archive on the final phase. Missing/stale evidence reopens GS; the feature layer never creates a second review/test loop. |
| `status` | print the active STATE position (same as `8sync feature status`). |
| `switch <slug>` | flip ACTIVE (same as `8sync feature switch <slug>`), then re-ground. |
| `list` | list features + archived (same as `8sync feature list`). |
| _(empty)_ | read STATE → suggest the next action from `next_action`. |

## 2. `--auto` (autonomous full-phase)
If `$ARGUMENTS` contains `--auto`, load `auto.md` in `references/` FIRST, then run the phase
autonomously: the GS run uses auto-mode gates (independent critic replaces the user plan
gate); a hard blocker (credential / real external data) → SKIP the item, record NEEDS-CONFIRM
in VERIFICATION/STATE, and finish the rest of the phase. Stop at the next phase boundary
(never auto-advance phases). This mirrors `/gs --auto` discipline, scoped to one phase.
GS still requires `/gs approve uat`; each destructive/outward action needs one-shot consent bound to the exact command hash.

## Guardrails
Work must belong to the `active_phase` (stop + ask if it drifts; `--auto` → SKIP+NEEDS-CONFIRM).
Verify-gate before every commit (`gs_verify` enforces it; `gs_advance` refuses to leave a stage
whose gate is unmet). Scope edits to the change + `su-code/` memory. NO `git push` / PR unless
asked. Every AC maps to ≥1 UC in REQUIREMENTS; every task maps to ≥1 AC — no orphan work.

## Model + context budget
Use the models in `~/.config/8sync/models.toml` (view/edit: `8sync harness model`); route per
task class. When context nears the limit (auto-compacts at 50%), write a handoff into the
active feature's `su-code/planning/<slug>/STATE.md` (< 100 lines, digest) BEFORE it fires.

Begin: load the skill, ground on ACTIVE + STATE + config, then act on `$ARGUMENTS`.
