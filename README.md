# su-code (`8sync`)

> **VI (mặc định):** Bộ harness cho coding với AI agent theo kiểu **dùng terminal bình thường**, còn AI đứng phía sau để quan sát ngữ cảnh dự án, học theo lịch sử làm việc và thực thi lệnh khi bạn yêu cầu.
>
> **EN:** A coding harness where you keep using your normal terminal workflow, while AI agents observe project context, learn from session memory, and execute tasks on demand.

---

## VI — Tổng quan nhanh

`8sync` **không thay terminal của bạn** và cũng **không ép bạn vào “AI terminal” riêng**.

Bạn vẫn:
- mở terminal như thường ngày,
- `cd` vào thư mục bất kỳ,
- chạy lệnh dev/build/test như bình thường.

AI agent sẽ:
- bám theo ngữ cảnh thư mục/project hiện tại,
- đọc memory trong `agents/*` + `AGENTS.md`,
- hỗ trợ phân tích, code, review, refactor, ship,
- học dần theo quyết định kỹ thuật và phong cách làm việc của bạn.

Nói ngắn gọn: **terminal-first, AI-observed, memory-driven**.

---

## VI — Cách hoạt động (đúng ý “đi tới đâu AI quan sát tới đó”)

Khi bạn vào project và chạy `8sync .`:

1. Xác định root project (`.git` / `Cargo.toml` / `package.json` / ...).
2. Tạo hoặc cập nhật memory dùng chung (`AGENTS.md`, `agents/*`).
3. Mở layout làm việc trong Kitty để bạn code nhanh hơn.
4. AI nạp ngữ cảnh từ memory, theo sát project hiện tại.

Khi bạn di chuyển qua project khác (`cd` sang thư mục khác + `8sync .`), AI sẽ dùng context của project đó.

---

## VI — Cài đặt

```bash
git clone https://github.com/8-Sync-Dev/su-code.git
cd su-code
bash scripts/bootstrap.sh
```

Sau đó:

```bash
8sync setup --dry-run
# kiểm tra trước khi cài thật (khuyên dùng)

8sync setup
forge login
8sync doctor
```

---

## VI — Lệnh chính (daily workflow)

| Lệnh | Mô tả |
|---|---|
| `8sync .` | Vào/attach session của project hiện tại |
| `8sync ai [prompt]` | Gọi AI theo ngữ cảnh đang làm |
| `8sync find <kw>` | Tìm code nhanh (rg/fzf + mở editor) |
| `8sync run [dev\|build\|test\|fmt\|lint]` | Chạy tác vụ chuẩn theo project |
| `8sync ship "msg"` | Add/commit/push/PR flow |
| `8sync end` | Chốt phiên, đúc kết knowledge vào memory |
| `8sync setup` | Cài môi trường dev đầy đủ (idempotent) |
| `8sync up` | Cập nhật tool |
| `8sync doctor` | Kiểm tra sức khỏe môi trường |

---

## VI — Memory dự án (điểm mạnh cốt lõi)

Trong mỗi project, `8sync` dùng thư mục `agents/` để lưu “trí nhớ làm việc”:

- `agents/PROJECT.md`
- `agents/KNOWLEDGE.md`
- `agents/DECISIONS.md`
- `agents/PREFERENCES.md`
- `agents/STATE.md`
- `agents/NOTES.md`

Nhờ vậy AI không chỉ trả lời theo prompt hiện tại mà còn bám lịch sử quyết định kỹ thuật của chính project đó.

---

## VI — Hình ảnh minh hoạ

> Bạn có thể thêm ảnh demo vào phần này (screenshot layout, before/after, flow thực tế) để README trực quan hơn.

Gợi ý ảnh nên có:
1. Session layout khi chạy `8sync .`
2. Flow `find -> edit -> run -> ship`
3. Ví dụ memory `agents/*` sau vài phiên làm việc

Khi có ảnh, thêm theo mẫu Markdown:

```md
![8sync session layout](https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/assets/demo/session-layout.png)
![8sync workflow](https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/assets/demo/workflow.png)
```

---

## VI — GitHub mô tả, links cộng đồng & hashtag (để dễ discover)

Bạn có thể dùng đoạn mô tả ngắn này cho repo:

> **Terminal-first AI coding harness for CachyOS/Arch + Kitty + Helix. Keep your normal CLI workflow while AI agents observe project context, learn memory, and execute tasks.**

Links chính thức:

- Website: https://8-sync-dev.github.io/su-code
- GitHub Org: https://github.com/8-Sync-Dev
- Repo `su-code`: https://github.com/8-Sync-Dev/su-code
- Community Discussions: https://github.com/orgs/8-Sync-Dev/discussions

GitHub Pages (project site):

- URL: https://8-sync-dev.github.io/su-code
- Source: `docs/index.html`
- Auto deploy workflow: `.github/workflows/pages.yml`
- Cập nhật link tác giả (Facebook/YouTube/TikTok): sửa tại `docs/index.html` mục `Author &amp; Social`

Hashtag gợi ý:

`#8sync #AIAgent #VibeCoding #CodingHarness #TerminalWorkflow #DeveloperTools #RustLang #KittyTerminal #HelixEditor #ArchLinux #CachyOS #OpenSource`

---

## EN — Quick Overview

`8sync` is a **terminal-first AI coding harness**.

You keep your normal CLI workflow (`cd`, edit, run, test, ship). AI agents then:
- observe the active project context,
- load shared memory from `AGENTS.md` + `agents/*`,
- execute coding tasks on request,
- improve continuity across sessions.

It is not a separate “AI-only terminal”; it augments your existing terminal practice.

### Install

```bash
git clone https://github.com/8-Sync-Dev/su-code.git
cd su-code
bash scripts/bootstrap.sh
8sync setup --dry-run
8sync setup
forge login
8sync doctor
```

### Core commands

- `8sync .` — open/attach project session
- `8sync ai [prompt]` — run AI in current context
- `8sync find <kw>` — fast code search + jump
- `8sync run [dev|build|test|fmt|lint]` — standard tasks
- `8sync ship "msg"` — commit/push/PR flow
- `8sync end` — capture session knowledge

---

## License

MIT. See `LICENSE`.
