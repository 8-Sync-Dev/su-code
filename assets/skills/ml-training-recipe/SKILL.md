---
name: ml-training-recipe
description: Find ranked, implementable ML training recipes backed by papers, datasets, docs, and code. Use when the user wants a concrete training/fine-tuning approach for a task, with verified dataset and implementation grounding rather than a generic tutorial.
---

# ML Training Recipe

Ported from `companion-inc/feynman`'s `/recipe` slash-command. Self-contained omp-native version.

## Tool mapping

Search papers/docs/HF hub pages → `web_search` + `read <url>` (Hugging Face dataset/repo pages are plain URLs — read them directly; there is no dedicated HF tool in omp). Delegate a broad paper/code sweep → `task` (role "Researcher"). Verify a candidate → `task` (role "Verifier") or do it yourself for a narrow scope.

Derive a slug from the task (lowercase, hyphens, ≤5 words). This is an execution request — continue immediately after the plan, don't stop to explain.

## Required artifacts

`outputs/.plans/<slug>-recipe.md`, `outputs/.drafts/<slug>-recipe-research.md`, `outputs/<slug>-recipe.md`, `outputs/<slug>-recipe.provenance.md`.

## Workflow

1. **Plan** — Write the plan: target task, benchmark/desired behavior, candidate source types, feasibility constraints, task ledger. Continue automatically.
2. **Research** — `task` (role "Researcher") for a broad sweep; direct `web_search` for narrow tasks. Start from evidence of actual results, not example scripts.
3. **Recipe extraction** — For each promising approach, link the result to the exact recipe: paper/report, benchmark/result, dataset, training method, key hyperparameters, compute assumptions, implementation code path, current docs.
4. **Dataset validation** — Check availability, splits/columns, format match via `read` on the dataset's hub/docs page. Mark anything not directly checked `unverified` — never imply it's usable without checking.
5. **Implementation grounding** — Find working code or official docs (`read` the repo's README/source files directly, or `web_search` + `read` for docs). Prefer current, actively-maintained repos. Record exact file paths, function/class names, command patterns.
6. **Synthesis** — Write `outputs/.drafts/<slug>-recipe-research.md` first, then promote a concise ranked brief to `outputs/<slug>-recipe.md`.
7. **Verification** — For the top-ranked recipe, verify key source URLs and dataset/code availability before delivery. Anything unchecked stays labeled `blocked`/`unverified`.
8. **Provenance** — `outputs/<slug>-recipe.provenance.md`: date, sources consulted, accepted/rejected, verification status, artifact paths.

## Required final shape

- **Recommendation** — the one recipe to try first, and why.
- **Ranked recipe table** — one row per candidate: paper/source, result, dataset, method, hyperparameters, compute, code/docs, verification status.
- **Dataset notes** — schema, split, size, license/access constraints when checked.
- **Implementation plan** — minimal steps to run the top recipe.
- **Known gaps** — missing code, inaccessible data, unclear hyperparameters, benchmark mismatch.
- **Sources** — URL for every paper/repo/dataset/doc used.

Use `verified`, `unverified`, `blocked`, `inferred` precisely. Never claim a method is state-of-the-art, replicated, or production-ready unless the checks actually prove it.
