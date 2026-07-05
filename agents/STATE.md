# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**/auto (Unreleased): bench-driven optimization — DONE, 4/4 tasks (`engine_status`).**
Review verdict: dashboard 15 pages × 0 console errors; project switcher propagates end-to-end
(state/bench/codegraph follow the designated project); A1 PASS; audit hits = historical false-positives.
Gap found & fixed: bench measured but didn't DRIVE optimization.
- `BenchMetrics` + `/api/bench` now expose `core_tok`/`spine_tok`/`naive_tok` + `spine_advice`
  (warn when spine >50% of upfront) — `bench.rs::spine_advice`.
- CLI `harness bench` prints the advisory (`!` line).
- Web Bench page: auto-loads on mount (was empty until manual click), upfront breakdown meters
  (prefix/CORE/spine share), advisory card, `.meter-val-wide` CSS. Browser-verified, 0 errors.
- Spine trim applied: STATE narrative archived → `agents/archive/STATE-1783219303.md`.
- Warning sweep: dead `assets.rs::web_asset_iter` removed · `LocalModel` pub(crate) · `_ctx`.

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
