# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành một **super agent-team** token-optimal: omp = core, su-code = tools, học từ gstack + gsd-pi; điều khiển bằng **một lệnh `/gs`** chạy team tự động.

## Definition of Done
- [x] Loop-engineering v2 (Phase A–E) shipped + đo bằng `8sync harness bench`
- [x] `/gs` — một lệnh chạy team tự động; `auto` không dừng; hint `[auto|<goal>|status|next|stop]`; QA+Closeout gate
- [x] Bare `8sync harness` = auto-setup đầy đủ (MCP + skills + /gs + memory + inject + index)
- [x] **Doc-hygiene code-backed** (`8sync harness audit`: stale paths / oversized / churn) + wired vào `doctor` + `/gs` (v0.22.0)
- [x] **AI-engine health check** trong `doctor` — codegraph/cbm/headroom present + registered ("luôn xài") (v0.22.0)
- [x] **Loop correctness** — codegraph verbs đúng (query/callers/callees/impact) · force-load dedup theo frontmatter name · impeccable `.agents`→`agents` path fix (v0.22.0)
- [x] **Loop quality probe** `8sync harness eval` (omp task-suite + verify.sh + baseline diff) — verified 3/3 (v0.23.0)
- [x] **/gs L3 worktree isolation** cụ thể hoá: `git worktree add .gs/wt/<slug> -b gs/<slug>` (v0.23.0)

## Current step
**Executing plan `harness-web-and-recall-plan`** (3 phases). **Phase A DONE** (anti-forget): `ensure_omp_memory_config` (compaction@50% + idle) + `ensure_recall_hook` (`~/.omp/hooks/pre/8sync-recall.ts`, inject skill-index+STATE at agent-start + compaction summary) + doctor report. Key-based config detection (omp strips sentinels). Verified: omp 16.2.1 OK, doctor "anti-forget ON". Installed 8sync 0.24.0 (local, Unreleased). **Next: Phase B** (`8sync harness web` — axum+tokio + Vite FE embed) then **Phase C** (workspaces+team+submodules).

## Next (chưa làm — tùy chọn)
- [ ] **Phase 3b — gstack host `omp`** (DEFERRED, không regression): role `/qa`,`/ship` đã fallback bundled; host nằm TRONG submodule gstack (foreign repo, pinned SHA) — KHÔNG thuộc binary su-code. Chỉ làm khi muốn role tool-backed chạy thật qua gstack: `git submodule update --init reference/gstack` → đọc `docs/ADDING_A_HOST.md` → implement → `./setup --host omp` → deinit lại.
- [ ] (tùy) Chạy `8sync harness eval --baseline` định kỳ để theo dõi chất lượng loop qua thời gian (kết quả ở `.cache/8sync/eval/`, gitignored).
- [x] **(P2 — DONE) Mnemopi memory** wired vào `8sync harness`+`init` (`deploy.rs::ensure_mnemopi_memory`, idempotent sentinel-block, không clobber) + `doctor` báo ON/OFF; bật máy này (`~/.omp/agent/config.yml`, API-only `llmMode:smol`+`noEmbeddings`). omp 16.1.20 load OK.
- [ ] (tùy) Loại `reference/` khỏi codegraph (không honor exclude — xem failure trong KNOWLEDGE); tạm deinit.

## Open questions / blockers
_none._

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 treo bật bằng `/gs auto` + `8sync harness up --timer`.
- Reference submodules để deinit mặc định (ưu tiên token-lean hơn là luôn-có-sẵn nội dung).

## Handoff (đổi máy — làm theo thứ tự)
1. `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code`
2. `bash scripts/bootstrap.sh` (hoặc đã có 8sync thì `8sync up`) → build + cài `8sync` ≥ 0.23.0
3. `8sync harness` — auto-setup hết (MCP + skills + `/gs` + memory + index)
4. `gh auth login` (để `8sync ship` / release hoạt động)
5. Muốn đọc repo tham khảo: `git submodule update --init reference/gstack reference/gsd-pi` (xong thì `git submodule deinit -f reference/<name>` cho index gọn)
6. Mở omp → `/gs <mục tiêu>` để giao việc, `/gs auto` để chạy không dừng.
- Toàn bộ lịch sử quyết định + bài học: đọc `agents/KNOWLEDGE.md` (mục Learnings, đọc các entry `validated:`/`failure:` gần nhất trước).
- Kế hoạch gốc đầy đủ: `outputs/harness-loop-engineering-v2-plan.md`.
