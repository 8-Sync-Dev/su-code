# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)
> **Active feature:** none — `multi-session` SHIPPED in v0.53.0. Next large work: `ai-router-hub` M1 (in monorepo `8sync-startup`, blocked on B3 creds).

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — 2026-08-08 (RELEASED v0.53.0)

**Repo state (su-code):** branch `main`, **RELEASED tag `v0.53.0`** @ `8379798` (Cargo.toml+lock bumped 0.52.0→0.53.0). Tree clean, `origin/main` == HEAD, tag pushed → Release CI built + published 5 platform assets (linux x86_64/aarch64-musl, darwin x86_64/arm64, windows-msvc). `8sync up` now SAFE — it reinstalls v0.53.0 which carries every Luồng-A fix + the new session layer.

**Repo state (monorepo `8sync-startup`):** branch `main` @ `ddfff79` — KHÔNG đụng tới session này; code `ai-router-hub` nằm ở `deploy/ai-router-hub/backend-go/` nhưng **gitignored** (`/deploy/*`), nên nó **sống trên disk máy này thôi** → xem "backend-go-snapshot" bên dưới.

**Session này làm 2 luồng song song — cả hai ở repo `su-code`:**

### Luồng A — 8sync binary fixes (self-update + serena) · TRẠNG THÁI: xong, chờ release tag
`git diff --stat` (files touched + WHY, 1 dòng mỗi cái):
- `crates/cli/src/verbs/selfup.rs` (+158) — `8sync up` từng hard-code `ASSET_SUFFIX="-linux-x86_64"` → trên Windows tải nhầm binary Linux, ghi file không `.exe`, `rename` đè lên `.exe` đang chạy (Windows cấm) → popup "Select an app to open '8sync'". Fix: `asset_label()` map theo `platform::os()`+`ARCH` (macOS arm = `arm64`), install vào `std::env::current_exe()`, Windows-safe replace (rename `.exe` sống → `.8sync.old.<pid>` rồi trượt bản mới vào).
- `crates/cli/src/verbs/skill/deploy.rs` (+11) — `ensure_serena_mcp` giờ register serena `--enable-web-dashboard False` (serena mặc định mở 1 tab dashboard + bind HTTP mỗi session → 16 proc/878 MB); flag truyền qua **command line** (serena tự rewrite `serena_config.yml` nên sửa file không sống). Kèm step0 tool-routing + bashInterceptor deploy.
- `AGENTS.md`, `CLAUDE.md` (−mỗi cái ~30 net) — re-inject sentinel block (RULE#0 step0 + serena note); managed block, đừng sửa tay ngoài sentinel.
- `CHANGELOG.md` (+14) — 2 mục `[Unreleased] Fixed`: selfup cross-platform + serena dashboard off. (Đã có sẵn, không cần thêm.)

### Luồng B — sản phẩm `ai-router-hub` M0 · ĐÃ DỜI sang monorepo (canonical)
- `ai-router-hub` là product của cụm `8sync-startup` → toàn bộ memory (planning + code snapshot + _RESTORE + feature STATE) dời về **monorepo** `~/Projects/startup/8sync-startup/su-code/planning/ai-router-hub/` (commit `be18d7e`), cạnh mind0-brain-go / news-admin / zus. Repo `tools/su-code` này chỉ cho binary 8sync — KHÔNG giữ product memory nữa.
- Trạng thái: **M0 DONE** (Go-level, podman vet/build/test PASS, review READY); **M1 BLOCKED** trên B3 credentials. Tiếp ở monorepo: đọc `8sync-startup/su-code/STATE.md` (khối "Parked: ai-router-hub") → restore theo `backend-go-snapshot/_RESTORE.md`.

**Done ✓**
- [x] Luồng A: selfup cross-platform (native Linux build clean; nhánh `#[cfg(windows)]` type-check OK), serena dashboard off (no listener :24282), step0/interceptor deploy.
- [x] Luồng B: M0 scaffold authored + Go-verified (5/5 engine task), CI gate PASS, independent review = READY, `M0-VERIFICATION.md` ghi.
- [x] **multi-session feature (v0.53.0):** `8sync .` named sessions (new/ls/mv/rm/merge) + `--worktree` isolation + git-shell-out merge engine. 4 phases M0–M3, all smoke-verified (`multi-session/M3-VERIFICATION.md`), zero new deps.
- [x] **RELEASED v0.53.0** — bumped Cargo.toml+lock, tagged, pushed; Release CI publishing 5-platform assets. `8sync up` safe again.

**Next / TODO ▸**
- [ ] **M1 (ai-router-hub)** — đã dời sang monorepo `8sync-startup`; tiếp ở đó (restore snapshot + `/feature plan` M1). Cần B3 credentials (Postgres + provider account + CLIProxyAPI host).
- [ ] **Máy có Docker:** `encore run`/`encore test` backend-go + kiểm **Risk #1** (`Response.Result interface{}` — Encore schema parser có thể reject; fix 1 dòng). Xem `M0-VERIFICATION.md`.
- [ ] `8sync harness toolstats` sau vài session enforcement: kỳ vọng `grep`→0, `cbm`/`serena`>0 (baseline 66.7% optimizer).

**Blockers ⚠**
- **M1 = true blocker (per-machine, NOT in git):** cần (a) Postgres, (b) ≥1 account provider (Claude/Gemini/Codex subscription) để onboard, (c) host chạy CLIProxyAPI binary. Credentials/data ngoài tầm agent → `/auto` dừng đúng luật.
- ~~`8sync up` reverts Luồng A~~ **RESOLVED by v0.53.0** — `8sync up` now reinstalls v0.53.0 (all fixes + session layer). Máy mới vẫn build từ source (`bootstrap.sh`) cho HEAD chưa release, else `8sync up` OK.
- **bashInterceptor/serena config per-machine** (`~/.omp/agent/config.yml`, `mcp.json`): mỗi máy cần `8sync harness global`; `omp update` có thể rewrite config → re-run.
- **`codegraph callers` FALSE NEGATIVE** — dùng `mcp__serena_find_referencing_symbols`. **Đừng `rm -rf .codegraph`** → `codegraph index --force` hoặc `8sync harness`.

**New-machine runbook (ordered):**
1. `git pull` (repo su-code)
2. `bash scripts/bootstrap.sh` → build từ source (lấy mọi fix Luồng A) + install `~/.local/bin/8sync`. **KHÔNG** `8sync up`.
3. `8sync setup` (omp + codegraph + MCP/skills + gh + PATH)
4. `8sync harness` → redeploy extension + bashInterceptor + serena(no-dashboard) + skills + memory + inject + codegraph index
5. `8sync doctor` — phải thấy `✓ STEP-0 allowlist matches omp's tool list`
6. Tiếp `ai-router-hub`: restore code theo `su-code/planning/ai-router-hub/backend-go-snapshot/_RESTORE.md`, rồi đọc `su-code/planning/ai-router-hub/STATE.md`.
- Decisions + lessons: `su-code/KNOWLEDGE.md` (+ `su-code/archive/`).


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
