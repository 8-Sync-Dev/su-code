# ROADMAP — FEATURE_NAME

> Bản đồ phase + dependency. Thưa — chỉ tick khi phase bắt đầu/xong.
> Task chi tiết KHÔNG ở đây (ở phases/M<x>-<name>/M<x>-NN-PLAN.md). Tiến độ task ở STATE.md.

**Created:** DATE

## Phases (theo dependency)

- [ ] **M0 Foundation** — [mục tiêu] · UC: [ids]
- [ ] **M1 [tên]** — [mục tiêu] · cần: M0 · UC: [ids]
- [ ] **M2 [tên]** — [mục tiêu] · cần: M0,M1 · UC: [ids]
- [ ] **M3 [tên]** — [mục tiêu] · cần: M0,M2 · UC: [ids]

Status: `[ ]` chưa · `[~]` đang làm · `[x]` xong

## Dependency graph

```
M0 ──┬─→ M1 ──→ M2 ──┐
     └────────────────┴─→ M3
```

## Integration Contracts (khớp giữa phase — chống lệch)

- **M0 export:** [hàm/interface/schema] → M1,M2,M3 dùng
- **M1 export:** [...] → M2,M3 dùng
- **M2 export:** [...] → M3 dùng

## Phase log (append khi ship)

- (chưa có)
