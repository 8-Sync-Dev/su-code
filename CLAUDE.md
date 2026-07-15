# CLAUDE.md — guidance for Claude Code

<!-- 8sync:skills:begin -->
## 🚨 STEP 0 — CODE INTELLIGENCE FIRST (codegraph + codebase-memory-mcp; bắt buộc)

Mọi câu hỏi về code → dùng code-intelligence engine TRƯỚC grep/read (tiết kiệm ~99% token). Bạn (AI) **PHẢI**:

1. **codegraph** (local index): `codegraph index .` 1 lần/session; rồi `codegraph query/explore/node/callers/callees/impact` thay cho `grep`/`rg`/`fd`/`Grep`/`Glob` và `Read` toàn file.
2. **codebase-memory-mcp** (MCP, LUÔN có trong tool list — gọi đúng tên đăng ký): `mcp__codebase_memory_mcp_search_graph`, `_trace_path`, `_get_architecture`, `_get_code_snippet` — knowledge graph 158 ngôn ngữ, query sub-ms. Full catalog visible (`query_graph`, `detect_changes`, …); server khác/mới thêm → 1 lệnh `search_tool_bm25`.
3. Tìm/hiểu/định vị code · impact · route→handler · dead code · architecture → ƯU TIÊN 2 engine trên. Chỉ `Read` raw file khi sắp SỬA nó (read-before-edit). Serena LUÔN có trong tool list: `mcp__serena_find_symbol` / `mcp__serena_find_referencing_symbols` / `mcp__serena_get_symbols_overview`.
4. **Nén những gì BẠN phát lại:** báo cáo / subagent prompt / nội dung dài sắp re-emit → `mcp__headroom_compress` (60–95% ít token). omp tự spill output quá dài ra artifact — KHÔNG paste lại blob đã spill vào context.

Lý do: 5 query cấu trúc ≈ 3.4k token vs ≈ 412k token grep từng file (−99%). Dump cả file / grep mù = đốt token = bug.

## 🚨 STEP 1 — skills 2 tầng: CORE (đọc ngay) · SPECIALIST + on-demand (đọc khi cần)

Mỗi skill = 1 directory (Agent Skills open standard): `SKILL.md` có frontmatter `name`+`description`. Skill vendored ở `su-code/skills/<name>/` (bản commit trong repo, mirror từ `~/.omp/skills/`). Mỗi skill liệt kê 1 lần.

### ⛔ CORE always-on — ĐỌC NGAY (body), trước tool call đầu tiên (không skip)

Nhỏ + dùng cho MỌI task. **Thứ tự = ưu tiên (đọc top-down).** Mở `SKILL.md` ở path dưới rồi mới gọi tool đầu tiên:

  1. `/home/alexdev/Projects/tools/su-code/su-code/skills/codegraph/SKILL.md`
  2. `/home/alexdev/Projects/tools/su-code/su-code/skills/karpathy-guidelines/SKILL.md`
  3. `/home/alexdev/Projects/tools/su-code/su-code/skills/ponytail/SKILL.md`
  4. `/home/alexdev/Projects/tools/su-code/su-code/skills/8sync-cli/SKILL.md`

### 🧩 SPECIALIST always-on — biết khả năng, đọc body KHI task khớp (progressive disclosure)

KHÔNG đọc body mỗi phiên (giữ prefix gọn, tiết kiệm KV-cache). Khi task khớp → mở `SKILL.md` tương ứng NGAY. **`impeccable` = design system CHUẨN, BẮT BUỘC mở body ngay khi có việc UI/design/redesign/audit** (kèm `references/house/*`); `assp` cho copy/offer; `taste` chống slop; `image-routing` khi xử lý ảnh/diff/PDF.

- `assp-skill` — `/home/alexdev/Projects/tools/su-code/su-code/skills/assp-skill/SKILL.md`
- `impeccable` — `/home/alexdev/Projects/tools/su-code/su-code/skills/impeccable/SKILL.md`
- `design-taste-frontend` — `/home/alexdev/Projects/tools/su-code/su-code/skills/taste-skill/SKILL.md`
- `image-routing` — `/home/alexdev/Projects/tools/su-code/su-code/skills/image-routing/SKILL.md`
- `locate-anything` — `/home/alexdev/Projects/tools/su-code/su-code/skills/locate-anything/SKILL.md`

### 🔎 On-demand — tên = trigger; mở `SKILL.md` của skill khi task khớp (mô tả ở frontmatter, KHÔNG nhồi ở đây)

