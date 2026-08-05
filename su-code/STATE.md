# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)
> **Active feature:** none open. `lean-binary` shipped (see ✅ below); omp-17 ext fix shipped (see HANDOFF).

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — 2026-08-06 (cold resume from here)

**Repo state:** branch `main`, HEAD **`0b535e4`** `chore(extensions): redeploy 8sync-engine.ts with the omp-17 factory fix`. Tag **v0.52.0** (Cargo.toml still 0.52.0 — WIP checkpoint, **not** a release). Tracked tree **clean**. **20 commits ahead of `origin/main`, nothing pushed yet** (this `/push-now` fixes that).

**What changed THIS session (2 commits on top of the 18-commit lean-binary feature):**
- `bf885f7` **fix(extensions): omp 17 mutable-default ParseError** — omp 17.2.9 added a schema validator (`HF0` in `cli.js`) that rejects any zod `.default(<array/object literal>)`; it must be a **factory**. One site: `assets/extensions/8sync-engine.ts:146`, `verify: z.array(z.string()).default([])` → `.default(() => [])`. Every other default is a primitive (`false`/`0`/`""`/`3`) — untouched. `8sync-workflow.ts` has no defaults. The asset is rust-embedded, so this needs `cargo build` + `8sync harness` to redeploy; live copies refreshed in 3 projects. **Verified:** `omp -p "ok"` loads the extension with zero warnings.
- `0b535e4` — `8sync harness` re-stamped the project copy `.omp/extensions/8sync-engine.ts` from the rebuilt embedded asset (byte-identical to source).
- Cleaned **stray root memory files** (`STATE.md`/`KNOWLEDGE.md`/`skills/`(74)… at repo root): leftovers from a stale *release* binary that still had the detection bug — the PATH binary now has the fix (re-verified both directions). `CHANGELOG.md` + `su-code/KNOWLEDGE.md` updated.

**Done ✓**
- [x] lean-binary feature (18 commits, `589807e`→`b07e2c3`): binary −24 %, zero features removed (x86_64 6 407 848 → **4 859 696**; aarch64-musl stub → **4 151 328**; `--no-default-features` **3 109 496**). Details: `su-code/planning/lean-binary/M*-VERIFICATION.md`.
- [x] omp-17 extension ParseError fixed + verified (this session).

**Next / TODO ▸**
- [ ] **`--continue` history loss** (user-reported) — NOT caused by the extension (omp loads each ext in its own try/catch and continues; this ext registers tools only, no `session_start`). omp's changelog shows `--continue` has its own bug class (resume-into-subagent-transcript / session-resume-hang / auto-thinking-dropped — all "fixed" by 17.2.9, so a remaining loss is a **fresh regression**). **Action:** ask the user to retest `--continue` now that the ext loads clean; if it persists, diagnose the terminal breadcrumb + which session `--continue` picks vs `~/.omp/agent/sessions/<project-dir>/`.
- [ ] **Push/release the 20 local commits** so `8sync up` no longer reverts them — see gotcha ⚠ below.
- [ ] Ratchet the size ceiling down as headroom appears (`bash scripts/size-report.sh` → `scripts/size-gate.sh`).
- [ ] REQUIREMENTS v2: un-embed `impeccable/scripts` (1.6 MB) behind a lazy fetch — last big chunk of the 665 392 B still over goal.
- [ ] Real mac/Windows **runtime** verify needs the actual OSes (code compiles cross-platform; hasn't run on a live mac/Win).

**Blockers ⚠ (per-machine — NOT in git)**
- **`8sync up` reverts all local fixes.** It self-installs the latest GitHub release binary (v0.52.0), which **lacks** the detection fix (`b331832`) and the extension fix (`bf885f7`). Running `8sync up` reintroduces BOTH: stray root memory files return + the omp-17 extension breaks again. Until the 20 commits are **pushed + tagged**, do NOT run `8sync up`; the fixed binary is a local build (`cargo build --release`) installed via `cp target/release/8sync ~/.local/bin/8sync`. `which 8sync` → `~/.local/bin/8sync`.
- The fixed extension must be redeployed on each machine after building (it's rust-embedded): `8sync harness global` + `8sync harness` per project, or copy `assets/extensions/8sync-engine.ts` into `<proj>/.omp/extensions/` + `~/.omp/agent/extensions/`.

**New-machine runbook (ordered):**
1. `git pull` (or `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code`)
2. `bash scripts/bootstrap.sh` → builds from source (gets the fixes) + installs `~/.local/bin/8sync`. **Do NOT** then run `8sync up`.
3. `8sync setup` (new-machine harness: omp + codegraph + MCP/skills + gh + PATH)
4. `8sync harness` → redeploys the fixed extension from the embedded asset + MCP + skills + memory + inject + codegraph index
5. `gh auth login` (for `8sync ship` / release)
6. `8sync doctor` to verify
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
