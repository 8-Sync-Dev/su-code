# Draft — Native tooling for a lighter, smoother `su-code`

## Executive summary
The binary is **6.11 MiB stripped, 57 % over the 4 MB budget AGENTS.md §8 sets for itself.** The cause is not "Rust is heavy" and not a missing compiler flag — the size knobs are already at their optimum (proven by A/B: `opt-level="s"` is *worse* by 307 KB, `force-unwind-tables=no` buys 704 bytes). The cause is that **three optional subsystems and two large asset trees compile into every copy of the binary unconditionally**, because `crates/cli/Cargo.toml` has no `[features]` section at all.

Zig is worth adopting — but as **build tooling, not as a language**. There is no hot compute path in an orchestration CLI whose `--version` runs in 11.6 ms; rewriting anything in Zig would be pure cost.

## Findings by theme

### 1. The weight is optional features, not the language
Grouped `cargo-bloat` `.text` attribution:
- `harness web` (axum/hyper/tokio/tower/http) ≈ **354 KiB**
- marketplace HTML scraping (scraper/cssparser/html5ever) ≈ **124 KiB**
- `harness toolstats` (SQLite: 39.8 KiB Rust bindings + the bulk of the 780 KiB `[Unknown]` C surface; the amalgamation `.a` is 2.1 MB pre-GC)
- Embedded assets: `assets/` 3.0 MB (impeccable alone 2.1 MB, `scripts/` 1.6 MB) + `web/dist` 1.9 MB → compressed into 2.19 MB `.rodata`

Every one of these serves a subcommand most invocations never reach.

### 2. A compressor is shipped in a binary that only ever decompresses
`rust-embed`'s `compression` feature → `include-flate` → `include-flate-compress`, which is shared between the build-time proc-macro and the runtime crate. Result: **both `libflate` and `zstd` compressors link into the shipped binary**; `zstd_sys` still holds 58.4 KiB of `.text` after fat LTO.

### 3. Compiler knobs are exhausted — measured, not assumed
| Variant | Δ | Verdict |
|---|---:|---|
| `-C force-unwind-tables=no` | −704 B | reject |
| `opt-level="s"` | +307 KB | reject |
| `-C relocation-model=static` | build error (proc-macros need PIC) | invalid |

The existing profile (`opt-level="z"`, `lto="fat"`, `codegen-units=1`, `strip`, `panic="abort"`) is already correct.

### 4. Zig's real role: the release pipeline
CI ships 5 assets and uses `cross` (Docker) for `aarch64-unknown-linux-musl`. `cargo-zigbuild` replaces that with Zig's bundled clang/lld — no Docker — and offers `universal2-apple-darwin`, collapsing the two macOS assets into one. **Caveat, from the upstream README: only Linux and macOS targets are supported**, so the Windows MSVC leg must stay as-is.

### 5. What NOT to do
- Don't replace the 9 `curl` shell-outs with `reqwest`/`ureq`: shell-out costs 0 bytes, AGENTS.md §8 forbids the heavy dep, and `curl` ships on Win10+/macOS/Linux.
- Don't rewrite `toolstats` from SQL to a hand-rolled store before simply feature-gating it.
- Don't chase startup latency: 11.6 ms including fork+exec.

## Caveats
- `cargo-bloat`'s own output warns its numbers are guesswork; it cannot attribute C code (hence `[Unknown]`).
- Section sizes are exact; per-subsystem savings are *estimates* until a feature-gated build is measured.
- The 2.1 MB SQLite `.a` is pre-link-GC; the shipped cost is a fraction of it.

## Open questions
- Does gating `web` also let `web/dist` drop out of the embed set? (rust-embed folder paths are compile-time)
- Is `universal2` acceptable for the installer's asset-naming scheme?

## Sources

### Verified locally (this machine, 2026-08-02)
- Binary size / sections — `stat -c%s target/release/8sync`, `size -A target/release/8sync`
- Per-crate attribution — `cargo bloat --release --crates -n 25` (cargo-bloat v0.12.1)
- C blobs — `du -h .../libsqlite3-sys-*/out/libsqlite3.a`, `find target/release/build -name '*.o'`
- zstd provenance — `cargo tree -i zstd-sys`
- Assets — `du -sh assets/* web/dist`
- Startup — `time (for i in $(seq 10); do ./target/release/8sync --version; done)`
- A/B builds — `RUSTFLAGS="-C force-unwind-tables=no"` and `CARGO_PROFILE_RELEASE_OPT_LEVEL=s`, both with `--target x86_64-unknown-linux-gnu`
- Repo facts — `Cargo.toml`, `.cargo/config.toml`, `crates/cli/Cargo.toml`, `.github/workflows/release.yml`, `AGENTS.md` §8

### External (URL fetched and read)
- cargo-zigbuild README — https://github.com/rust-cross/cargo-zigbuild
  - glibc-version-suffixed targets (`aarch64-unknown-linux-gnu.2.17`)
  - `universal2-apple-darwin` support (Rust ≥ 1.64)
  - Caveat 1: "Currently only Linux and macOS targets are supported"
  - Caveat: `-C target-feature=+crt-static` for glibc "is **not supported**"
- Zig as a drop-in C cross-compiler — https://andrewkelley.me/post/zig-cc-powerful-drop-in-replacement-gcc-clang.html (linked from the README above)
- rusqlite `bundled` vs redb sizing — general guidance from web search; **not independently measured here**, therefore treated as [INFERENCE] and not used as the basis of the primary recommendation.
