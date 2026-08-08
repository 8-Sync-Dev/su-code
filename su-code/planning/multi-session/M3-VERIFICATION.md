# Verification — Multi-Session (all phases)

Method: `cargo build --release` clean after every phase; behavior exercised end-to-end with a
**stub `omp`** (records argv, exits 0) in throwaway git repos under an isolated `XDG_CONFIG_HOME`,
so launches don't block on a TTY and the real registry + `git` engine are exercised.

| UC | Acceptance | Result | Evidence |
|---|---|---|---|
| UC-1 | `new <name>` creates; refuses existing | ✅ | `new feat-a`→created; `new feat-a` again → `Error: session 'feat-a' already exists` |
| UC-2 | `. <name>` resume; `.` resumes latest; unknown name creates | ✅ | bare `. feat-a` launched omp `--continue` with feat-a's `--session-dir`; `.` resumed last-used; unknown name auto-created |
| UC-3 | `ls`/`--list` shows name, title, last-active, branch/dirty | ✅ | `ls` printed `★ feat-b … 8sync/feat-b *dirty`; `--json` emitted structured array |
| UC-4 | `mv <old> <new>` renames (+ branch/worktree) | ✅ | `mv feat-b feat-c` renamed registry + dir; worktree case moves dir + `git branch -m` |
| UC-5 | `rm <name>` safe; guards dirty/unmerged; `--force` deletes | ✅ | `rm feat-a` kept transcript + warned; dirty worktree `rm` refused without `--force`; unmerged branch kept with warning |
| UC-6 | `--worktree`: two sessions edit same file concurrently, no collision | ✅ | `new a --worktree`+`new b --worktree` → branches `8sync/a`,`8sync/b`; both edited `shared.txt` independently (main=line0, a=A-change, b=B-change) |
| UC-7 | `merge`: merge-tree preflight → merge → rebase-to-unblock → cleanup; multi-source; conflict-skip; local-only | ✅ | clean merge of `feat-x feat-y`→main landed both + cleaned worktree/branch/session; same-line conflict → rebase → abort → skip with manual-resolve msg, main untouched; `--keep-worktree` preserved branch |
| UC-8 | `--json` + `8sync doctor` session health + AGENTS.md §5 updated | ✅ | `ls --json` valid array; `doctor` → `✓/! sessions (this repo): N · M worktree(s) · K dirty`; AGENTS.md §5 rewritten to the real surface |

## Invariants held
- **STEP-0 + advisor survive** every launch: captured omp argv carries `--tools read,bash,…` allowlist
  (grep/glob dropped) + `--advisor` + role flags via `ModelConfig::resume_flags`.
- **No new dependencies:** entire worktree/merge engine is `git`/`omp` shell-out (ECC's
  rusqlite/git2/tokio NOT adopted). Build warnings unchanged (only pre-existing `proc-macro-error2`).
- **Thin over omp:** registry stores only name/worktree/last-used; transcripts + resume owned by omp.
- **Safe by default:** merge refuses a dirty main tree; `rm`/`merge` never destroy dirty/unmerged work
  without `--force`; nothing is pushed.

## Not done (deferred, recorded)
- v2 UC-9 context-merge, UC-10 feature-slug binding, UC-11 daemon/TUI, UC-12 shared-dep sync.
- Fresh-repo note: `8sync . new` seeds untracked `AGENTS.md`/`su-code/` in the main repo, so a first
  `merge` asks you to commit them (real repos already track these — a fresh-repo artifact, not a bug).
