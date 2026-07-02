---
name: preview
description: Preview Markdown, LaTeX, PDF, or code artifacts. Use when the user wants to review a written artifact, export a report, or view a rendered document.
---

# Preview

Ported from `companion-inc/feynman`'s `preview` skill (referenced a `/preview` command that doesn't exist in omp — omp-native fallback below is the only path, not a fallback).

## Markdown / code

`read <path>` renders it directly in the response — usually sufficient. For a rendered HTML view in an actual browser, write a minimal wrapper and open it:

```bash
pandoc input.md -o /tmp/preview.html && true
```

Then `browser` → `open` the resulting `file:///tmp/preview.html` and `screenshot`/`extract` to confirm it rendered as expected — never claim a visual result without actually looking.

## PDF export

```bash
pandoc input.md -o output.pdf   # requires a LaTeX engine (texlive) for PDF output
```

If `pandoc`/LaTeX isn't installed, say so explicitly rather than claiming the export happened — check with `which pandoc` first.

## Existing PDF/image artifacts

Use `read <path-or-url>` — PDFs are extracted to text directly; images are decoded inline. For a project already using `8sync`, prefer `8sync pdf-img <file>` (PDF page → PNG) or `8sync diff-img` (git diff → PNG) over raw `pandoc`/`convert` shell-outs when those verbs are available.
