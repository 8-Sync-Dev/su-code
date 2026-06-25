<!-- 8sync:harness:begin -->
## 🧠 8sync harness

- **Always-on (đọc theo thứ tự; CORE đọc body ngay, SPECIALIST đọc khi task khớp):** codegraph → karpathy-guidelines → ponytail → assp-skill → impeccable → taste-skill → 8sync-cli → image-routing.
- **Cách tận dụng:** codegraph = explore code (query/callers/callees, không grep) · karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · impeccable = design CHUẨN, BẮT BUỘC khi UI/design (đọc body lúc đó) + taste chống slop.
- **Output lớn (>~50 dòng) → BẮT BUỘC `headroom_compress`** trước khi vào context.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (Unreleased) + ghi học được vào file này (prefix `validated:` nếu test/build xác nhận, `hypothesis:` nếu chưa).
<!-- 8sync:harness:end -->

# KNOWLEDGE (8sync managed — append-only)

## Learnings (append-only — ghi DƯỚI đây; KHÔNG sửa block `8sync:harness` ở trên)

- **skills.toml = update source-of-truth.** `skill::discover::read_registry` parses it
  (`toml` crate → `BTreeMap<String, SkillEntry { src, when }>`); `skill::update::update_skills`
  reinstalls per recorded `src`: git deduped by URL (clone once → reinstall all sub-skills),
  `builtin:` → embedded assets (`assets::install_tree`), `path:` → symlink. Best-effort per source.
- **`.gitignore` portability rule** (`harness::memory::seed_gitignore` via `upsert_block` sentinels):
  COMMIT learned/decided (`agents/*.md`, `AGENTS.md`, `CHANGELOG.md`, `agents/skills/`); IGNORE
  derived (`.codegraph/`, `.cache/8sync/`) + secrets (`.env*`, keep `.env.example`). Note: a
  trailing-slash pattern (`.codegraph/`) only matches once the dir exists — verify `git check-ignore`
  on a path INSIDE it, not the bare name.
- **KNOWLEDGE.md managed block** (`<!-- 8sync:harness:* -->`) is overwritten every `harness up`;
  durable learnings MUST live below it in the seeded `## Learnings` zone.
- **validated: `harness init` was NOT a superset of bare `harness`.** `init` (init.rs)
  only deployed bundled skills + 2 hardcoded external packs (ponytail, addyosmani) and
  never called `update_skills` — so manifest skills (feynman: deep-research, …) never
  reached `agents/skills/` via `init`. Only bare `8sync harness` (auto.rs:46) and
  `harness up --pull` read `agents/skills.toml`. Fix: init.rs now runs
  `update::update_skills(env, global_toml, None)` as step 5/9 before the mirror step.
  Verified: temp project + feynman manifest → `8sync harness init` produces
  `agents/skills/deep-research/SKILL.md` (all 20 feynman skills vendored).
- **validated: Phase A loop-eng v2 (token/prefix discipline) shipped.** (1) `inject.rs`
  +`always_on_core()` (codegraph/karpathy/ponytail/8sync-cli) → generated block renders CORE
  (read-now, numbered) vs SPECIALIST (read-on-trigger). (2) `headroom_compress` mandatory
  >~50 dòng ở STEP 0 + invariants + `00-force-load.md`. (3) `memory.rs` breadcrumb bỏ
  `now_stamp()` epoch → byte-stable. Verified /tmp: AGENTS.md có CORE(4)/SPECIALIST(4) +
  headroom bắt buộc; `harness init` ×2 → `git status` rỗng (prefix byte-identical = KV-cache win).
  Grounding: Manus KV-cache + Anthropic progressive-disclosure (outputs/harness-loop-engineering-v2-plan.provenance.md).
- **validated: `8sync harness bench` quantifies Phase A.** Deterministic (no model calls):
  reuses `inject::build_force_load()` (refactored as shared single-source) to measure upfront
  budget (force-load prefix + CORE bodies) vs deferred (SPECIALIST + on-demand), A2 saving, and
  an A1 stable-prefix gate (rebuild byte-identical). Baseline on THIS repo: upfront ~5,542 tok vs
  naive ~37,850 tok = **85% upfront cut**; deferred ~117k tok; SPECIALIST footprint 1971 KB
  (impeccable) no longer loaded each session; A1 PASS. token est = chars/4 (relative, not billing).
  Phase A applied to repo via `8sync harness up` (AGENTS.md → CORE/SPECIALIST, breadcrumb stable).
- **validated: Phase B loop-eng v2 (live memory & recitation) shipped.** (B1) `memory.rs`
  `STATE_TEMPLATE` → `agents/STATE.md` seeded as structured live plan (Goal/DoD/Checklist/Current/
  Next/Open-q/Handoff) = recitation anchor (Manus todo.md). (B2/B3) `00-force-load.md` loop section
  + `inject.rs` generated-block invariant gain recitation + compaction (near-limit handoff→reinit,
  `headroom_compress` as summarizer) + budget-awareness. `harness bench` extended to count the
  memory spine in upfront. Verified on this repo: upfront ~6,611 tok (prefix 1,871 + CORE 3,726 +
  spine 1,014), A2 saved 83% (abs 32,308 tok), A1 PASS; `harness up` reseeded structured STATE.md +
  injected Loop/STATE invariant. Grounding: Manus recitation + Anthropic compaction.
