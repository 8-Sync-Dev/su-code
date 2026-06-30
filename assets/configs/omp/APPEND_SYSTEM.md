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

## Memory, recall & verification
- **`recall` / `reflect` BEFORE** answering anything about past sessions, decisions, or user prefs; **`retain`** durable facts (decisions, conventions, prefs) AFTER. omp Mnemopi long-term memory — the recall hook also auto-injects the live skill index + STATE every turn.
- **`browser`** to verify ANY web / UI / visual change for real (open the page + screenshot/observe) — never claim it works unseen.
- `agents/STATE.md` is the live plan — read it first; rewrite at every phase boundary. Record learnings in `agents/KNOWLEDGE.md` (`validated:` / `failure:`); update `CHANGELOG.md` after any change.
- Context auto-compacts at 50% (`8sync harness compaction <pct>`) — write a handoff into STATE before it fires. This block is never compressed, so it stays terse by design; `headroom_compress` is for large tool OUTPUTS.
