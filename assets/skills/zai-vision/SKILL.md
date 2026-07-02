---
name: zai-vision
description: Use this skill whenever the model needs to "see" an image, screenshot, PDF page, diagram, chart, git diff render, or video — GLM-5.2 (the default omp model) is TEXT-ONLY. Apply it as the bridge step after any browser screenshot (`8sync shot`), PDF render (`8sync pdf-img`), diff render (`8sync diff-img`), or omp's own `inspect_image` when a specialized read (OCR, UI-to-code, error diagnosis, diagram/chart understanding, visual regression) is needed. Covers every combination with the rest of the stack: browser, codegraph/codebase-memory-mcp, serena, headroom, retain/recall, advisor.
---

# zai-vision — GLM-5V bridge for a text-only GLM-5.2

**Why this exists**: omp's default model is `zai/glm-5.2:xhigh` — **text-only**, it cannot read pixels. `8sync harness` auto-installs and registers the `zai-vision` MCP (`@z_ai/mcp-server`, npm), which exposes **GLM-5V** (Zhipu/Z.AI's vision family) as 8 model-callable tools, authed with the **SAME Z.AI key** already used for `glm-5.2` (pulled via `omp token zai`, no separate signup). The bridge pattern is always:

```
image (screenshot / PDF page / diff render / chart) → zai-vision tool → TEXT → GLM-5.2 acts on the text
```

Never hand raw image bytes to GLM-5.2 expecting it to "look" — it can't. Always route through a vision tool first.

## Verified working setup (tested end-to-end on 2026-07-01)

`8sync harness` registers the server in `~/.omp/agent/mcp.json`:

```jsonc
"zai-vision": {
  "type": "stdio",
  "command": "zai-mcp-server",           // installed via `bun add -g @z_ai/mcp-server`
  "args": [],
  "env": {
    "Z_AI_MODE": "ZAI",                   // api.z.ai endpoint (matches the zai/glm-5.2 key)
    "Z_AI_VISION_MODEL": "glm-4.6v-flash", // THE ONLY MODEL VERIFIED WORKING ON A STOCK KEY
    "Z_AI_API_KEY": "<pulled from `omp token zai`>"
  }
}
```

**Model choice is load-bearing.** Per Z.AI's pricing page (`docs.z.ai/guides/overview/pricing`), vision models are: `GLM-5V-Turbo` ($1.2/M), `GLM-4.6V` ($0.3/M), `GLM-4.5V` ($0.6/M), `GLM-OCR` ($0.03/M), `GLM-4.6V-FlashX` ($0.04/M), and **`GLM-4.6V-Flash` — completely FREE**. A stock `zai/glm-5.2` Coding-Plan key has NO vision resource package, so every paid model 400s with `"1113: Insufficient balance or no resource package"` until you buy one. `glm-4.6v-flash` is the only one that works out of the box — that's why `8sync harness` defaults `Z_AI_VISION_MODEL` to it. If you buy a vision package, override it in `~/.omp/agent/mcp.json` → `mcpServers.zai-vision.env.Z_AI_VISION_MODEL`.

The free model is occasionally overloaded (`"1305: service may be temporarily overloaded"`, transient, unrelated to `thinking` mode — both were tested independently). **Retry 2-3× with a short backoff before concluding it's broken.**

### Real proof (actually run, not illustrative)

A local HTML card ("Weekly Plan / Mon Review notes / Tue Design draft / Wed Team sync / Thu Write docs / Fri Ship") was screenshotted with the browser tool and fed through the REAL `zai-mcp-server` stdio process (JSON-RPC `tools/call`, not just the raw API):

```
tool: extract_text_from_screenshot
args: {"image_source":"/tmp/zai-clean.png","prompt":"List the text you see, verbatim."}
```
```
<output>
**Extracted Text**
```
Weekly Plan
Team summary

Mon     Review notes
Tue     Design draft
Wed     Team sync
Thu     Write docs
Fri     Ship
```
**Content Type**
A text-based weekly plan or schedule UI component.
...
</output>
```

Text extracted verbatim, matching the source HTML exactly. `analyze_image` on the same screenshot ("Describe the layout, colors, and purpose") correctly identified the two-column layout, the exact hex-ish colors (blue title, gray subtitle, green "Ship" tag), and inferred it was a team task tracker — all from pixels GLM-5.2 alone cannot see.

## Tool catalog (exact params, from the installed package source)

All 8 tools take `image_source` (or `video_source`) as a **local file path OR a remote URL** (not base64 — the server reads the file / fetches the URL itself) + a `prompt` describing the ask:

