# Research notes — measured on this machine (2026-08-02)

Host: CachyOS x86_64 · rustc stable (`rust-toolchain.toml`, profile minimal) · repo `su-code` @ `589807e` + WIP.

## A. Binary & sections

```
$ stat -c%s target/release/8sync
6406696                      # 6.11 MiB stripped
$ size -A target/release/8sync
.text                2854517
.rodata              2188928
.eh_frame             482684
.eh_frame_hdr          70596
.rela.dyn             419856
Total                8824839
```

AGENTS.md §8 states the budget: *"Binary size target: < 4 MB stripped (tăng từ 2 MB khi bundle `impeccable`)"*. Actual = **6.11 MiB → 57 % over budget.**

## B. Per-crate `.text` attribution (`cargo bloat --release --crates -n 25`)

```
 8.2%  28.0% 780.0KiB [Unknown]        # C code — not attributable by cargo-bloat
 6.0%  20.5% 571.4KiB std
 4.3%  14.5% 405.1KiB _8sync           # our own code
 2.3%   7.8% 216.6KiB axum
 1.3%   4.4% 122.0KiB clap_builder
 1.1%   3.7% 104.2KiB scraper
 0.8%   2.8%  78.3KiB toml_edit
 0.6%   2.1%  58.4KiB zstd_sys
 0.5%   1.7%  46.1KiB tokio
 0.4%   1.4%  39.8KiB libsqlite3_sys
 0.4%   1.3%  37.0KiB hyper
 ...
29.3% 100.0%   2.7MiB .text section size
```

Grouped by subsystem:
- **`harness web`** (axum + axum_core + hyper + http + tower + tokio + serde_path_to_error) ≈ **354 KiB** `.text`
- **`harness web` marketplace scraping** (scraper + cssparser + html5ever) ≈ **124 KiB** `.text`
- **`harness toolstats`** (libsqlite3_sys 39.8 KiB Rust + the SQLite amalgamation inside `[Unknown]`)

## C. C blobs

```
$ du -h .../libsqlite3-sys-*/out/libsqlite3.a
2,1M      # SQLite amalgamation static lib (linker GC keeps only reachable parts)
$ find target/release/build -name '*.o'   # zstd-sys
huf_compress.o 132K · zstd_double_fast.o 132K · zstd_fast.o 140K ·
zstdmt_compress.o 72K · fastcover.o 48K · zstd_ldm.o 48K · …
```

`[Unknown]` 780 KiB in `.text` is overwhelmingly this C surface.

## D. Surprise finding — a compressor shipped in a binary that only decompresses

```
$ cargo tree -i zstd-sys
zstd-sys v2.0.16+zstd.1.5.7
└── zstd-safe → zstd v0.13.3
    └── include-flate-compress v0.3.3
        ├── include-flate v0.3.3 → rust-embed v8.11.0 → su-code
        └── include-flate-codegen (proc-macro) → include-flate
```

`include-flate-compress` is shared by the **proc-macro** (compress at build time) and the **runtime** (decompress). The compressing half of both `libflate` and `zstd` is linked into the shipped binary. `zstd_sys` still shows **58.4 KiB** of `.text` after fat LTO, plus its `.rodata` tables.

## E. Embedded assets

```
assets/            3,0M   (impeccable 2,1M — of which scripts/ 1,6M · reference/ 384K)
web/dist           1,9M   (Vite frontend for `harness web`)
```
≈ 4.9 MB raw, DEFLATE-compressed into the 2.19 MB `.rodata`.

## F. Runtime

```
10 × `8sync --version`  → 116 ms wall  (≈11.6 ms/run, incl. fork+exec)
1  × `8sync help`       → 10 ms
```
Not a bottleneck. Any "make it faster" work here is premature optimisation.

## G. Dependency + shell-out surface

- `Cargo.lock`: **205 packages**
- Shell-outs (`Command::new`): `curl` ×9, `systemctl` ×7, `git` ×6, `sh -c` ×6, plus `pacman`, `npm`
- No `[features]` section exists in `crates/cli/Cargo.toml` — everything compiles unconditionally.

## H. A/B experiments (all built with `--target x86_64-unknown-linux-gnu` so RUSTFLAGS skip host proc-macros)

| Variant | Bytes | Δ vs baseline | Verdict |
|---|---:|---:|---|
| baseline (`opt-level="z"`, `lto="fat"`, `panic="abort"`, `strip`) | 6 406 696 | — | keep |
| `-C force-unwind-tables=no` | 6 405 992 | **−704 B (0.01 %)** | reject — `.eh_frame` stayed 482 772 B |
| `CARGO_PROFILE_RELEASE_OPT_LEVEL=s` | 6 714 088 | **+307 392 B (+4.8 %)** | reject |
| `-C relocation-model=static` (no explicit target) | — | build error | invalid — proc-macros require PIC |

## I. CI release matrix (`.github/workflows/release.yml`)

5 assets: `x86_64-unknown-linux-musl` (native + `musl-tools`), `aarch64-unknown-linux-musl` (**via `cross`, i.e. Docker**), `x86_64-apple-darwin` + `aarch64-apple-darwin` (both on `macos-14`), `x86_64-pc-windows-msvc`.

## J. External sources consulted

- `cargo-zigbuild` — uses Zig's bundled clang/lld as the cross linker; supports glibc-version-pinned triples (`aarch64-unknown-linux-gnu.2.17`) and musl static targets without Docker sysroots.
- `rusqlite` `bundled` vs `redb` — bundled compiles the SQLite C amalgamation in (several hundred KB → >1 MB); `redb` is pure Rust, smaller, and cross-compiles without a C toolchain, but is key-value, not SQL.
