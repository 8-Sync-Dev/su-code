---
name: image-routing
description: Use this skill on EVERY read request AND before dumping large context, to decide between text and image representation. Apply whenever the AI is about to open a PDF, screenshot a URL, review a UI, inspect a long git diff, process diagrams, OR dump a big graph / call-tree / dependency map as text. First self-check your own modality (can you see images?), then consult the decision table. Picking wrong wastes tokens — but image is NOT always cheaper (see the honest economics below).
---

# image-routing — image vs text, by content type AND your modality

Two directions, one rule: **pick the modality that fits the content.**
- **Incoming** (you must read a PDF/UI/diagram/graph) — decision table below.
- **Outgoing** (about to emit a big dump — a codegraph, dep-tree, dashboard) — if you can
  see images, render the STRUCTURE to one image instead of dumping its text form.

## STEP 0 — self-check your modality

**Can I see pixels?**
- **Vision model (Opus-class): yes.** Use images for STRUCTURE/overview (below).
- **Text-only (GLM-5.2 — the 8sync default): no.** You cannot read PNGs at all. Read text;
  route any real incoming image through the **`zai-vision` MCP** (`~/.omp/skills/zai-vision/SKILL.md`)
  to get TEXT back, then act.

## Honest token economics (read before assuming image is cheaper)

- Claude bills images as **28×28-px patches**: `⌈W/28⌉ × ⌈H/28⌉` tokens — roughly **pay-per-pixel**.
  Opus 4.7+ removed the old cheap ~1.15 MP downscale cap and charges for pixels sent (~3× more at
  max res). So a big screenshot is NOT automatically cheap.
- Rendering **dense text as an image is only ~1.3-2× cheaper** than the text, and risks
  illegibility (resize/JPEG artifacts merge characters). Grounding: "Text or Pixels?" arXiv 2510.18279.
- The famous **10× / 90% reduction (DeepSeek-OCR, arXiv 2510.18234) needs a DEDICATED optical
  encoder** (~100 vision-tokens/page) — you do NOT get that by sending a PNG to Opus or GLM. Do not
  promise 90% for screenshots.
- **Where image genuinely wins: STRUCTURE.** A 12k-edge call graph, an architecture diagram, a
  dashboard, a rendered UI — the *text* equivalent (adjacency lists, DOM) is far larger AND loses
  layout. Here image is a real, large win because you read structure, not OCR'd characters.
- **Image reading is LOSSY** — exact code, line numbers, config, hashes MUST stay text.

## Decision table

| Content type                          | Use       | Why                                            |
|---------------------------------------|-----------|------------------------------------------------|
| Source code (`.rs`, `.ts`, `.py`, …)  | **text**  | exact lines; OCR'd code costs MORE + loses nums|
| Exact config / hashes / line data     | **text**  | precise + lossless                             |
| Plain text doc < ~50 KB               | text      | already cheap + exact                          |
| Codegraph / call- or dep-graph / tree | **image** | structure ≪ its adjacency-list text (vision)   |
| Architecture / diagram / mermaid      | **image** | layout is the information                       |
| Rendered UI / dashboard review        | **image** | one shot ≪ DOM + styles                        |
| Long PDF (> 5 pages, figures/scans)   | **image** | `8sync pdf-img <file>`                          |
| Git diff > 500 lines                  | image     | `8sync diff-img <ref>` (phase 2 stub)          |
| Terminal output                       | text      | already text                                   |

## Tools

- **`8sync shot <url|file>`** — real (bundled/system Chromium → PNG; prints the vision-token
  estimate). Renders any URL incl. `8sync harness web` pages, or local HTML.
- **`8sync pdf-img <file>`** — PDF pages → PNGs (poppler).
- **Memory the OCR-Memory way** (arXiv 2604.26622): use an image to LOCATE a segment (the graph /
  an indexed page), then fetch the CONTENT as exact text from codebase-memory-mcp or the file.
  Image to find, text to read — never OCR precise content back out of a picture.

## Examples

Review a UI: `8sync shot http://localhost:3000/login` → one PNG instead of `LoginScreen.tsx` +
styles (~8k tok). GLM: pass the PNG to zai-vision → text.

Grok the architecture (vision model): boot `8sync harness web`, then
`8sync shot 'http://127.0.0.1:8731/codegraph?shot=1' -o /tmp/cg.png` — **`?shot=1` = canvas-only**
(the React-Flow package graph full-viewport, big + legible, ~2k vision tokens). Never full-page-capture
that page: clusters/hotspots/search are exact text via `/api/codegraph/overview|search|trace`. Then for
a precise change, read the exact file slice as text. Non-vision model: read node positions from the
shot with `8sync locate /tmp/cg.png "<node label>"` (automatic — see locate-anything skill).
