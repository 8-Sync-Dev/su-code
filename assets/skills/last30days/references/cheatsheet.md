# last30days cheatsheet

One-page reference. For when-to-use rules and invocation laws see `../SKILL.md`.
Authoritative runtime contract is the upstream `SKILL.md` inside the install dir.

## Install / update / remove

| Action | Command |
|---|---|
| Install (global, any host) | `npx skills add mvanhorn/last30days-skill -g` |
| Install for a specific host | `npx skills add mvanhorn/last30days-skill -g -a codex` (or `cursor`, `gemini-cli`, `opencode`, `claude-code`) |
| Claude Code native plugin | `/plugin marketplace add mvanhorn/last30days-skill` then `/plugin install last30days` |
| Update | `npx skills update last30days -g` |
| List / remove | `npx skills list -g` · `npx skills remove last30days -g` |

## Invocation flags

| Flag | Effect |
|---|---|
| `<topic>` | Person, company, product, technology, or `A vs B [vs C]` comparison |
| `--emit=html` | Save a self-contained dark-mode HTML brief (Slack/Notion/email ready) |
| `--emit=compact` | Compact stdout synthesis (default model-facing emit) |
| `--competitors` | Auto-discover top 2 peers and run a 3-way comparison |
| `--github-user=<handle>` | Person-mode GitHub: author-scoped PRs/repos/releases instead of keyword search |
| `--x-handle=<handle>` / `--x-related=<list>` | Pin the X account(s) for a person/topic |
| `--subreddits=<list>` | Target specific subreddits |
| `--plan <file>` | Pass a host-model-generated JSON query plan (REQUIRED for named entities) |
| `--store` | Persist findings into SQLite for trend monitoring across runs |
| `--save-dir <path>` / `--save-suffix=<name>` | Where the `*-raw.md` brief is written / keep variants apart |
| `INCLUDE_SOURCES=` / `EXCLUDE_SOURCES=` | Opt-in/out comma-separated sources (e.g. `perplexity`, `tiktok`, `youtube_comments`) |

## Sources & keys

| Sources | Needs | Cost |
|---|---|---|
| Reddit (+comments), Hacker News, Polymarket, GitHub | nothing | free |
| X / Twitter | logged-in x.com session (`AUTH_TOKEN`/`CT0`) | free |
| YouTube transcripts | `yt-dlp` on PATH | free |
| Bluesky | `BSKY_HANDLE`/`BSKY_APP_PASSWORD` (app password) | free |
| TikTok / Instagram / Threads / Pinterest / YT+TikTok comments | `SCRAPECREATORS_API_KEY` | 100 free credits, then PAYG |
| Perplexity Sonar | `OPENROUTER_API_KEY` + `INCLUDE_SOURCES=perplexity` | PAYG |
| Web search | `BRAVE_API_KEY` | 2,000 free/mo |

## Runtime requirements

- Python 3.12+ on PATH (`python3.12`/`python3.13`/`python3.14`).
- Node.js (vendored Bird client for X search).
- Output saved under `LAST30DAYS_MEMORY_DIR` (default `~/Documents/Last30Days/`).

## Output shape (what valid runs look like)

- First line is the badge: `🌐 last30days v{VERSION} · synced {YYYY-MM-DD}`.
- GENERAL/NEWS queries: `What I learned:` + bold-lead-in paragraphs + `KEY PATTERNS from the research:` numbered list.
- COMPARISON queries: `# A vs B: What the Community Says` + Quick Verdict + per-entity + Head-to-Head + Bottom Line.
- Always ends with the engine's `✅ All agents reported back!` emoji-tree footer (passed through verbatim).
- No trailing `Sources:` / `References:` block — citations are inline `[name](url)`.
