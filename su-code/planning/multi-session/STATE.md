---
gsd_state_version: '1.0'
feature: multi-session
ticket: ""
branch: ""
status: executing
active_phase: "M1"
next_action: M1-worktree-isolation
next_phases: [M2, M3]
progress:
  total_phases: 4
  completed_phases: 1
  percent: 25
last_updated: "2026-08-08"
---

# State — Multi-Session

## Project Reference

See: su-code/planning/multi-session/PROJECT.md · ROADMAP: su-code/planning/multi-session/ROADMAP.md
**Core value:** run several features at once in one repo — each a named, isolated omp session
(optional git worktree), with `merge` to land finished ones back to base.

## Current Position

Phase: M0 of 4 (Session registry + named CRUD) — **planning, awaiting Gate 1**
Status: `new` scaffolded + 4 files drafted from research; architecture NOT yet approved.
Why here: `/feature new` — 3 references researched (ECC/prompt-optimizer/tsgo); design synthesized.
Last activity: 2026-08-08 — drafted PROJECT/REQUIREMENTS/ROADMAP; ECC gives the git-shell-out
merge blueprint (MIT). Awaiting user approval of architecture + CLI-surface fork.

## Decisions (pending confirmation at Gate 1)

- **Isolation:** one omp `--session-dir` per name (resume via omp's own `--continue`, no uuid
  capture) + optional `git worktree add -b 8sync/<name>` for filesystem isolation.
- **Merge = real git branch merge** (ECC blueprint): `git merge-tree` preflight → `git merge
  --no-edit` → `git rebase` to unblock → cleanup. Pure `git` shell-out, zero new deps. NOT
  context/history merge (that is v2 UC-9).
- **Registry** machine-local: `~/.config/8sync/sessions/<project-key>.json`.
- **CLI surface (OPEN FORK):** canonical under `8sync .` subcommands vs also adding flat
  top-level `8sync new/rm/merge` aliases. Recommend namespaced (verb budget 26>22; generic `rm`).
- **Rejected refs:** prompt-optimizer (AGPL + wrong layer), tsgo (transparent, no change).

## Open questions for Gate 1

1. CLI surface: namespaced-under-`.` only (recommended) vs + flat `8sync new/rm/merge` aliases?
2. Worktree default: opt-in via `--worktree` (recommended; many sessions are just parallel
   conversations) vs every named session always gets a worktree?

## Session Continuity

Stopped at: 4 planning files written; ACTIVE = multi-session; awaiting Gate 1.
Next: user approves architecture + answers the 2 forks → `/feature plan` for M0.
