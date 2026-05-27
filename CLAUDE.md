# CLAUDE.md — guidance for Claude Code

<!-- 8sync:skills:begin -->
## 🚨 STEP 0 — `codegraph` FIRST (mandatory, no exception)

`codegraph` là **core tool** cho mọi câu hỏi liên quan đến code trong repo này. Bạn (AI) **PHẢI**:

1. Chạy `codegraph index .` **1 lần** đầu session để build/refresh semantic index.
2. Dùng `codegraph search "<query>"` thay cho `grep`/`rg`/`fd`/`Grep`/`Glob`.
3. Dùng `codegraph deps <file>` thay cho `Read` toàn file để hiểu dependency graph.
4. Dùng `codegraph callers <symbol>` / `codegraph defs <symbol>` thay cho find-references thủ công.

Lý do: ~35% rẻ hơn token, ~70% ít tool call hơn, 100% local. Dump cả file = đốt token vô ích.

## 🚨 STEP 1 — đọc TẤT CẢ skill dưới TRƯỚC khi gọi tool đầu tiên

Không skip, không suy đoán, không viết tắt. AGENTS.md chỉ là index — nội dung thực ở các `SKILL.md` được liệt kê.

**READ NOW (in order). Do NOT skip. Open each file BEFORE the first tool call:**

  1. `/home/alexdev/.omp/skills/codegraph/SKILL.md`
  2. `/home/alexdev/.omp/skills/8sync-cli/SKILL.md`
  3. `/home/alexdev/.omp/skills/image-routing/SKILL.md`
  4. `/home/alexdev/.omp/skills/karpathy-guidelines/SKILL.md`
  5. `/home/alexdev/Projects/su-code/agents/skills/codegraph/SKILL.md`
  6. `/home/alexdev/Projects/su-code/agents/skills/8sync-cli/SKILL.md`
  7. `/home/alexdev/Projects/su-code/agents/skills/image-routing/SKILL.md`
  8. `/home/alexdev/Projects/su-code/agents/skills/karpathy-guidelines/SKILL.md`

Mỗi skill là 1 directory theo [Agent Skills open standard](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview): `SKILL.md` ở root có YAML frontmatter (`name`, `description`). Description cho biết **khi nào** dùng skill.

### Global skills (always-on — `~/.omp/skills/`)
1. **`codegraph`** — `~/.omp/skills/codegraph/SKILL.md`
     _Use this skill when the user mentions codegraph or related concepts. ~35% cheaper · ~70% fewer tool calls · 100% local_
2. **`8sync-cli`** — `~/.omp/skills/8sync-cli/SKILL.md`
     _Use this skill in EVERY session inside a repo whose AGENTS.md mentions 8sync. It teaches the AI which 8sync verbs (shot/diff-img/pdf-img/find/note/ship/skill/run) to use instead of raw shell equivalents — saving 3-10× tokens and keeping session memory in agents/* consistent. The AI MUST prefer the listed 8sync verbs over rg/fd/git/curl/etc when an equivalent exists._
3. **`image-routing`** — `~/.omp/skills/image-routing/SKILL.md`
     _Use this skill on EVERY read request to decide between text and image representation. Apply whenever the AI is about to open a PDF, screenshot a URL, review a UI, inspect a long git diff, or process diagrams — picking the wrong format wastes 3-10× tokens. The AI MUST consult the decision table here before issuing any read tool call on non-trivial content._
4. **`karpathy-guidelines`** — `~/.omp/skills/karpathy-guidelines/SKILL.md`
     _Use this skill before EVERY non-trivial coding task. It enforces Andrej Karpathy-style engineering discipline — read-before-write, test-before-refactor, small steps, boring-is-better, delete-more-than-you-add. Apply whenever the user asks for code, refactor, debug, or review work; the AI MUST cite a rule from this skill before claiming "done"._

### Project-local skills (BẮT BUỘC dùng cho repo này — `agents/skills/`)
1. **`codegraph`** — `agents/skills/codegraph/SKILL.md`
     _Use this skill when the user mentions codegraph or related concepts. ~35% cheaper · ~70% fewer tool calls · 100% local_
2. **`8sync-cli`** — `agents/skills/8sync-cli/SKILL.md`
     _Use this skill in EVERY session inside a repo whose AGENTS.md mentions 8sync. It teaches the AI which 8sync verbs (shot/diff-img/pdf-img/find/note/ship/skill/run) to use instead of raw shell equivalents — saving 3-10× tokens and keeping session memory in agents/* consistent. The AI MUST prefer the listed 8sync verbs over rg/fd/git/curl/etc when an equivalent exists._
3. **`image-routing`** — `agents/skills/image-routing/SKILL.md`
     _Use this skill on EVERY read request to decide between text and image representation. Apply whenever the AI is about to open a PDF, screenshot a URL, review a UI, inspect a long git diff, or process diagrams — picking the wrong format wastes 3-10× tokens. The AI MUST consult the decision table here before issuing any read tool call on non-trivial content._
4. **`karpathy-guidelines`** — `agents/skills/karpathy-guidelines/SKILL.md`
     _Use this skill before EVERY non-trivial coding task. It enforces Andrej Karpathy-style engineering discipline — read-before-write, test-before-refactor, small steps, boring-is-better, delete-more-than-you-add. Apply whenever the user asks for code, refactor, debug, or review work; the AI MUST cite a rule from this skill before claiming "done"._

### Quy tắc bất biến

- **`codegraph` FIRST** cho mọi câu hỏi explore code (Step 0). Bypass = bug.
- Đọc TẤT CẢ `SKILL.md` / `CLAUDE.md` ở 2 list trên TRƯỚC khi gọi tool đầu tiên.
- Nếu skill có `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.
- Nếu skill có `references/` → đọc on-demand khi task chạm chủ đề.
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.
- Nếu skill local có description match task hiện tại, bạn **MUST** đọc nó trước khi sửa code.
<!-- 8sync:skills:end -->
