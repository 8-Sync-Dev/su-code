# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)
> **Active feature:** none open. `lean-binary` shipped (see ✅ below); omp-17 ext fix shipped (see HANDOFF).

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — 2026-08-06 (cold resume from here)

**Repo state:** branch `main`. `origin/main` = `50462db` (**IN SYNC** — prior `/push-now` pushed all 20 commits incl. the omp-17 ext fix `bf885f7` + the 18-commit lean-binary feature). Tag **v0.52.0** (Cargo.toml still 0.52.0; **not** bumped — WIP checkpoint). This commit adds **STEP-0 tool-routing enforcement** (1 new commit on top of `50462db`). Tracked tree clean after this push.

**What changed THIS session (STEP-0 enforcement — "the rule is now code, not prose"):**
Measured problem (`8sync harness toolstats`): the STEP-0 MCP stack was connected AND callable (probed live: `xd://mcp__codebase_memory_mcp_list_projects` returned 5 indexed projects, `xd://mcp__headroom_compress` returned a hash) yet **UNUSED** — `cbm 0 · serena 0 · headroom 0` agent calls; every code lookup fell to the built-in `read`/`grep`. Prose directives in `APPEND_SYSTEM.md`/AGENTS.md lost 3×. **Lesson:** a zero-friction built-in always beats an instruction; if a rule keeps losing, DELETE the thing it competes with. So:
- `crates/cli/src/models.rs` — new `STEP0_TOOLS` allowlist (drops `grep`+`glob`, keeps `lsp`), emitted via the shared `--tools` flag at the `omp_flags()`/`resume_flags()` chokepoint (covers both `8sync ai` + `8sync .`). **Safe because** `--tools` filters BUILT-INS only — verified by capturing a real provider request (`omp -p ""` 400s and logs the full body to `~/.omp/logs/http-400-requests/`): under `--tools=read,bash,todo` the request still carried **48 `mcp__*` tools + `engine_*` + `wf_state_*`**. Dropping grep/glob costs 0 MCP.
- `crates/cli/src/verbs/skill/deploy.rs` — new `ensure_bash_interceptor()` writes `bashInterceptor.patterns` into `~/.omp/agent/config.yml` (omp shape `{pattern, reason}`; verified against omp's own `explicitExclusions` schema + the runtime `Blocked by bash pattern: ${match}`). Blocks `\brg\b` + recursive `grep`; **single-file / log grep stays allowed**. Closes the shell-escape hatch the `--tools` allowlist leaves open. Idempotent, never clobbers a user `bashInterceptor:` key.
- `crates/cli/src/verbs/harness/global.rs` + `init.rs` — wired `ensure_bash_interceptor` into both deploy paths (machine-wide global + per-project init).
- `crates/cli/src/verbs/ai.rs` — `--no-step0` escape flag threaded through to model launch.
- `assets/configs/omp/APPEND_SYSTEM.md` — RULE #0 rewritten to match reality: states grep/glob are GONE, gives a cheapest-first routing decision tree (cbm→serena→codegraph), and **DROPS the stale omp-16 `mcp.discoveryDefaultServers` claim** (omp ≥17 mounts MCP as `xd://` devices — that key no longer exists).
- `assets/configs/models.toml` — added `step0 = true` toggle + doc comment (default ON; `--no-step0` for one run, `step0 = false` in file).
- `AGENTS.md` / `CLAUDE.md` — sentinel-block re-injected by the harness refresh (STEP-0 mandate now carries the routing tree).
- `CHANGELOG.md` + `su-code/KNOWLEDGE.md` — documented enforcement + the full omp enforcement-surface map (allowlist / bashInterceptor / hooks / advisor) + the reusable "**a rejected request still logs its full tool array**" trick.

**Done ✓**
- [x] STEP-0 enforcement shipped + deployed + verified (this session): allowlist embedded in binary ✓; omp re-wrote config.yml (accepted bashInterceptor schema) ✓; `omp models --json` loads clean ✓; doctor all green ✓.
- [x] omp-17 extension ParseError fixed (`bf885f7`, prior session, now on origin).
- [x] lean-binary feature (18 commits, on origin).

**Next / TODO ▸**
- [ ] **Verify optimizer ratio shifts (forward-looking):** baseline `toolstats` = 66.7 % optimizer (codegraph 6, cbm/serena 0, grep 3). After a few NEW sessions under enforcement, re-run `8sync harness toolstats`; expect `grep`→0 and cbm/serena>0. **Cannot be measured in the session that created the enforcement.**
- [ ] **Verify bashInterceptor blocks at runtime (the one unproven link):** I confirmed omp ACCEPTS the schema + rewrites config.yml (proving parse success) but did NOT run a live session that triggers a block. Run `8sync ai "find where foo is used"` in a code project; confirm `rg`/`grep -r` gets blocked with the reason string. If it over-blocks, edit `~/.omp/agent/config.yml` `bashInterceptor.patterns` or delete the key.
- [ ] **`--continue` history loss** (user-reported, still open) — NOT caused by the ext (loads in its own try/catch). Fresh omp-17 regression. Action: ask user to retest `--continue` now that the ext loads clean; if it persists, inspect the terminal breadcrumb + which session `--continue` picks vs `~/.omp/agent/sessions/<project-dir>/`.
- [ ] **Tag a release** when ready (stops `8sync up` from reverting — see blocker). `/push-now` does NOT tag.
- [ ] Ratchet the size ceiling down as headroom appears (`bash scripts/size-report.sh` → `scripts/size-gate.sh`).
- [ ] REQUIREMENTS v2: un-embed `impeccable/scripts` (1.6 MB) behind a lazy fetch — last big chunk of the 665 392 B still over goal.
- [ ] Real mac/Windows **runtime** verify needs the actual OSes.

**Blockers ⚠ (per-machine — NOT in git)**
- **`8sync up` still reverts local fixes** until a new tag is cut: it pulls the latest *release* (v0.52.0), which lacks the ext fix (`bf885f7`) AND the STEP-0 enforcement (this commit). Until a new version is tagged + pushed, `8sync up` reintroduces both. Fixed binary = local `cargo build --release` → `cp target/release/8sync ~/.local/bin/8sync`. `which 8sync` → `~/.local/bin/8sync`.
- **bashInterceptor is per-machine config** (`~/.omp/agent/config.yml`): each machine must re-run `8sync harness global` to write it. omp's own `omp update` also rewrites config.yml to minimal defaults (see KNOWLEDGE gotcha at line ~226), dropping it — re-run `8sync harness global`.
- **Fixed extension is rust-embedded:** must rebuild + `8sync harness` per machine to redeploy `8sync-engine.ts` (or copy `assets/extensions/8sync-engine.ts` into `<proj>/.omp/extensions/` + `~/.omp/agent/extensions/`).

**New-machine runbook (ordered):**
1. `git pull`
2. `bash scripts/bootstrap.sh` → builds from source (gets the ext fix + STEP-0 enforcement) + installs `~/.local/bin/8sync`. **Do NOT** then run `8sync up` (reverts to the v0.52.0 release).
3. `8sync setup` (omp + codegraph + MCP/skills + gh + PATH)
4. `8sync harness` → redeploys the fixed extension + MCP + bashInterceptor + skills + memory + inject + codegraph index
5. `8sync doctor` to verify
- Decisions + lessons: `su-code/KNOWLEDGE.md` (+ `su-code/archive/`).

## ✅ SHIPPED — `lean-binary` feature complete (2026-08-02)
**How, in order (details: `su-code/planning/lean-binary/M*-VERIFICATION.md`):**
1. **M0** — landed 5 pending deliverables (`8sync omp update` verb · `branch-sync` skill + `/sync-pr` · `harness global` auto-stamp · `deep-research` §5 native-audit protocol + the binary brief).
2. **M1** — first-ever `[features]` table; A/B'd every gate with `scripts/size-report.sh`. `cargo bloat` under-attributed SQLite **~26×**.
3. **M2** — deleted what the data pointed at: `rusqlite` (**−1 035 384 B**; the DB stored nothing — ingest opened with `DELETE FROM calls`) and `elkjs` → dagre (**−512 768 B**; 85 % of the FE chunk was a GWT-compiled Java layout engine). Output byte-identical under frozen input; dashboard verified headless. **Bug fixed mid-flight (`b331832`):** a directory merely *named* `su-code` made its parent look like an omp project → blank memory + 74 skills into repo root. `discover::is_omp_project` now requires real memory content. Note `discover::MEMORY_DIR` = `"su-code"`, **not** `brand::NS` (= `"8sync"`).
4. **M3** — `cross` → `cargo-zigbuild` for aarch64 (Docker leg had no JS toolchain → embedded the **stub dashboard**), plus `scripts/size-gate.sh` enforcing a 5 MiB ceiling / 4 MiB goal per asset. `universal2` rejected (doubles every Mac download).

**Size table (final):** x86_64 default **4 859 696** (+15.87 % vs 4 MiB goal) · aarch64-musl **4 151 328** (−1.02 %) · `--no-default-features` **3 109 496** (−25.86 %) · `web` gate cost 1 750 136 B. Remaining overshoot 665 392 B owned by `assets/` (impeccable 2.1 MB) + dashboard — no cheap owner left.

**Prior shipped:** omp-17 MCP fix + Lark (`589807e`) · STEP-0 MCP fix omp-16 (`64bd650`) · `/push-now`+`/pull-now` (`c402209`, `6bb38ae`) · v0.52.0 (`8sync vpn`).

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 bật bằng `/auto` + `8sync harness up --timer`.
- Reference submodules để deinit mặc định (token-lean hơn luôn-có-sẵn).
- Spine advisory threshold = spine >50% upfront (relative, không absolute floor).
