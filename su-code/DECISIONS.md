# DECISIONS (8sync managed — append-only)

## 2026-08-08 — multi-session (named per-project sessions)
- **Build ON omp's session core, don't reinvent it** (user directive: "core base từ omp có sẵn hết
  rồi chỉ viết để extends"). omp owns transcript storage + `--continue`/`--resume`; 8sync adds only
  the 3 things omp lacks: a human **name → `--session-dir`** map, **git worktree** isolation, and
  **merge**. Registry (`~/.config/8sync/sessions/<repo>/index.json`) stores nothing omp already has.
- **One `--session-dir` per name** beats tracking omp session UUIDs — omp's own `--continue` then
  resumes the right conversation with zero bookkeeping.
- **Merge = real git branch integration**, adopted wholesale from affaan-m/ECC (MIT): `git
  merge-tree --write-tree` read-only preflight → `git merge --no-edit` → rebase-to-unblock → cleanup.
  ALL via `git` CLI shell-out (which is how ECC does it too) → zero new deps. ECC's
  rusqlite/git2/tokio/ratatui stack REJECTED (busts the 5 MiB budget). NOT context/history merge.
- **CLI namespaced under `8sync .`** (not new top-level `new/rm/merge` verbs): verb count is already
  26 vs the ≤22 target, and a top-level `8sync rm` is a dangerously generic name.
- **prompt-optimizer (AGPL-3.0) and TypeScript-7/tsgo evaluated and NOT integrated** — the first
  optimizes human prose (wrong layer) + is license-incompatible; the second is transparent to
  `8sync run` (delegates to package.json), no code change.
