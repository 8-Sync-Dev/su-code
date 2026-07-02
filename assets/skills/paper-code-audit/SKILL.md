---
name: paper-code-audit
description: Compare a paper's claims against its public codebase and identify mismatches, omissions, and reproducibility risks. Use when the user wants to audit whether a paper's reported methods/metrics/defaults actually match the released code.
---

# Paper-Code Audit

Ported from `companion-inc/feynman`'s `/audit` slash-command. Self-contained omp-native version.

## Tool mapping

Read the paper → `web_search` + `read <url>`. Read the codebase → `read`/`grep` on the cloned repo (clone with `bash: git clone` first if it's not local), or `codebase-memory-mcp`'s `search_graph`/`get_code_snippet` if the repo is already indexed. Delegate evidence gathering → `task` (role "Researcher"). Verify sources/citations → `task` (role "Verifier").

Derive a slug from the audit target (lowercase, hyphens, ≤5 words).

## Workflow

1. **Plan** — Outline which paper, which repo, which claims to check. Write to `outputs/.plans/<slug>.md`, summarize briefly, continue immediately (don't wait for confirmation unless asked).
2. **Gather** — Use `task` (role "Researcher") for non-trivial audits to pull claims from the paper and matching implementation details from the code; `task` (role "Verifier") to verify sources and add inline citations. For small audits, do both yourself.
3. **Compare** — Check claimed methods, defaults, metrics, and data handling against the actual code (`read`/`grep` the relevant files — don't guess from the README alone).
4. **Report** — Call out missing code, mismatches, ambiguous defaults, reproduction risks. Save exactly one artifact: `outputs/<slug>-audit.md`, ending with a `Sources` section (paper + repo URLs).
