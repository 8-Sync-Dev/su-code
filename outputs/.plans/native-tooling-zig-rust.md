# Plan — Native tooling (Zig / Rust) to make `su-code` lighter & smoother

Slug: `native-tooling-zig-rust` · Date: 2026-08-02 · Mode: **direct** (no subagents)

## Key questions
1. Where does the `8sync` binary weight actually come from? (measured, not guessed)
2. Which `[profile.release]` / RUSTFLAGS knobs still have headroom? (test, don't assume)
3. Does **Zig** have a real role — as a language, or as build/link tooling?
4. Which Rust deps are over-weight for what they deliver, and what are the native alternatives?
5. What should explicitly **NOT** change (ponytail / YAGNI guard)?

## Evidence needed
- Binary size + section breakdown (`size -A`), per-crate `.text` attribution (`cargo-bloat`)
- Embedded asset weight (`assets/`, `web/dist`), C blob weight (SQLite `.a`, zstd `.o`)
- Startup latency, dependency count, shell-out surface
- Empirical A/B builds for each proposed compiler knob
- CI release matrix (what cross-compiles today and how)

## Scale decision
Explainer/audit answerable in ~15 grounded tool calls on a codebase already indexed here → **direct mode**, no subagent fan-out. Inflating this into a 4-agent survey would violate the scale rule.

## Task ledger
- [x] Ground binary size, sections, profile, toolchain
- [x] Per-crate attribution via `cargo-bloat --crates`
- [x] Asset + C-blob weight
- [x] A/B: `force-unwind-tables=no`, `opt-level="s"`, `relocation-model=static`
- [x] Dependency provenance for surprise crates (`cargo tree -i`)
- [x] CI matrix review
- [x] External research: `cargo-zigbuild`, redb vs bundled SQLite
- [x] Draft → cite → review → deliver

## Verification log
| Hypothesis | Method | Result |
|---|---|---|
| `panic="abort"` leaves droppable `.eh_frame` | A/B build w/ `-C force-unwind-tables=no` | **FALSIFIED** — −704 B |
| `opt-level="s"` may beat `"z"` | A/B build via `CARGO_PROFILE_RELEASE_OPT_LEVEL` | **FALSIFIED** — +307 KB |
| `relocation-model=static` cuts `.rela.dyn` | A/B build | **BLOCKED** — breaks proc-macro build |
| Binary is over the 4 MB budget in AGENTS.md §8 | `stat -c%s` | **CONFIRMED** — 6.11 MiB |
| A compressor is linked but never used at runtime | `cargo tree -i zstd-sys` | **CONFIRMED** — via `include-flate` |
