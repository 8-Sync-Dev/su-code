---
name: source-comparison
description: Compare multiple sources on a topic and produce a source-grounded matrix of agreements, disagreements, and confidence. Use when the user wants to compare claims/approaches/products/papers across several sources rather than a single-source summary.
---

# Source Comparison

Ported from `companion-inc/feynman`'s `/compare` slash-command. Self-contained omp-native version.

## Tool mapping

Gather sources → `web_search` + `read <url>`. Delegate a broad gather → `task` (role "Researcher"). Verify + cite → `task` (role "Verifier").

Derive a slug from the comparison topic (lowercase, hyphens, ≤5 words).

## Workflow

1. **Plan** — Which sources to compare, which dimensions to evaluate, expected output structure. Write to `outputs/.plans/<slug>.md`, summarize briefly, continue immediately (don't wait for confirmation unless asked).
2. **Gather** — `task` (role "Researcher") when the comparison set is broad; direct `web_search`/`read` for a narrow set.
3. **Matrix** — Build a comparison table: source, key claim, evidence type, caveats, confidence. Generate a chart only when quantitative metrics are involved and source-backed; otherwise a table. Mermaid only for source-supported method/architecture comparisons.
4. **Cite** — `task` (role "Verifier") to verify sources and add inline citations to the final matrix.
5. **Deliver** — Distinguish agreement / disagreement / uncertainty clearly. Save exactly one artifact to `outputs/<slug>-comparison.md`, ending with a `Sources` section (direct URL per source).
