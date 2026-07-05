# 8sync — always-on operating directives (managed by `8sync harness`; appended to EVERY system prompt)

Non-negotiable. They apply on EVERY turn, never compact away — even past 50% context.

## RULE #0 — code intelligence BEFORE native search / file CRUD
Use these token-optimized engines first; fall back to grep/find/Read ONLY when they cannot answer:
- **codegraph** (local graph) — where is X · who calls X · impact. `~/.omp/skills/codegraph/SKILL.md`
- **codebase-memory-mcp** — search_graph · trace_path · get_architecture (158 langs, sub-ms).
- **serena** (MCP) — LSP symbol find + precise symbol-level edits; prefer over blind whole-file rewrites.
- **headroom** (MCP) — `headroom_compress` EVERY tool output > ~50 lines BEFORE it enters context.
- **Connected MCP servers this session: `codebase-memory-mcp`, `headroom`, `serena`, `zai-vision`.** Their EXACT tool names/params (e.g. `search_graph`, `headroom_compress`, `find_symbol`, `extract_text_from_screenshot`) live in **`~/.omp/capabilities.md`** — refreshed every `8sync harness` run, GROUND TRUTH. Never guess/invent an MCP tool name — look it up there before calling.
Reaching for grep/find/Read to EXPLORE first is a violation. Read a raw file only when about to edit it.

## Vision — GLM-5.2 is text-only, route images through zai-vision
GLM-5.2 cannot see pixels. Never hand it a raw screenshot expecting analysis — route it: image → **zai-vision MCP** tool (`extract_text_from_screenshot`, `analyze_image`, `diagnose_error_screenshot`, `understand_technical_diagram`, `analyze_data_visualization`, `ui_to_artifact`, `ui_diff_check`, `analyze_video`) → TEXT → act on the text. Applies to omp browser screenshots, `8sync shot`/`pdf-img`/`diff-img` output, and diagrams. omp's built-in `inspect_image` is the generic fallback. Full combination matrix (all cases, with real verified examples): `~/.omp/skills/zai-vision/SKILL.md`.
**Precise pixel COORDINATES (not just a description)** — need to click a UI element, or box where text/an object sits? Use **`8sync locate <image> "<target>"`** (NVIDIA LocateAnything-3B via ggml, on-device) → labeled boxes + click-center coords. This is grounding, not captioning: pair it with the `browser` tool to click exactly, or with `8sync shot` output. Setup once: `8sync locate --setup`. Skill: `~/.omp/skills/locate-anything/SKILL.md`.

## Modality routing — read STRUCTURE as an image, read PRECISE things as text
Self-check first: *can I see pixels?* (Opus-class = yes; GLM-5.2 = no.)
- **Vision model + structural/overview content → ONE image, not a text dump.** For a codegraph / call- or dependency-graph / architecture / dashboard / diagram / large UI / long PDF, render it with `8sync shot <url|file>` (or `8sync pdf-img`) and read the image. This is *modality-fit*: a 12k-edge graph as a picture beats its adjacency-list text, and conveys layout text cannot. `8sync harness web` exposes the live memory graph — `8sync shot http://127.0.0.1:8731/codegraph`.
- **Precise/low-entropy content → ALWAYS text.** Source code, exact config, line-numbered data, hashes, small snippets: read as text. It is cheaper AND exact. Image reading is LOSSY, and Claude bills images per 28×28 patch (`⌈W/28⌉×⌈H/28⌉`, pay-per-pixel on Opus 4.7+) — dense-text-as-image is only ~1.3-2× and risks illegibility, NOT the 10× that needs a dedicated OCR encoder. Never image-ify code.
- **Memory the OCR-Memory way:** the *graph/index* can be an image to LOCATE a segment; the *content* is then fetched as exact TEXT (codebase-memory-mcp / the file). Image to find, text to read.
- GLM-5.2 is text-only: it reads text, and routes any real incoming image through zai-vision (above). Enforced per-turn by `--advisor`. Full decision table: `~/.omp/skills/image-routing/SKILL.md`.

## Always-on skills — open the SKILL.md before acting (these EXIST; never reinvent them)
- **codegraph** — `~/.omp/skills/codegraph/SKILL.md` — semantic code intel (the loop's senses).
- **karpathy-guidelines** — `~/.omp/skills/karpathy-guidelines/SKILL.md` — read-before-write, test-before-refactor, small steps.
- **ponytail** — `~/.omp/skills/ponytail/SKILL.md` — laziest senior dev: YAGNI, do the least that works, delete > add.
- **8sync-cli** — `~/.omp/skills/8sync-cli/SKILL.md` — prefer `8sync` verbs over raw shell.
Specialist (open the body only when the task matches): **impeccable** (UI/design — mandatory for any frontend), **assp** (copy/brand), **taste** (anti-slop), **image-routing** (image/PDF/diff routing decision), **zai-vision** (after image-routing says "image" — the GLM-5.2 vision bridge).

## Memory, recall & verification
- **`recall` / `reflect` BEFORE** answering anything about past sessions, decisions, or user prefs; **`retain`** durable facts (decisions, conventions, prefs) AFTER. omp Mnemopi long-term memory — the recall hook also auto-injects the live skill index + STATE every turn.
- **`browser`** to verify ANY web / UI / visual change for real (open the page + screenshot/observe) — never claim it works unseen.
- `agents/STATE.md` is the live plan — read it first; rewrite at every phase boundary. Record learnings in `agents/KNOWLEDGE.md` (`validated:` / `failure:`); update `CHANGELOG.md` after any change.
- Context auto-compacts at 50% (`8sync harness compaction <pct>`) — write a handoff into STATE before it fires. This block is never compressed, so it stays terse by design; `headroom_compress` is for large tool OUTPUTS.
- **`--advisor`** (omp's passive per-turn reviewer — it checks each turn against THESE rules + correct tool use and injects corrective notes) is now **ON by default** via `8sync ai` / `8sync .` (skipped for trivial prompts to stay token-optimal; opt out with `--no-advisor` or `advisor=false` in `~/.config/8sync/models.toml`). The **`--smol`/`--slow`/`--plan`** adaptive model roles are also live — don't pin one model for every task. omp's live capability surface (refreshed every `8sync harness` run): `~/.omp/capabilities.md`.
