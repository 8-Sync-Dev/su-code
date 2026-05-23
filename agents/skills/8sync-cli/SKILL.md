---
name: 8sync-cli
description: Use this skill in EVERY session inside a repo whose AGENTS.md mentions 8sync. It teaches the AI which 8sync verbs (shot/diff-img/pdf-img/find/note/ship/skill/run) to use instead of raw shell equivalents — saving 3-10× tokens and keeping session memory in agents/* consistent. The AI MUST prefer the listed 8sync verbs over rg/fd/git/curl/etc when an equivalent exists.
---

# 8sync-cli — bạn (AI) ĐANG chạy bên trong 8sync harness

**LOAD: bắt buộc cho mọi project có `AGENTS.md` đề cập 8sync.**

Bạn chạy dưới `omp` (oh-my-pi) được wrap bởi `8sync`. Dùng đúng các tool 8sync sẽ tiết kiệm token gấp 3-10× và giữ memory project nhất quán.

---

## 1. Quy tắc tuyệt đối

1. **LUÔN đọc `~/.omp/skills/karpathy-guidelines/SKILL.md` đầu tiên** trước mọi non-trivial task.
2. **LUÔN đọc `~/.omp/skills/image-routing/SKILL.md`** trước khi fetch/đọc bất kỳ tài nguyên hình ảnh, PDF, hoặc diff lớn.
3. **LUÔN đọc các file `agents/*.md` của project** trước khi bắt đầu — chúng chứa memory tích lũy của các session trước.
4. **Đọc project-local skills** trong `<repo>/agents/skills/<name>/` khi task chạm vào lĩnh vực tương ứng — AGENTS.md đã liệt kê chúng giữa cặp sentinel `<!-- 8sync:skills:* -->`.
5. **KHÔNG bao giờ chỉnh sửa trực tiếp `agents/*.md`** — omp tự quản memory qua `retain` / `recall` / auto-compact.

---

## 2. CLI tool 8sync bạn ĐƯỢC PHÉP dùng

Ưu tiên các lệnh này hơn equivalent của shell:

| Tình huống | Dùng | Thay vì |
|---|---|---|
| Cần screenshot UI/web route | `8sync shot <url\|file>` | mô tả layout bằng text |
| Cần đọc diff lớn (>300 dòng) | `8sync diff-img [git-ref]` | `git diff` text dump |
| Cần đọc PDF | `8sync pdf-img <file>` | OCR/parse text |
| Tìm file/symbol nhanh | `8sync find <kw>` | `rg`/`fd` thô |
| Ghi nhớ ý tưởng/note thoáng qua | `8sync note "..."` | sửa file `agents/` tay |
| Cài skill mới từ GitHub | `8sync skill add <url>` | clone tay |
| Commit + push + PR | `8sync ship "msg"` | `git add && commit && push && gh pr create` từng bước |
| Khởi session ở project khác | `8sync . to <name>` | `cd` rồi mở omp mới |
| Chạy dev/build/test theo recipe | `8sync run dev` | nhớ `pnpm dev`/`bun dev`/`cargo run` |
| Liệt kê session đang sống | `8sync . ls` | `ps aux \| grep omp` |

Khi không chắc lệnh nào, gọi `8sync help` hoặc `8sync flow`.

---

## 3. Cấu trúc memory dự án

```
<repo>/
├── AGENTS.md                       ← bạn đọc đầu, link sang dưới + liệt kê skills
└── agents/                         ← 8sync managed
    ├── PROJECT.md                  facts cố định (stack, entrypoint)
    ├── KNOWLEDGE.md                append-only: bạn học được gì
    ├── DECISIONS.md                append-only: quyết định kiến trúc
    ├── PREFERENCES.md              append-only: style user thích
    ├── STATE.md                    việc đang dở, next-step
    ├── NOTES.md                    quick notes via `8sync note`
    └── skills/                     project-local skills (cloned via `8sync skill add <url>`)
        └── <name>/                 đọc SKILL.md / CLAUDE.md / README.md bên trong
```

**Đọc tất cả tại session start.** Trích dẫn khi áp dụng (vd "Theo `agents/DECISIONS.md:42` đã chốt dùng zustand").

---

## 4. Session memory

`omp` tự quản lý memory session: `retain` để ghim, `recall` để lấy lại, auto-compact khi dài. **Không** chỉnh tay `agents/*.md` — chỉ omp / `8sync note` được phép ghi.

---

## 5. Token discipline

- File code > 800 dòng → dùng outline tool (tree-sitter / lsp symbols) thay vì đọc cả file.
- Tool output dài → tóm tắt vào nháp trước, rồi mới đọc slice cụ thể.
- Đừng dump `git log`/`ls -la` toàn bộ; gọi với filter cụ thể.

---

## 6. Cite convention

- Code reference: `path/to/file.rs:23-58` hoặc `file.rs:23` (single line).
- KHÔNG dùng "file lines 23-58" hoặc "in file.rs around 23".

---

## 7. Khi bạn KHÔNG biết phải làm gì

1. Đọc lại karpathy-guidelines.
2. Đọc `agents/KNOWLEDGE.md` + `agents/STATE.md`.
3. Gõ `8sync help` (qua shell tool) xem các lệnh.
4. Hỏi user — ngắn, cụ thể, không lan man.
