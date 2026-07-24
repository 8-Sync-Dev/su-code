# /feature go

> Đã load `references/feature-rules.md` (R3 load skill 2-lớp, R8 commit, **R10 code-intelligence FIRST — áp dụng cả subagent**) ở Dispatch chưa? Nếu chưa → load trước. Bước dưới KHÔNG lặp lại các luật đó.

Execute phase hiện tại theo PLAN.

## Nguyên lý: feed GS engine có sẵn — KHÔNG dựng engine mới

**Feature layer KHÔNG tự viết vòng lặp thực thi/swarm.** Nó FEED chính GS engine mà `/gs` lái (native omp extension ở `~/.omp/agent/extensions/8sync-gs/`, durable state ở `.cache/8sync/gs/state.json`, verify-gate + doom-loop guard + worktree đã enforce trong CODE). Feature layer chỉ sở hữu phần GS engine KHÔNG quản: hợp đồng ROADMAP nhiều phase + AC + bookkeeping STATE/PLAN. Mirror đúng kỷ luật GS engine (native extension) + guardrail: clarify → plan → plan_review → implement → verify → review → uat → closeout.

## Trước khi code

- **Đọc `M<x>-CONTEXT.md` TRƯỚC** → 📌 Requirement scope (UC từ REQUIREMENTS.md) + 🎯 Goal + ✅ AC + **Decisions (D1, D2…)** + ranh giới. Đây là "tại sao + nghiệm thu" — PLAN chỉ có "làm gì + file". Code đúng task nhưng sai UC/Decision = vẫn hỏng.
- Đọc `M<x>-NN-PLAN.md` → wave + task + file ownership + cột `[skill:]` + `[UC:]` + `[AC:]` mỗi task.
- Đọc STATE.next_action → biết task tiếp (resume đúng chỗ nếu session trước dở).
- **Load skill repo theo cột `[skill:]` (R3, BẮT BUỘC):** với MỖI skill ghi trong task, orchestrator đọc `su-code/skills/<skill>/SKILL.md` (hoặc `~/.omp/skills/<skill>/SKILL.md`) + references TRƯỚC khi code/spawn — KHÔNG mirror module có sẵn rồi suy đoán convention.

> **Orchestrator giữ CONTEXT trong đầu suốt phase.** Mỗi task code phải tôn trọng Decisions + nhắm AC nó gánh. Lệch Decision → DỪNG (như lệch ROADMAP, R6).

## Bước 1 — khởi tạo GS run rồi nạp phase

GS tools từ chối mutation nếu chưa có active run. Vì vậy:
1. Nếu `gs_status` chưa có run, gọi slash command **`/gs <Goal phase>`** (assisted) hoặc **`/gs --auto <Goal phase>`** (auto) để tạo run và route coordinator.
2. Ở stage `clarify`, gọi **`gs_define`** với requirements + AC literal từ CONTEXT/REQUIREMENTS.md. `gs_define` không nhận `goal`; goal đã được khóa khi tạo run.
3. Đi qua research (nếu risk yêu cầu) tới stage `plan`, rồi gọi **`gs_plan`** đúng một lần:
   - `slices` = các **Wave** trong PLAN (mỗi wave = 1 slice).
   - Mỗi `task` = task `T<n>` literal, có ownership, dependencies, skills và AC IDs.
   - Mỗi `verify` = lệnh lint/test/build THẬT dạng direct argv (`program` + `args`), không shell string.

> Nếu GS đã có run cho phase này, dùng `gs_status`/`gs_next` để resume; không tạo hoặc plan lại làm mất lease/evidence hiện tại.

## Bước 2 — chạy state machine `gs_next → task → gs_verify → gs_advance`

Lặp tới `done`/`blocked`; mỗi bước obey instruction literal từ `gs_next`:

