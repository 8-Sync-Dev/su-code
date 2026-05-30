---
name: last30days
description: Use this skill when the user asks "what are people saying about X", "research <topic> recently", "what's trending on Reddit/X/YouTube about Y", pre-meeting/pre-call briefings, "last 30 days of Z", competitor scans, or any recency-grounded social research. It runs the `/last30days` agent skill (separately installed engine) that searches Reddit, X, YouTube, TikTok, Hacker News, Polymarket, GitHub, Bluesky and the web in parallel, scores by real engagement, and synthesizes one cited brief. Prefer it over ad-hoc WebSearch when the user wants what the community actually thinks RIGHT NOW.
---

# last30days — recency-grounded multi-source research

`/last30days` is an installable Agent Skill (by `mvanhorn`, MIT) that researches what people
actually say about any topic across Reddit, X, YouTube, TikTok, Hacker News, Polymarket, GitHub,
Bluesky and the web — scored by upvotes/likes/odds, then synthesized into one cited brief.

It is **NOT bundled with 8sync** — it ships its own Python engine (`scripts/last30days.py`) and is
installed through the host's skill mechanism. This doc tells you when to reach for it and how it is
invoked; the authoritative runtime contract lives in the upstream `SKILL.md` inside the install dir.

## 1. When to use it

| User intent | Reach for |
|---|---|
| "What are people saying about `<topic>` lately?" | `/last30days <topic>` |
| Pre-meeting / pre-sales brief on a person or company | `/last30days <name>` |
| "What's trending on Reddit/X/YouTube about `<X>`?" | `/last30days <X>` |
| Compare tools/companies by community sentiment | `/last30days A vs B [vs C]` |
| "What do users actually want in `<product/space>`?" | `/last30days what users want in <space>` |
| Shareable HTML brief for Slack/Notion | `/last30days <topic> --emit=html` |

Use this over a raw `WebSearch` loop whenever the answer depends on **recent, engagement-weighted,
multi-platform** community signal (Reddit comments, X threads, YouTube transcripts, Polymarket odds)
rather than editorial SEO results.

## 2. Install (engine is separate — do this once per machine)

```bash
# Any of 50+ Agent Skills hosts (codex, cursor, gemini-cli, opencode, claude-code, ...)
npx skills add mvanhorn/last30days-skill -g          # -g = global, available in every project

# Claude Code native plugin (auto-updates):
#   /plugin marketplace add mvanhorn/last30days-skill
#   /plugin install last30days
```

Reddit (+comments), Hacker News, Polymarket, and GitHub work with **zero config**. The first
`/last30days` run launches a 30-second setup wizard that unlocks the keyed sources below.

## 3. Sources & optional keys

| Sources | Needs | Cost |
|---|---|---|
| Reddit (+comments), Hacker News, Polymarket, GitHub | nothing | free |
| X / Twitter | logged-in x.com browser session (`AUTH_TOKEN`/`CT0`) | free |
| YouTube transcripts | `yt-dlp` on PATH | free |
| Bluesky | app password (`BSKY_HANDLE`/`BSKY_APP_PASSWORD`) | free |
| TikTok, Instagram, Threads, Pinterest, YT/TikTok comments | `SCRAPECREATORS_API_KEY` | 100 free credits, then PAYG |
| Perplexity Sonar | `OPENROUTER_API_KEY` + `INCLUDE_SOURCES=perplexity` | PAYG |
| Web search | `BRAVE_API_KEY` | 2,000 free/mo |

## 4. Invocation rules (when YOU are the host model running it)

- **Run the engine — do not answer from WebSearch alone.** A prose summary built only from a few
  WebSearch calls is NOT valid output; the Python engine is the skill.
- **No topic given → ask once for a topic.** Do not research a blank query.
- **Named-entity topics REQUIRE a query plan** (`--plan <file>`): you are the planner — generate the
  JSON plan upstream and pass it; never inline apostrophes in `--plan`.
- **Pass the engine's emoji-tree footer (`✅ All agents reported back!`) through verbatim** — a
  response without it did not actually run the skill.
- Output is a single cited brief, not a `Sources:` link dump. Citations are inline `[name](url)`.

Full flag reference, output LAWs, and the per-source key matrix: see `references/cheatsheet.md`.

## 5. Citation convention

Echo the brief's inline `[name](url)` citations verbatim. Research files are saved under
`LAST30DAYS_MEMORY_DIR` (default `~/Documents/Last30Days/`); cite the saved `*-raw.md` path when the
user wants the durable artifact.

Homepage: https://github.com/mvanhorn/last30days-skill
