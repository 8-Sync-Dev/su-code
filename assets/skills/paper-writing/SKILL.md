---
name: paper-writing
description: Turn research findings into a polished paper-style draft with equations, sections, and explicit claims. Use when the user wants a formal write-up of research (paper, technical report) rather than a casual summary.
---

# Paper Writing

Ported from `companion-inc/feynman`'s `/draft` slash-command. Self-contained omp-native version.

## Tool mapping

Draft from collected notes → `task` (role "Writer") when producing from already-gathered material, then `task` (role "Verifier") for citations. Small drafts → write directly yourself.

Derive a slug from the topic (lowercase, hyphens, ≤5 words).

## Workflow

1. **Outline** — Proposed title, sections, key claims, source material to draw from, a verification log for critical claims/figures/calculations. Write to `outputs/.plans/<slug>.md`, summarize briefly, continue immediately (don't wait for confirmation unless asked).
2. **Draft** — `task` (role "Writer") from already-collected notes, or write directly. Include at minimum: title, abstract, problem statement, related work, method/synthesis, evidence/experiments, limitations, conclusion. Clean Markdown; LaTeX only where equations materially help.
3. **Provenance discipline** — Every result, figure, chart, image, table, benchmark, or quantitative comparison needs a traceable source. Missing evidence → a placeholder or proposed experimental plan, never a claimed outcome. Generate a chart only when the underlying source-backed data supports it; otherwise a table or chart spec. Mermaid only for source-supported architectures/pipelines.
4. **Sweep** — Before delivery, sweep for claims stronger than their support. Mark tentative results as tentative; remove unsupported numerics rather than letting review catch them.
5. **Cite** — `task` (role "Verifier") to add inline citations and verify sources.
6. **Deliver** — Save exactly one draft to `papers/<slug>.md`, ending with a `Sources` appendix (direct URLs for every primary reference).
