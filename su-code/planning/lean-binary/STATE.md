---
gsd_state_version: '1.0'
feature: lean-binary
ticket: ""
branch: ""
status: in-progress
active_phase: "M1"
next_action: plan-phase
next_phases: ["M1","M2","M3"]
progress:
  total_phases: 4
  completed_phases: 1
  percent: 25
last_updated: "2026-08-02"
---

# State — Lean Binary

## Project Reference

See: su-code/planning/lean-binary/PROJECT.md · ROADMAP: su-code/planning/lean-binary/ROADMAP.md
**Core value:** get `8sync` back under its own 4 MB budget without losing a single user-visible feature.
**Current focus:** M1 — feature gating + per-gate byte attribution.

## Current Position

Phase: M1 of 4 (Feature gating + attribution)
Plan: 0 of 0 (M1 not planned yet)
Status: in-progress
Vì sao phase này: you cannot delete a dep before you have measured what it actually costs; gates are the measuring instrument.
Last activity: 2026-08-02 — M0 closed, 4 commits replayed green, baseline 6 407 848 B recorded.

## Accumulated Context

### Decisions
- [M0]: commit deliverable-shaped, not file-shaped — `git log` stays a readable history of *what shipped*.
- [M0]: hand-roll the commits, keep the engine's verify-gate — `engine_advance {commit:true}` does `git add -A` and cannot be trusted to split a dirty tree.
- [M0]: AC-02 proven by worktree replay, not by N green `engine_verify` on one tree.
- [M1 pre]: gate, measure, *then* decide what to delete. No dep is removed on `cargo bloat` guesswork alone.

### Contract — phase sau CẦN BIẾT
- [M0]: baseline `6 407 848 B` at `97e906d`+docs; budget `4 194 304 B`; overshoot `2 213 544 B`.
- [M0]: heavy deps are single-module — `axum`/`tokio`/`tower-http` → `verbs/harness/web.rs`; `rusqlite` → `verbs/harness/toolstats.rs`; `scraper` → `verbs/harness/marketplace.rs`. Embeds: `assets::Assets` (`assets/`) and `assets::WebAssets` (`web/dist/`) + `assets::web_asset()`.
- [M0]: `crates/cli/Cargo.toml` has **no** `[features]` table — M1 creates it.

### Files touched
- [M0]: `verbs/omp.rs` (new), `main.rs`, `verbs/mod.rs`, `verbs/up.rs` — 532dea9
- [M0]: `assets/skills/branch-sync/**`, `assets/commands/sync-pr.md`, `.omp/commands/sync-pr.md`, `verbs/skill/deploy.rs` — bc0adc4
- [M0]: `verbs/harness/global.rs` — 3c8c008
- [M0]: `assets/skills/deep-research/SKILL.md`, `outputs/native-tooling-zig-rust*` — 97e906d
- [M0]: CHANGELOG/KNOWLEDGE/STATE/AGENTS/CLAUDE + planning tree — T5

### Blockers/Concerns
- `8sync doctor`: `! docs: 17 stale path(s) / 2 oversized`. Pre-existing, owned by M3.
- Open question for M1: does `default = ["web","toolstats","marketplace"]` (CI ships full) actually deliver a *user-visible* win, or only a dev-build win? If gating alone does not shrink the shipped binary, M2's elimination work is the whole payload.

## Session Continuity

Stopped at: M0 closed and verified (`M0-VERIFICATION.md`).
Next: `/feature plan` for M1 — write `M1-CONTEXT.md` (AC on measured byte deltas) + `M1-01-PLAN.md`.
