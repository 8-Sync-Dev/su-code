---
gsd_state_version: '1.0'
feature: fedora-harness
ticket: ""
branch: ""
status: complete
active_phase: "M5"
next_action: none
next_phases: []
progress:
  total_phases: 6
  completed_phases: 6
  percent: 100
last_updated: "2026-08-09"
---

# State — Fedora-First Core + Enforced omp Harness

## Project Reference

See: su-code/planning/fedora-harness/PROJECT.md · ROADMAP: su-code/planning/fedora-harness/ROADMAP.md
**Core value:** Fedora-first, minimal, Rust-fast harness that makes omp *actually* use codegraph /
serena / codebase-memory / browser / skills — enforced in code, not requested in prose.

## Current Position

Phases M0 through M5 — **100% DONE & SHIPPED** in v0.54.0 / v0.54.1.
All distro-dispatch package operations, enforced tool routing, dynamic registries, asset minimization,
authoring commands, engine dedup, and CI gates are live.

| Component | State | Evidence |
|---|---|---|
| Fedora dev stack | **DONE** | `fedora-setup` all 10 modules OK (fcitx-unikey, docker, node 22.23.1, pnpm 11.20.0, uv 0.12.3, gh 2.97.0, vercel 58.7.1, bitwarden, warp, terminal) |
| su-code / `8sync` | **NOT INSTALLED** | `which 8sync` → not found |
| Rust toolchain | **ABSENT** | `cargo` not found — cannot build from source yet |
| omp skills | **EMPTY — root cause found** | `~/.omp/skills` empty because `8sync setup` aborts at step 3 of 8: `setup.rs:141-143` gates the `paru` step on `Os::Linux` (not Arch), it fails on Fedora, and `try_step`'s `?` propagates for a non-`yolo` run — so `codegraph`/`configs`/`skills` never execute |
| AGENTS.md rules | **STALE/DEAD** | root `AGENTS.md` + `CLAUDE.md` point at `/home/alexng/Projects/tools/su-code/...` — nonexistent here |

## Research completed (8 parallel scouts)

- **FedoraPortAudit** — zero Fedora support. ✎ *Corrected by plan review:* **23** package-manager
  call sites outside `pkg.rs` (platform ×1, setup ×8, profile ×5, vpn ×5, browser ×3, doctor ×1),
  not ~74/~117 — the larger figure counted all 169 textual occurrences of `pacman|paru|yay|makepkg`.
  `pkg.rs` = ✎ 209 lines of literal pacman, no trait. `platform.rs:93` param literally named
  `pacman`. `env_detect.rs:17-23` reads only `ID` (and Fedora 44 ships **no `ID_LIKE`**).
  `install.sh` + `selfup.rs` already distro-neutral. ✎ `vpn.rs:93-114` + `harness/browser.rs:51-57`
  are Arch-only verbs the first plan revision missed entirely.
- **SkillsRulesAudit** — 38 skill dirs, 2 438 KiB. `impeccable` = 1 864.6 KiB = **74.1 %** of all
  assets, of which **683.2 KiB is pre-bundled browser JS build output**. Only 20/38 auto-deploy;
  17 orphaned; `alpha-research` advertised but has no dir. Four contradictory bundled counts.
  ✎ *Corrected:* five call sites iterate assets dynamically, and the best template is
  `profile::load_all()` (`profile.rs:82-115`) — embedded enumeration **plus** a user-override dir
  with last-wins dedup — not `harness/eval.rs:37`.
- **HarnessOmpPatchAudit** — **root cause of the dead-path bug**: `skill/inject.rs:174`
  `let abs = p.join(entry)` → emitted at :177 (CORE) and :180 (SPECIALIST); the on-demand tier at
  :190-193 already builds **relative** strings, proving it is an oversight. Second site
  ✎ `index.rs:102`/`:121`. `harness audit` cannot catch it — ✎ `audit.rs:55-57` skips `/`-tokens.
