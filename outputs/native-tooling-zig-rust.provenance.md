# Provenance: Native tooling (Zig / Rust) for a lighter `su-code`

- **Date:** 2026-08-02
- **Slug:** `native-tooling-zig-rust`
- **Mode:** direct (no subagents — scale rule: audit answerable in ~15 grounded tool calls)
- **Rounds:** 1 research round + 1 verification round (1 FATAL, 3 MAJOR, 3 MINOR found and fixed)

## Sources
- **Consulted:** 3 external (cargo-zigbuild README via `read`, 2 web searches) + 12 local measurement commands + 6 repo files
- **Accepted:** cargo-zigbuild README (fetched and quoted directly); all local measurements
- **Rejected / demoted:** redb-vs-rusqlite sizing (web-search generality, not independently measured) → marked `[INFERENCE]`, kept out of the primary recommendation

## Verification: PASS WITH NOTES
- 4 hypotheses tested empirically; **3 falsified** (`force-unwind-tables=no`, `opt-level="s"`, `relocation-model=static`) and reported as negative results rather than dropped
- 1 arithmetic FATAL caught in review (invented "57 %" overshoot) — replaced with raw bytes + multiplier
- Remaining `[INFERENCE]`: the projected 1.0–1.5 MB saving from feature-gating. It sums `cargo-bloat` rows, which the tool itself labels guesswork, and excludes `.rodata`. Must be confirmed by re-running `cargo bloat` on a gated build.

## Artifacts
- Plan: `outputs/.plans/native-tooling-zig-rust.md`
- Raw measurements: `outputs/.drafts/native-tooling-zig-rust-research-direct.md`
- Draft: `outputs/.drafts/native-tooling-zig-rust-draft.md`
- Cited draft: `outputs/.drafts/native-tooling-zig-rust-cited.md`
- Review: `outputs/.drafts/native-tooling-zig-rust-verification.md`
- Final: `outputs/native-tooling-zig-rust.md`

## Reproduce
```bash
cargo install cargo-bloat --locked
cargo bloat --release --crates -n 25
size -A target/release/8sync
cargo tree -i zstd-sys
CARGO_PROFILE_RELEASE_OPT_LEVEL=s cargo build --release \
  --target x86_64-unknown-linux-gnu --target-dir /tmp/8sync-s
```
