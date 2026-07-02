# CLAUDE.md — guidance for Claude Code

<!-- 8sync:skills:begin -->
## 🚨 STEP 0 — CODE INTELLIGENCE FIRST (codegraph + codebase-memory-mcp; bắt buộc)

Mọi câu hỏi về code → dùng code-intelligence engine TRƯỚC grep/read (tiết kiệm ~99% token). Bạn (AI) **PHẢI**:

1. **codegraph** (local index): `codegraph index .` 1 lần/session; rồi `codegraph query/callers/callees/impact` thay cho `grep`/`rg`/`fd`/`Grep`/`Glob` và `Read` toàn file.
2. **codebase-memory-mcp** (MCP, auto-setup bởi `8sync harness`): `search_graph`, `semantic_query`, `trace_path`, `get_architecture`, `detect_changes`, `query_graph`, `get_code_snippet` — knowledge graph 158 ngôn ngữ, query sub-ms.
3. Tìm/hiểu/định vị code · impact · route→handler · dead code · architecture → ƯU TIÊN 2 engine trên. Chỉ `Read` raw file khi sắp SỬA nó (read-before-edit).
4. **BẮT BUỘC nén output lớn:** mọi output > ~50 dòng (log/diff/test/tool dump) phải qua `headroom` MCP (`headroom_compress`) TRƯỚC khi vào context (60–95% ít token). Dump cả khối lớn vào context = vi phạm.

Lý do: 5 query cấu trúc ≈ 3.4k token vs ≈ 412k token grep từng file (−99%). Dump cả file / grep mù = đốt token = bug.

## 🚨 STEP 1 — skills 2 tầng: CORE (đọc ngay) · SPECIALIST + on-demand (đọc khi cần)

Mỗi skill = 1 directory (Agent Skills open standard): `SKILL.md` có frontmatter `name`+`description`. Skill vendored ở `agents/skills/<name>/` (bản commit trong repo, mirror từ `~/.omp/skills/`). Mỗi skill liệt kê 1 lần.

### ⛔ CORE always-on — ĐỌC NGAY (body), trước tool call đầu tiên (không skip)

Nhỏ + dùng cho MỌI task. **Thứ tự = ưu tiên (đọc top-down).** Mở `SKILL.md` ở path dưới rồi mới gọi tool đầu tiên:

  1. `/home/alexdev/Projects/tools/su-code/agents/skills/codegraph/SKILL.md`
  2. `/home/alexdev/Projects/tools/su-code/agents/skills/karpathy-guidelines/SKILL.md`
  3. `/home/alexdev/Projects/tools/su-code/agents/skills/ponytail/SKILL.md`
  4. `/home/alexdev/Projects/tools/su-code/agents/skills/8sync-cli/SKILL.md`

### 🧩 SPECIALIST always-on — biết khả năng, đọc body KHI task khớp (progressive disclosure)

KHÔNG đọc body mỗi phiên (giữ prefix gọn, tiết kiệm KV-cache). Khi task khớp → mở `SKILL.md` tương ứng NGAY. **`impeccable` = design system CHUẨN, BẮT BUỘC mở body ngay khi có việc UI/design/redesign/audit** (kèm `references/house/*`); `assp` cho copy/offer; `taste` chống slop; `image-routing` khi xử lý ảnh/diff/PDF.

- `assp-skill` — `/home/alexdev/Projects/tools/su-code/agents/skills/assp-skill/SKILL.md`
- `impeccable` — `/home/alexdev/Projects/tools/su-code/agents/skills/impeccable/SKILL.md`
- `design-taste-frontend` — `/home/alexdev/Projects/tools/su-code/agents/skills/taste-skill/SKILL.md`
- `image-routing` — `/home/alexdev/Projects/tools/su-code/agents/skills/image-routing/SKILL.md`

### 🔎 On-demand — tên = trigger; mở `SKILL.md` của skill khi task khớp (mô tả ở frontmatter, KHÔNG nhồi ở đây)

