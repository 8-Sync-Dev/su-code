# 8sync — always-on operating directives (managed by `8sync harness`; appended to EVERY system prompt)

Non-negotiable. They apply on EVERY turn, never compact away — even past 50% context.

## RULE #0 — code intelligence BEFORE native search / file CRUD
Use these token-optimized engines first; fall back to grep/find/Read ONLY when they cannot answer:
- **codegraph** (local graph) — where is X · who calls X · impact. `~/.omp/skills/codegraph/SKILL.md`
- **codebase-memory-mcp** — search_graph · trace_path · get_architecture (158 langs, sub-ms).
- **serena** (MCP) — LSP symbol find + precise symbol-level edits; prefer over blind whole-file rewrites.
- **headroom** (MCP) — `headroom_compress` EVERY tool output > ~50 lines BEFORE it enters context.
Reaching for grep/find/Read to EXPLORE first is a violation. Read a raw file only when about to edit it.

## Always-on skills — open the SKILL.md before acting (these EXIST; never reinvent them)
- **codegraph** — `~/.omp/skills/codegraph/SKILL.md` — semantic code intel (the loop's senses).
- **karpathy-guidelines** — `~/.omp/skills/karpathy-guidelines/SKILL.md` — read-before-write, test-before-refactor, small steps.
- **ponytail** — `~/.omp/skills/ponytail/SKILL.md` — laziest senior dev: YAGNI, do the least that works, delete > add.
- **8sync-cli** — `~/.omp/skills/8sync-cli/SKILL.md` — prefer `8sync` verbs over raw shell.
Specialist (open the body only when the task matches): **impeccable** (UI/design — mandatory for any frontend), **assp** (copy/brand), **taste** (anti-slop), **image-routing** (image/PDF/diff).

## Memory + compaction
- `agents/STATE.md` is the live plan — read it first; rewrite it at every phase boundary.
- Record learnings in `agents/KNOWLEDGE.md` (`validated:` / `failure:`); after any change update `CHANGELOG.md`.
- Context auto-compacts at 50% (tune: `8sync harness compaction <pct>`); write a handoff into STATE before it fires.
