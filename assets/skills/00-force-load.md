# 00 — Force Load Skills (managed by `8sync skill sync`)

## 🔴 RULE #0 — CODEGRAPH FIRST, ALWAYS

Before any other tool call in a session, run **`codegraph`** to answer codebase questions whenever the project has a `.codegraph/` directory (or one can be initialized). CodeGraph is a pre-indexed knowledge graph: ~35% cheaper, ~70% fewer tool calls than grep/find/Read for code exploration.

- **Default to `codegraph` queries** for: "how does X work", "where is X defined", "who calls X", "what depends on X", impact analysis, route → handler mapping (Django/Flask/FastAPI/Express/NestJS/Laravel/Rails/Spring/Gin/Axum/etc.).
- **Initialize once per repo** with `cd <repo> && codegraph init -i` if `.codegraph/` is missing.
- **Skill file**: `~/.omp/skills/codegraph/SKILL.md` (or `CLAUDE.md`) — read it.
- Falling back to `rg`/`fd`/`Read` without checking codegraph first is a **violation** of this rule on any non-trivial exploration.

## ⛔ MANDATORY READING ORDER — before any non-trivial task

Read these in order. No skipping, no skimming, no shortcuts.

1. **`~/.omp/skills/codegraph/SKILL.md`** (or `CLAUDE.md`) — semantic code intelligence. Always-on.
2. **`~/.omp/skills/karpathy-guidelines/SKILL.md`** — engineering discipline (read-before-write, test-before-refactor, small steps).
3. **`~/.omp/skills/8sync-cli/SKILL.md`** — you're running inside the 8sync harness; prefer 8sync verbs over raw shell.
4. **`~/.omp/skills/image-routing/SKILL.md`** — image vs text routing for cheap visual context.

If inside a project (cwd has `.git` / `Cargo.toml` / `package.json` / …):

5. **`<repo>/AGENTS.md`** — project-specific guidance. Note the `<!-- 8sync:skills:begin -->` … `<!-- 8sync:skills:end -->` block listing project-local skills under `<repo>/agents/skills/<name>/`. Read those that match the task.
6. **`<repo>/agents/{PROJECT,KNOWLEDGE,DECISIONS,PREFERENCES,STATE}.md`** — accumulated project memory.

## Fast lookup table

| Task type | Order to read |
|---|---|
| ANY code exploration (how does X work? where is X?) | **codegraph → karpathy → 8sync-cli → project-local** |
| Refactor / impact analysis | **codegraph (callers/callees) → karpathy → project-local** |
| Review UI / PDF / diff | karpathy → **image-routing** before fetching |
| Inside an 8sync repo | all 4 globals + `agents/*.md` + `agents/skills/*/` |
| Simple one-liner question | codegraph if codebase-related, else karpathy |

## Invariants (no exceptions)

- **NEVER skip codegraph for code exploration.** It exists because grep wastes 3-10× tokens.
- **NEVER skip karpathy.** Engineering discipline is non-negotiable.
- **NEVER skip 8sync-cli** when AGENTS.md mentions 8sync — using raw shell instead of `8sync` verbs misses memory + skill auto-load.
- **Project-local skill in `agents/skills/<name>/` matches the task description?** Read it BEFORE touching code.
- **Cite code as `path:line` or `path:start-end`.** Never natural language ("around line 50").
- **Never dump long tool output** into context. Summarize, then keep the artifact ID for retrieval.