- **PastProjectsGhAudit + direct gh check** — `8syncdev/8sync-startup` and
  `8-Sync-Dev/content-post-agency` exist and are **private** (the scout's 404 was unauthenticated).
  Every onboarded repo hardcodes another machine's `$HOME`: 8sync-startup & zus →
  `/home/alexdev/Projects/startup/…`, open-musik → `/home/alexdev/Projects/tools/…`,
  agentic-cloudgo-v1 → `/home/alexng/Projects/works/…`; comicforge has `su-code/` but **0**
  sentinel blocks. `agentic-cloudgo-v1` contains a file literally named `KNOWLEDGE.md:1-25`.
  Recorded incidents: `harness web` cross-project screenshot leak (content-post-agency);
  codegraph watch-timer OOM at 5.3 GB RSS on `zus`.
- **OmpDocsDistill** — 54 omp docs distilled into `su-code/omp-reference/` (8 files, 120 031 B),
  including `LEVERS.md` (26 levers, 412 table rows).
- **InstallerUpdateUX** — `omp.sh/install` is a plain **302 → raw.githubusercontent**. Releases API
  returns per-asset `digest: "sha256:…"` (free verification). Defects: auto-check blocks every
  command up to 5 s (`main.rs:145-148`), `up.rs:32-33` hardcodes `force=true` (~5 MB every run),
  non-atomic replace on Fedora (mktemp `/tmp` tmpfs → `~/.local/bin` btrfs = EXDEV), string-equality
  version compare, Windows-arm64 asset missing.
- **RustNativesTs7** — **No** third-party Rust native addon for omp: natives are internal-only
  (`user-facing-packages.md:10`, `natives-binding-contract.md:3`), loader accepts only omp's own
  version sentinel. TS 7 GA 2026-07-08 (8–12× faster) but its **API is `not ready`** until 7.1
  (~Oct 2026); Bun strips types, so su-code's shipped `.ts` are type-checked **nowhere** today.
- **GraphDagLoopResearch** — omp already ships the DAG layer, a genuine **graph** memory backend
  (mnemopi: SQLite vector+FTS, episodic graph, `proactiveLinking`, 4-voice `polyphonicRecall`), and
  ~10 loop/retry mechanisms. `8sync-workflow.ts` is a **100 % duplicate** → delete.
  `8sync-engine.ts` is ~70 % duplicate; genuinely additive = verify gate + gitleaks commit gate.
  `8sync-recall.ts` is the most valuable artifact — promote off legacy `HookAPI`.

## Plan review (config `plan_review: "complex"` → executed)

Adversarial review returned **REWORK** with 3 BLOCKER + 11 MAJOR + 7 MINOR findings. All three
blockers were independently re-verified against the code on this box and the plan was amended to
revision 2 before requesting approval:

| # | Blocker | Verified | Amendment |
|---|---|---|---|
| B1 | "Stage A already works on Fedora" was **false** — `setup.rs:141-143` gates `paru` on `Os::Linux`, `?` aborts, so `codegraph`/`configs`/`skills` never run | read `setup.rs:109-149` | M0-CONTEXT finding 2 rewritten; new task T5 gates the step on `Family::Arch`; AC-07 now asserts `~/.omp/skills` non-empty |
| B2 | `[packages.arch]` rename would **silently zero** existing user profiles (`profile.rs:52-63` all `#[serde(default)]`, no `deny_unknown_fields`, user-override dir at `:99-112`) | read `profile.rs:46-63` | D-M0-5: schema is **additive**; `pacman`/`aur`/`aur_yay` stay canonical; add `deny_unknown_fields` |
| B3 | `backend() -> Box<dyn PkgBackend>` had no arm for `Family::Other`, regressing Debian/openSUSE users who today get a clean warn-and-continue (`platform.rs:103-110`) | read `platform.rs` audit | D-M0-2: `-> Option<Box<dyn PkgBackend>>`; new AC-12 |

Other corrections folded in: dnf5 (`dnf5-plugins`, **not** `dnf-plugins-core`) verified live on this
box; Fedora 44 has **no `ID_LIKE`**; port surface 23 sites not ~74; `vpn.rs`/`harness/browser.rs`
added to the migration set; `--dry-run` hoist above `profile.rs:215-221`/`:237`; AC-01/02/06/09/10/11
rewritten because each could previously pass while the feature was broken; stale line numbers fixed
(`audit.rs:55-57`, `deploy.rs:1160-1199`, `index.rs:102`, `pkg.rs` 209 lines); T0 (toolchain + first
`#[cfg(test)]` module + CI `cargo test`) promoted from footnote to task; `skill` column restored to
the task tables.

## Decisions (locked by research, pending Gate 1)

- **D-1 — Enforce, don't ask.** TTSR rule + `tool_call` hook veto + `bashInterceptor` replace prose
  as the primary routing mechanism. Prose stays only as the cheap always-true layer.
- **D-2 — No native addon.** The Rust lever is the `8sync` binary; TS extensions stay thin.
- **D-3 — Inherit omp's DAG/graph/loop.** Keep only the verify gate + gitleaks commit gate.
- **D-4 — Distro dispatch, not a Fedora fork.** `trait PkgBackend`; Arch behavior unchanged.
- **D-5 — Dynamic asset registry** replaces four hardcoded lists; it is also what makes
  `/create-command` possible without a Rust edit.
- **D-6 — No new crates.** 383 184 B headroom (7.3 %).

## Verification

None yet — planning only. M0 AC table in `M0-CONTEXT.md`.

## Session Continuity

Stopped at: plan written, awaiting Gate 1 + Gate 2. Nothing built, nothing committed.
Next: approve → `/feature go` (M0), or amend the roadmap first.
