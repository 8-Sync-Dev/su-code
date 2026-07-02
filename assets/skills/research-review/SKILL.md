---
name: research-review
description: Run an internal research critique with likely objections, severity, and a concrete revision plan. Use when the user wants a critical review of an AI/ML research artifact (paper, arXiv id, local draft) — novelty, rigor, baselines, reproducibility, claims validity.
---

# Research Review

Ported from `companion-inc/feynman`'s `/review` slash-command. Self-contained omp-native version. (Registered as `research-review` — feynman renamed this skill upstream from `peer-review`; use this name.)

## Tool mapping

Fetch the artifact → `read <url-or-path>` (arXiv id/URL, local file, or PDF — `read` handles PDF extraction directly). Delegate evidence gathering and review → `task` (role "Researcher"/"Reviewer") only when the artifact is large enough to benefit; otherwise do the review directly.

Derive a slug from the artifact name (lowercase, hyphens, ≤5 words). This is an execution request — carry out the workflow with tools and durable files, don't stop after a plan or describe what you'd do.

## Required artifacts

`outputs/.plans/<slug>-review-plan.md`, `outputs/.drafts/<slug>-review-evidence.md`, `outputs/<slug>-review.md`.

## Workflow

1. Write the plan: artifact identifier + source type (arXiv id, URL, local file, PDF, Markdown); review criteria (novelty, empirical rigor, baselines, reproducibility, claims validity, figures/tables, metrics, related work, writing quality); verification checks needed. Continue immediately — don't stop after planning.
2. **Inspect** — Local files: `read` directly. PDFs: `read` (built-in PDF extraction); if it fails, record the failure and still produce a blocked/partial review. arXiv id/URL: `read` the paper/source, record the URL. Inspect linked code/datasets/citations when reachable and material to the review.
3. Write evidence notes to `outputs/.drafts/<slug>-review-evidence.md` before the final review: quoted/paraphrased claims, observed methods, reported metrics, baseline comparisons, reproducibility facts, every inspected source path/URL.
4. Use `task` (role "Researcher"/"Reviewer") only if the artifact is large enough to benefit from delegation; otherwise do the review yourself. Never claim a subagent ran without actually calling `task`.
5. Write exactly one final artifact `outputs/<slug>-review.md`: Summary Assessment, Strengths, Critical Issues, Major Issues, Minor Issues, Reproducibility and Verification, Inline Annotations (tied to sections/claims/figures), Recommendation, Sources.
6. If the artifact can't be parsed or critical evidence is unavailable, still write `outputs/<slug>-review.md` — mark affected sections `Verification: BLOCKED` with the exact failure, distinguishing blocked checks from actual weaknesses.
7. Before responding, verify on disk that `outputs/<slug>-review.md` exists; if not, create it immediately as a blocked review with the failure reason. Never end with planning-only chat or claim completion without the file on disk.
