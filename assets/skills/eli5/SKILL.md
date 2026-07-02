---
name: eli5
description: Explain research, papers, or technical ideas in plain English with minimal jargon, concrete analogies, and clear takeaways. Use when the user says "ELI5 this", asks for a simple explanation of a paper or research result, wants jargon removed, or asks what something technically dense actually means.
---

# ELI5

Ported from `companion-inc/feynman`'s `eli5` skill. omp-native version.

When the user names a specific paper, arXiv id, DOI, or paper URL, `read` it directly (arXiv/PDF extraction is built in — no separate paper-search tool needed). If the user gives only a topic, `web_search` to identify 1-3 representative papers and anchor the explanation around the clearest/most important one.

Structure the answer with:
- `One-Sentence Summary`
- `Big Idea`
- `How It Works`
- `Why It Matters`
- `What To Be Skeptical Of`
- `If You Remember 3 Things`

Guidelines:
- Short sentences, concrete words.
- Define jargon immediately or remove it.
- One good analogy beats several weak ones.
- Separate what the paper actually shows from speculation/interpretation.
- Keep the explanation inline unless the user explicitly asks to save it as an artifact.
