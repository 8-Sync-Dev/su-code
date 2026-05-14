# 00 — Force Load Skills (managed by `8sync skill sync`)

## ⛔ MANDATORY RULE — đọc trước mọi việc khác

Trước khi bắt đầu **bất kỳ** task không tầm thường, bạn (AI) PHẢI đọc theo thứ tự:

1. **`~/.forge/skills/karpathy-guidelines/SKILL.md`** — kỷ luật suy nghĩ (always first, no exception).
2. **`~/.forge/skills/8sync-cli/SKILL.md`** — bạn đang chạy trong 8sync harness, dùng đúng tool 8sync.
3. **`~/.forge/skills/image-routing/SKILL.md`** — chọn đọc image hay text để tiết kiệm token.

Sau đó (nếu trong 1 project):

4. **`<repo>/AGENTS.md`** — guidance riêng project.
5. **`<repo>/.gsd/PROJECT.md`** + **`KNOWLEDGE.md`** + **`DECISIONS.md`** + **`PREFERENCES.md`** + **`STATE.md`** — memory tích lũy.

## Bảng tra cứu nhanh

| Task type | Skills đọc theo thứ tự |
|---|---|
| Mọi task coding | karpathy → **8sync-cli** → image-routing |
| Review UI / PDF / diff | karpathy → **image-routing** trước khi fetch |
| Trong project 8sync | tất cả + `.gsd/*.md` |
| Câu hỏi đơn giản (1 câu) | karpathy (vẫn bắt buộc) |

## Quy tắc bất biến

- **Không skip karpathy.** Nếu không chắc skill nào áp dụng → vẫn đọc karpathy trước.
- **Không skip 8sync-cli** khi `AGENTS.md` của project đề cập 8sync. Bạn không phải forge thuần — bạn được wrap.
- **Không dump output dài** vào context. Tóm tắt trước.
- **Cite code dạng** `path:line` hoặc `path:start-end`. Không dùng natural language line ref.
