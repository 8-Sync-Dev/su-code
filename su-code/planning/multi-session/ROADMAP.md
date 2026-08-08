# Roadmap — Multi-Session

Cut by dependency: you cannot resume/list/merge sessions that a **registry** does not yet track;
you cannot isolate work without a **worktree**; you cannot **merge** branches that don't exist.
Foundation (registry + named conversations) → isolation (worktree) → integration (merge) → UX.

| Phase | Name | Serves | Demo after this phase |
|---|---|---|---|
| **M0** | Session registry + named CRUD | UC-1,2,3,4,5 | Create/list/resume/rename/delete named omp sessions per repo; `8sync .` still resumes latest. No worktrees yet. |
| **M1** | Worktree isolation | UC-6 | `8sync . new x --worktree` + `new y --worktree`; both edit the same file concurrently with zero collision; `ls` shows branch + dirty state. |
| **M2** | Merge engine | UC-7 | `8sync . merge main-work x y` predicts conflicts, merges the ready ones, rebases/blocks the rest, cleans up worktrees — all via `git`. |
| **M3** | UX + integration + docs | UC-8 | `--json` + `8sync doctor` session health + shell completion + AGENTS.md §5 rewritten to match reality; final CLI-surface aliases wired. |

## Integration contracts

- **M0 → M1:** registry API (`Registry::load/save`, `Session{name, session_dir, worktree:Option,
  base_branch, created, last_active}`) + resolver (`resolve(name|latest)`), stored at
  `~/.config/8sync/sessions/<project-key>.json`. omp launched via `--session-dir <per-name>`
  reusing `models.rs` flags. M1 adds the `worktree` field + branch, not a new store.
- **M1 → M2:** each worktree session carries `{path, branch=8sync/<name>, base_branch}`. M2's merge
  engine consumes exactly this — mirrors ECC `WorktreeInfo{path, branch, base_branch}`.
- **M2 → M3:** merge + worktree lifecycle stable and JSON-describable, so `doctor` and completion
  read the same registry + `git` status the merge engine already computes.

## Dependency reasoning

M0 first because every other verb resolves a name through the registry. M1 before M2 because a
merge needs branches to merge (no worktree ⇒ no branch). M3 last because docs/doctor/aliases
describe the finished surface — writing them earlier would document a moving target.

## CLI-surface decision (Gate 1 — see ask)

Canonical home = subcommands of `8sync .` (`8sync . new|ls|rm|mv|merge <name>`, `8sync . <name>`,
`8sync .`), matching AGENTS.md §5's already-reserved design and avoiding top-level verb sprawl
(already 26 vs ≤22 target) + a dangerously generic top-level `8sync rm`. The user-requested flat
forms (`8sync new/rm/merge`) can be added as thin aliases if desired — that is the Gate-1 fork.
