# Project — Lean Binary

## What

Bring the `8sync` binary back under its own stated weight budget, and land the
pending WIP that is blocking a clean tree — **without removing a single
user-visible feature**.

## Core value

`8sync` is installed by a `curl | sh` one-liner onto machines that may be
remote, metered, or freshly imaged. Every megabyte is download latency on first
install and on every `8sync up`. The binary is currently **6 406 696 B
(6.11 MiB)** against the `AGENTS.md` §8 budget of **"< 4 MB stripped"** — a
self-imposed contract the repo is silently ~1.5× over.

## Cắm vào codebase

Measured in `outputs/native-tooling-zig-rust.md` (2026-08-02). The weight is in
three optional subsystems that are **unconditionally linked** because
`crates/cli/Cargo.toml` has no `[features]` section at all:

| subsystem | crate(s) | entry point | evidence |
|---|---|---|---|
| dashboard | `axum` + `tokio` + `tower-http` | `verbs/harness/web.rs` | 354 KiB `.text` |
| MCP marketplace | `scraper` (html5ever + cssparser) | `verbs/harness/marketplace.rs` | 124 KiB `.text` |
| tool-call tracker | `rusqlite` (`bundled` → SQLite C amalgamation) | `verbs/harness/toolstats.rs` | `libsqlite3.a` 2.1 MB pre-GC; dominates the 780 KiB `[Unknown]` C row |

Plus embedded asset trees via `rust-embed` in `assets.rs`: `Assets`
(`assets/`, 3.0 MB raw) and `WebAssets` (`web/dist/`, 1.9 MB raw) →
`.rodata` 2 188 928 B after `compression`.

Usage is cleanly contained — each heavy crate is referenced from exactly one
module (verified by symbol search), so gating is mechanical, not a refactor.

## Ràng buộc

- **No feature loss.** The released binary keeps `harness web`, `marketplace`
  and `toolstats`. Gates exist so a build *can* be lean; elimination work must
  preserve behaviour.
- **No new heavy deps.** `AGENTS.md` §8: no `reqwest`; HTTP stays `curl`
  shell-out. `tokio`/`axum` remain allowed *only* behind the `web` gate.
- **Release-profile knobs are closed.** A/B'd and recorded as `failure:` in
  `su-code/KNOWLEDGE.md`: `opt-level="s"` is +307 392 B vs `"z"`;
  `force-unwind-tables=no` saves 704 B; `relocation-model=static` breaks
  proc-macro builds. Do not re-litigate.
- **Measure every step.** Per `deep-research` SKILL §5: no size claim without
  an A/B byte count from a scratch `--target-dir` with an explicit `--target`.
- Verb count ≤ 22 flat; binary target < 4 MB (`AGENTS.md` §8).

## KHÔNG đụng

- The 9 `curl` shell-out call sites (0 bytes; replacing them *adds* ~1 MB of TLS).
- Startup time (`--version` 11.6 ms incl. fork+exec — there is no hot path).
- `[profile.release]` in the workspace `Cargo.toml`.
- The `impeccable` skill's content (2.1 MB of `assets/`) — it is an always-on
  skill; un-embedding it needs a network fallback and is out of scope here.
