# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**v0.43.0 ship (this session).** Bench-driven optimization (breakdown + `spine_advice` >50% upfront,
CLI `!` line + `/api/bench` + Bench page auto-load/meters/advisory — browser-verified 0 errors;
spine trim → advisory tự tắt sau consolidation, upfront 13.6k→12.0k tok). Plus:
- **`/codegraph?shot=1`** — canvas-only React-Flow graph capture cho `8sync shot` (~2k vision tok);
  auto-rule trong APPEND_SYSTEM + codegraph/image-routing/locate-anything skills: model không vision
  đọc vị trí/phân bổ ảnh → **tự động `8sync locate`** (LocateAnything-3B, CPU/GPU).
- **README + GitHub Pages 100% English-first** + 5 screenshot demo (docs/assets/dashboard-*.png,
  leak-checked — không lộ repo khác; web-session từng restore content-post-agency → phải activate
  su-code TRƯỚC khi capture).
- Release: tag v0.43.0 + gh release kèm binary `8sync-v0.43.0-linux-x86_64`.

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
