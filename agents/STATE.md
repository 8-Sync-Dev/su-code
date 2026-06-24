# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành một **super agent-team** token-optimal: omp = core, su-code = tools, học từ gstack + gsd-pi; điều khiển bằng **một lệnh `/gs`** chạy team tự động.

## Definition of Done
- [x] Loop-engineering v2 (Phase A–E) shipped + đo bằng `8sync harness bench`
- [x] `/gs` — một lệnh chạy team tự động (plan→delegate→verify→commit→advance off `agents/STATE.md`)
- [x] `/gs auto` chạy không dừng (Autonomy contract: không hỏi, research→assume→làm); `/gs stop` để dừng
- [x] `/gs [tab]` hiện hint `[auto | <goal> | status | next | stop]`
- [x] QA + test là gate bắt buộc + Closeout review trước khi bàn giao
- [x] Submodule tham khảo `reference/gstack` + `reference/gsd-pi` (deinit để giữ index lean)
- [x] Bare `8sync harness` = auto-setup đầy đủ (MCP + skills + /gs + memory + inject + index)

## Current step
Shipped **v0.20.1** (HEAD `c7ae2a0`). Working tree CLEAN, đã push origin/main + tag. Sẵn sàng đổi máy.

## Next (chưa làm — tùy chọn)
- [ ] Thêm **host `omp` cho gstack** (1 file TS theo `reference/gstack` docs/ADDING_A_HOST.md) → `./setup --host omp` để role tool-backed (`/qa`, `/ship`, browser) chạy thật trong omp; hiện `/gs` dùng role bundled nên vẫn chạy được không cần gstack.
- [ ] (tùy) Tự động hoá **git-worktree isolation** cho `/gs auto` L3 (hiện ở mức protocol trong `gs.md`).
- [ ] (tùy) Tìm cách loại `reference/` khỏi codegraph (codegraph KHÔNG honor exclude/gitignore — xem failure trong KNOWLEDGE); tạm thời deinit.

## Open questions / blockers
_none._

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 treo bật bằng `/gs auto` + `8sync harness up --timer`.
- Reference submodules để deinit mặc định (ưu tiên token-lean hơn là luôn-có-sẵn nội dung).

## Handoff (đổi máy — làm theo thứ tự)
1. `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code`
2. `bash scripts/bootstrap.sh` (hoặc đã có 8sync thì `8sync up`) → build + cài `8sync` ≥ 0.20.1
3. `8sync harness` — auto-setup hết (MCP + skills + `/gs` + memory + index)
4. `gh auth login` (để `8sync ship` / release hoạt động)
5. Muốn đọc repo tham khảo: `git submodule update --init reference/gstack reference/gsd-pi` (xong thì `git submodule deinit -f reference/<name>` cho index gọn)
6. Mở omp → `/gs <mục tiêu>` để giao việc, `/gs auto` để chạy không dừng.
- Toàn bộ lịch sử quyết định + bài học: đọc `agents/KNOWLEDGE.md` (mục Learnings, đọc các entry `validated:`/`failure:` gần nhất trước).
- Kế hoạch gốc đầy đủ: `outputs/harness-loop-engineering-v2-plan.md`.
