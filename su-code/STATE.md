# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**v0.46.0 ship (this session).** Renamed the agent-memory folder `agents/` → **`su-code/`** as a distinctive project marker.
- **Marker + detection:** `is_omp_project` (sweep) + project-root detection now key on `su-code/` (`agents/` = legacy migration trigger only). Sweep = `8sync harness global --sweep [DIR]`.
- **Auto-migration** (`memory::migrate_legacy_layout`, in here/init/up/bare/sweep): renames `agents/`→`su-code/` + rewrites refs in anchor + live memory md. Guarded on real 8sync memory files (source pkg named `agents/` untouched); `.agents/`/`subagents/` protected; idempotent.
- All code/assets/skills/docs/recall-hook → `su-code/` (recall hook keeps `agents/` fallback). Historical CHANGELOG left as-written.
- Dogfood: this repo migrated (`git mv agents su-code`, `.gitignore` skill-mirror rule updated). E2E: legacy migrated + guard skipped non-memory dir + build clean.
- Release: tag v0.46.0 + gh release with binary `8sync-v0.46.0-linux-x86_64`.

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
- Lịch sử quyết định + bài học: `su-code/KNOWLEDGE.md` (+ `su-code/archive/`).
