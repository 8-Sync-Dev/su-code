# Provenance — raising-tool-adherence-research.md

**Generated:** 2026-06-29 · **Workflow:** deep-research (web_search ×4 + targeted read + repo grounding). Claims cross-verified across ≥2 independent sources where load-bearing.

## Claim → source map

| Claim | Source(s) | Confidence |
|---|---|---|
| Prompts are suggestions, not constraints; model decides per call | AWS-guardrails (dev.to/aws/ai-agent-guardrails-rules-that-llms-cannot-bypass-596d); ATA arXiv:2510.16381; system-prompts disc #242 | High (3 sources) |
| 3 bypass patterns (param/completeness/tool-bypass) | ATA arXiv:2510.16381 (via AWS-guardrails) | High |
| System prompts 245–1490 lines; silent conflict resolution | Arbiter arXiv:2603.08993 | High |
| Neurosymbolic `BeforeToolCall` hook: 3/3 blocked, 0 false-pos, message LLM can't override | AWS-guardrails (Strands demo) | High |
| "Steer, don't block" — guide self-correction vs fail | AWS "Runtime Guardrails for AI Agents — Steer, Don't Block" (dev.to/aws/...-278n) | Med (single source, but consistent) |
| omp `tool_call` pre-hook returns {block,reason}; hooks persist state + inject msgs | repo `omp://hooks.md` (primary, authoritative for this codebase) | High |
| Tool-description refinements → dramatic gains (SWE-bench SOTA) | Anthropic *Writing tools for agents*; *Advanced tool use* | High |
| Fewer/less-ambiguous tools; selection degrades >30–50 tools | Anthropic *Define tools* / *Tool search tool* (Opus 4.5 79.5→88.1%) | High |
| Namespacing tool names | Anthropic *Define tools* | High |
| `tool_choice` modes + incompatible with extended thinking | Anthropic *Define tools* / *implement-tool-use* | High |
| Tool-response quantity/structure affects eval | Anthropic *Writing tools for agents* | High |
| Skills "undertrigger" → make descriptions "pushy"; what+when+keywords | Anthropic skill-creator SKILL.md; *Equipping agents with Agent Skills* | High |
| Progressive disclosure 3-tier; ~30–50 tok name+desc; reduce if >20–50 skills | Anthropic *Equipping agents…*; *Complete Guide to Building Skills*; agentskills.io | High |
| serena was broken (transport) until v0.29.3 → explains serena=0 | this repo (CHANGELOG 0.29.3; toolstats data) | High (observed) |
| Baseline optimizer = 25–34% of code-lookups | `8sync harness toolstats` on su-code + agentic-cloudgo-v1 | High (measured) |

## Searches run
1. "why LLM coding agents ignore system prompt instructions … tool selection steering 2025" → 26 sources (OWASP, Arbiter, ATA, steering papers).
2. "Anthropic Claude tool use best practices tool_choice … descriptions" → 27 sources (Anthropic docs + engineering blog).
3. "Claude Code PreToolUse hook block tool enforce policy" → misfired (clarification); compensated by repo `omp://hooks.md` + AWS-guardrails read.
4. "Anthropic Agent Skills best practices … progressive disclosure triggers 2025" → 20 sources (Anthropic skills docs + guide).
+ targeted read: dev.to AWS-guardrails article (full).

## Caveats / gaps
- "Steer-don't-block" specifics from one AWS post (Part 3.2) not deep-read (title-level); the principle is corroborated by the neurosymbolic block demo.
- omp live tool COUNT not yet measured (action item §3) — the >30–50 ambiguity claim is conditional on it.
- Activation-steering (EAST/WAS) papers noted but excluded as out-of-scope (requires model internals; not available via omp).
- No A/B yet that the proposed hook raises the ratio — must be validated with `toolstats` after shipping (explicitly flagged as a loop, not a proven outcome).
