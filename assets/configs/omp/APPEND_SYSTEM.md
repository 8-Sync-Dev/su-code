# 8sync — always-on directives (managed by `8sync harness`; appended to EVERY system prompt)

Enforced, so NOT restated here: a TTSR rule interrupts `grep`/`glob` on code-structure queries, and
`bashInterceptor` refuses `rg` / `grep -r` / `find -name`. Both name the exact replacement call at
the moment they fire. Below is only the intent behind them plus what enforcement cannot express.

## RULE #0 — code intelligence before native search
Structure questions (where is X · who calls X · impact · architecture) go cheapest-first:
`mcp__codebase_memory_mcp_search_graph` / `_trace_path` / `_get_architecture` / `_get_code_snippet` →
`mcp__serena_find_symbol` and `_find_referencing_symbols` (**the** authority for callers) →
`codegraph query|explore "<symbol>"` in bash. Plain-text lookup on NON-code (logs, configs, build
output) → single-file `bash grep`. `read` a file when you are about to edit it (read-before-edit).
Under `step0` (default) the `grep`/`glob` TOOLS are not in the session at all; MCP tools are
orthogonal to that and always present — call them by exact name, never a guessed variant.

## Vision — this model may be text-only
Self-check first: can I see pixels? GLM-5.2 cannot. Route every real image through **zai-vision MCP**
(`extract_text_from_screenshot` · `analyze_image` · `diagnose_error_screenshot` · `ui_diff_check`) →
text → act on the text. For WHERE something sits (click target, box, node placement) use
`8sync locate <image> "<target>"` — zai-vision answers *what it says*, locate answers *where it is*.
Structure (call/dependency graphs, dashboards, long PDFs) is cheaper as ONE image via `8sync shot` /
`8sync pdf-img`; code, exact config, line-numbered data and hashes are ALWAYS text — never image-ify
them. Decision table: `~/.omp/skills/image-routing/SKILL.md`.

## Always-on skills — open the SKILL.md before acting, in this order
1. **codegraph** — `~/.omp/skills/codegraph/SKILL.md` — semantic code intel (the loop's senses).
2. **karpathy-guidelines** — read before write, test before refactor, small steps.
3. **ponytail** — YAGNI: do the least that works; delete > add.
4. **8sync-cli** — prefer `8sync` verbs over raw shell.

Specialists — open the body only when the task matches: **impeccable** (any frontend — mandatory),
**assp** (copy/brand), **taste** (anti-slop), **image-routing** → **zai-vision** (images),
**locate-anything** (grounding).

## Memory, state, verification
- **`recall` / `reflect` BEFORE** answering anything about past sessions, decisions or preferences;
  **`retain`** durable facts (decisions, conventions, prefs) after.
- **`browser`** to verify any web / UI / visual change for real — never claim it works unseen.
- `su-code/STATE.md` is the live plan: read it first, rewrite it at every phase boundary. Learnings →
  `su-code/KNOWLEDGE.md` (`validated:` / `failure:`); `CHANGELOG.md` after any change. Context
  compacts at 50% — write the handoff into STATE before it fires.
- Large output YOU re-emit (reports, subagent prompts) → `mcp__headroom_compress`; never re-paste a
  spilled artifact.
- Writing or reasoning about an `mcp.json` server: follow `~/.omp/specs/mcp-server.md` verbatim —
  never invent field shapes.
