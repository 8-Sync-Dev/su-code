---
name: literature-review
description: Run a literature review on a topic, lab, PI, or author using paper search and primary-source synthesis. Use when the user wants a survey of prior work, a lab/author's publication trajectory, or a source-grounded state-of-the-art summary.
---

# Literature Review

Ported from `companion-inc/feynman`'s `/lit` slash-command. Self-contained omp-native version.

## Tool mapping

Web/paper search → `web_search`. Fetch a URL (paper page, lab site, arXiv/OpenReview/Semantic Scholar) → `read <url>`. Delegate paper-triage or trajectory synthesis → `task` (role "Researcher"/"Verifier"/"Reviewer"). Ask the user → `ask`.

Derive a slug from the topic/lab/author (lowercase, hyphens, ≤5 words); use it for every file this run.

## Workflow

1. **Plan** — Outline: key questions, source types (papers/web/repos), time period, expected sections, task ledger, verification log. If the input names a lab, PI, author, or institution page, treat this as a **publication-corpus review**: resolve the identity first, then collect the reachable publication list before mapping the trajectory. Write to `outputs/.plans/<slug>.md`, summarize briefly to the user, and continue immediately — do not wait for confirmation unless explicitly asked to review the plan.
2. **Gather** — Delegate with `task` (role "Researcher") when the sweep is wide; search directly (`web_search`) for narrow topics. Researcher outputs → `<slug>-research-*.md`. For publication-corpus reviews, own identity resolution yourself and write `notes/<slug>-publications.md` (titles/years/venues/URLs/DOIs + gaps) before delegating trajectory synthesis. Prefer lab pages, author profiles, arXiv/OpenReview/Semantic Scholar, and search results with stable URLs. Mark every assigned question `done`/`blocked`/`superseded` — never silently drop one.
3. **Synthesize** — Separate consensus / disagreements / open questions. For publication-corpus reviews, identify 3-5 research trajectories and the 3-5 most direction-changing papers, ranked by contrastive originality and methodology strength (not author prestige). Propose concrete next experiments/reading when useful. Use a Mermaid diagram for taxonomies/pipelines/trajectory maps only when source-supported.
4. **Cite** — `task` (role "Verifier"): add inline citations to the draft, verify every source URL via `read`.
5. **Verify** — `task` (role "Reviewer"): check the cited draft for unsupported claims, logical gaps, zombie sections, single-source critical findings. Fix FATAL before delivery; note MAJOR in Open Questions; if FATAL was found, run one more verification pass.
6. **Deliver** — Save to `outputs/<slug>.md` + `outputs/<slug>.provenance.md` (date, sources consulted/accepted/rejected, verification status, intermediate files used; for publication-corpus reviews also the publication-log path and unresolved gaps). Verify both files exist on disk before responding — never stop at an intermediate cited draft alone.
