# Requirements — Lean Binary

## v1 (in scope)

| UC | Use case |
|---|---|
| UC-1 | A maintainer with a dirty tree of finished-but-uncommitted deliverables can land them as atomic, verified commits, so the repo has a clean base to optimise from. |
| UC-2 | A developer can build a minimal `8sync` (`--no-default-features`) that drops the dashboard, marketplace scraping and the tool-call tracker, and get a materially smaller binary. |
| UC-3 | A maintainer can attribute binary weight to a specific subsystem with one command, so "what costs what" is measured, never argued. |
| UC-4 | An end user gets the same full feature set from the released binary, at a smaller download, because dead weight (unused C code, redundant parsers) is eliminated rather than merely gated. |
| UC-5 | A reader of `AGENTS.md` sees a size budget that matches reality and a CI matrix that actually enforces it. |
| UC-6 | A release build cross-compiles to `aarch64` Linux without a Docker/`cross` leg, and macOS ships one universal asset instead of two. |

## v2 (later)

- Un-embed `impeccable/scripts` (1.6 MB) with a lazy GitHub-raw fetch on first use.
- `install.sh` picking a lean vs full asset per host capability.

## Out of scope

- Rewriting any subsystem in Zig/C/asm (no compute hot path exists — measured).
- Replacing `curl` shell-outs with a Rust HTTP client.
- Startup-latency work.
- Changes to `[profile.release]` (all knobs A/B'd and falsified).