1. **`gs_next`** cấp lease chính xác: agent, model và task IDs. Ở implement, spawn đúng `gs-worker` cho từng task trong lease, cùng một batch, không thêm helper agent. Mỗi prompt ghi đúng task ID, ownership, AC IDs, skills, UC/Decisions và ground truth liên quan.
2. Worker chỉ sửa file thuộc ownership và trả `task_id` + `changed_files` + observed behavior. Worker không commit và không tự đánh dấu pass.
3. Sau khi matching worker evidence đã được engine ghi nhận, gọi **`gs_verify {taskId}`** cho từng task. Engine chỉ chạy direct argv đã plan, trong project; shell/destructive/outward/cwd escape bị từ chối. FAILED → sửa nguyên nhân rồi verify lại; 3 fail giống nhau hoặc quá retry limit → BLOCK.
4. Khi toàn bộ task của wave/phase đã verify pass, gọi **`gs_advance`** (không có `taskId`) để sang verifier → independent review/security → user UAT → closeout. Ở mỗi agent stage, lại gọi `gs_next` và spawn đúng lease trước khi advance.
5. **Bookkeeping feature-side:**
   - Append `su-code/planning/<slug>/STATE.md` `## Log`: `- DATE M<x> T<n> ✓ <việc> [file]`.
   - Tick task trong `M<x>-NN-PLAN.md` sau verify PASS.
   - Cập nhật STATE `Current Position` + `next_action`.
6. **Guardrail R6**: việc phải thuộc `active_phase`; lệch ROADMAP/Decision → dừng (auto: SKIP + NEEDS-CONFIRM).

> **Isolate slice rủi ro/lớn** bằng **`gs_worktree`** (open → work → merge squash → remove) — tránh nửa chừng làm bẩn nhánh chính.

## Commit — qua GS engine (verify-gated, ở closeout)

GS engine commit **một lần ở closeout** sau khi MỌI gate pass (verify + review + UAT) và gitleaks clean — **KHÔNG commit per-task**, không param commit trên `gs_advance`, không `/commit` thủ công. Trong lúc implement, hook của GS chặn `git commit` khi còn task chưa verify. Luật:
- **Verify-gate trước commit**: GS chỉ commit ở closeout sau khi mọi task verify pass — enforce trong code.
- **Branch**: mặc định nhánh hiện tại (commit local checkpoint). Nếu user chọn feature branch → verify `git branch --show-current` khớp `STATE.branch` trước khi commit; lệch → DỪNG.
- **Message tiếng Anh** (Conventional Commits), milestone ở đầu: `<type>: M<x> - <desc>` (vd `feat: M2 - sync group-info onto the group record`). `type` ∈ feat/fix/docs/refactor. KHÔNG `[<Category>]` prefix, no AI ref.
- **KHÔNG `git push` / PR** trừ khi user yêu cầu. Ghi commit hash vào STATE.Log để `ship`/revert truy ngược.
- Task hỏng giữa chừng: KHÔNG advance task dở. `gs_verify` fail → không advance; sửa xong mới advance.

## `--auto` (autonomous) — GS auto-mode, không user-gate giữa task

`/feature go --auto` = đúng loop trên chạy ở **GS auto-mode** (independent critic thay gate requirements/plan), **scoped 1 phase**, dừng ở ranh giới phase kế:
- Không yield giữa các task; chạy hết `gs_next/gs_verify/gs_advance` của phase.
- Task block (dữ liệu/môi trường subagent+repo+DB bó tay) → SKIP + ghi NEEDS-CONFIRM vào VERIFICATION/STATE, làm task khác. Không stall cả phase.
- Ranh giới an toàn không được bypass: UAT vẫn cần `/gs approve uat`; destructive/outward action cần consent một lần gắn với đúng command hash mà engine báo. Không dùng plan/UAT approval làm giấy phép chung.

## Khi hết task của phase

- STATE: `status: executing` → sẵn sàng verify. `next_action: ship-phase`.
- **Self-check nhanh**: mọi UC/AC trong CONTEXT map sang canonical GS acceptance evidence; thiếu evidence thì run chưa thật sự complete — reopen/continue GS.
- KHÔNG tự review/test lần hai ở feature layer. `/feature ship` chỉ project GS evidence vào VERIFICATION matrix và đóng phase.

Next: `/feature ship`.
