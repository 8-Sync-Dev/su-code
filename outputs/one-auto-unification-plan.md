# Plan — Unify to ONE `/auto`, gsd-pi-grade autonomous team (token-lean)

**Date:** 2026-06-29 · **Status tracker** (tick boxes as done; this file = source of truth, committed → survives machine switch). Mirror the current step into `agents/STATE.md`.

## North star
**Một lệnh duy nhất trong omp: `/auto [prompt]`** = cả team kỹ thuật tự động: research → plan → slices → tasks → verify từng task → QA/UAT → re-review → handoff. omp = core, su-code = tools. Token-lean (codegraph/serena/cbm/headroom ưu tiên hơn tool omp). Model theo **một** yml + fallback model đã auth. Prompt thường vẫn bám rules/skills/mem + ponytail (qua `APPEND_SYSTEM.md`).

## Grounding — học từ ref repos (đã đọc submodule thật)
- **gsd-pi** (`reference/gsd-pi/docs/user-docs/`): `auto-mode.md` — 1 entry `/gsd auto`, state-machine **Plan (research tích hợp) → Execute per-task (FRESH session/unit) → Complete → Reassess → UAT → Validate-Milestone**; `verification_commands` chạy cơ học sau mỗi task + auto-fix retry; context-pressure wrap @70%; git isolation none/worktree/branch; crash-recovery; cost tracking. `dynamic-model-routing.md` — tier light/standard/heavy + capability scoring, **downgrade-only** (model config = trần), `escalate_on_failure`, `budget_pressure`, cross-provider; config ở `PREFERENCES.md`.
- **gstack** (`reference/gstack/`): 23 role slash-command (CEO/eng/designer/reviewer/QA/CSO/release) = nguồn "team roles" cho việc delegate.
- Bài học chính cần bê về: **research-in-plan**, **fresh-context-per-task**, **mechanical verify gate**, **downgrade-only model routing**, **UAT/validate trước closeout**, **context-pressure wrap**.

## Hiện trạng (từ lần check trước) — 5 gap
1. Đang có **2 lệnh** `/gs` + `/auto` (rối).
2. Model: `models.toml` chỉ steer khi 8sync **launch** omp; trong session/`/auto` không áp per-task → **2 nguồn config** (8sync `models.toml` ≠ omp `config.yml`).
3. `/auto` **thiếu research-first** (feynman/deep-research) trước plan.
4. Thiếu quy ước verify UI: web→browser tool; **Tauri v2 → web-debug port → browser tool**.
5. `harness up` không deploy `APPEND_SYSTEM`/`/auto`/engine (chỉ bare `harness`/`init`).

---

## Phases (checklist tiến độ)

### Phase 1 — Hợp nhất về 1 lệnh `/auto`
- [ ] Gỡ `/gs`: xoá `assets/commands/gs.md`, skill `assets/skills/gs/`, `deploy::ensure_gs_command` + mọi call ở `auto.rs`/`init.rs`/`up.rs`, mục `/gs` trong help/flow/force-load/KNOWLEDGE breadcrumb.
- [ ] `/auto [prompt|status|resume]` là entry **duy nhất**; bare prompt nhỏ → làm thẳng (right-size, ponytail), không engine ceremony.
- [ ] Help/flow/`00-force-load.md`/`APPEND_SYSTEM.md` chỉ còn nhắc `/auto`.

### Phase 2 — `/auto` flow đạt chuẩn gsd-pi
- [ ] **Plan có research tích hợp**: trước khi slice → scout codebase (codegraph/cbm/serena) + research (feynman/`deep-research`/`autoresearch`/`last30days`/web_search). Ghi giả định vào STATE `## Assumptions`.
- [ ] Slices → tasks; **mỗi task fresh context** (task subagent / engine unit) chống context-bloat.
- [ ] **Verify-gate cơ học mỗi task** (`engine_verify`, code-enforced; chạy lint/test/build thật, auto-fix retry, BLOCK khi hết retry) — đã có, xác nhận + nối `verification_commands` kiểu gsd-pi.
- [ ] **Closeout bắt buộc**: full test suite + QA/UAT (browser) + independent re-review vs Definition-of-Done + handoff summary → port từ gs sang `/auto`.
- [ ] **Context-pressure wrap** ở ngưỡng (nối `compaction.thresholdPercent` 50%): ghi handoff vào STATE trước khi đầy.

### Phase 3 — Model: 1 nguồn + routing thông minh
- [ ] Chốt **một nguồn config model** (đề xuất: `~/.config/8sync/models.toml` làm gốc; mirror sang omp `config.yml` `modelRoles` để in-session dùng đúng) + lệnh **`8sync harness model`** (giống gsd-pi `/gsd config`) để xem/sửa.
- [ ] `/auto` chọn model **per-task-class** (task subagent model override theo class) — áp tinh thần gsd-pi: **downgrade-only** (config = trần) + `escalate_on_failure`.
- [ ] **Fallback model đã auth**: dựa omp `retry.modelFallback` (default ON) + (tùy) `8sync doctor`/`harness model` cảnh báo model cấu hình chưa login → gợi ý model đã auth.

### Phase 4 — Ưu tiên tool token + quy ước verify UI
- [ ] Xác nhận `/auto` + `APPEND_SYSTEM.md` ép **codegraph/serena/cbm/headroom 100% trước** grep/Read/native (RULE #0 đã có — kiểm + nhắc trong `/auto`).
- [ ] Thêm quy ước: **web → `browser` tool của omp; Tauri v2 desktop → bật dev web-debug port (WRY/WebKit) → trỏ chính `browser` tool vào port đó** (ghi vào `/auto` + `APPEND_SYSTEM`).

### Phase 5 — Deploy nhất quán (bare `8sync harness` = auto hết)
- [ ] `harness up` cũng deploy `APPEND_SYSTEM` + `/auto` + engine (hiện chỉ bare/init) → mọi lối đều đủ.
- [ ] Verify bare `8sync harness` = full auto-setup 1 lệnh (MCP + skills + `/auto` + memory + APPEND_SYSTEM + index).

### Phase 6 — Verify + ship
- [ ] `8sync harness bench` (token: A1 PASS, upfront lean) + `8sync harness eval` (quality) xanh.
- [ ] Build + tag + release; cập nhật `CHANGELOG.md` + `agents/KNOWLEDGE.md` + `agents/STATE.md` + tick plan này.

---

## Quy tắc tracking
- File này = nguồn sự thật tiến độ; tick `[x]` khi xong, commit mỗi mốc.
- `agents/STATE.md` §Next trỏ về file này + phase đang làm.
- Ref repos: `git submodule update --init reference/<name>` khi cần đọc; `git submodule deinit -f reference/<name>` sau đó (giữ index lean — codegraph KHÔNG có exclude).
- Non-goal: KHÔNG giữ cả `/gs` lẫn `/auto`; KHÔNG hai nguồn model config; KHÔNG index `reference/` vào codegraph.
