# /feature ship

> Đã load `references/feature-rules.md` (R1 config-resolve, R5 AC nghiệm thu, **R10 code-intelligence FIRST**) ở Dispatch chưa? Nếu chưa → load trước.

Project the completed GS run evidence into the feature's AC matrix, close the phase, and archive when the feature ends. `/feature ship` does not create a second review/test loop.
Gate: GS is not `done`, any required AC lacks PASS evidence, or UAT is absent → reopen/continue `/gs`; never paper over the failed gate here.

## Step 0 — Load hợp đồng nghiệm thu (BẮT BUỘC trước khi import evidence)

Đọc `M<x>-CONTEXT.md` → **📌 Requirement scope (UC từ REQUIREMENTS.md) + 🎯 Goal + bảng ✅ Acceptance Criteria (AC-NN)**. Chuẩn nghiệm thu phase.
- Requirement scope + AC là **nguồn chân lý** để map GS evidence. Không tự đặt tiêu chí ngoài AC hoặc đòi vượt Goal (ranh giới phase).
- CONTEXT thiếu Goal/AC → DỪNG, quay lại `/feature plan` bổ sung (không nghiệm thu mò).
- Đầu ra cuối: `M<x>-VERIFICATION.md` (dùng `templates/M-VERIFICATION.md`) có **bảng AC → verdict** (mỗi AC: PASS/FAIL + bằng chứng cụ thể). Phase done ⇔ MỌI AC PASS.

## Step 1 — import the GS evidence (single source of execution truth)

Read `.cache/8sync/gs/state.json` for the phase. Require:
- run status/stage is `done`;
- goal/plan hash corresponds to this phase's CONTEXT/PLAN;
- every required AC has objective evidence;
- verifier, independent review/security (when risk requires it), and user UAT gates passed.

Map each `state.acceptance[]` entry to the matching UC/AC in `M<x>-CONTEXT.md`. Preserve the concrete command hash, review verdict, browser/smoke/API proof, and timestamp. Do not synthesize PASS from task status or prose.

If evidence is missing or stale, **do not spawn an ad-hoc reviewer/Tester from the feature layer**. Resume/reopen the native engine (`/gs continue` or `/gs reject <stage> <reason>`) so its lease, model-independence, verification, retry, and UAT policies remain enforced. Then import the new evidence after GS reaches `done`.

Optional supplemental analysis may discover a gap, but it cannot approve an AC. Record the finding, reopen the corresponding GS stage, fix through the GS loop, and let GS produce canonical evidence.

### Ghi `M<x>-VERIFICATION.md` — bắt buộc dạng AC-matrix
```markdown
# M<x>-VERIFICATION
## UC/AC verdicts (nguồn: REQUIREMENTS.md + M<x>-CONTEXT)
| UC | AC | Verdict | Bằng chứng (output/SQL/file:line) |
|----|----|---------|-----------------------------------|
| UC-15 | AC-01 | PASS | test/test-...  → "handler not called" ✓ |
| UC-16 | AC-05 | FAIL | migration lỗi dòng X |
...
## Review findings (per dimension) — đã fix / còn lại
## Kết luận: <N/M AC PASS>. Phase done? YES/NO
```
**Gate cứng:** còn ≥1 UC/AC FAIL, thiếu evidence, hoặc UC trong Requirement scope chưa có AC verdict → phase CHƯA done. Reopen/continue GS → re-verify → import lại; KHÔNG ship khi matrix còn FAIL.

## Step 2 — Ship (đóng phase)

1. **Commit**: code do GS engine commit (verify-gated ở closeout) rồi — KHÔNG commit gộp lại. Ship chỉ:
   - Commit nốt phần phụ của phase chưa thuộc task nào (`M<x>-VERIFICATION.md`/STATE/ROADMAP đổi): `docs: M<x> close phase <tên>` (Conventional Commits, tiếng Anh, milestone ở đầu — xem `execute.md` §Commit).
   - Verify **AC matrix trong M<x>-VERIFICATION.md MỌI AC = PASS** (Step 0+1+2) TRƯỚC khi tính phase done. Còn FAIL → KHÔNG đóng phase.
   - **KHÔNG `git push` / mở PR** — chỉ push khi user yêu cầu rõ.
2. **ROADMAP**: phase `[~]` → `[x]` + ghi Phase log (range commit `<first>..<last>` của phase, contract đã export).
3. **STATE cập nhật**:
   - `progress.completed_phases` +1, `percent` lại.
   - `active_phase` → phase tiếp (unblocked theo dependency) hoặc `null` nếu hết.
   - `next_action` → `plan-phase` cho phase sau, hoặc `done`.
   - `## Session Continuity`: ghi vừa ship phase nào.

## Step 3 — Nếu là phase CUỐI: chống drift + archive

BẮT BUỘC khi feature hoàn tất:
- [ ] `su-code/KNOWLEDGE.md` — append nghiệp vụ/gotcha mới học (`validated:`/`failure:`). Lớn → spawn `task` (`agent: task`).
- [ ] `su-code/DECISIONS.md` — append quyết định kiến trúc feature chốt.
- [ ] `PROJECT.md` Validated — tick UC đã ship.
- [ ] `REQUIREMENTS.md` — rà UC thực tế bỏ/đổi, cập nhật intent.
- [ ] `CHANGELOG.md` (Unreleased) — thêm dòng feature (convention su-code).
- [ ] Archive: `mv su-code/planning/<slug> su-code/planning/_archive/` + clear `su-code/planning/ACTIVE.md` (dòng slug về trống) + `config.active_feature = ""`.

## Sau ship

Báo user: phase X done. Còn phase nào → next `/feature plan`. Hết → feature hoàn thành, đã archive.
