# 00 — Force Load Skills (managed by `8sync harness init`)

## 🔴 RULE #0 — CODE INTELLIGENCE FIRST, ALWAYS (codegraph + codebase-memory-mcp)

Before any other tool call, answer codebase questions with a code-intelligence engine — NOT grep/find/Read. Both are ~99% cheaper than file-by-file exploration:

- **codegraph** (local pre-indexed graph): `codegraph init -i` once per repo (if `.codegraph/` missing), then `codegraph search/deps/callers/defs`. Skill: `~/.omp/skills/codegraph/SKILL.md`.
- **codebase-memory-mcp** (MCP server — auto-set-up by `8sync harness`; 158 languages, sub-ms, auto-indexes on connect): `search_graph`, `semantic_query`, `trace_path`, `get_architecture`, `detect_changes`, `query_graph`, `get_code_snippet`, `manage_adr`.
- **Default to these** for "how does X work / where is X / who calls X / what depends on X", impact analysis, route→handler, dead code, architecture.
- **BẮT BUỘC nén output lớn:** mọi output > ~50 dòng (logs / diffs / test / tool dumps) phải qua `headroom` MCP (`headroom_compress`) TRƯỚC khi vào context — 60–95% ít token (auto-set-up bởi `8sync harness`). Dump nguyên khối lớn = vi phạm.
- Only `Read` a raw file when you're about to edit it (read-before-edit). Falling back to `rg`/`fd`/`Read` for exploration first is a **violation**.

## ⛔ READING ORDER — 2 tầng (progressive disclosure, giữ prefix gọn cho KV-cache)

### CORE — đọc body NGAY, trước tool call đầu tiên (không skip, đúng thứ tự)

Nhỏ + dùng cho MỌI task: codegraph → karpathy → ponytail → 8sync-cli.

1. **`~/.omp/skills/codegraph/SKILL.md`** — semantic code intelligence (senses của loop).
2. **`~/.omp/skills/karpathy-guidelines/SKILL.md`** — kỷ luật engineering (read-before-write, test-before-refactor, bước nhỏ).
3. **`~/.omp/skills/ponytail/SKILL.md`** — "laziest senior dev": YAGNI, làm ít nhất, xoá > thêm.
4. **`~/.omp/skills/8sync-cli/SKILL.md`** — đang chạy trong harness 8sync; ưu tiên verb 8sync hơn shell thô.

### SPECIALIST always-on — biết khả năng, đọc body KHI task khớp (đừng đọc mỗi phiên)

- **`assp-skill`** — brand DNA 8 Sync Dev + ASSP validate-before-build. Mở khi: UI copy, landing/pricing, email/error, greenlight feature mới.
- **`impeccable`** — **design system CHUẨN; BẮT BUỘC mở body NGAY khi có việc UI/design/redesign/audit** (chạy `scripts/context.mjs`, kèm `references/house/*`).
- **`taste-skill`** — anti-slop frontend taste. Mở khi: landing/portfolio/redesign.
- **`image-routing`** — image-vs-text routing. Mở khi: xử lý ảnh/diff/PDF.

On-demand (đọc khi task khớp description): `code-review-and-quality`, `senior-security`, `senior-frontend`, `full-flow`, `last30days`; `encore-deploy` (chỉ khi project dùng Encore); `social-growth` (opt-in — `8sync skill add builtin:social-growth`).

If inside a project (cwd có `.git` / `Cargo.toml` / `package.json` / …) — đọc thêm:

- **`<repo>/AGENTS.md`** — guidance riêng dự án. Block `<!-- 8sync:skills:begin -->`…`end` liệt kê skill project-local; đọc cái khớp task.
- **`<repo>/agents/{PROJECT,KNOWLEDGE,DECISIONS,PREFERENCES,STATE}.md`** — memory tích luỹ. Đọc STATE đầu phiên.

## Fast lookup table

| Task type | Order to read |
|---|---|
| ANY code exploration (how does X work? where is X?) | **codegraph → karpathy → 8sync-cli → project-local** |
| Refactor / impact analysis | **codegraph (callers/callees) → karpathy → project-local** |
| User-facing copy / UI text / landing / pricing / new product feature | **karpathy → assp → impeccable + taste** |
| Frontend design / redesign / UI build / audit | **karpathy → impeccable → taste** (+ assp for any copy) |
| Review UI / PDF / diff | karpathy → **image-routing** before fetching |
| Inside an 8sync repo | CORE always-on (đọc ngay) + specialist/on-demand khi khớp + `agents/*.md` (STATE đầu phiên) |
| Simple one-liner question | codegraph if codebase-related, else karpathy |

