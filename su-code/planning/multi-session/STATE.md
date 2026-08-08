---
gsd_state_version: '1.0'
feature: multi-session
ticket: ""
branch: ""
status: complete
active_phase: "M3"
next_action: archive
next_phases: []
progress:
  total_phases: 4
  completed_phases: 4
  percent: 100
last_updated: "2026-08-08"
---

# State — Multi-Session

## Project Reference

See: su-code/planning/multi-session/PROJECT.md · ROADMAP: su-code/planning/multi-session/ROADMAP.md
**Core value:** run several features at once in one repo — each a named, isolated omp session
(optional git worktree), with `merge` to land finished ones back to base.

## Current Position

Phase: M3 of 4 (UX + docs) — **DONE**. Feature COMPLETE; all 4 phases shipped + smoke-verified.
Status: complete. Gate 1 approved: namespaced under `8sync .` + opt-in `--worktree`.
Last activity: 2026-08-08 — M0 registry+CRUD, M1 worktree, M2 merge, M3 `--json`/doctor/docs. All
built clean + smoke-tested with a stub-omp harness; committed atomically per phase.

## Decisions (locked)

- **Thin layer over omp's session core** (per user directive): omp owns transcripts + `--continue`;
  8sync adds only name→`--session-dir`, git worktree, merge. Registry stores just what omp lacks.
- **Isolation:** one omp `--session-dir` per name; `--worktree` (opt-in) = `git worktree add -b
  8sync/<name>`. CWD = worktree when isolated, else repo root.
- **Merge = real git branch merge** (ECC/MIT blueprint): `git merge-tree` preflight → `git merge
  --no-edit` → rebase-to-unblock → cleanup. Pure `git` shell-out, zero new deps.
- **CLI:** namespaced under `8sync .` (no top-level verb sprawl); `rm` guards dirty/unmerged.
- **Rejected refs:** prompt-optimizer (AGPL + wrong layer), tsgo (transparent, no change).

## Verification

See M3-VERIFICATION.md — every AC PASS via stub-omp smoke tests (create/refuse/ls/mv/rm/resume,
parallel worktree edits with no collision, merge clean + conflict-skip + --keep-worktree, --json,
doctor health clean+dirty). `cargo build --release` clean each phase.

## Session Continuity

Stopped at: feature complete, 4 phase commits on `main` (M0 3ef2929, M1 f282d3e, M2 000d3ab, M3 next),
tree clean, nothing pushed. Next: `/feature ship` archive, or fold into the pending release tag.
