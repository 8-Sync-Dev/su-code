# image-routing — choose image vs text reads to save tokens

**Apply on every read request. Pick the cheaper representation by content type.**

## Decision table

| Content type                          | Use         | Tool                                 |
|---------------------------------------|-------------|--------------------------------------|
| Source code (`.rs`, `.ts`, `.py`, …)  | **text**    | `read`                               |
| Plain text doc < ~50 KB               | text        | `read`                               |
| Long PDF (> 5 pages)                  | **image**   | `8sync pdf-img <file>`               |
| Rendered UI / screenshot review       | **image**   | `8sync shot <url|file>`              |
| Diagram / chart / mermaid / graphviz  | **image**   | `8sync chart-img …` (phase 2)        |
| Git diff > 500 lines                  | **image**   | `8sync diff-img <ref>`               |
| Terminal output (commands)            | text        | regular tool output                  |

## Rules

1. For source code, **always read as text**. Vision token cost for OCR'd code is higher than direct text, and text gives precise line numbers.
2. For visual content (UI, charts, screenshots, large PDFs), **prefer image**. A single 1500-token image vs 5,000+ tokens of describing layout = 3-10× savings.
3. For long code files (> 800 LoC), request an **AST outline** first, then read narrow slices. Don't dump whole file.
4. Always cite `filepath:start-end` after reading.

## Example

User: "Review the login screen UI."

Wrong: open `LoginScreen.tsx` (might be 600 LoC) + `LoginStyles.tsx` + `auth.css` … (~8k tokens).

Right: `8sync shot http://localhost:3000/login` → attach one PNG (~1500 tokens). Then if code change needed, read narrow file slices.
