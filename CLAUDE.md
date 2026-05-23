# CLAUDE.md — guidance for Claude Code

<!-- 8sync:skills:begin -->
## 🚨 MANDATORY — đọc TRƯỚC khi làm bất cứ task gì

Bạn (AI) **PHẢI** đọc đầy đủ các skill liệt kê dưới đây **trước dòng code đầu tiên** trong session này. Không skip, không suy đoán, không viết tắt.

**READ NOW (in order). Do NOT skip. Open each file BEFORE the first tool call:**

  1. `/home/alexdev/.omp/skills/codegraph/CLAUDE.md`
  2. `/home/alexdev/.omp/skills/8sync-cli/SKILL.md`
  3. `/home/alexdev/.omp/skills/image-routing/SKILL.md`
  4. `/home/alexdev/.omp/skills/karpathy-guidelines/SKILL.md`
  5. `/home/alexdev/Projects/su-code/agents/skills/codegraph/CLAUDE.md`
  6. `/home/alexdev/Projects/su-code/agents/skills/8sync-cli/SKILL.md`
  7. `/home/alexdev/Projects/su-code/agents/skills/image-routing/SKILL.md`
  8. `/home/alexdev/Projects/su-code/agents/skills/karpathy-guidelines/SKILL.md`

Mỗi skill là 1 directory theo [Agent Skills open standard](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview): `SKILL.md` ở root có YAML frontmatter (`name`, `description`). Description cho biết **khi nào** dùng skill.

### Global skills (always-on — `~/.omp/skills/`)
1. **`codegraph`** — `~/.omp/skills/codegraph/CLAUDE.md`
     _Claude-Code-style skill — entrypoint: CLAUDE.md (no Agent-Skills SKILL.md)_
2. **`8sync-cli`** — `~/.omp/skills/8sync-cli/SKILL.md`
     _Use this skill in EVERY session inside a repo whose AGENTS.md mentions 8sync. It teaches the AI which 8sync verbs (shot/diff-img/pdf-img/find/note/ship/skill/run) to use instead of raw shell equivalents — saving 3-10× tokens and keeping session memory in agents/* consistent. The AI MUST prefer the listed 8sync verbs over rg/fd/git/curl/etc when an equivalent exists._
3. **`image-routing`** — `~/.omp/skills/image-routing/SKILL.md`
     _Use this skill on EVERY read request to decide between text and image representation. Apply whenever the AI is about to open a PDF, screenshot a URL, review a UI, inspect a long git diff, or process diagrams — picking the wrong format wastes 3-10× tokens. The AI MUST consult the decision table here before issuing any read tool call on non-trivial content._
4. **`karpathy-guidelines`** — `~/.omp/skills/karpathy-guidelines/SKILL.md`
     _Use this skill before EVERY non-trivial coding task. It enforces Andrej Karpathy-style engineering discipline — read-before-write, test-before-refactor, small steps, boring-is-better, delete-more-than-you-add. Apply whenever the user asks for code, refactor, debug, or review work; the AI MUST cite a rule from this skill before claiming "done"._

### Project-local skills (BẮT BUỘC dùng cho repo này — `agents/skills/`)
1. **`codegraph`** — `agents/skills/codegraph/CLAUDE.md`
     _Claude-Code-style skill — entrypoint: CLAUDE.md (no Agent-Skills SKILL.md)_
2. **`8sync-cli`** — `agents/skills/8sync-cli/SKILL.md`
     _Use this skill in EVERY session inside a repo whose AGENTS.md mentions 8sync. It teaches the AI which 8sync verbs (shot/diff-img/pdf-img/find/note/ship/skill/run) to use instead of raw shell equivalents — saving 3-10× tokens and keeping session memory in agents/* consistent. The AI MUST prefer the listed 8sync verbs over rg/fd/git/curl/etc when an equivalent exists._
3. **`image-routing`** — `agents/skills/image-routing/SKILL.md`
     _Use this skill on EVERY read request to decide between text and image representation. Apply whenever the AI is about to open a PDF, screenshot a URL, review a UI, inspect a long git diff, or process diagrams — picking the wrong format wastes 3-10× tokens. The AI MUST consult the decision table here before issuing any read tool call on non-trivial content._
4. **`karpathy-guidelines`** — `agents/skills/karpathy-guidelines/SKILL.md`
     _Use this skill before EVERY non-trivial coding task. It enforces Andrej Karpathy-style engineering discipline — read-before-write, test-before-refactor, small steps, boring-is-better, delete-more-than-you-add. Apply whenever the user asks for code, refactor, debug, or review work; the AI MUST cite a rule from this skill before claiming "done"._

### Quy tắc bất biến

- Đọc TẤT CẢ `SKILL.md` / `CLAUDE.md` ở 2 list trên **TRƯỚC** khi gọi tool đầu tiên.
- **Codegraph FIRST** cho mọi câu hỏi explore code: `codegraph` thay vì grep/find/Read.
- Nếu skill có thư mục `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.
- Nếu skill có `references/` → đọc on-demand khi task chạm vào chủ đề tương ứng.
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.
- Nếu một skill local có vẻ liên quan đến task hiện tại (theo description), bạn **MUST** đọc nó trước khi sửa code.
<!-- 8sync:skills:end -->
