# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation mạnh kiểu gsd-pi = **`/auto`** (`8sync-engine`, 100% trên omp core: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read** (APPEND_SYSTEM + serena); terminal + web **glass**. (`/gs` = skill-backed lead, không còn là engine.)

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
Shipping **0.28.0**: executed `outputs/one-auto-unification-plan.md` — **P1–P5 DONE** (gỡ `/gs`; `/auto` là lệnh DUY NHẤT, gsd-pi-grade: research-in-plan · fresh-context · verify-gate · Closeout/UAT/re-review · Tauri/browser; `8sync harness model` config 1-nguồn; `harness up` deploy đủ; clean-cutover `cleanup_legacy_gs`). **P6**: build clean + bench A1 PASS → commit/tag/release. DEFERRED: full capability-scoring per-task router (TS engine).

## Next (chưa làm — tùy chọn)
- [x] **DONE: `outputs/one-auto-unification-plan.md` P1–P6** — `/auto` là lệnh tự động DUY NHẤT (v0.28.0).
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
