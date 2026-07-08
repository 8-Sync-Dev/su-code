# Mx-CONTEXT — <phase name>

> Hợp đồng "tại sao + nghiệm thu" của phase Mx. `/feature go` + `/feature ship` đọc file này làm chuẩn.
> BẮT BUỘC có đủ: 📌 Requirement scope + 🎯 Goal + ✅ Acceptance Criteria TRƯỚC khi plan/code.

## 📌 Requirement scope (UC từ REQUIREMENTS.md)

| UC | Mô tả (literal từ REQUIREMENTS.md) | Trong phase này làm gì | Không làm ở phase này |
|----|-----------------------------------|------------------------|-----------------------|
| UC-01 | [copy mô tả] | [phạm vi cụ thể phase này] | [ranh giới future/out-of-scope] |

## 🎯 Goal

[1 câu: output đo được của phase + ranh giới (CHƯA làm gì → tránh review đòi hỏi quá phạm vi).]

## ✅ Acceptance Criteria (UAT)

> Mỗi UC ⇒ ≥1 AC. AC đo được (số/trạng thái/output cụ thể — KHÔNG "chạy ổn"). Cột "Cách verify" cũng là `verify` của engine task ở `/feature go`.

| AC | UC | GIVEN / WHEN / THEN (đo được) | Cách verify | Tier | Task nguồn |
|----|----|-------------------------------|-------------|------|------------|
| AC-01 | UC-01 | GIVEN [tiền đề] WHEN [hành động] THEN [kết quả đo được] | [lệnh/SQL/script/thao tác] | must-test / verify-sql / verify-only | T1 |

## Decisions (D1, D2… — quyết định riêng phase, append khi chốt)

- D1: [quyết định — vì sao]. (nguồn: discuss / `ask` / (auto-decided via <role>))

## Plan-review notes (điền sau Step 3.5 nếu có chạy)

- (chưa có)

---

**Phase DONE khi mọi AC PASS (ghi ở Mx-VERIFICATION.md). AC FAIL → không ship.**
