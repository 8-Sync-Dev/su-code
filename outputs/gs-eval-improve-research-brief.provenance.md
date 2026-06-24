# Provenance — gs-eval-improve-research-brief

**Generated:** 2026-06-24 · **Workflow:** deep-research (researcher → verifier → reviewer, executed in-harness: web_search + repo eval + `8sync harness bench` measurement). `/deepresearch` slash-command unavailable in this runtime; methodology approximated faithfully (multi-source, cross-verified, primary-source-pinned).

## Eval inputs (this repo, observed)
- `8sync harness bench` @ v0.20.1: upfront ~8506 tok (force-load 1948 + CORE 3726 + memory 2832); deferred ~117k; A2 saved 79%; A1 stable-prefix PASS. ⇒ token budget is healthy; regression is not prefix bloat.
- `.omp/commands/gs.md` = 93 lines, dense prescriptive process (plan→team→verify→Closeout→re-review) mandated on every invocation; `auto` = "never ask".
- `assets/skills/gs/SKILL.md` = 24 lines (trigger + protocol summary).
- Source-of-truth: `assets/commands/gs.md` (harness deploys → `~/.omp/agent/commands/gs.md` + repo `.omp/commands/gs.md`).

## Search streams
1. multi-agent worse than single agent / context loss / Cognition "Don't Build Multi-Agents" / subagent delegation pitfalls
2. autonomous coding agent over-engineering / ceremony / quality degradation / right-sizing
3. AI coding agent documentation rot / detect stale junk docs / AGENTS.md drift / pruning

## Verification status
- **VERIFIED (aligned primary voices):** Cognition "Don't Build Multi-Agents" + Anthropic multi-agent post, synthesized by philschmid + Cognition's "Multi-Agents: What's Actually Working" (2026-04) `[1][2]` — single-agent default; multi-agent only when task exceeds one agent; coordination-overhead caveat.
- **VERIFIED (primary):** Agent-READMEs empirical study arXiv 2511.12884 + Augment "junk drawer"/"good AGENTS.md" (100–150-line sweet spot; >20% cost; "more context worse compliance") `[3][4][5]`. CodeTaste arXiv 2603.04177 (quality erosion across iterations) `[7]`. Anthropic 2026 Agentic Coding Trends Report ("agents learning when to ask") `[6]`.
- **CORROBORATED (multi-secondary):** doc-rot detection/pruning (aihero AGENTS.md guide, SoftwareSeni code-rot, Factory/agents.md best practices) `[9][10]`; right-sizing (Scrum.org) `[8]`; delegation anti-patterns (FlowHunt synthesis of Anthropic/Cognition contracts) `[2]`.

## Source list (deduped)
- [1] Cognition "Don't Build Multi-Agents" / "Multi-Agents: What's Actually Working" (cognition.com/blog/multi-agents-working) + jxnl.co synthesis + philschmid.de/single-vs-multi-agents
- [2] FlowHunt "Multi-Agent AI Systems 2026: What the Research Actually Says" (delegation contracts, supervisor-round-trip ~50% loss) ; Augment multi-agent-orchestration guide
- [3] Agent READMEs: Empirical Study of Context Files — arXiv 2511.12884
- [4] Augment Code — "Your agent's context is a junk drawer" / "How to write good AGENTS.md" (100–150 lines; instruction-budget)
- [5] Gloaguen et al. "Evaluating AGENTS.md" arXiv 2602.11988 (context files reduce success +20% cost — corroborating)
- [6] Anthropic — 2026 Agentic Coding Trends Report (resources.anthropic.com)
- [7] CodeTaste — arXiv 2603.04177 (quality erosion / over-structuring)
- [8] Scrum.org — right-sizing work items
- [9] aihero.dev "A Complete Guide to AGENTS.md" ; agents.md ; Factory AGENTS.md docs
- [10] SoftwareSeni "Code Rot… AI Coding at Scale" ; MindStudio context-rot/sub-agents

## Limitations
- 2026 sources skew recent + partly secondary/vendor; load-bearing claims cross-checked, INFERENCE-grade consensus flagged.
- No A/B on this repo's `/gs` quality (would need task-suite runs); diagnosis is evidence- + bench-grounded, not measured head-to-head. Recommend a small before/after task set post-redesign.
