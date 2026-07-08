# Mx-VERIFICATION — <phase name>

> Nghiệm thu phase Mx bám AC. Nguồn chân lý: REQUIREMENTS.md + Mx-CONTEXT.md.
> Phase done ⇔ MỌI AC = PASS. Còn ≥1 FAIL hoặc UC chưa có verdict → phase CHƯA done.

## UC/AC verdicts

| UC | AC | Verdict | Bằng chứng (output/SQL/file:line) |
|----|----|---------|-----------------------------------|
| UC-01 | AC-01 | PASS | [test/script → output thật ✓] |
| UC-02 | AC-05 | FAIL | [lỗi cụ thể — file:line / migration lỗi dòng X] |

> Verdict ∈ PASS / FAIL / NEEDS-CONFIRM (auto-mode: item block chưa confirm được).

## Review findings (per dimension)

- **security:** [finding đã fix / còn lại — file:line, hoặc "clean"]
- **correctness:** [...]
- **convention:** [...]

## Kết luận

<N/M AC PASS>. Phase done? **YES / NO**
