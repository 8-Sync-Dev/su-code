---
name: image-gen
description: Generate AI images, icons, logos, diagrams, illustrations, and mockups via `gpt-image-2` using `8sync gen-img` or standalone scripts. Use whenever the user or task asks to create, draw, generate, or render an image or visual asset.
---

# image-gen — AI Image Generation with gpt-image-2

The system has native image generation capability powered by **`gpt-image-2`** (OpenAI Images API via `https://api.apikey.fun/v1/images/generations`).

## How to Generate Images

### 1. Primary Method — `8sync gen-img`
```bash
8sync gen-img "<prompt>" -o <output_path.png> [--size 1024x1024] [--model gpt-image-2]
```

**Examples:**
```bash
# Generate a logo or brand icon
8sync gen-img "a modern minimalist vector logo for 8sync developer tool, dark theme, high contrast" -o ./assets/logo.png

# Generate UI mockup or illustration
8sync gen-img "clean SaaS dashboard analytics interface showing revenue charts, modern dark UI" -o ./docs/dashboard-concept.png --size 1536x1024

# Generate architecture or concept diagram
8sync gen-img "detailed technical diagram illustration showing cloud microservices event bus flow" -o ./docs/architecture.png
```

### 2. Standalone Script Alternative
If running in an environment where `8sync` binary is not in PATH:
- **Windows PowerShell:**
  ```powershell
  .\scripts\gen-img.ps1 "prompt" -Output "path.png"
  ```
- **Bash / Linux / macOS:**
  ```bash
  bash scripts/gen-img.sh "prompt" "path.png"
  ```

### 3. Direct API Call (curl)
```bash
curl -s -X POST https://api.apikey.fun/v1/images/generations \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "x-openai-actor-authorization: apikey.fun" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-image-2", "prompt": "<prompt>", "size": "1024x1024", "n": 1}'
```

## Prompt Engineering for gpt-image-2

- **Be specific about style:** "flat vector illustration", "3D isometric render", "minimalist dark UI mockup", "photorealistic macro shot", "cyberpunk neon concept art".
- **Specify background & contrast:** "isolated on clean dark background", "transparent-style neutral backdrop", "high contrast edges".
- **Aspect Ratios / Sizes:** `1024x1024` (square, default), `1536x1024` (landscape), `1024x1536` (portrait).

## Viewing the Output (VLM Loop)
Once generated, vision-capable agents (GLM-5.3-Flash, Claude Opus) can immediately view and inspect the result in the session:
```
read path: "<output_path.png>"
```
No sidecar API is needed — inspect the generated image directly to verify quality, composition, and visual appeal.
