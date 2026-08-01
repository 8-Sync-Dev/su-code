# M0 — Land pending WIP

## 📌 Requirement scope

UC-1 — land finished-but-uncommitted deliverables as atomic, verified commits.

## 🎯 Goal

`main` has a clean working tree at a known-good commit, so every byte measured
in M1 is attributable to a committed state rather than to loose WIP.

## Inventory (measured `git status`, 2026-08-02, base `589807e`)

| group | paths | what it is |
|---|---|---|
| G1 `8sync omp update` verb | `crates/cli/src/verbs/omp.rs` (new), `main.rs`, `verbs/mod.rs`, `verbs/up.rs` | native verb that updates omp and auto-repairs a blocked install (`EEXIST` / `Fail extracting tarball`) |
| G2 `branch-sync` skill + `/sync-pr` | `assets/skills/branch-sync/**`, `assets/commands/sync-pr.md`, `.omp/commands/sync-pr.md`, `verbs/skill/deploy.rs` | multi-branch audit / deep-preview / zero-conflict sync + its slash command |
| G3 harness global auto-stamp | `crates/cli/src/verbs/harness/global.rs` | auto-detect `su-code/` projects in cwd; no explicit `--sweep` needed |
| G4 `deep-research` §5 + brief | `assets/skills/deep-research/SKILL.md`, `outputs/native-tooling-zig-rust*.md`, `outputs/.plans/**`, `outputs/.drafts/**` | native/binary-weight audit protocol + the audit it produced |
| G5 memory / docs | `CHANGELOG.md`, `su-code/KNOWLEDGE.md`, `su-code/STATE.md`, `AGENTS.md`, `CLAUDE.md` | changelog + validated/failure learnings + regenerated skill index |

`AGENTS.md` / `CLAUDE.md` are **generated** between the `8sync:skills` sentinels
by `harness global`; they travel with G2/G4 rather than as their own commit.

## Decisions

- **D-M0-1** — commit in dependency order G1 → G2 → G3 → G4 → G5, each
  independently building. Deliverable-shaped, not file-shaped.
- **D-M0-2** — `outputs/` is a tracked path (`git check-ignore` says not
  ignored) and the brief is a real deliverable → commit it. Scratch build dirs
  were already removed.
- **D-M0-3** — no `git push`, no PR (`feature-rules` R8, su-code convention).
- **D-M0-4** — no feature branch; commit straight to `main` as local checkpoints.

## ✅ Acceptance Criteria

| AC | Criterion | How verified |
|---|---|---|
| AC-01 | `git status --porcelain` is empty | command output is zero-length |
| AC-02 | Every commit compiles | `cargo build --release` exits 0 at HEAD after each `engine_advance` |
| AC-03 | Runtime smoke passes at final HEAD | `8sync --version`, `8sync help`, `8sync omp -h`, `8sync feature list`, `8sync harness -h` all exit 0 |
| AC-04 | `8sync doctor` reports no regression | STEP-0 stack (codegraph · cbm · serena · headroom · zai-vision) still green |
| AC-05 | Commit messages are Conventional Commits in English, no AI attribution | `git log --oneline` inspection |
| AC-06 | Baseline recorded | size of `target/release/8sync` at final M0 HEAD written into `M0-VERIFICATION.md` as the M1 baseline |
