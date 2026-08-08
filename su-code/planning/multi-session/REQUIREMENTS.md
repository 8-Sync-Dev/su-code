# Requirements — Multi-Session

Use cases (UC). Each ROADMAP AC maps back to ≥1 UC.

## v1 (in scope)

- **UC-1 — Create a named session.** `8sync . new <name>` creates a fresh omp conversation bound
  to name, scoped to the current repo. Re-creating an existing name is refused (suggest resume).
- **UC-2 — Resume by name / latest.** `8sync . <name>` resumes that session's conversation;
  `8sync .` (no name) resumes the **latest-active** session in this repo (today's behavior kept).
  Resuming an unknown name offers to create it.
- **UC-3 — List sessions.** `8sync . ls` / `8sync . --list` shows every session in this repo:
  name, omp auto-title, last-active, and (if any) branch + dirty/clean + merge-readiness.
- **UC-4 — Rename.** `8sync . mv <old> <new>` renames a session (registry + branch when a
  worktree exists).
- **UC-5 — Delete safely.** `8sync . rm <name>` removes a session. Guards: never drop a
  dirty/unmerged worktree or delete the transcript without `--force`/confirm. `--worktree` also
  removes the worktree+branch (when clean/merged).
- **UC-6 — Isolated working tree.** `8sync . new <name> --worktree` puts the session on its own
  `git worktree add -b 8sync/<name> <wt-root>/<name> HEAD`, so two sessions edit the same files
  concurrently without collision. `8sync .` inside a worktree resumes that session.
- **UC-7 — Merge finished sessions back.** `8sync . merge <target> <src...>` integrates source
  session branches. Read-only conflict **preflight** (`git merge-tree --write-tree`) first; only
  Ready branches `git merge --no-edit`; multiple sources ordered by branch-vs-branch conflict
  (queue: "merge B after A"); a conflicted source is `git rebase`d onto the target to unblock
  (auto-abort on failure); STOP + report on true conflict (never force). Merged worktrees+branches
  cleaned up unless `--keep-worktree`.
- **UC-8 — Scriptable + discoverable.** `--json` on mutating/list commands; `8sync doctor`
  reports session count + any stuck/dirty/blocked worktree; AGENTS.md §5 updated to match reality.

## v2 (out of scope for v1 — recorded proposals)

- **UC-9 — Context merge.** Optionally fold a one-line summary of merged sessions into the target
  omp conversation. (ECC offers no precedent — code-branch merge only. Defer; keep merge = git.)
- **UC-10 — Feature-slug binding.** Bind `8sync . <name>` ↔ a `/feature` planning slug so the
  worktree and `su-code/planning/<slug>/` align. Deferred.
- **UC-11 — Session daemon / heartbeat + TUI dashboard** (ECC has this). YAGNI for a CLI harness;
  deferred unless demanded.
- **UC-12 — Shared dependency sync into new worktrees** (node_modules-style cache linking, ECC
  `sync_shared_dependency_dirs`). Nice-to-have; deferred.

## Explicitly NOT doing

- No rusqlite/git2/tokio/ratatui (ECC's deps) — budget. Pure `git`/`omp` shell-out + JSON registry.
- No prompt-optimizer integration (AGPL + wrong problem). No tsgo/TS7 special-casing (transparent).
- No `git push` / PR from any session verb (su-code convention). Merge is local-only.
