# Project — Multi-Session (named parallel work sessions)

## What

Named, per-project work sessions so a user can drive **several features at once** in one
repo — each session an isolated omp conversation, optionally on its own git worktree+branch,
with a `merge` that folds finished sessions back to the base branch.

Today `8sync .` just runs `omp --continue` (latest conversation for the cwd). There is **no**
way to keep N distinct lines of work, name them, list them, or merge them. AGENTS.md §5 already
*reserves* `8sync . ls/to/new/rm/mv/…` but none of it is implemented (`here::Args` is empty).

## Core value

`8sync . <name>` = jump straight back into feature X's conversation **and** its isolated
working tree; `8sync . merge feat-x` = land it. Parallel features stop trampling each other's
files and each other's context.

## Cắm vào codebase (anchors — do NOT re-describe)

- **Entry verb:** `crates/cli/src/main.rs` `Cmd::Here` = `#[command(name=".", alias="here")]`
  → `verbs/here.rs` (`Args {}` empty; `run()` execs `omp --cwd <root> --continue`).
- **omp launch config:** `verbs/ai.rs` + `models.rs` (`ModelConfig::resume_flags`/`omp_flags`,
  STEP-0 `--tools` allowlist, advisor). Any new omp exec MUST reuse these (STEP-0 + advisor
  must survive).
- **Project root detection:** `here::detect_project_root` (walks up to a git/omp marker).
- **omp session store (mechanism we build on):** path-scoped JSONL at
  `~/.omp/agent/sessions/<slug-of-cwd>/<ts>_<uuid>.jsonl` (+ sidecar dir). Flags:
  `--continue` (latest), `--resume <id|path|picker>`, `--session-dir <dir>` (storage+lookup
  root), `--profile`, `--no-session`. **No native named session** — titles are auto free-text.
- **Isolation mechanism (chosen):** one **`--session-dir` per name** ⇒ omp's own `--continue`
  resumes exactly that session (zero uuid capture); one **git worktree+branch `8sync/<name>`**
  per name for filesystem isolation. Merge engine = pure `git` CLI shell-out.
- New verb module: `crates/cli/src/verbs/session.rs` (or extend `here.rs`); wired in
  `verbs/mod.rs` + `main.rs`.

## Ràng buộc (invariants)

- **No heavy deps.** Everything via `git`/`omp` shell-out (matches curl/systemctl style).
  NO rusqlite/git2/tokio — a JSONL/JSON registry + `Command::new("git")` covers it (this is
  exactly how ECC implements the same engine). Size budget: ceiling 5 MiB, goal 4 MiB.
- **Default KHÔNG ĐÈ** (AGENTS.md §8): never delete a dirty/unmerged worktree or a session
  transcript without an explicit `--force`/confirm. `rm` is destructive → guard it.
- **STEP-0 + advisor survive** every omp launch (reuse `models.rs`, never a bare `omp`).
- **Verb budget** (AGENTS.md §8, ≤22 target; already 26): prefer namespacing under `8sync .`
  over new top-level verbs. CLI surface is a Gate-1 decision (see REQUIREMENTS).
- Registry is **machine-local** (`~/.config/8sync/sessions/<project-key>.json`) — it points at
  machine-local omp session dirs + worktree paths; committing it would break on another box
  (mirrors omp's own ~/.omp scoping).

## Evaluated references (user asked to review — recorded so we don't re-litigate)

- **affaan-m/ECC (`ecc2/`)** — MIT. **Blueprint, borrow-ideas-only.** Its git-worktree-per-session
  isolation + `git merge-tree --write-tree` conflict-preflight + `git merge --no-edit` +
  rebase-to-unblock + branch-vs-branch merge queue are **all done via `git` CLI shell-out** →
  directly transplantable with zero deps. We DON'T copy its rusqlite+git2+tokio+ratatui stack
  (busts the budget). ECC lacks a **name** selector (uuid/`latest` only) — that gap is our core
  addition (name → `8sync/<name>` slug).
- **linshenkx/prompt-optimizer** — AGPL-3.0. **NOT integrated (unchanged verdict).** Optimizes
  human-authored *prose* prompts (the layer that already failed here); AGPL vs our MIT (no
  vendoring); irrelevant to session/merge. No change vs the prior eval.
- **TypeScript 7 / typescript-go (`tsgo`)** — Apache-2.0. **Irrelevant + no code change.** TS7 is
  transparent to 8sync because `8sync run` delegates to `package.json` scripts; `detect_stack`
  only needs `package.json`. At most a mental model (snapshot/overlay = "isolated consistent view
  per unit of work" — which git worktrees already give us).
