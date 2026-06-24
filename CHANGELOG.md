# Changelog

Mọi thay đổi đáng kể của `8sync` ghi vào đây. Format theo [Keep a Changelog](https://keepachangelog.com),
versioning theo [SemVer](https://semver.org). **8sync rule:** mỗi PR cập nhật mục `Unreleased`.

## [Unreleased]

## [0.22.0] — 2026-06-24

### Added

- **`8sync harness audit` — code-backed doc-hygiene** (was prompt-only advice with zero code behind it).
  Scans committed docs (AGENTS.md/CLAUDE.md/README/CHANGELOG + `agents/*.md`) for **stale path references**
  (repo-relative paths in docs that no longer exist), **oversized docs** (>400 lines / >120-line force-load
  block), and **30-day churn hotspots** (history-awareness — docs near churned code are likeliest stale).
  Report-only: never auto-deletes (heuristic; illustrative paths flagged "review before editing"). Skips
  absolute / `~`-rooted / URL paths so the harness's own machine-generated refs don't false-positive.
  `8sync doctor` surfaces a one-line summary; `/gs` + the `gs` skill doc-hygiene step now run the audit
  instead of eyeballing.
- **`8sync doctor` AI-engine health check** — verifies the token-optimization stack is installed AND
  registered with omp ("luôn xài"): codegraph (local index) · codebase-memory-mcp (semantic graph) ·
  headroom (output compression). A missing or unregistered engine silently defeats STEP 0 token
  discipline, so doctor now flags it with the one-command fix (`8sync harness`).

### Fixed

- **codegraph STEP 0 verbs were wrong** in the force-load prefix, the subfolder-index block, and the
  KNOWLEDGE breadcrumb: they taught `codegraph search/deps/defs`, none of which exist. Corrected to the
  real CLI surface `codegraph query/callers/callees/impact` (verified against codegraph 0.9.6) so the
  agent's first explore call doesn't error out.
- **Duplicate always-on skill in the force-load list.** A stale/external `karpathy` dir alongside the
  canonical `karpathy-guidelines` (identical frontmatter `name`) made the skill appear twice — once in
  CORE, once in on-demand. `build_force_load` now dedups by frontmatter name, keeping the higher-ranked
  dir, so each logical skill is listed exactly once. Future-proof against any dir/name collision.
- **impeccable setup scripts couldn't run under 8sync's layout.** The bundled design skill referenced
  `.agents/skills/impeccable/scripts/*.mjs` (leading dot) but 8sync mirrors skills to `agents/skills/`
  (no dot). Fixed 28 references across SKILL.md + 4 reference docs → `agents/skills/`.

### Changed

- Managed `.gitignore` block now ignores `.gs/` (per-run worktree + `/gs stop` marker — machine-local).

## [0.21.0] — 2026-06-24

### Changed

- **`/gs` redesigned to right-size effort (fixes the post-`/gs` quality regression).** Eval +
  deep-research (`outputs/gs-eval-improve-research-brief.md`) found the drop was process
  over-engineering, not tokens (`harness bench`: ~8.5k upfront, 79% saved, KV-cache stable):
  the 93-line command forced a team + full Closeout on every task and `auto` "never asked".
  - **Right-size first** — classify trivial/small → **solo** (no team, no Closeout) · medium →
    solo + one verifier · large → full loop + roles + Closeout. A team is the exception you justify
    (Cognition/Anthropic: single-agent default; multi-agent only when it clears the bar).
  - **Solo-by-default delegation** — subagents only for parallel-independent / context-isolation /
    specialization; scoped objective + summary return (never free-form, never inline transcript).
  - **Autonomy confidence-gated** — strong `auto`, but a high-stakes hard-to-undo low-confidence call
    is now a blocker (Anthropic 2026: "agents learning when to ask"); prefer reversible, never compound.
  - **Doc-hygiene step** — detect stale paths / junk / superseded docs → fix or **delete** (no addition
    without the matching deletion); keep docs lean. Stale docs poison agent context.
  - **Codebase-history** — `git log/blame` + DECISIONS + cbm `detect_changes` before load-bearing edits.
  - **Leaner command** — `assets/commands/gs.md` 93 → 56 lines (lower constraint density → better
    instruction-following); full protocol stays in the `gs` skill (progressive disclosure).

## [0.20.1] — 2026-06-23

### Fixed

- **`/gs auto` actually runs unattended now.** Added an **Autonomy contract** to the `/gs` command +
  `gs` skill: in `auto`/L3 the agent NEVER calls `ask` or stops on ambiguity — it resolves unknowns by
  research (codegraph/cbm → `agents/*`/PLAYBOOKS → skills → `web_search`/`autoresearch`/`deep-research`),
  picks the boring/reversible option, logs it under a new `## Assumptions` section in `agents/STATE.md`,
  and proceeds. "Blocker" is tightened to ONLY missing credential / external approval / destructive-
  irreversible action; design choices, naming and scope are no longer stops. Note: a slash command
  cannot bypass omp's approval gate — keep `tools.approvalMode: yolo` (default) for true unattended runs.
- **`/gs` argument hint.** Added `argument-hint` frontmatter and front-loaded the description with
  `[auto | <goal> | status | next | stop]` so the autocomplete dropdown shows the modes when you type
  `/gs ` (omp renders per-argument hints only for built-ins; the description is what surfaces for
  file-based commands).
- **QA + test are now first-class gates in `/gs`.** Per-slice verify-gate explicitly runs tests + a QA
  pass and forbids skipping/weakening tests; added a mandatory **Closeout** step — full test suite +
  end-to-end QA + independent re-review against the Definition-of-Done + a handoff summary — that must
  pass before the loop reports "done". Never hands back unverified work.

### Added

- **Reference submodules `reference/gstack` + `reference/gsd-pi`** (git submodules, MIT) for studying
  the engineering-team + autonomous-loop patterns that informed `/gs`. Pointers are committed
  (reproducible) but the working trees are **deinitialized by default** so they never bloat the
  codegraph/cbm index (codegraph honors no exclude/ignore — populating them ballooned the index to
  ~3k files / 110 MB). Study on demand: `git submodule update --init reference/<name>`; re-shrink with
  `git submodule deinit -f reference/<name>`. `reference/` is also gitignored as a cbm-index guard.

## [0.20.0] — 2026-06-23

### Added

- **`/gs` — one-command autonomous engineering-team loop (omp slash command).** `/gs <goal>` plans +
  runs, bare `/gs` resumes, `/gs auto` runs unattended (L3), `/gs status|next|stop`. Drives the loop
  off `agents/STATE.md`: plan → delegate to specialist roles (`task` subagents / gstack roles if
  installed) → verify-gate → commit → record (KNOWLEDGE/PLAYBOOKS) → advance until Definition-of-Done
  or a blocker. Token-lean (codegraph + codebase-memory-mcp + headroom mandatory) and guardrailed
  (verify-gate before commit, worktree isolation + no push/PR at L3, hard-stop via `/gs stop`).
  Modeled on gsd-pi `/gsd auto`.
- **Deploy + team-sharing.** `8sync harness`/`init`/`up` write it to `~/.omp/agent/commands/gs.md`
  (global) and `<repo>/.omp/commands/gs.md` (committed → whole team gets `/gs`). New on-demand `gs`
  skill documents the protocol; `8sync harness up --timer` runs it 24/7.

## [0.19.0] — 2026-06-23

### Changed

- **Loop engineering v2 — Phase A (token & stable-prefix discipline).**
  - Force-load block (`inject.rs`) + master `00-force-load.md` split always-on skills into
    **CORE** (codegraph · karpathy · ponytail · 8sync-cli — đọc body upfront) và **SPECIALIST**
    (assp · impeccable · taste · image-routing — biết khả năng, đọc body khi task khớp /
    progressive disclosure). Thu nhỏ tập đọc-ngay; `impeccable` vẫn bắt buộc ngay khi có việc UI/design.
  - `headroom_compress` nâng từ khuyến nghị → **bắt buộc** cho output > ~50 dòng (STEP 0 + invariants).
  - KNOWLEDGE breadcrumb (`memory.rs`) bỏ timestamp `epoch:` volatile → byte-stable giữa các lần
    `harness` (thân thiện KV-cache, hết git churn). `now_stamp()` vẫn dùng cho tên file archive.
  - Plan + provenance: `outputs/harness-loop-engineering-v2-plan.md`.
- **Loop engineering v2 — Phase B (live memory & recitation).**
  - `agents/STATE.md` seeded as a structured **live plan** (Goal · DoD · Checklist · Current ·
    Next · Open-questions · Handoff) — recitation anchor (Manus todo.md pattern): read at session
    start, rewritten at each phase boundary to keep the plan in recent context.
  - Loop section (`00-force-load.md`) + generated block (`inject.rs`) gain **recitation**,
    **compaction** (near-limit → structured handoff to STATE + lessons to KNOWLEDGE → reinit, with
    `headroom_compress` as summarizer), and **budget-awareness** rules.
  - `harness bench` now counts the memory spine in the upfront budget (more honest accounting).
- **Loop engineering v2 — Phase C (maker/checker + Reflexion).**
  - Loop section + generated block: `task` implementer ↔ **independent verifier** (build/test in
    its own context, verify-gate before commit), explicit objective/boundaries/output per subagent,
    share-full-trace for dependent work, parallel only when subtasks are independent.
  - **Reflexion failure-capture**: a failed verify writes a `failure:` entry to KNOWLEDGE (symptom
    + cause + fix); recent failures are read at session start to avoid repeating them.
- **Loop engineering v2 — Phase D (procedural memory / playbooks).**
  - `agents/PLAYBOOKS.md` seeded (Voyager-style skill library): validated multi-step procedures
    distilled into reusable runbooks indexed by a `When:` line — retrieved + adapted, not re-derived.
  - Memory tiering: KNOWLEDGE = verbal lessons · PLAYBOOKS = verified procedures · DECISIONS = ADR.
    `harness bench` now counts PLAYBOOKS in the spine (6 files).
- **Loop engineering v2 — Phase E (phased autonomy + guardrails).**
  - L1 report · L2 assisted · L3 unattended defined, with guardrails (verify-gate before commit,
    gitleaks, commit scoped to `agents/`+docs, no auto `push`/PR at L3). `harness up --timer`
    per-tick job documented (read STATE → Next → verify → update spine → optional commit).

### Added

- **`8sync harness bench`** — deterministic loop-engineering benchmark (no model calls): upfront
  context budget (force-load prefix + CORE skill bodies) vs deferred (SPECIALIST + on-demand),
  the A2 progressive-disclosure saving, and an A1 KV-cache stable-prefix gate. Refactors a shared
  `inject::build_force_load()` (single source of truth for inject + bench). Baseline on this repo:
  upfront ~5.5k tok vs naive ~37.9k tok → **85% upfront cut**; A1 PASS.

## [0.18.1] — 2026-06-23

### Fixed

- **`8sync harness init` now pulls registered manifest skills** — `init` calls
  `skill update` against `agents/skills.toml` (git collections like `feynman`:
  deep-research, autoresearch, …) before mirroring, making it a true superset of
  bare `8sync harness`. Previously `init` only deployed the bundled skills + 2
  hardcoded external packs (ponytail, addyosmani), so manifest-only skills never
  reached `agents/skills/` via `init` — only bare `8sync harness` / `up --pull` did.

## [0.18.0] — 2026-06-21

### Added

- **Headroom context-compression wired as an omp MCP** — `8sync harness`/`init` auto-installs
  `headroom-ai[mcp]` (uv → pipx → pip fallback) and registers it in `~/.omp/agent/mcp.json`
  (`headroom mcp serve`, stdio). Tools `headroom_compress` / `headroom_retrieve` / `headroom_stats`
  compress long tool outputs / logs / diffs 60–95% before they reach the model. Force-injected into
  STEP 0 + `00-force-load.md`. Researched alongside PixelRAG + LocateAnything3D — **skipped**:
  PixelRAG (screenshot-RAG) overlaps `8sync shot`/`read`/`browser`; LocateAnything3D is a 3D-vision
  model (out of scope for a coding harness).

## [0.17.1] — 2026-06-21

### Fixed

- **Skills now propagate to other machines.** `8sync harness` / `skill update` write a
  committed project manifest `agents/skills.toml` (mirroring the machine-local registry) and
  read it back on any machine — so a fresh clone re-pulls the exact same skills. Previously only
  the machine-local `~/.config/8sync/skills.toml` recorded `skill add`-ed sources, so custom
  skills (e.g. git collections like feynman) never reached a second machine via harness — only
  the 15 binary-embedded skills + 2 hardcoded external packs did. (`agents/skills.toml` is a
  file, so it travels even when the `agents/skills/` directory is gitignored.)

## [0.17.0] — 2026-06-21

### Added

- **codebase-memory-mcp = first-class code-intelligence engine** — `8sync harness`/`init`
  auto-installs the binary (upstream installer, binary-only), sets `auto_index true`, and
  registers it as an omp MCP server in `~/.omp/agent/mcp.json` (idempotent, preserves other
  servers). `harness`/`up` index the repo. Mirrors `ensure_codegraph` — zero manual MCP config.
- **Code intelligence FIRST (STEP 0)** — the injected force-load block + `00-force-load.md`
  mandate codegraph + codebase-memory-mcp BEFORE grep/read for all code exploration
  (~99% token saving); raw `Read` only for read-before-edit.
- **Loop-engineering principles** (Addy Osmani / Boris Cherny) in `00-force-load.md`:
  STATE/KNOWLEDGE spine, maker/checker via `task` sub-agents, verify-gate, phased
  L1→L3 autonomy via `harness up --timer`.

## [0.16.0] — 2026-06-21

### Added

- **`8sync harness` (bare) = ONE command** — idempotent driver that makes a project
  agent-ready in a single pass: deploy/update skills + mirror (additive) + inject
  force-load + seed memory & gitleaks hook + consolidate learnings + re-index codegraph.
  `harness init` = explicit full bootstrap (progress UI); `harness up` = light refresh;
  `harness up --timer 30m` = background loop.
- **Additive skill mirror + `--force`** — `harness`/`harness init` never clobber an
  already-vendored (possibly edited) `agents/skills/<name>`; only missing skills are
  written. `harness init --force` re-mirrors everything. `harness up` now also seeds
  the gitleaks pre-commit hook.
- **`8sync skill update [name]`** — re-pull registered skills from their recorded
  source in `skills.toml` (git URL / `builtin:` / `path:`). Git sources are deduped
  per URL (a collection repo is cloned once, all sub-skills reinstalled); best-effort
  per source (offline / missing `git` warns + skips, exit 0). `name` updates just one.
- **`8sync harness up --pull`** — refresh AND re-pull every registered skill before
  re-injecting. Default `up` stays network-free + fast (timer/loop unaffected).
- **`8sync harness up --commit`** — close the self-learning loop: stage + `git commit`
  ONLY the refreshed agent memory (`agents/`, `AGENTS.md`, `CLAUDE.md`, `CHANGELOG.md`,
  `.gitignore`; never your code) so learnings persist to git in the same pass. No-op
  when nothing changed (no empty-commit spam on `--timer`); default off.
- **`8sync harness help`** — one-screen cheatsheet: commands, skill tiers, the
  commit-vs-ignore file taxonomy, and the new-machine runbook.
- **Portability**: `harness init`/`up` seed a managed `.gitignore` block (between
  `# >>> 8sync (managed) >>>` sentinels) — ignore derived (`.codegraph/`, `.cache/8sync/`)
  + secrets (`.env`, `.env.*`, keep `!.env.example`), keep agent memory + `agents/skills/`
  committed. `8sync doctor` now errors if any durable `agents/*.md` / `AGENTS.md` /
  `CHANGELOG.md` is gitignored (learnings wouldn't survive a move to a new machine).
- **`agents/KNOWLEDGE.md`** seeded with an append-only `## Learnings` zone below the
  managed breadcrumb block (overwritten each `harness up`) so learnings persist.

### Hardened (research-driven — see `outputs/harness-selfimprove-research-brief.md`)

- **Lean force-load context** — the injected on-demand skill list is now names+path
  only (one line each); full descriptions live in each `SKILL.md` (progressive
  disclosure). `8sync doctor` warns if the `AGENTS.md` force-load block exceeds 120
  lines. *Why:* Gloaguen et al. arXiv 2602.11988 (138 repos) — bloated/duplicative
  context files cut agent success and add >20% inference cost.
- **Skill version pinning (lockfile)** — `8sync skill add <url>@<ref>` pins a git
  commit/tag/branch; the resolved SHA is recorded as `rev` in `skills.toml` and
  `skill update` checks out exactly that rev (reproducible). Unpinned entries track
  latest. *Why:* mirrors Claude Code plugin marketplace (SHA pin = reproducible).
- **Secret-scanned auto-commit** — `harness up --commit` runs `gitleaks protect
  --staged` (if installed) and ABORTS on detection; `harness init` installs a
  gitleaks pre-commit hook (non-destructive); `8sync doctor` reports gitleaks.
  *Why:* GitGuardian 2026 — AI-assisted commits leak secrets ~2× baseline.
- **Bounded memory (anti context-rot)** — `harness up` consolidates the
  `## Learnings` zone past ~200 lines, archiving older entries to `agents/archive/`
  with a pointer. *Why:* 4-lever consolidation; "remember everything → remember nothing".
- **Verifier-gated learnings** — seeded `KNOWLEDGE.md` instructs prefixing entries
  `validated:` (test/build confirmed) vs `hypothesis:`. *Why:* Reflexion verifiability
  constraint — no reliable improvement beyond what's objectively verified.

## [0.15.1] — 2026-06-17

### Added

- **impeccable house design references** (`assets/skills/impeccable/references/house/`): bundled
  `frontend-agent-workflow.md` (senior coding-agent workflow) + `clouds-f.md` (senior front-end
  orchestration) + `clouds-f-rules/*.mdc` (design-redesign / responsive / performance / fix /
  refactor / security keyword routers). impeccable's SKILL.md auto-references them.

### Changed

- **Emphasised `impeccable` as THE house design system** across the force-load flow (AGENTS.md /
  CLAUDE.md block, `00-force-load.md`, sub-folder index, KNOWLEDGE breadcrumb): mandatory for any
  UI / design / redesign / audit, read with `references/house/*`.

## [0.15.0] — 2026-06-16

### Added

- **`8sync harness` verb** — one command to stand up the full agent harness.
  - `harness init`: deploy mọi bundled skill + codegraph binary + external skill
    packs (best-effort clone), mirror vào `agents/skills/`, `codegraph init`,
    seed `agents/*` memory + `CHANGELOG.md`, inject force-load vào AGENTS.md/CLAUDE.md
    + một index gọn vào **mọi sub-folder code** (progressive disclosure). Có progress
    UI `[i/N]` + thời gian.
  - `harness up`: refresh theo state hiện tại (re-inject + refresh `agents/KNOWLEDGE.md`
    breadcrumb + `codegraph index`). `--loop <dur>` chạy foreground; `--timer <dur>|off`
    cài/gỡ systemd **user timer** (đúng cách cho chạy nền, mirror `8sync clean --timer`).
- **6 bundled skill mới**: `ponytail` (always-on, lazy-senior YAGNI), `code-review-and-quality`,
  `senior-security`, `senior-frontend`, `full-flow`, `encore-deploy` (on-demand). Trước đó
  (0.14.x → nội bộ) đã thêm `assp-skill`, `impeccable`, `taste-skill`. Tổng **15 bundled**.
- **Always-on order** (đọc top-down, ưu tiên): codegraph → karpathy → ponytail → assp →
  impeccable → taste → 8sync-cli → image-routing. Inject block dạy rõ *cách tận dụng* từng skill.
- **Tech-gated skills**: `encore-deploy` chỉ hiện trong force-load block khi project dùng
  Encore (`encore.app` / `encore.dev`).
- **Opt-in skills**: `social-growth` (chiến dịch social/branding/lead-gen cho FB/YouTube/TikTok,
  page setup, insight, monthly plan + target) — KHÔNG auto-bật; bật bằng
  `8sync skill add builtin:social-growth`.
- **`8sync skill add` collection-aware**: clone repo rồi cài mọi `skills/<name>/SKILL.md`
  (vd `addyosmani/agent-skills` 24 skill, `ponytail` full); `builtin:<name>` deploy
  bundled skill từ embedded assets.
- **Sub-folder `AGENTS.md` index** + **`agents/KNOWLEDGE.md` breadcrumb** + **`CHANGELOG.md`**
  seeding tự động, để agent không bỏ sót rule và tự học theo state dự án.

### Changed

- **`8sync skill sync` → `8sync harness init`** (clean cutover, không giữ alias). `skill sync`
  in cảnh báo trỏ sang lệnh mới.
- `crates/cli/src/verbs/skill.rs` (~1340 dòng) tách thành module tree `verbs/skill/`
  (`mod` · `meta` · `discover` · `list` · `spec` · `add` · `gen` · `deploy` · `inject` · `index`),
  mỗi file < 500 dòng. Harness logic ở `verbs/harness/` (`mod` · `init` · `up` · `memory` · `external`).
- `8sync .` giờ cũng inject sub-folder index (nearest-AGENTS.md wins).
- Binary size target: < 4 MB (binary ~3.8 MB stripped, gồm 15 bundled skill).

## [0.14.2] — 2026-06-02

- fix(bt): Bluetooth vanishing after cold boot (USB autosuspend).

## [0.14.1] — 2026-05-31

- clean is project-safe: never touches models / Playwright / download caches.

## [0.14.0] — 2026-05-31

- `8sync clean`: disk/RAM reclaim + CPU/GPU report + periodic timer.

## [0.13.0] — 2026-05-31

- `8sync bt` bluetooth verb; Caelestia desktop install removed.

## [0.12.1] — 2026-05-30

- two-tier skill injection (always-on vs on-demand).
