# CLAUDE.md — guidance for Claude Code

<!-- 8sync:skills:begin -->
## 🚨 STEP 0 — `codegraph` FIRST (mandatory, no exception)

`codegraph` là **core tool** cho mọi câu hỏi liên quan đến code trong repo này. Bạn (AI) **PHẢI**:

1. Chạy `codegraph index .` **1 lần** đầu session để build/refresh semantic index.
2. Dùng `codegraph search "<query>"` thay cho `grep`/`rg`/`fd`/`Grep`/`Glob`.
3. Dùng `codegraph deps <file>` thay cho `Read` toàn file để hiểu dependency graph.
4. Dùng `codegraph callers <symbol>` / `codegraph defs <symbol>` thay cho find-references thủ công.

Lý do: ~35% rẻ hơn token, ~70% ít tool call hơn, 100% local. Dump cả file = đốt token vô ích.

## 🚨 STEP 1 — skills: always-on (đọc ngay) vs on-demand (đọc khi cần)

Mỗi skill = 1 directory (Agent Skills open standard): `SKILL.md` có frontmatter `name`+`description`. Skill vendored ở `agents/skills/<name>/` (bản commit trong repo, mirror từ `~/.omp/skills/`). Mỗi skill liệt kê 1 lần.

### ⛔ Always-on — ĐỌC NGAY, trước tool call đầu tiên (không skip)

  1. `/home/alexdev/Projects/su-code/agents/skills/codegraph/SKILL.md`
  2. `/home/alexdev/Projects/su-code/agents/skills/8sync-cli/SKILL.md`
  3. `/home/alexdev/Projects/su-code/agents/skills/image-routing/SKILL.md`
  4. `/home/alexdev/Projects/su-code/agents/skills/karpathy-guidelines/SKILL.md`

### 🔎 On-demand — CHỈ đọc khi task khớp mô tả (bỏ qua nếu không liên quan)

- **`last30days`** — `agents/skills/last30days/SKILL.md`
     _Use this skill when the user asks "what are people saying about X", "research <topic> recently", "what's trending on Reddit/X/YouTube about Y", pre-meeting/pre-call briefings, "last 30 days of Z", competitor scans, or any recency-grounded social research. It runs the `/last30days` agent skill (separately installed engine) that searches Reddit, X, YouTube, TikTok, Hacker News, Polymarket, GitHub, Bluesky and the web in parallel, scores by real engagement, and synthesizes one cited brief. Prefer it over ad-hoc WebSearch when the user wants what the community actually thinks RIGHT NOW._

### Quy tắc bất biến

- **`codegraph` FIRST** cho mọi câu hỏi explore code (Step 0). Bypass = bug.
- Đọc TẤT CẢ skill **always-on** TRƯỚC khi gọi tool đầu tiên.
- Skill **on-demand**: chỉ mở khi description khớp task hiện tại — đừng đọc thừa.
- Nếu skill có `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.
- Nếu skill có `references/` → đọc on-demand khi task chạm chủ đề.
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.
<!-- 8sync:skills:end -->