- `alpha-research` — `su-code/skills/alpha-research/SKILL.md`
- `api-and-interface-design` — `su-code/skills/api-and-interface-design/SKILL.md`
- `autoresearch` — `su-code/skills/autoresearch/SKILL.md`
- `browser-testing-with-devtools` — `su-code/skills/browser-testing-with-devtools/SKILL.md`
- `ci-cd-and-automation` — `su-code/skills/ci-cd-and-automation/SKILL.md`
- `code-review-and-quality` — `su-code/skills/code-review-and-quality/SKILL.md`
- `code-simplification` — `su-code/skills/code-simplification/SKILL.md`
- `context-engineering` — `su-code/skills/context-engineering/SKILL.md`
- `debugging-and-error-recovery` — `su-code/skills/debugging-and-error-recovery/SKILL.md`
- `deep-research` — `su-code/skills/deep-research/SKILL.md`
- `deprecation-and-migration` — `su-code/skills/deprecation-and-migration/SKILL.md`
- `docker` — `su-code/skills/docker/SKILL.md`
- `documentation-and-adrs` — `su-code/skills/documentation-and-adrs/SKILL.md`
- `doubt-driven-development` — `su-code/skills/doubt-driven-development/SKILL.md`
- `eli5` — `su-code/skills/eli5/SKILL.md`
- `feature` — `su-code/skills/feature/SKILL.md`
- `frontend-ui-engineering` — `su-code/skills/frontend-ui-engineering/SKILL.md`
- `full-flow` — `su-code/skills/full-flow/SKILL.md`
- `git-workflow-and-versioning` — `su-code/skills/git-workflow-and-versioning/SKILL.md`
- `idea-refine` — `su-code/skills/idea-refine/SKILL.md`
- `incremental-implementation` — `su-code/skills/incremental-implementation/SKILL.md`
- `interview-me` — `su-code/skills/interview-me/SKILL.md`
- `jobs` — `su-code/skills/jobs/SKILL.md`
- `last30days` — `su-code/skills/last30days/SKILL.md`
- `literature-review` — `su-code/skills/literature-review/SKILL.md`
- `ml-training-recipe` — `su-code/skills/ml-training-recipe/SKILL.md`
- `modal-compute` — `su-code/skills/modal-compute/SKILL.md`
- `observability-and-instrumentation` — `su-code/skills/observability-and-instrumentation/SKILL.md`
- `paper-code-audit` — `su-code/skills/paper-code-audit/SKILL.md`
- `paper-writing` — `su-code/skills/paper-writing/SKILL.md`
- `performance-optimization` — `su-code/skills/performance-optimization/SKILL.md`
- `planning-and-task-breakdown` — `su-code/skills/planning-and-task-breakdown/SKILL.md`
- `ponytail-audit` — `su-code/skills/ponytail-audit/SKILL.md`
- `ponytail-debt` — `su-code/skills/ponytail-debt/SKILL.md`
- `ponytail-gain` — `su-code/skills/ponytail-gain/SKILL.md`
- `ponytail-help` — `su-code/skills/ponytail-help/SKILL.md`
- `ponytail-review` — `su-code/skills/ponytail-review/SKILL.md`
- `preview` — `su-code/skills/preview/SKILL.md`
- `replication` — `su-code/skills/replication/SKILL.md`
- `research-review` — `su-code/skills/research-review/SKILL.md`
- `runpod-compute` — `su-code/skills/runpod-compute/SKILL.md`
- `security-and-hardening` — `su-code/skills/security-and-hardening/SKILL.md`
- `senior-frontend` — `su-code/skills/senior-frontend/SKILL.md`
- `senior-security` — `su-code/skills/senior-security/SKILL.md`
- `session-log` — `su-code/skills/session-log/SKILL.md`
- `session-search` — `su-code/skills/session-search/SKILL.md`
- `shipping-and-launch` — `su-code/skills/shipping-and-launch/SKILL.md`
- `social-growth` — `su-code/skills/social-growth/SKILL.md`
- `source-comparison` — `su-code/skills/source-comparison/SKILL.md`
- `source-driven-development` — `su-code/skills/source-driven-development/SKILL.md`
- `spec-driven-development` — `su-code/skills/spec-driven-development/SKILL.md`
- `test-driven-development` — `su-code/skills/test-driven-development/SKILL.md`
- `token-bench` — `su-code/skills/token-bench/SKILL.md`
- `using-agent-skills` — `su-code/skills/using-agent-skills/SKILL.md`
- `watch` — `su-code/skills/watch/SKILL.md`
- `zai-vision` — `su-code/skills/zai-vision/SKILL.md`

### Quy tắc bất biến

- **Code-intelligence FIRST** (codegraph + codebase-memory-mcp) cho mọi câu hỏi explore code (Step 0). Bypass = bug.
- **Output > ~50 dòng → BẮT BUỘC `headroom_compress`** trước khi vào context — không dump thô.
- Đọc body **CORE** (codegraph → karpathy → ponytail → 8sync-cli) TRƯỚC tool call đầu tiên. **SPECIALIST** (assp · impeccable · taste · image-routing) đọc body KHI task khớp — `impeccable` bắt buộc ngay khi có việc UI/design.
- Skill **on-demand**: chỉ mở khi description khớp task hiện tại — đừng đọc thừa.
- Nếu skill có `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.
- Khi áp dụng skill, **cite** rõ: ví dụ `su-code/skills/<name>/SKILL.md:line`.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (mục Unreleased) + ghi học được vào `su-code/KNOWLEDGE.md`.
- **Doc-hygiene**: chạy `8sync harness audit` khi đụng vùng có docs — path lệch→fix, doc rác/superseded→xóa (thêm doc phải kèm xóa cái cũ), oversized→trim.
- **Loop / STATE spine**: đọc `su-code/STATE.md` đầu phiên; rewrite ở mỗi phase-boundary (Goal·Checklist·Current·Next). Context gần đầy → handoff vào STATE + bài học vào KNOWLEDGE rồi reinit. Đo loop: `8sync harness bench`.
- **Loop discipline (C/D/E)**: implementer↔verifier qua `task` (verifier chạy build/test ĐỘC LẬP, verify-gate TRƯỚC commit); FAIL → ghi `failure:` vào KNOWLEDGE, đọc đầu phiên để khỏi lặp; quy trình `validated:` → distill vào `su-code/PLAYBOOKS.md` (index theo `When:`); autonomy L1 report · L2 assisted · L3 unattended — không tự `push`/PR ở L3 mặc định.
<!-- 8sync:skills:end -->
