# STATE (8sync managed — live plan; rewrite at every phase boundary)

## Goal
Replace the retired `/auto` prompt + `engine_*` extension with one native `/gs`
agent-team loop on omp core: clarify → research → plan → independent plan review →
implement → verify → independent review/security → user UAT → closeout.

## HANDOFF — 2026-07-24 cold resume

### Repo state
- Branch: `feature/native-gs`.
- Migration checkpoint: `8240559 feat: replace legacy auto loop with native gs engine`.
- Final handoff checkpoint: run `git log -1 --oneline` after pulling; its SHA cannot
  self-reference from the file it commits. Subject: `all note check kĩ`.
- Latest release tag: `v0.52.0`; the native-GS work is not separately tagged here.
- Upstream after this handoff: `origin/feature/native-gs` (created with `git push -u`).
- Working tree after the handoff checkpoint: clean. Local Serena caches remain ignored by
  `.serena/.gitignore`.

### What changed in this session
- `assets/extensions/8sync-gs/`: added the durable native GS state/config/policy/machine/
  store/evidence engine, its omp extension tools, lifecycle hooks, and UI helpers.
- `assets/agents/gs-*.md`: added seven scoped research/planning/worker/review/security/
  verification agents with explicit tool and output contracts.
- `assets/configs/8sync/gs.json`: added conservative machine defaults and safety limits.
- `crates/cli/src/verbs/skill/deploy.rs`: deploys GS globally/project-locally, preserves
  user-owned files, and retires only byte-matched or sentinel-managed legacy assets.
- `crates/cli/src/verbs/harness/{init,global,up,auto,memory,model}.rs` and dispatch/docs:
  install, refresh, migrate, and expose native GS without the retired `/auto` loop.
- `crates/cli/src/{brand.rs,verbs/{root,flow,feature}.rs}` and bundled `/feature` assets:
  rename the old auto flow and feed approved feature phases into the single GS engine.
- `crates/cli/src/verbs/harness/web.rs`, `web/src/{App.tsx,api.ts}`: dashboard Engines
  page reads the real `.cache/8sync/gs/state.json` and preserves done/skipped/running/
  blocked states.
- `tests/gs/` plus Rust deploy/memory/web tests: cover state gates, leases, evidence,
  consent hashes, config monotonicity, safe cleanup, migration, and dashboard adapters.
- `README.md`, `CHANGELOG.md`, `su-code/{KNOWLEDGE,STATE}.md`: document the native engine,
  safety invariants, verification evidence, and this cold handoff.
- `.serena/{project.yml,.gitignore}`: committed project-local Serena configuration while
  excluding generated cache, language-server, local override, and memory directories.
- `outputs/raising-tool-adherence-research{,.provenance}.md`: preserved the source-backed
  research that motivated runtime tool-call steering and measurable adherence.
- `outputs/claude-desktop-linux-installer-state.md`: archived an unrelated completed root
  `STATE.md` so it no longer conflicts with the managed `su-code/STATE.md`.

### Done / next / blockers
- DONE: native GS implementation, independent correctness/security remediation, 192 GS
  tests, 23 Rust tests, release build, fresh isolated-home deploy/idempotency smoke,
  dashboard browser QA for idle and active boards, and `8sync harness audit`.
- DONE: local migration commit `8240559`; this handoff commit adds the remaining local
  Serena/research artifacts, provenance, changelog, durable learning, and cold runbook.
- NEXT: on the other machine, pull and bootstrap using the ordered runbook below.
- NEXT: run `/gs status` or open the dashboard Engines page only if continuing GS work.
  There is no open implementation task; release/tag only when explicitly requested.
- BLOCKERS: none. Cross-machine authentication and model/browser setup are machine-local.

### Per-machine reapplication
- omp frequently rewrites `~/.omp/agent/config.yml` during upgrades. Run
  `8sync harness global` or `8sync harness` to restore managed MCP visibility, hooks,
  compaction, skills, agents, GS extension, and APPEND_SYSTEM configuration.
- Restore the preferred two-model split with `8sync harness model=claude+glm`
  (direct Anthropic Opus for plan/design/review; Z.ai GLM for mechanical work).
- Chromium/Playwright on Arch/CachyOS: run `8sync harness browser` rather than relying on a
  broken package-manager browser shim.
- Feynman auth is machine-local: run `8sync feynman auth-omp`. If Feynman's bundled
  `node_modules/.bin/npm` or `npx` wrapper is broken, repair that wrapper before launching;
  the exact prior failure and recovery are recorded in `su-code/KNOWLEDGE.md`.
- Git identity may not be globally configured. This repo used local author
  `8 Sync Dev - Anh Tu Dev <tuan8165@gmail.com>` for the checkpoint.
- GS user config is seeded at `~/.config/8sync/gs.json`; project overrides belong in
  `.omp/gs.json` and may strengthen, never weaken, global safety requirements.

### Ordered new-machine runbook
1. `git pull`
2. `bash scripts/bootstrap.sh`
3. `8sync setup`
4. `8sync harness`
5. `8sync harness model=claude+glm`
6. `8sync harness browser`
7. `8sync feynman auth-omp`
8. `bun test tests/gs && cargo test`
9. `git status --short` — expect no tracked or untracked project files.

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
