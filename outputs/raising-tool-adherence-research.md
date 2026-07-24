# Research — Raising the optimizer tool-adherence metric (rules · skills · tool calls per request)

**Date:** 2026-06-29 · **Method:** deep-research (multi-source web + repo grounding) · **Scope:** how to make the omp agent actually *read the rules/skills and pick the token-optimized tools* (codegraph / codebase-memory-mcp / serena) instead of defaulting to grep / read / search. Measured baseline (`8sync harness toolstats`): optimizer = **25–34% of code-lookup calls**; serena/headroom ≈ 0.

---

## TL;DR — the one thing that actually works

**Prompts are suggestions, not constraints.** Every credible 2025–26 source says the same: rules in a system prompt or tool docstring are *interpreted text the model may ignore on any given call* — confirmed by our own data (8sync already injects RULE #0 into **every** system prompt via `APPEND_SYSTEM.md`, and the agent still uses the optimizer only ~25–34% of lookups).

The only lever proven to change tool-selection behaviour is **runtime interception** (a "neurosymbolic" hook that runs *before* the tool executes), and the right flavour for our case is **"steer, don't block"** — nudge the agent to self-correct (try codegraph first) rather than hard-fail. omp supports exactly this (`tool_call` pre-hook → `{ block, reason }`). Everything else (descriptions, fewer tools, reliability) is supporting cast.

---

## 1. Why the agent ignores the rules (root cause, multi-source)

- **Architectural, not a wording problem.** "The root cause is architectural: prompts are text that the LLM interprets. Business rules embedded in docstrings or system prompts become *suggestions, not constraints*. The model decides whether to follow them on every call." [AWS-guardrails; ATA arXiv:2510.16381]
- **Three bypass patterns** prompt-engineering cannot fix: parameter errors, completeness errors, and **tool-bypass behaviour** (the agent reaches a result without the mandated tool). [ATA]
- **Silent conflict resolution.** Coding-agent system prompts run 245–1490 lines; when sections conflict the model resolves it "silently through whatever heuristic its training provides, with no error raised." [Arbiter arXiv:2603.08993]
- **Free text ≠ execution.** The runtime drives tool calls from structured fields; procedure written in free text is not what selects tools. [system-prompts discussion #242]
- **Habit / familiarity bias + tool overload** (see §3): when the tool surface is large or ambiguous, the model falls back to the tools it "knows" (read/grep).

> Implication: making `APPEND_SYSTEM.md` / `00-force-load.md` longer or louder has **diminishing returns**. We are already at the strong-prose ceiling.

## 2. The proven fix — runtime hook, "steer don't block" (HIGHEST leverage)

- **Neurosymbolic enforcement.** A `BeforeToolCall` hook intercepts every call *before* execution and can cancel/replace it with a message "the LLM receives … [and] cannot override." Demo: **3/3 invalid ops blocked, 0 false positives, zero changes to tools or prompts** — just one hook. [AWS-guardrails; ATA]
- **Steer, don't block** (the variant that fits us): runtime controls that *guide the agent to self-correct* a sub-optimal action instead of failing the workflow. [AWS "Runtime Guardrails — Steer, Don't Block"]
- **omp already supports it.** `pi.on("tool_call", (event, ctx) => { … return { block: true, reason } })` in `~/.omp/hooks/pre/*.ts`; the `reason` becomes the text the model sees, and the underlying tool never runs (`omp://hooks.md`). Hooks can also persist state (`pi.appendEntry`) and inject per-turn messages (`before_agent_start`, `turn_start`).

**Design for 8sync (`.omp/hooks/pre/codegraph-first.ts`, deployed by `8sync harness`):**
- Trigger on `grep` / `search` / `find` / `glob` used for **code exploration** (skip when no `.codegraph/` index exists, or when the query is obviously non-code).
- **Steer once, then allow:** on the first such call (per session, or until codegraph/serena is used once) return `block` with a `reason`: *"Token rule: try `codegraph search <X>` / serena `find_symbol` first (≈99% cheaper). If it can't answer, re-run this exact search and it will pass."* Set a flag; subsequent calls pass → a nudge, not a wall (avoids false-positive friction the research warns about).
- Never gate `read` (read-before-edit is legitimate) — matches our `toolstats` categorisation.

This is the only mechanism that *deterministically* moves the metric; it's local, model-free, and reversible.

## 3. Supporting levers (Anthropic tool-use + Agent-Skills guidance)

| Lever | Evidence | 8sync action | Effort |
|---|---|---|---|
| **Sharper, "pushy" tool/skill descriptions** (what + *WHEN* + trigger keywords) | "Even small refinements to tool descriptions yield dramatic improvements" (SWE-bench SOTA); skills "undertrigger — make descriptions a little bit pushy" | codegraph is a bare `bash` CLI (no description → invisible to selection). Give it a first-class, pushy "use me for where-is/who-calls **instead of grep/read**" surface; tighten the codegraph SKILL.md description + force-load routing table | Low–Med |
| **Fewer / less-ambiguous tools** | "Selection degrades significantly past 30–50 tools"; "fewer, more capable tools reduce ambiguity"; "reduce enabled skills if >20–50" | Count omp's live tools (built-ins + serena 22 + cbm 7 + headroom…). If >30–50, the model defaults to familiar read/grep. Trim always-on skills; consider `defer_loading` / tool-search | Med |
| **Namespacing** (`codegraph_*`, `serena__*`) | "Meaningful namespacing makes selection unambiguous" | Keep MCP names namespaced; surface codegraph under a clear name | Low |
| **Tool reliability** | a tool that fails gets abandoned by the model | serena was *broken* (transport closed) until v0.29.3 → **explains serena = 0**. Fixed. Ensure codegraph/cbm 0-fail | Done/Med |
| **Tool-response quality** (concise, structured, paginated) | "optimise quantity of context returned … response structure affects eval performance" | Make codegraph/cbm output tight so the model is *rewarded* for using them | Med |
| **Few-shot example** in APPEND_SYSTEM | "agents pattern-match against exact examples" | Add ONE worked example: *"where is `foo`? → `codegraph search foo` (not grep)"* | Low |
| **`tool_choice` forcing** (`any`/`tool`) | API-level force | Per-call, can't be selectively applied in-session, and **incompatible with extended thinking** → not usable for "codegraph-before-grep" | n/a |
| **Measure + iterate** | "monitor how the agent uses skills, iterate on real trajectories" | `8sync harness toolstats` (built) is the feedback loop — re-measure after each lever | Done |

## 4. Recommended sequence (highest leverage first)

1. **Ship the steer-don't-block hook** (`.omp/hooks/pre/codegraph-first.ts`, deployed by `8sync harness`) — the only deterministic lever. Nudge-once semantics. **Measure with `toolstats` before/after.**
2. **Give codegraph a first-class, pushy description** (it's currently invisible `bash`); add a one-line worked example to `APPEND_SYSTEM.md`.
3. **Audit the live tool/skill count**; if >30–50, trim always-on + defer the rest (reduce ambiguity).
4. **Confirm reliability** (serena fixed; codegraph/cbm 0-fail) and **tighten tool-response size**.
5. **Re-measure** each step; keep only what moves the `toolstats` ratio. Prose stays as-is (ceiling reached).

## 5. Honest expectations

- The hook can *make* the agent try codegraph first (deterministic) — that genuinely raises the ratio. Descriptions/fewer-tools help the model *want* to. Prose alone won't.
- "Steer, don't block" risks false positives (a grep that was actually correct) → mitigate with nudge-once + index-exists gating + an env escape hatch.
- This is omp-extension work (TS) + measurement; treat it as a loop (ship → `toolstats` → adjust), not a one-shot fix.

---

## Sources (see `.provenance.md` for the full map)
- AWS/Strands — *AI Agent Guardrails: Rules That LLMs Cannot Bypass* + *Runtime Guardrails: Steer, Don't Block* (neurosymbolic `BeforeToolCall` hook; steer-don't-block).
- ATA: *Autonomous Trustworthy Agents* (arXiv:2510.16381) — prompts-as-suggestions, 3 bypass patterns.
- *Arbiter* (arXiv:2603.08993) — system-prompt interference / silent conflict resolution.
- Anthropic — *Advanced tool use*, *Writing tools for agents*, *Define tools* (descriptions, fewer tools, namespacing, tool_choice, tool-search 30–50 limit, response quality).
- Anthropic — *Equipping agents with Agent Skills* + *Complete Guide to Building Skills* (progressive disclosure, "pushy" descriptions, reduce >20–50 skills, code-execution, monitor+iterate).
- Repo — `omp://hooks.md` (tool_call pre-hook), `assets/configs/omp/APPEND_SYSTEM.md`, `assets/skills/00-force-load.md`, `8sync harness toolstats` baseline.
