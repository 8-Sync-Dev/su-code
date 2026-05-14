# 8sync-cli — bạn (AI) ĐANG chạy bên trong 8sync harness

**LOAD: bắt buộc cho mọi project có `AGENTS.md` đề cập 8sync.**

Bạn không phải forge thuần — bạn được wrap bởi `8sync`. Dùng đúng các tool 8sync sẽ tiết kiệm token gấp 3-10×, tăng đáng kể chất lượng việc nhớ.

---

## 1. Quy tắc tuyệt đối

1. **LUÔN đọc `~/.forge/skills/karpathy-guidelines/SKILL.md` đầu tiên** trước mọi non-trivial task.
2. **LUÔN đọc `~/.forge/skills/image-routing/SKILL.md`** trước khi fetch/đọc bất kỳ tài nguyên hình ảnh, PDF, hoặc diff lớn.
3. **LUÔN đọc các file `.gsd/*.md` của project** trước khi bắt đầu — chúng chứa memory tích lũy của các session trước.
4. **KHÔNG bao giờ chỉnh sửa trực tiếp `.gsd/*.md`** — chỉ append qua format end-of-session (mục 4).

---

## 2. CLI tool 8sync bạn ĐƯỢC PHÉP dùng

Ưu tiên các lệnh này hơn equivalent của shell:

| Tình huống | Dùng | Thay vì |
|---|---|---|
| Cần screenshot UI/web route | `8sync shot <url\|file>` | mô tả layout bằng text |
| Cần đọc diff lớn (>300 dòng) | `8sync diff-img [git-ref]` | `git diff` text dump |
| Cần đọc PDF | `8sync pdf-img <file>` | OCR/parse text |
| Cần render mermaid/graphviz | `8sync chart <data>` (phase 2) | mô tả node-edge bằng text |
| Tìm file/symbol nhanh | `8sync find <kw>` | `rg`/`fd` thô |
| Ghi nhớ ý tưởng/note thoáng qua | `8sync note "..."` | sửa file `.gsd/` tay |
| Commit + push + PR | `8sync ship "msg"` | `git add && commit && push && gh pr create` từng bước |
| Khởi session ở project khác | `8sync . to <name>` | `cd` rồi mở forge mới |
| Chạy dev/build/test theo recipe | `8sync run dev` | nhớ `pnpm dev`/`bun dev`/`cargo run` |
| Liệt kê session đang sống | `8sync . ls` | `ps aux \| grep forge` |

Khi không chắc lệnh nào, gọi `8sync help` hoặc `8sync flow`.

---

## 3. Cấu trúc memory dự án

```
<repo>/
├── AGENTS.md                  ← bạn đọc đầu, link sang dưới
└── .gsd/                      ← 8sync managed
    ├── PROJECT.md             facts cố định (stack, entrypoint)
    ├── KNOWLEDGE.md           append-only: bạn học được gì
    ├── DECISIONS.md           append-only: quyết định kiến trúc
    ├── PREFERENCES.md         append-only: style user thích
    └── STATE.md               việc đang dở, next-step
```

**Đọc tất cả tại session start.** Trích dẫn khi áp dụng (vd "Theo `.gsd/DECISIONS.md:42` đã chốt dùng zustand").

---

## 4. End-of-session capture (khi user gõ `8sync end`)

Output đúng format dưới — 8sync parse → append vào `.gsd/*.md`:

```
<DECISIONS>
- decision in ≤ 1 line
- chỉ ghi quyết định MỚI session này
</DECISIONS>

<KNOWLEDGE>
- fact về codebase mới phát hiện session này
- vd: "src/auth/middleware.ts:34-58 chứa logic JWT refresh"
</KNOWLEDGE>

<PREFERENCES>
- pattern style user thích/không thích
- vd: "User prefers named exports over default"
</PREFERENCES>

<STATE>
- task đang dở
- next-step concrete
</STATE>
```

Nếu không có gì đáng ghi cho 1 block → để `_none_`.

---

## 5. Token discipline

- File code > 800 dòng → dùng `8sync mcp get_project_outline` (tree-sitter) thay vì đọc cả file.
- Tool output dài → tóm tắt vào nháp trước, rồi mới đọc slice cụ thể.
- Đừng dump `git log`/`ls -la` toàn bộ; gọi với filter cụ thể.

---

## 6. Cite convention

- Code reference: `path/to/file.rs:23-58` hoặc `file.rs:23` (single line).
- KHÔNG dùng "file lines 23-58" hoặc "in file.rs around 23".

---

## 7. Session boundary

- `8sync .` = session bắt đầu → đọc `.gsd/*` + `AGENTS.md` ngay.
- `8sync end` = session kết thúc → output 4 block format trên.
- Nếu user không gõ `end` mà chỉ Ctrl+D / đóng pane → 8sync vẫn capture lần forge respond cuối nếu có 4 block.

---

## 8. Khi bạn KHÔNG biết phải làm gì

1. Đọc lại karpathy-guidelines.
2. Đọc `.gsd/KNOWLEDGE.md` + `.gsd/STATE.md`.
3. Gõ `8sync help` (qua shell tool) xem các lệnh.
4. Hỏi user — ngắn, cụ thể, không lan man.
