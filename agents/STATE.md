# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**v0.45.0 ship (this session).** MCP `server.json` standard conformance + the spec as a machine-wide default + `/auto` review.
- **Tool conformance:** `official_install` (marketplace.rs) projects registry `server.json` → `mcp.json` per schema
  `2025-12-11`: `registryType`→runtime (npm/pypi/oci/nuget), version pin, runtime/packageArguments, transport,
  **env/headers as `{NAME:value}` maps** (was a broken array). Threaded env/headers end-to-end (web.rs/api.ts/App.tsx).
  E2E on live registry via UI (docker apithreshold + pypi armor-mcp), 13 unit tests (incl scoped-npm pin regression).
- **Spec = machine default + AI-forced:** `assets/specs/mcp-server.md` bundled → `ensure_mcp_spec` deploys to
  `~/.omp/specs/` (global/init/up) + short rule in APPEND_SYSTEM points every omp session at it.
- **`/auto` reviewed:** functional-tested (verify-gate/doom-loop/refuse-unverified all pass); fixed gap — added a
  gitleaks gate before `engine_advance {commit:true}` so unattended runs can't leak secrets when the hook is absent.
- Release: tag v0.45.0 + gh release with binary `8sync-v0.45.0-linux-x86_64`.

## Next (chưa làm — tùy chọn)
- [ ] Phase 3b — gstack host `omp` (DEFERRED; xem archive + `reference/gstack` docs/ADDING_A_HOST.md).
- [ ] (tùy) `8sync harness eval --baseline` định kỳ (kết quả `.cache/8sync/eval/`, gitignored).
- [ ] (tùy) Loại `reference/` khỏi codegraph (không honor exclude — failure trong KNOWLEDGE archive); tạm deinit.

## Open questions / blockers
_none._

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 bật bằng `/auto` + `8sync harness up --timer`.
- Reference submodules để deinit mặc định (token-lean hơn luôn-có-sẵn).
- Spine advisory threshold = spine >50% upfront (relative, không absolute floor).

## Handoff (đổi máy — làm theo thứ tự)
1. `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code`
2. `bash scripts/bootstrap.sh` (hoặc `8sync up`) → build + cài `8sync`
3. `8sync harness` — auto-setup (MCP + skills + memory + inject + index)
4. `gh auth login` (cho `8sync ship` / release)
5. Mở omp → `/auto <mục tiêu>` để chạy engine tự động.
- Lịch sử quyết định + bài học: `agents/KNOWLEDGE.md` (+ `agents/archive/`).
