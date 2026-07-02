---
name: deep-research
description: Run a thorough, source-heavy investigation on any topic. Use when the user asks for deep research, a comprehensive analysis, an in-depth report, or a multi-source investigation. Produces a cited research brief with provenance tracking.
---

# Deep Research

Ported from `companion-inc/feynman`'s `/deepresearch` slash-command (feynman-only runtime — not available in omp). This is a self-contained omp-native version: everything below runs with omp's own tools, no `feynman` CLI required.

## Tool mapping

| Need | omp tool |
|---|---|
| Web search | `web_search` |
| Fetch a URL | `read <url>` |
| Delegate a sub-investigation | `task` (one subagent per research angle; give each a `role` like "Researcher" or "Verifier") |
| Ask the user a question | `ask` |
| Persist a durable fact for later sessions | `retain` |

This is an execution request, not a request to explain the workflow. Start with tool calls that create the plan artifact.

## Required artifacts

Derive a slug from the topic (lowercase, hyphenated, ≤5 words). Every run leaves:
- `outputs/.plans/<slug>.md` — the plan
- `outputs/.drafts/<slug>-draft.md` — first draft
- `outputs/.drafts/<slug>-cited.md` — cited draft
- `outputs/<slug>.md` — final brief
- `outputs/<slug>.provenance.md` — provenance sidecar

If any step fails, continue in degraded mode and still write a blocked/partial final output + provenance. Never end with chat-only output after the plan is approved.

## Workflow

1. **Plan** — Write `outputs/.plans/<slug>.md`: key questions, evidence needed, scale decision (see below), task ledger, verification log. Briefly summarize to the user; ask "Proceed with this plan? Reply yes, or tell me what to change." Do not gather evidence before confirmation.
2. **Scale** — Single fact / "what is X" explainer answerable in 3-10 tool calls → do it directly, no subagents. Direct comparison of 2-3 items → 2 `task` subagents. Broad survey → 3-4. Complex multi-domain → 4-6. Never inflate a simple explainer into a multi-agent survey.
3. **Gather** — Direct mode: run ≥3 distinct `web_search` queries (definition/history, mechanism, current usage/comparison); record exact queries in `outputs/.drafts/<slug>-research-direct.md`. Subagent mode: `task` with one assignment per angle, each writing its own `<slug>-research-N.md`; wire them with `context` describing the shared goal, not the mechanics of the tool calls.
4. **Draft** — Write `outputs/.drafts/<slug>-draft.md` yourself (do not delegate synthesis): executive summary, findings by theme, evidence-backed caveats, open questions. No invented sources, figures, or numbers. Sweep every claim/number/table for a source URL or note reference; downgrade or remove what isn't backed; mark inferences as inferences.
5. **Cite** — Direct mode: verify every URL yourself (`read`), copy the draft to `<slug>-cited.md` with inline citations + a Sources section. Subagent mode: spawn a `task` with role "Verifier" to add citations and verify every URL, output to `<slug>-cited.md`; confirm the file landed on disk after.
6. **Review** — Direct mode: review the cited draft yourself, write `<slug>-verification.md` with FATAL/MAJOR/MINOR findings, fix FATAL before delivery. Subagent mode: spawn a `task` with role "Reviewer" against the cited draft for the same categories; fix FATAL, note MAJOR in Open Questions, accept MINOR. Re-review once if FATAL issues were found.
7. **Deliver** — Copy the final candidate to `outputs/<slug>.md` (or `papers/<slug>.md` for paper-style output). Write `outputs/<slug>.provenance.md`:
   ```markdown
   # Provenance: [topic]
   - Date: [date]
   - Rounds: [N]
   - Sources consulted / accepted / rejected: [counts]
   - Verification: PASS / PASS WITH NOTES / BLOCKED
   - Plan: outputs/.plans/<slug>.md
   ```
   Before responding, verify on disk that all artifacts exist (`read`/`grep`). Final chat response is brief: link the final file, provenance file, and any blocked checks.
