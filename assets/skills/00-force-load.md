# 00 — Force Load Skills (managed by `8sync harness init`)

## 🔴 RULE #0 — CODE INTELLIGENCE FIRST, ALWAYS (codegraph + codebase-memory-mcp)

Before any other tool call, answer codebase questions with a code-intelligence engine — NOT grep/find/Read. Both are ~99% cheaper than file-by-file exploration:

- **codegraph** (local pre-indexed graph): `codegraph init -i` once per repo (if `.codegraph/` missing), then `codegraph search/deps/callers/defs`. Skill: `~/.omp/skills/codegraph/SKILL.md`.
- **codebase-memory-mcp** (MCP server — auto-set-up by `8sync harness`; 158 languages, sub-ms, auto-indexes on connect): `search_graph`, `semantic_query`, `trace_path`, `get_architecture`, `detect_changes`, `query_graph`, `get_code_snippet`, `manage_adr`.
- **Default to these** for "how does X work / where is X / who calls X / what depends on X", impact analysis, route→handler, dead code, architecture.
- **Output dài** (logs / diffs / test output / tool dumps) → nén bằng `headroom` MCP (`headroom_compress`) thay vì dump nguyên khối — 60–95% ít token (auto-set-up bởi `8sync harness`).
- Only `Read` a raw file when you're about to edit it (read-before-edit). Falling back to `rg`/`fd`/`Read` for exploration first is a **violation**.

## ⛔ MANDATORY READING ORDER — before any non-trivial task

Read these in order. No skipping, no skimming, no shortcuts. Order = priority (top-down): codegraph → karpathy → ponytail → assp → impeccable + taste → 8sync-cli → image-routing.

1. **`~/.omp/skills/codegraph/SKILL.md`** (or `CLAUDE.md`) — semantic code intelligence. Always-on.
2. **`~/.omp/skills/karpathy-guidelines/SKILL.md`** — engineering discipline (read-before-write, test-before-refactor, small steps).
3. **`~/.omp/skills/ponytail/SKILL.md`** — "laziest senior dev": YAGNI, do the least that works, delete > add, stdlib before deps.
4. **`~/.omp/skills/assp-skill/SKILL.md`** — 8 Sync Dev brand DNA + ASSP validate-before-build framework. Mandatory for any user-facing copy (UI microcopy, landing/pricing, emails, errors) and before greenlighting a new product feature.
5. **`~/.omp/skills/impeccable/SKILL.md`** — **design system chuẩn của 8 Sync Dev: BẮT BUỘC cho MỌI design/redesign/audit/UI.** Run its Setup (`scripts/context.mjs`) first; auto-reference `references/house/*` (frontend-agent-workflow, clouds-f orchestration, keyword routers).
6. **`~/.omp/skills/taste-skill/SKILL.md`** — anti-slop frontend taste for landing pages, portfolios, redesigns (brief inference, the three dials, bias correction).
7. **`~/.omp/skills/8sync-cli/SKILL.md`** — you're running inside the 8sync harness; prefer 8sync verbs over raw shell.
8. **`~/.omp/skills/image-routing/SKILL.md`** — image vs text routing for cheap visual context.

On-demand (read only when the task matches): `code-review-and-quality`, `senior-security`, `senior-frontend`, `full-flow`, `last30days`; `encore-deploy` (only when the project uses Encore); `social-growth` (opt-in — enable with `8sync skill add builtin:social-growth`).

If inside a project (cwd has `.git` / `Cargo.toml` / `package.json` / …):

8. **`<repo>/AGENTS.md`** — project-specific guidance. Note the `<!-- 8sync:skills:begin -->` … `<!-- 8sync:skills:end -->` block listing project-local skills under `<repo>/agents/skills/<name>/`. Read those that match the task.
9. **`<repo>/agents/{PROJECT,KNOWLEDGE,DECISIONS,PREFERENCES,STATE}.md`** — accumulated project memory.

## Fast lookup table

| Task type | Order to read |
|---|---|
| ANY code exploration (how does X work? where is X?) | **codegraph → karpathy → 8sync-cli → project-local** |
| Refactor / impact analysis | **codegraph (callers/callees) → karpathy → project-local** |
| User-facing copy / UI text / landing / pricing / new product feature | **karpathy → assp → impeccable + taste** |
| Frontend design / redesign / UI build / audit | **karpathy → impeccable → taste** (+ assp for any copy) |
| Review UI / PDF / diff | karpathy → **image-routing** before fetching |
| Inside an 8sync repo | all 8 always-on + `agents/*.md` + `agents/skills/*/` |
| Simple one-liner question | codegraph if codebase-related, else karpathy |

## Invariants (no exceptions)

- **NEVER skip code intelligence (codegraph + codebase-memory-mcp) for code exploration.** Grep / Read-all wastes 10–100× tokens.
- **NEVER skip karpathy or ponytail.** Engineering discipline + YAGNI (do the least that works, delete > add) is non-negotiable.
- **Building UI / redesign / any frontend?** `impeccable` is THE house design system — mandatory, with `references/house/*` (workflow + clouds-f). Pair with `assp` (brand voice/offer) for copy and `taste` (anti-slop). Shipping UI without impeccable is a violation.
- **NEVER skip 8sync-cli** when AGENTS.md mentions 8sync — using raw shell instead of `8sync` verbs misses memory + skill auto-load.
- **Project-local skill in `agents/skills/<name>/` matches the task description?** Read it BEFORE touching code.
- **Cite code as `path:line` or `path:start-end`.** Never natural language ("around line 50").
- **Never dump long tool output** into context. Summarize, then keep the artifact ID for retrieval.
- **After every change:** update `CHANGELOG.md` (Unreleased) + record what you learned in `agents/KNOWLEDGE.md`.

## 🔁 Loop engineering — operate as a designed loop, not one-off prompts

Inspired by Addy Osmani / Boris Cherny "loop engineering" (github.com/cobusgreyling/loop-engineering). The 8sync harness IS the loop; operate accordingly:

- **Memory / STATE spine** — `agents/STATE.md` (work in flight) + `agents/KNOWLEDGE.md` (validated learnings) are the durable spine outside any chat. Read at start, update at end.
- **Maker / checker** — use `task` sub-agents to split implement vs verify; never self-approve risky or irreversible work.
- **Verify-gate** — a learning is `validated:` only when a test/build/benchmark confirms it; otherwise `hypothesis:`.
- **Phased autonomy** — L1 report → L2 assisted fixes → L3 unattended. `8sync harness up --timer <dur>` schedules the loop in the background.
- **Senses + hands** — code-intelligence (codegraph + codebase-memory-mcp) are the loop's senses; STATE/KNOWLEDGE its memory; `task` sub-agents its hands; `harness` keeps them current.
