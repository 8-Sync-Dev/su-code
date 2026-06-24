# Research Brief — Why `/gs` regressed quality, and how to fix it

**Date:** 2026-06-24 · **Method:** deep-research (multi-source web + repo eval, `8sync harness bench` measurement, cross-verified) · **Scope:** eval the v0.20.1 `/gs` autonomous-team loop, explain the post-`/gs` quality drop, and design improvements (stronger autonomy, leaner team, token-optimal, self-learning/research, doc-hygiene, codebase-history). Sources in `.provenance.md`.

## TL;DR — the regression is process over-engineering, not tokens
`8sync harness bench` shows the token budget is already good (**~8.5k upfront, 79% saved by progressive disclosure, KV-cache-stable**). So `/gs` did **not** bloat the prefix. The quality drop comes from **forced ceremony**: `/gs` is a **93-line prescriptive command** that mandates plan → delegate-to-a-team → verify → Closeout → re-review on **every** invocation, plus `auto`'s **"never ask"** rule. 2026 research says all three of these *reduce* quality when applied indiscriminately:

| `/gs` behavior today | What research says | Fix |
|---|---|---|
| Forces team/subagent delegation every task | "coordination overhead exceeds the cost of doing the work manually" → fails; multi-agent +3–10× cost, instability; managers "default to overly prescriptive… backfires when the manager lacks deep codebase context" `[1][2]` | **Solo by default**; delegate only when task exceeds single-agent context / needs parallelism / specialization |
| 93-line dense process injected as steering prompt | "instruction-following degrades as constraint density increases"; "more context, worse compliance"; "every line must justify its rent"; 100–150-line docs are top performers `[3][4][5]` | **Shrink command**; move full protocol to the `gs` skill (progressive disclosure) |
| `auto` = NEVER ask, always assume | "the most valuable 2026 capability is agents learning **when to ask for help**, not blindly attempting every task" `[6]` | Keep autonomous, but **confidence-gate irreversible calls**; prefer reversible; don't compound |
| Full Closeout (full suite + QA + re-review) on every goal | over-engineering → "code becomes longer and structurally strained"; right-size work `[7][8]` | **Scale verify/Closeout to task size** |
| No doc-maintenance step | "stale docs actively poison context"; agents read docs every request with no skepticism `[9][10]` | **Add doc-hygiene**: detect + prune/update stale docs |

## Findings

### F1 — Forced multi-agent delegation is the primary regression
Cognition ("Don't Build Multi-Agents") and Anthropic's multi-agent post are *aligned*: single-agent is right unless the task truly exceeds one agent's context; multi-agent "works when tasks exceed what a single agent can hold… fails when coordination overhead exceeds doing the work manually" `[1]`. Documented anti-patterns `[2]`: free-form delegation ("research X"), inlining subagent transcripts (pollutes orchestrator context), supervisor paraphrase round-trips (~50% of the measured loss), and managers being "overly prescriptive when lacking deep codebase context" — which is exactly a 93-line team-lead prompt delegating to fresh subagents. **`/gs` applies the team to everything; it should be the exception, not the default.**

### F2 — Constraint-density / instruction overload
The Agent-READMEs study (arXiv 2511.12884) + Augment's "junk drawer" analysis: context files can *reduce* success and add >20% cost; "more context, worse compliance"; **100–150-line files are top performers, gains reverse past that** `[3][4][5]`. The `/gs` command body (93 lines) + `gs` skill + loop-eng force-load stack into a heavy steering prompt. Lever: cut the command to the essential routing + spine + the adaptive-effort rule; push the detailed protocol into the skill body (read on trigger).

### F3 — "Never ask" backfires
Anthropic's 2026 Agentic Coding Trends: the highest-value capability is **agents learning when to ask vs attempt** `[6]`. `/gs auto`'s absolute "never ask, always assume" compounds wrong assumptions on irreversible/high-stakes choices. Keep strong autonomy (the user's goal) but add a **confidence × reversibility gate**: low-confidence + hard-to-undo ⇒ treat as a blocker (log + do other slices); otherwise pick the reversible option, log, proceed.

### F4 — Over-engineering / ceremony erodes quality
CodeTaste (arXiv 2603.04177): agents "accrete technical debt… code becomes longer and structurally strained as requirements evolve" `[7]`; a real autonomous browser-engine build *stalled on its own over-structured code*. Right-sizing `[8]`: work items neither too big nor too small. **Mandatory full Plan→team→Closeout on a one-line change is the wrong size.** Classify the task first; match the machinery.

### F5 — Doc-rot is real and unmanaged (the user's new ask)
"Stale information actively poisons the context" because agents read docs every request without skepticism `[9]`; stale context yields *plausible but architecturally-wrong* code that compounds across PRs `[10]`. Detection: compare doc-referenced file paths vs the real tree (codegraph), track doc bulk/entropy, success-rate before/after. Prune: file-structure/API/lint-rule restatements (agent can see them); **keep** what the agent *can't* see (deploy, test cmds, why-decisions). Discipline: "reject doc additions without deletions"; "treat every line like ad space." **`/gs` needs a doc-hygiene step that finds junk/stale docs and deletes or updates them.**

## Plan — improve `/gs` (research-backed)

**P0 — right-size + de-ceremony (fixes the regression)**
1. **Adaptive effort gate** (first step of every `/gs` run): classify task → `trivial | small | medium | large`. *Trivial/small* = solo, CORE discipline (codegraph+karpathy+ponytail), implement + lightweight verify, no team, no Closeout. *Medium* = solo + verifier subagent. *Large/multi-slice* = full loop + team + Closeout. Default solo; escalate only on evidence.
2. **Solo-by-default delegation** — delegate only when the task exceeds single-agent context, needs real parallelism, or specialization the lead lacks; scoped objective + **summary return** (never free-form, never inline transcript).
3. **Shrink the command** — `assets/commands/gs.md` to a lean router (modes + spine + adaptive-effort + token discipline + autonomy gate + pointer); full protocol → `gs` SKILL body (progressive disclosure).

**P1 — autonomy + verification quality**
4. **Confidence×reversibility autonomy** — strong `auto`, but irreversible + low-confidence ⇒ blocker; reversible ⇒ assume+log+proceed. Never compound; prefer the boring/undoable option.
5. **Scale verify/Closeout to size** — lightweight verify for small; full Closeout only for large/multi-slice.

**P2 — new capabilities the user asked for**
6. **Doc-hygiene step** — detect stale/junk docs (paths moved via codegraph, oversized `AGENTS.md` via `bench`, superseded content), then **update or delete**; "no doc addition without a corresponding deletion when it supersedes." Keep docs ≤ ~150 lines / lean.
7. **Codebase-history understanding** — before touching load-bearing code: `git log/blame` + `agents/DECISIONS.md` + codebase-memory-mcp (`detect_changes`, history) to learn *why* it is the way it is.

**Non-goals / keep:** the token discipline (codegraph+cbm+headroom) and self-learning spine (STATE/KNOWLEDGE/PLAYBOOKS, Reflexion `failure:`) are good — keep. Don't add more agents; remove forced ones.
