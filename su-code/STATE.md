# STATE (8sync managed — live plan; rewrite at every phase boundary)

## Goal
Replace the retired `/auto` prompt + `engine_*` extension with one native `/gs`
agent-team loop on omp core: clarify → research → plan → independent plan review →
implement → verify → independent review/security → user UAT → closeout.

## Checklist
- [x] Implement durable GS state/config/policy/machine/store/evidence modules.
- [x] Register native `gs_*` tools, `/gs`, lifecycle hooks, and seven scoped agents.
- [x] Deploy GS globally + per project; seed config without clobbering user files.
- [x] Import legacy state conservatively and preserve a rollback copy.
- [x] Retire managed `/auto` + `8sync-engine.ts` copies safely.
- [x] Feed feature phases into GS and render live GS state in the dashboard.
- [x] Resolve independent correctness and security review findings.
- [x] Run GS/Rust tests, frontend/release builds, fresh deploy smoke, and browser QA.
- [x] Run documentation hygiene and update changelog/project knowledge.
- [x] Create one local checkpoint commit for the complete migration.

## Current
Native GS migration is complete and verified:
- `bun test tests/gs`: 192 passed.
- `cargo test`: 23 passed.
- `bun run build` and `cargo build --release`: passed.
- Fresh isolated HOME/project: global + local GS extensions and seven agents deployed;
  legacy managed files absent; second install byte-identical.
- Browser QA: idle board and active fixture both rendered; done/skipped/running/blocked
  counts and current-task projection matched `/api/engine`.
- Correctness/security remediations enforce stage gates, exact evidence/leases,
  one-shot command-hash consent, monotonic safety, path containment, fail-closed state,
  and provenance-safe legacy cleanup.
- `8sync harness audit`: reviewed 18 heuristic stale paths. Hits are historical
  changelog entries, external-document references, existing relative paths, or the
  unrelated untracked root `STATE.md`; no live migration doc path is stale.

## Next
Release/tag/push only when explicitly requested. Keep unrelated local artifacts
(`STATE.md`, `.serena/`, `outputs/*`) outside the GS migration commit.

## Open questions / blockers
- None.
