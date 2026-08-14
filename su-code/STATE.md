# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)
> **Active feature:** none — **v0.57.0 released** (STEP-0 deny-list; `8sync up` is safe again). Next large work: `ai-router-hub` M1 (in monorepo `8sync-startup`, blocked on B3 creds).

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — 2026-08-14 (STEP-0 deny-list: `8sync .` could not launch omp)

**Repo state (su-code):** branch `main`, still 0.56.0, **101 tests green**, size gate OK
(4 859 696 B — under the 5 MiB ceiling, over the 4 MiB goal as always). Binary rebuilt and
installed to `~/.local/bin/8sync`; `8sync harness` re-run on this box.

**Shipped this session — the launcher was dead, not the sessions**
- `8sync . <name>` / `8sync ai` had been exiting instantly with
  `CliUsageError: Unknown tools in --tools: ast_grep, github, checkpoint, rewind, security_scan`.
  STEP-0 drove omp with `--tools`, an ALLOWLIST, so 8sync mirrored omp's whole built-in set;
  omp 17.3 renamed/dropped 5 of those names. Because omp died before drawing a frame, the user
  fell back to a bare `omp --continue` — omp's DEFAULT per-cwd store, not the named session's —
  and the named session looked lost. Nothing was ever lost.
- STEP-0 is now a deny-list: `models.rs` writes `~/.config/8sync/omp-step0.yml`
  (`grep.enabled: false`, `glob.enabled: false`) and passes `--config <that file>`. Names only
  what must go, so no omp release can brick a launch. `STEP0_TOOLS`, `omp_valid_tools()` and
  `step0_tool_drift()` are gone.
- `8sync doctor` now probes ENFORCEMENT (`omp --tools grep,glob` must be rejected under the
  overlay) instead of comparing a constant against a list that is not even stable per version.
- A named session prints `omp --session-dir … --continue` on launch, so the other lane is
  reachable by hand.
- Registered the 4 foundation skills v0.56.0 forgot (`tauri-v2`, `nextjs-app`, `encore-eino-go`,
  `ai-microservice-design`) in `BUNDLED_SKILLS` — embedded + in AGENTS.md but never deployed.

**Verified live (not inferred)**
- `8sync . core` in `~/Projects/startup/8sync-startup` → omp v17.3.2 TUI up, no usage error.
- Scratch project: create → turn → the jsonl lands in the NAMED store and NOT in omp's default
  store; re-`8sync . <name>` resumes the SAME file and the model recalled the earlier word.
- `8sync ai "…"` one-shot clean; `8sync doctor` → "STEP-0 in force: omp rejects grep/glob".
- Isolation is sound: `--session-dir <empty dir> --continue` starts fresh, never leaks into the
  default store.

**Done ✓**
- [x] STEP-0 deny-list (`models.rs`, `doctor.rs`, `assets/configs/models.toml`) — 101 tests green.
- [x] Named-session store hint (`session.rs`).
- [x] 4 foundation skills registered in `BUNDLED_SKILLS` (`skill/deploy.rs`) + deployed here.
- [x] **`sx-` commands are now machine-wide.** `~/.omp/agent/commands/` = 10 `sx-*`, 0 unprefixed.
      `8sync harness global --sweep` stamped **10 omp projects**; the 6 repos still holding
      pre-prefix `auto/feature/pull-now/push-now/sync-pr.md` (defensible-cv, auto-work-cloudgo,
      agentic-cloudgo-v1, agentic-cloudgo-gitlab, box-work, 8sync-startup) are clean.
      `defensible-cv/.omp/commands/omp-update.md` intentionally survives — user-authored, and the
      deletion gate is content-based, so it is never eaten.
- [x] **RELEASED v0.57.0** — `Cargo.toml`+lock bumped, CHANGELOG cut, tag `v0.57.0` pushed;
      Release CI publishes the 5 platform assets, so `8sync up` now carries the fix.

**Next / TODO ▸**
- [ ] `8sync harness audit` — doctor reports 9 stale doc paths / 2 oversized.
- [ ] **M1 (ai-router-hub)** — in monorepo `8sync-startup`; needs B3 credentials.

**Blockers ⚠**
- **Any machine still on ≤v0.56.0 with omp ≥17.3 cannot launch `8sync .` at all** (the `--tools`
  usage error). Cured by `8sync up` now that v0.57.0 is tagged; before upgrading, the escape
  hatch on such a box is `8sync ai --no-step0`.
- M1 needs Postgres + a provider account + a CLIProxyAPI host — outside agent reach.

**Per-machine (NOT in git) — re-apply on the other box**
- `8sync harness global --sweep` is REQUIRED per machine: `~/.omp/agent/commands/`,
  `~/.omp/skills/`, `APPEND_SYSTEM.md`, MCP registrations and the per-project `.omp/` layers all
  live in `~`, not in the repo. Without it the other box still shows the pre-prefix `/push-now`
  and no `/sx-*`. This is exactly what bit this session.