- **validated: Phase C/D/E loop-eng v2 (full) shipped → v0.19.0.** (C) loop section + generated
  block: `task` implementer↔independent verifier (verify-gate before commit, objective/boundaries/
  output per subagent, share-trace for dependent, parallel only independent); FAIL → `failure:` in
  KNOWLEDGE seed prefix, read first at session start. (D) `memory.rs` `PLAYBOOKS_TEMPLATE` →
  `agents/PLAYBOOKS.md` (Voyager procedural memory, index by `When:`); memory tiering KNOWLEDGE/
  PLAYBOOKS/DECISIONS; bench spine now 6 files. (E) L1→L3 + guardrails (no auto push/PR at L3);
  `up.rs` per-tick job documented. Verified: PLAYBOOKS.md seeded on `harness up`, generated AGENTS.md
  carries Loop-discipline invariant; final bench upfront ~7,095 tok, A2 saved 81% (abs 32,308 tok),
  A1 PASS. Grounding: Anthropic orchestrator + Cognition share-trace + Voyager + Reflexion.
- **validated: `/gs` one-command team loop shipped → v0.20.0.** New omp slash command
  `assets/commands/gs.md` (arg-routed: `<goal>` plan+run · bare resume · `auto` L3 · `status|next|stop`)
  driving the A–E loop off `agents/STATE.md`; token-lean (codegraph/cbm/headroom mandatory) +
  guardrails (verify-gate before commit, worktree + no push/PR at L3, hard-stop `.gs/STOP`). Modeled on
  gsd-pi `/gsd auto`. `deploy::ensure_gs_command(home, root?)` writes `~/.omp/agent/commands/gs.md`
  (global) + `<repo>/.omp/commands/gs.md` (team, committed); wired into harness auto/init/up. On-demand
  `gs` skill (bundled #15) documents protocol. **Key facts:** omp discovers commands at
  `~/.omp/agent/commands/*.md` + `<cwd>/.omp/commands/*.md` (`omp://slash-command-internals.md`),
  native precedence 100, body is a prompt template with `$ARGUMENTS`. Verified: `/gs` deploys both
  paths, valid frontmatter, gs skill on-demand (not in upfront); bench A1 PASS, upfront ~7,322 tok,
  A2 saved 81%. failure: gstack tool-backed roles (qa/ship) still need gstack `bin/` + deps installed.
- **validated: `/gs` autonomy + hint + QA + reference submodules → v0.20.1.** (1) `/gs auto` wasn't
  unattended because the agent kept calling `ask` — added an **Autonomy contract** (NEVER ask in
  `auto`; research → assume → log under `## Assumptions` in STATE → proceed; "blocker" = only
  credential/external-approval/destructive). omp default `tools.approvalMode: yolo` already auto-approves
  tools, so the stalls were `ask`/clarifying, NOT the approval gate — a slash command can't bypass that
  gate anyway. (2) Hint: omp shows per-arg hints only for BUILTINS; file commands surface only their
  `description` — so front-loaded modes into description + added `argument-hint` frontmatter (YAML must
  be quoted/clean: a value starting with `[` or containing `: ` breaks the parser). (3) QA/test made
  first-class: per-slice verify-gate runs tests+QA, plus a mandatory **Closeout** (full suite + e2e QA +
  independent re-review vs DoD + handoff summary) before reporting done. (4) Added `reference/gstack` +
  `reference/gsd-pi` submodules. **failure: codegraph honors NO exclude — not `.gitignore` (even
  `index -f`), no flag, no ignore-file; populating reference/ ballooned the index to ~3k files/110MB.**
  Fix: commit submodule pointers but `git submodule deinit -f` the working trees (lean by default,
  fetch on demand); cbm DOES respect `.gitignore` (it excludes `agents/skills`), so `reference/` is
  gitignored as a cbm guard. Verified bare `8sync harness` = full auto-setup (MCP + skills + `/gs` +
  memory + inject + index) in one command; bench A1 PASS, ~7.6k upfront, A2 80%.
- **validated: doc-hygiene + AI-engine health + loop correctness → v0.22.0.** (1) `harness/audit.rs`
  `harness_audit` + `stale_summary`: hand-rolled path scanner (no regex crate) flags STALE repo-relative
  doc paths, oversized docs (>400 lines / >120-line force-load block), 30d churn hotspots. **Heuristic
  rules that matter:** trim only TRAILING sentence punctuation (leading `.`/`/` are meaningful); SKIP
  absolute (`/home/…`), `~`-rooted/`<placeholder>`-derived `/…` fragments, URLs, and dotdir first-segs
  (`github.com`, `.cargo`) — else the harness's own machine-generated CORE paths false-positive. Wired
  into `doctor` (one-line summary) + `/gs` doc-hygiene. Verified: scratch repo flags only the planted
  `src/gone.rs`; su-code 30→4 after the skip rules. (2) `doctor::check_ai_engines(home)` enforces the
  token-optimization stack is installed AND registered in `~/.omp/agent/mcp.json`: codegraph 0.9.6 +
  codebase-memory-mcp 0.8.1 + headroom 0.27.0 (all green here). (3) **failure: codegraph STEP 0 verbs
  were wrong** — force-load/index/breadcrumb taught `codegraph search/deps/defs`, NONE exist; real CLI
  (0.9.6) = `query/callers/callees/impact/context/files/affected/sync/status`. Fixed all 3 strings.
  (4) **failure: a stale `~/.omp/skills/karpathy` dir beside canonical `karpathy-guidelines` (identical
  frontmatter `name`) double-listed the skill** (CORE + a redundant on-demand). `build_force_load` now
  dedups by frontmatter `name` after the rank-sort (keeps higher-ranked dir) — each logical skill once;
  future-proof. Note: bundled `assets/skills/karpathy/` deploys to target `karpathy-guidelines` via the
  explicit (asset,target) map in `deploy.rs`/`setup.rs` — dir name ≠ skill name is fine. (5) **failure:
  bundled `impeccable` referenced `.agents/skills/impeccable/scripts/*.mjs` (leading dot) but 8sync
  mirrors to `agents/skills/`** — its setup scripts couldn't run; fixed 28 refs across SKILL.md + 4
  reference docs. Note: headroom's router PROTECTS code/recent content (`router:protected:recent_code`)
  → won't compress small code samples (0 saved); it compresses genuine large logs.
- **validated: harness eval + concrete /gs worktree → v0.23.0.** `harness/eval.rs` `harness_eval` runs
  bundled `assets/eval/<name>/` fixtures through `omp -p --no-session --auto-approve --max-time 300`
  (cwd = a fresh `.cache/8sync/eval/<name>`), scores each with the fixture's `verify.sh` (verifier OWNS
  the assertion — agent can't game it), writes JSON scorecard + `--baseline` to the gitignored cache,
  diffs later runs. 3 fixtures: fix-failing-test / add-fn-with-test / locate-symbol. Verified 3/3 twice;
  baseline diff prints `3/3 → 3/3 (+0)`. **Key omp facts:** `-p` non-interactive, `--auto-approve` for
  headless tool use, `--max-time`, `--no-session` ephemeral; `omp worktree` manages ~/.omp/wt.
  `/gs` guardrail now spells out L3 worktree: `git worktree add .gs/wt/<slug> -b gs/<slug>` → work+verify
  +commit there → `git worktree remove`; `.gs/` is gitignored (v0.22.0). Verified worktree add/list/remove
  + `git check-ignore .gs/wt/slice`. **Phase 3b (gstack omp host) DEFERRED:** additive (roles fall back to
  bundled), and the host lives inside the deinitialized gstack submodule (foreign repo, pinned SHA) — not
  su-code's binary; out of proportion to value given the tool/skill-verification focus.
- **note: shell PATH pollution across bash calls.** A sandbox env in one bash/eval call can drop
  `~/.local/bin` from the persistent shell's PATH (codegraph/omp then "command not found" in a later
  call though the binary exists). Pass an explicit `env: { PATH: "/home/alexdev/.local/bin:/home/alexdev/.bun/bin:/usr/local/bin:/usr/bin:/bin", HOME, XDG_CONFIG_HOME }` for any call that invokes 8sync/omp/codegraph.