| Tool | Extra params | Use for |
|---|---|---|
| `extract_text_from_screenshot` | `programming_language` (optional hint) | OCR: code, terminal output, logs, docs |
| `analyze_image` | — | Fallback / general description, layout, colors |
| `diagnose_error_screenshot` | — | Root-cause a stack trace / error dialog screenshot |
| `understand_technical_diagram` | — | Architecture diagrams, flowcharts, UML, ER, sequence diagrams |
| `analyze_data_visualization` | — | Charts/graphs → trends, anomalies, business read |
| `ui_to_artifact` | — | UI screenshot → frontend code / design spec / NL description |
| `ui_diff_check` | — | Visual regression: compare expected vs actual UI |
| `analyze_video` | `video_source` instead of `image_source` | Video content understanding |

## Combination matrix — every way this plugs into the existing stack

`8sync shot`/browser → image is only ONE case. All of these route the SAME way (image → zai-vision tool → text → act):

| Source (produces an image) | zai-vision tool | Then |
|---|---|---|
| omp **browser tool** screenshot (`tab.screenshot`) | `extract_text_from_screenshot` / `analyze_image` | Feed text back into the conversation; GLM-5.2 reasons over it |
| `8sync shot <url>` (render a live page) | `ui_to_artifact` (design→code) or `analyze_image` (review) | Generate/compare frontend code with **impeccable** skill |
| `8sync pdf-img <file>` (long PDF → per-page PNG) | `extract_text_from_screenshot` per page | Concatenate extracted text; treat as normal text doc |
| `8sync diff-img <ref>` (git diff render, >500 lines) | `ui_diff_check` (visual regression) or `extract_text_from_screenshot` (read the diff) | Review/comment as if reading the raw diff |
| Codegraph/**codebase-memory-mcp** `get_architecture` rendered as a diagram image | `understand_technical_diagram` | Cross-check the graph's textual answer against the rendered picture |
| **serena** symbol edits guided by a UI mock | `ui_to_artifact` → get a code skeleton | Hand the skeleton to `serena rename`/`code_actions` for precise symbol-level edits, not blind rewrites |
| Long/garbled vision tool output (>50 lines) | — | Pipe through **headroom** `headroom_compress` before it enters context (STEP 0 rule applies to vision output too) |
| omp's built-in `inspect_image` tool | — | Generic fallback when a specialized zai-vision tool doesn't fit (rare — zai-vision's 8 tools cover almost everything) |
| Chart/graph screenshot from a dashboard | `analyze_data_visualization` | Feed the trend/anomaly summary into `retain` if it's a durable project fact |
| Terminal error screenshot (can't copy-paste the text) | `diagnose_error_screenshot` | Root cause + fix, then act on the fix directly |
| A fact learned from any of the above worth remembering | — | `retain` it (Mnemopi long-term memory) so future sessions `recall` it without re-running vision |
| Any vision-guided edit before committing | — | `--advisor` (omp's passive turn reviewer) catches misrouted or hallucinated vision-derived changes |
| Deciding image vs text in the first place | — | Consult the **`image-routing`** skill's decision table FIRST; this skill is what happens AFTER that table says "image" |

## Setup / troubleshooting

- **Auto-setup**: `8sync harness` (bare or `init`) installs `@z_ai/mcp-server` via `bun add -g`, resolves the key via `omp token zai` (falls back to `$Z_AI_API_KEY`/`$ZAI_API_KEY` env), and registers it in `~/.omp/agent/mcp.json`. Re-run `8sync harness` any time; it self-heals if the command/env drifted.
- **`8sync doctor`** reports `zai-vision MCP ON/OFF` and whether it's registered.
- **No omp Z.AI auth yet?** The server registers WITHOUT a key (doctor/harness warns); run `omp token zai` once you've authed `zai/glm-5.2` in omp, then re-run `8sync harness` to pick it up.
- **`1113` insufficient balance** → you're on a paid vision model without a purchased vision package. Either buy one on z.ai, or stay on the default free `glm-4.6v-flash`.
- **`1305` temporarily overloaded** → transient; retry 2-3× with ~5s backoff (`with_retry` is already built into the server's `analyze_image`/etc. for network errors, but NOT for this HTTP 429-style overload — the caller must retry).
- **`1211` unknown model** → wrong `Z_AI_VISION_MODEL` string. Valid vision codes as of this writing: `glm-5v-turbo`, `glm-4.6v`, `glm-4.6v-flash`, `glm-4.6v-flashx`, `glm-4.5v`, `glm-ocr`. (`glm-4v`, `glm-4v-plus`, `glm-4v-flash`, `glm-4.5v-flash` do NOT exist on the `api.z.ai` endpoint — verified by direct testing.)
- **omp's live capability surface** (advisor/inspect_image/model-role flags, refreshed every `8sync harness` run) is at `~/.omp/capabilities.md`.