## Invariants (no exceptions)

- **NEVER skip code intelligence (codegraph + codebase-memory-mcp) for code exploration.** Grep / Read-all wastes 10–100× tokens.
- **NEVER skip karpathy or ponytail.** Engineering discipline + YAGNI (do the least that works, delete > add) is non-negotiable.
- **Building UI / redesign / any frontend?** `impeccable` is THE house design system — mandatory, with `references/house/*` (workflow + clouds-f). Pair with `assp` (brand voice/offer) for copy and `taste` (anti-slop). Shipping UI without impeccable is a violation.
- **NEVER skip 8sync-cli** when AGENTS.md mentions 8sync — using raw shell instead of `8sync` verbs misses memory + skill auto-load.
- **Project-local skill in `agents/skills/<name>/` matches the task description?** Read it BEFORE touching code.
- **Cite code as `path:line` or `path:start-end`.** Never natural language ("around line 50").
- **Output > ~50 dòng → BẮT BUỘC `headroom_compress`** trước khi vào context; không dump thô. Giữ artifact ID để retrieve.
- **After every change:** update `CHANGELOG.md` (Unreleased) + record what you learned in `agents/KNOWLEDGE.md`.

## 🔁 Loop engineering — operate as a designed loop, not one-off prompts

Inspired by Addy Osmani / Boris Cherny "loop engineering" (github.com/cobusgreyling/loop-engineering). The 8sync harness IS the loop; operate accordingly:

- **STATE spine + recitation** — `agents/STATE.md` (kế hoạch sống: Goal · DoD · Checklist · Current · Next · Open-questions) + `agents/KNOWLEDGE.md` (validated learnings) là spine bền ngoài chat. **Đọc STATE đầu phiên; rewrite STATE ở MỖI phase-boundary** (gạch việc xong, ghi bước kế) — đẩy kế hoạch xuống cuối context để bias attention, chống lost-in-the-middle.
- **Compaction (context gần đầy)** — đừng để bị cắt cụt: chủ động ghi handoff có cấu trúc vào STATE.md (Done · In-flight · Next · Open-questions) + bài học `validated:`/`failure:` vào KNOWLEDGE (dùng `headroom_compress` để tóm), rồi reinit phiên mới chỉ đọc spine + STATE.
- **Budget awareness** — ước lượng context còn lại; compact/handoff CHỦ ĐỘNG trước khi tràn, đừng đợi mất việc.
- **Maker / checker (C)** — tách bằng `task` sub-agents: *implementer* (own context) làm, *verifier* (own context) chạy build/test/benchmark ĐỘC LẬP, trả `validated|failed` + log đã `headroom_compress`. Orchestrator giao mỗi subagent **objective + boundaries + output-format** rõ (chống trùng việc/sót). Việc phụ thuộc → share full trace; chỉ parallel khi subtask độc lập. Never self-approve việc rủi ro/không đảo ngược.
- **Verify-gate + Reflexion (C)** — learning là `validated:` chỉ khi test/build/benchmark xác nhận; chưa thì `hypothesis:`. **FAIL → ghi `failure:`** vào KNOWLEDGE (triệu chứng + nguyên nhân + cách sửa); đầu phiên đọc các `failure:` gần nhất trước để KHÔNG lặp lỗi.
- **Procedural memory / playbooks (D)** — quy trình đa bước đã `validated:` → distill thành runbook tái dùng trong `agents/PLAYBOOKS.md`, index theo dòng **When:** (mô tả tình huống). Lần sau gặp tương tự → retrieve + adapt thay vì suy luận lại (Voyager skill-library). Tầng memory: KNOWLEDGE = bài học lời · PLAYBOOKS = quy trình đã verify · DECISIONS = ADR.
- **Phased autonomy + guardrails (E)** — **L1** report-only · **L2** assisted (đưa diff, chờ duyệt) · **L3** unattended (chỉ khi bật tường minh). Mọi level: **verify-gate TRƯỚC commit**, gitleaks chặn secret, scope commit chỉ `agents/`+docs trừ khi cho phép, **KHÔNG tự `push`/PR ở L3 mặc định**. `8sync harness up --timer <dur>` chạy loop nền; mỗi tick: đọc STATE → chọn `Next` → (L2+) làm → verify → cập nhật STATE/KNOWLEDGE → (`--commit`) commit. **Đo loop: `8sync harness bench`** trước/sau mỗi thay đổi.
- **Senses + hands** — code-intelligence (codegraph + codebase-memory-mcp) are the loop's senses; STATE/KNOWLEDGE its memory; `task` sub-agents its hands; `harness` keeps them current.