- **validated: declutter skill-registry (cắt feynman) + design lane.** Source-of-truth của skill set =
  **`agents/skills.toml` committed** (∪ machine-local `~/.config/8sync/skills.toml`); `8sync harness`
  re-pull từ đó (`update.rs:27-35`) và **git source reinstall MỌI sub-skill của collection**
  (`update.rs:49`) → cắt một phần một `src=<repo>` collection là vô ích; phải cắt HẾT entry chung URL.
  Đã bỏ 20 skill `companion-inc/feynman` khỏi cả 2 manifest + `rm` dir ở `~/.omp/skills/` +
  `agents/skills/` (repo này gitignore `agents/skills/` — `.gitignore:25` — nên đó là regen output;
  manifest mới là nguồn). `assets/configs/skills.toml` chỉ seed 4 builtin always-on (không feynman) → không
  mọc lại. Re-ran `8sync harness`: on-demand 55→35, feynman trong AGENTS.md = 0, force-load 1998→1717 tok,
  `harness bench` A1 PASS, `harness eval` 3/3 (vs baseline +0, không regression). Giữ addyosmani coding-eng
  + impeccable/taste/assp design payload.
- **note: chuẩn design UI/UX = impeccable (bundled always-on) + Lighthouse 4-gate (Perf/A11y/BP/SEO) +
  full-flow verify (browser ⨉ Encore trace).** Clouds F (`/home/alexdev/Documents/clouds-f`) là skill FE
  orchestration giàu hơn nhưng để **project-local** (không bundle vào su-code). Encode thành "UI/UX Design
  Lane" §4b trong `outputs/agent-team-workflow-automation-plan.md`.
