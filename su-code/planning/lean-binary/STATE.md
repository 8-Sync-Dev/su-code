---
gsd_state_version: '1.0'
feature: lean-binary
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
last_updated: "2026-08-02"
---

# State — Lean Binary

## Project Reference

See: su-code/planning/lean-binary/PROJECT.md · ROADMAP: su-code/planning/lean-binary/ROADMAP.md
**Core value:** get `8sync` back toward its 4 MB budget without losing a single user-visible feature.
**Current focus:** none — all four phases closed.

## Current Position

Phase: M3 of 4 (CI + budget truth) — **DONE**
Plan: 4 of 4 phases complete
Status: complete
Vì sao phase này: the budget is now enforced in CI and the docs state measured reality, so the feature's contract is met.
Last activity: 2026-08-02 — M3 closed; `cargo-zigbuild` replaces the Docker leg, size gate live.

## Outcome

| build | before | after |
|---|---:|---:|
| x86_64 default | 6 407 848 | **4 859 696** (−24.2 %) |
| aarch64-musl | (stub dashboard, unmeasured) | **4 151 328** (under the 4 MiB goal) |
| `--no-default-features` | n/a (no features existed) | **3 109 496** |

No feature was removed. Two dependencies were deleted (`rusqlite`, `elkjs`), one
was fixed (`cross` → `cargo-zigbuild`), and the budget became a gate.

## Accumulated Context

### Decisions
- [M0]: commit deliverable-shaped, not file-shaped; hand-roll commits because `engine_advance {commit:true}` does `git add -A`.
- [M1]: gate to MEASURE, not to diet — `cargo bloat` may rank suspects, only an A/B may state a number (it missed SQLite ~26×).
- [M2]: delete, don't gate; prove a store swap under frozen input; ship a UI swap only after browser proof.
- [M3]: a budget must be an enforced gate with a ceiling ABOVE current size (ratchet down), never an aspirational comment.
- [M3]: `universal2` REJECTED — a fat binary doubles every Mac user's download, reversing the M0 brief.

### Contract — what the next session needs
- `features` = `web` only (axum + tokio + tower-http + scraper + `WebAssets` + the `build.rs` Vite step). `toolstats` is dependency-free and always built.
- `web/src/layout.ts` → `layered(nodes, edges, "RIGHT"|"DOWN", nodeSep)`; dagre reports centres so it subtracts half the node box and filters edges to known ids.
- `discover::MEMORY_DIR` = `"su-code"` — **not** `brand::NS` (which is `"8sync"`). `discover::is_omp_project` is the single project test.
- `scripts/size-report.sh` attributes; `scripts/size-gate.sh` enforces (ceiling 5 242 880, goal 4 194 304) and runs per asset in `release.yml`.
- Remaining overshoot vs goal: 665 392 B on x86_64, owned by `assets/` (impeccable 2.1 MB) and the dashboard. Un-embedding `impeccable/scripts` with a lazy fetch is REQUIREMENTS v2, not started.

### Files touched
- [M0]: `verbs/omp.rs`, `main.rs`, `verbs/mod.rs`, `verbs/up.rs` — 532dea9 · `assets/skills/branch-sync/**`, `sync-pr.md`, `skill/deploy.rs` — bc0adc4 · `harness/global.rs` — 3c8c008 · `deep-research/SKILL.md`, `outputs/**` — 97e906d · docs — ebeccb8
- [M1]: `crates/cli/Cargo.toml` — fef75ea · `assets.rs`, `build.rs`, `here.rs`, `harness/bench.rs`, `harness/mod.rs` — 95a4a26 · `scripts/size-report.sh` — 265c465 · docs — 002c095
- [M2]: `harness/toolstats.rs` — 5c1af33 · `Cargo.toml`+`Cargo.lock`+`harness/mod.rs` — 1bc58a8 · `web/src/layout.ts`+`web/src/App.tsx`+`web/package.json` — 32540a2 · docs — dc74245 · `skill/discover.rs`+`harness/global.rs` — b331832 · docs — faf9b57
- [M3]: `.github/workflows/release.yml`, `scripts/size-gate.sh` — a83e679 · docs — this commit

### Blockers/Concerns
- `harness audit` still reports 16 stale paths / 2 oversized. All reviewed: historical `CHANGELOG` entries, omp's own docs, and source-layout-relative paths. Not defects — see `M3-VERIFICATION.md`.

## Session Continuity

Stopped at: feature complete, tree clean, 16 commits on `main`, nothing pushed.
Next: `/feature ship` to archive, or ratchet the size ceiling down after the v2 asset work.