- `alpha-research` — `agents/skills/alpha-research/SKILL.md`
- `api-and-interface-design` — `agents/skills/api-and-interface-design/SKILL.md`
- `autoresearch` — `agents/skills/autoresearch/SKILL.md`
- `browser-testing-with-devtools` — `agents/skills/browser-testing-with-devtools/SKILL.md`
- `ci-cd-and-automation` — `agents/skills/ci-cd-and-automation/SKILL.md`
- `code-review-and-quality` — `agents/skills/code-review-and-quality/SKILL.md`
- `code-simplification` — `agents/skills/code-simplification/SKILL.md`
- `context-engineering` — `agents/skills/context-engineering/SKILL.md`
- `debugging-and-error-recovery` — `agents/skills/debugging-and-error-recovery/SKILL.md`
- `deep-research` — `agents/skills/deep-research/SKILL.md`
- `deprecation-and-migration` — `agents/skills/deprecation-and-migration/SKILL.md`
- `docker` — `agents/skills/docker/SKILL.md`
- `documentation-and-adrs` — `agents/skills/documentation-and-adrs/SKILL.md`
- `doubt-driven-development` — `agents/skills/doubt-driven-development/SKILL.md`
- `eli5` — `agents/skills/eli5/SKILL.md`
- `frontend-ui-engineering` — `agents/skills/frontend-ui-engineering/SKILL.md`
- `full-flow` — `agents/skills/full-flow/SKILL.md`
- `git-workflow-and-versioning` — `agents/skills/git-workflow-and-versioning/SKILL.md`
- `idea-refine` — `agents/skills/idea-refine/SKILL.md`
- `incremental-implementation` — `agents/skills/incremental-implementation/SKILL.md`
- `interview-me` — `agents/skills/interview-me/SKILL.md`
- `jobs` — `agents/skills/jobs/SKILL.md`
- `last30days` — `agents/skills/last30days/SKILL.md`
- `literature-review` — `agents/skills/literature-review/SKILL.md`
- `ml-training-recipe` — `agents/skills/ml-training-recipe/SKILL.md`
- `modal-compute` — `agents/skills/modal-compute/SKILL.md`
- `observability-and-instrumentation` — `agents/skills/observability-and-instrumentation/SKILL.md`
- `paper-code-audit` — `agents/skills/paper-code-audit/SKILL.md`
- `paper-writing` — `agents/skills/paper-writing/SKILL.md`
- `performance-optimization` — `agents/skills/performance-optimization/SKILL.md`
- `planning-and-task-breakdown` — `agents/skills/planning-and-task-breakdown/SKILL.md`
- `ponytail-audit` — `agents/skills/ponytail-audit/SKILL.md`
- `ponytail-debt` — `agents/skills/ponytail-debt/SKILL.md`
- `ponytail-gain` — `agents/skills/ponytail-gain/SKILL.md`
- `ponytail-help` — `agents/skills/ponytail-help/SKILL.md`
- `ponytail-review` — `agents/skills/ponytail-review/SKILL.md`
- `preview` — `agents/skills/preview/SKILL.md`
- `replication` — `agents/skills/replication/SKILL.md`
- `research-review` — `agents/skills/research-review/SKILL.md`
- `runpod-compute` — `agents/skills/runpod-compute/SKILL.md`
- `security-and-hardening` — `agents/skills/security-and-hardening/SKILL.md`
- `senior-frontend` — `agents/skills/senior-frontend/SKILL.md`
- `senior-security` — `agents/skills/senior-security/SKILL.md`
- `session-log` — `agents/skills/session-log/SKILL.md`
- `session-search` — `agents/skills/session-search/SKILL.md`
- `shipping-and-launch` — `agents/skills/shipping-and-launch/SKILL.md`
- `social-growth` — `agents/skills/social-growth/SKILL.md`
- `source-comparison` — `agents/skills/source-comparison/SKILL.md`
- `source-driven-development` — `agents/skills/source-driven-development/SKILL.md`
- `spec-driven-development` — `agents/skills/spec-driven-development/SKILL.md`
- `test-driven-development` — `agents/skills/test-driven-development/SKILL.md`
- `token-bench` — `agents/skills/token-bench/SKILL.md`
- `using-agent-skills` — `agents/skills/using-agent-skills/SKILL.md`
- `watch` — `agents/skills/watch/SKILL.md`
- `zai-vision` — `agents/skills/zai-vision/SKILL.md`

### Quy tắc bất biến

- **Code-intelligence FIRST** (codegraph + codebase-memory-mcp) cho mọi câu hỏi explore code (Step 0). Bypass = bug.
- **Output > ~50 dòng → BẮT BUỘC `headroom_compress`** trước khi vào context — không dump thô.
- Đọc body **CORE** (codegraph → karpathy → ponytail → 8sync-cli) TRƯỚC tool call đầu tiên. **SPECIALIST** (assp · impeccable · taste · image-routing) đọc body KHI task khớp — `impeccable` bắt buộc ngay khi có việc UI/design.
- Skill **on-demand**: chỉ mở khi description khớp task hiện tại — đừng đọc thừa.
- Nếu skill có `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (mục Unreleased) + ghi học được vào `agents/KNOWLEDGE.md`.
- **Doc-hygiene**: chạy `8sync harness audit` khi đụng vùng có docs — path lệch→fix, doc rác/superseded→xóa (thêm doc phải kèm xóa cái cũ), oversized→trim.
- **Loop / STATE spine**: đọc `agents/STATE.md` đầu phiên; rewrite ở mỗi phase-boundary (Goal·Checklist·Current·Next). Context gần đầy → handoff vào STATE + bài học vào KNOWLEDGE rồi reinit. Đo loop: `8sync harness bench`.
- **Loop discipline (C/D/E)**: implementer↔verifier qua `task` (verifier chạy build/test ĐỘC LẬP, verify-gate TRƯỚC commit); FAIL → ghi `failure:` vào KNOWLEDGE, đọc đầu phiên để khỏi lặp; quy trình `validated:` → distill vào `agents/PLAYBOOKS.md` (index theo `When:`); autonomy L1 report · L2 assisted · L3 unattended — không tự `push`/PR ở L3 mặc định.
<!-- 8sync:skills:end -->
