# Mx-NN-PLAN — <phase name>

> Batch task của phase Mx. Mỗi task truy được về ≥1 AC + ≥1 UC (từ Mx-CONTEXT.md).
> **GS mapping:** mỗi task ↔ 1 GS task; mỗi wave ↔ 1 GS slice; `verify` của task = lint/test/build THẬT (cột "Cách verify" của AC). `/feature go` feeds bảng này vào `gs_plan`.

## Wave 1 (song song — độc lập, khác file)

- [ ] T1: <việc>   [file: path]   [skill: <name>]   [tier: must-test]   [UC: UC-01]   [AC: AC-01,AC-03]
- [ ] T2: <việc>   [file: path]   [skill: <name>]   [tier: verify-sql]   [UC: UC-02]   [AC: AC-05]

## Wave 2 (cần Wave 1)

- [ ] T3: <việc>   [file: path]   [skill: <name>]   [depends: T1]   [tier: verify-only]   [UC: UC-01]   [AC: AC-08]

## Checkpoints / Gates

- **Review dimensions:** [từ config.workflow.review_dimensions, vd security, correctness, convention]
- **Verify (GS gate):** mỗi task chạy lệnh lint/test/build thật của dự án qua `gs_verify`; `gs_advance` refuses to leave a stage whose gate is unmet.
- **Acceptance:** phase done ⇔ mọi AC trong Mx-CONTEXT PASS (verify ở `/feature ship` → Mx-VERIFICATION.md). KHÔNG dùng DoD mơ hồ — dùng AC.

## Kiểm tra phủ (trước khi trình gate 2)

- [ ] Mọi UC trong Requirement scope có ≥1 AC.
- [ ] Mọi AC-NN xuất hiện ở cột `[AC:]` của ≥1 task.
- [ ] Mọi task có `[UC:]` + `[AC:]` + `[skill:]` (task code `[skill: —]` = cờ đỏ).
- [ ] Task cùng wave thật sự độc lập + khác file (không race).
