---
gsd_state_version: '1.0'
feature: lean-binary
ticket: ""
branch: ""
status: in-progress
active_phase: "M3"
next_action: plan-phase
next_phases: ["M1","M2","M3"]
progress:
  total_phases: 4
  completed_phases: 3
  percent: 75
last_updated: "2026-08-02"
---

# State — Lean Binary

## Project Reference

See: su-code/planning/lean-binary/PROJECT.md · ROADMAP: su-code/planning/lean-binary/ROADMAP.md
**Core value:** get `8sync` back under its own 4 MB budget without losing a single user-visible feature.
**Current focus:** M1 — feature gating + per-gate byte attribution.

## Current Position

Phase: M3 of 4 (CI + budget truth)
Plan: 0 of 0 (M3 not planned yet)
Status: in-progress
Vì sao phase này: the binary is 24 % smaller and the remaining 665 392 B has no cheap owner — CI and the documented budget must now state measured reality.
Last activity: 2026-08-02 — M2 closed 8/8; 6 407 848 → 4 859 696 B with zero feature loss.

## Accumulated Context

### Decisions
- [M0]: commit deliverable-shaped, not file-shaped — `git log` stays a readable history of *what shipped*.
- [M0]: hand-roll the commits, keep the engine's verify-gate — `engine_advance {commit:true}` does `git add -A` and cannot be trusted to split a dirty tree.
- [M0]: AC-02 proven by worktree replay, not by N green `engine_verify` on one tree.
- [M1 pre]: gate, measure, *then* decide what to delete. No dep is removed on `cargo bloat` guesswork alone.
- [M1]: `marketplace` folds into `web` — one caller (`web.rs:1452`), so it never earned its own flag.
- [M1]: attribution ships as `scripts/size-report.sh`, not a verb — `AGENTS.md` §8 caps verbs at 22 and a verb would add bytes to the thing being measured.
- [M2]: delete, don't gate — `rusqlite` and `elkjs` were removed outright; gating only located them.
- [M2]: a store swap must be proven under FROZEN input (worktree-rebuilt old binary + copied session tree), never by comparing two live runs.
- [M2]: the dashboard layout swap shipped only after headless browser proof, per D-M2-4.
- [M1]: `cargo bloat` may RANK suspects; only an A/B build may state a number (it missed SQLite by ~26×).

### Contract — phase sau CẦN BIẾT
- [M0]: baseline `6 407 848 B` at `97e906d`+docs; budget `4 194 304 B`; overshoot `2 213 544 B`.
- [M0]: heavy deps are single-module — `axum`/`tokio`/`tower-http` → `verbs/harness/web.rs`; `rusqlite` → `verbs/harness/toolstats.rs`; `scraper` → `verbs/harness/marketplace.rs`. Embeds: `assets::Assets` (`assets/`) and `assets::WebAssets` (`web/dist/`) + `assets::web_asset()`.
- [M0]: `crates/cli/Cargo.toml` has **no** `[features]` table — M1 creates it.
- [M1]: features `web` (axum+tokio+tower-http+scraper, `WebAssets`, `build.rs` Vite step) and `toolstats` (rusqlite); `default = ["web","toolstats"]`. Gate helpers `harness::dispatch_web` / `dispatch_toolstats` bail with a `--features` hint when absent.
- [M1]: sizes — full 6 407 144 · web-only 5 346 304 · toolstats-only 4 144 576 · minimal 3 081 416. **minimal and toolstats-only are already under the 4 MiB budget.**
- [M1]: dashboard-only symbols now cfg'd: `harness::knowledge` (module), `bench::BenchMetrics`, `bench::bench_metrics`, `here::scaffold_project`, `assets::WebAssets`, `assets::web_asset`.
- [M2]: `features` = `web` only. `toolstats` is dependency-free and always built. `web/src/layout.ts` exports `layered(nodes, edges, "RIGHT"|"DOWN", nodeSep)`; dagre reports centres, so it subtracts half the node box and filters edges to known ids.
- [M2]: sizes — default **4 859 696 B**, minimal **3 109 496 B**, `web` gate 1 750 136 B, budget 4 194 304 B → **+665 392 B over**.

### Files touched
- [M0]: `verbs/omp.rs` (new), `main.rs`, `verbs/mod.rs`, `verbs/up.rs` — 532dea9
- [M0]: `assets/skills/branch-sync/**`, `assets/commands/sync-pr.md`, `.omp/commands/sync-pr.md`, `verbs/skill/deploy.rs` — bc0adc4
- [M0]: `verbs/harness/global.rs` — 3c8c008
- [M0]: `assets/skills/deep-research/SKILL.md`, `outputs/native-tooling-zig-rust*` — 97e906d
- [M0]: CHANGELOG/KNOWLEDGE/STATE/AGENTS/CLAUDE + planning tree — T5
- [M1]: `crates/cli/Cargo.toml` — fef75ea; `assets.rs`, `build.rs`, `here.rs`, `harness/bench.rs`, `harness/mod.rs` — 95a4a26; `scripts/size-report.sh` — 265c465

### Blockers/Concerns
- `8sync doctor`: `! docs: 17 stale path(s) / 2 oversized`. Pre-existing, owned by M3.
- Open question for M1: does `default = ["web","toolstats","marketplace"]` (CI ships full) actually deliver a *user-visible* win, or only a dev-build win? If gating alone does not shrink the shipped binary, M2's elimination work is the whole payload.

## Session Continuity

Stopped at: M0 closed and verified (`M0-VERIFICATION.md`).
Next: `/feature plan` for M1 — write `M1-CONTEXT.md` (AC on measured byte deltas) + `M1-01-PLAN.md`.