- `~/.config/8sync/omp-step0.yml` is written on demand by the binary — nothing to copy.
- The 3440x1440 panel is capped at **100 Hz** because the RTX 5080 is on **nouveau**.
  `8sync setup --profile nvidia` installs RPM Fusion `akmod-nvidia`; Secure Boot is **disabled**,
  so no MOK enrolment is needed. After a reboot, `8sync hz max` should offer 180 Hz. Not run —
  driver swaps are the user's call.
- `~/.omp/agent/models.yml` holds a plaintext agentrouter API key. Machine-local, not in git,
  but rotate it if that file was ever shared.
- Lessons: `su-code/KNOWLEDGE.md` §"STEP-0 must be a deny-list, not an allowlist (2026-08-14)".

**New-machine runbook (ordered):**
1. `git pull`
2. `8sync up` (v0.57.0+) — or `bash scripts/bootstrap.sh` when HEAD is ahead of the last tag.
3. `8sync setup`
4. `8sync harness global --sweep` — global rules + `sx-` commands + per-project layers.
5. `cd <repo> && 8sync harness` — full pass incl. codegraph index for the repo you work in.
6. `8sync doctor` — expect `✓ STEP-0 in force: omp rejects grep/glob`.

## Prior sessions — still-live facts only
- **`ai-router-hub` moved out.** Product memory lives in the monorepo
  `~/Projects/startup/8sync-startup/su-code/planning/ai-router-hub/` (commit `be18d7e`); this repo
  is the 8sync binary only. **M0 DONE** (Go vet/build/test PASS, review READY), **M1 BLOCKED** on
  credentials. Resume there: `8sync-startup/su-code/STATE.md` → `backend-go-snapshot/_RESTORE.md`.
- **Shipped and released:** v0.53.0 named sessions (`new/ls/mv/rm/merge` + `--worktree`), v0.54.1
  cross-platform `8sync up` (`selfup.rs::asset_label`), serena registered with
  `--enable-web-dashboard False` (default cost 16 proc / 878 MB), v0.56.0 `8sync hz` + `8sync lcd`.
- **`codegraph callers` gives FALSE NEGATIVES** — use `mcp__serena_find_referencing_symbols`.
  Never `rm -rf .codegraph`; re-index with `codegraph index --force` or `8sync harness`.
- **`omp update` can rewrite `~/.omp/agent/config.yml`** (bashInterceptor, MCP) — re-run
  `8sync harness global` after updating omp.
- **Docker box still owed:** `encore run`/`encore test` on ai-router-hub backend-go, checking
  Risk #1 (`Response.Result interface{}` may be rejected by Encore's schema parser) —
  see `M0-VERIFICATION.md`.
- `8sync harness toolstats` after a few enforced sessions: expect `grep`→0, `cbm`/`serena`>0
  (baseline 66.7% optimizer).

## ✅ SHIPPED — `lean-binary` feature (2026-08-02)
1. **M0** — landed 5 pending deliverables (`8sync omp update` verb · `branch-sync` skill + `/sync-pr` · `harness global` auto-stamp · `deep-research` §5 + binary brief).
2. **M1** — first `[features]` table; A/B'd every gate with `scripts/size-report.sh`. `cargo bloat` under-attributed SQLite **~26×**.
3. **M2** — deleted what the data pointed at: `rusqlite` (**−1 035 384 B**; the DB stored nothing — ingest opened with `DELETE FROM calls`) and `elkjs` → dagre (**−512 768 B**). Output byte-identical under frozen input. **Bug fixed mid-flight (`b331832`):** a directory merely *named* `su-code` made its parent look like a project → blank memory + 74 skills in the repo root. `discover::MEMORY_DIR` = `"su-code"`, **not** `brand::NS` (= `"8sync"`).
4. **M3** — `cross` → `cargo-zigbuild` for aarch64 (the Docker leg had no JS toolchain → embedded the **stub dashboard**), plus `scripts/size-gate.sh` (5 MiB ceiling / 4 MiB goal). `universal2` rejected.

**Size table:** x86_64 default **4 859 696** (+15.87 % vs goal) · aarch64-musl **4 151 328** (−1.02 %) · `--no-default-features` **3 109 496** (−25.86 %).

**Prior shipped:** omp-17 MCP fix + Lark (`589807e`) · STEP-0 MCP fix omp-16 (`64bd650`) · `/push-now`+`/pull-now` (`c402209`, `6bb38ae`) · v0.52.0 (`8sync vpn`).

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 bật bằng `/auto` + `8sync harness up --timer`.
- **prompt-optimizer (linshenkx) evaluated and NOT integrated** — it optimizes human-authored prose, which is the layer that already failed 3× here; it is AGPL-3.0 vs su-code MIT (no vendoring), and a 5th MCP server adds catalog pressure against omp's ~40-tool discovery cutoff. Use the hosted web app for authoring skill/system-prompt text if wanted.
- **eino (CloudWeGo) evaluated and NOT adopted** — omp is already the agent runtime; replacing it forfeits MCP xd:// devices, skills, sessions, extensions. eino would only make sense for a future Go sidecar exposing a deterministic pipeline as an MCP tool, a niche codegraph + cbm already fill.
- Division of labour that the evidence supports: **LLM does judgement, Rust does enforcement** (allowlist, interceptor, drift guard) — deterministic levers the model cannot talk its way around.
