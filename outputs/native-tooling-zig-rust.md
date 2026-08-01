# Native tooling for a lighter, smoother `su-code` — Zig & Rust

Research brief · 2026-08-02 · repo `su-code` @ `589807e` + WIP · all measurements taken on this machine (CachyOS x86_64, rustc stable)

---

## Executive summary

`8sync` ships at **6 406 696 bytes (6.11 MiB) stripped**. `AGENTS.md` §8 sets the budget at *"< 4 MB stripped"* — the binary is **≈1.5–1.6× that**, i.e. **~2.2–2.4 MB over**.

Three things are *not* the cause, and all three were tested rather than assumed:

- **Not the compiler flags.** The release profile is already optimal. `opt-level="s"` makes it **307 392 B bigger**; `-C force-unwind-tables=no` saves **704 bytes**.
- **Not the language.** `8sync --version` completes in **~11.6 ms** including `fork`+`exec`. There is no hot loop to rewrite.
- **Not "Rust is heavy."** Our own code is 405 KiB of `.text` — 14.5 % of it.

The cause is structural: **`crates/cli/Cargo.toml` has no `[features]` section**, so three optional subsystems and two large asset trees are compiled into every copy of the binary — including the copies that only ever run `8sync .`, `8sync doctor`, or `8sync harness`.

**Zig is worth adopting — as build tooling, never as a language here.**

---

## 1. Where the bytes actually are

```
$ stat -c%s target/release/8sync      →  6 406 696   (6.11 MiB)
$ size -A target/release/8sync
.text        2 854 517   (2.72 MiB)
.rodata      2 188 928   (2.09 MiB)   ← embedded, DEFLATE-compressed assets
.eh_frame      482 684
.rela.dyn      419 856
```

`cargo bloat --release --crates -n 25` (cargo-bloat v0.12.1), `.text` only:

| Crate | `.text` | Share |
|---|---:|---:|
| `[Unknown]` (C code) | 780.0 KiB | 28.0 % |
| `std` | 571.4 KiB | 20.5 % |
| **`_8sync` (our code)** | **405.1 KiB** | **14.5 %** |
| `axum` | 216.6 KiB | 7.8 % |
| `clap_builder` | 122.0 KiB | 4.4 % |
| `scraper` | 104.2 KiB | 3.7 % |
| `toml_edit` | 78.3 KiB | 2.8 % |
| `zstd_sys` | 58.4 KiB | 2.1 % |
| `tokio` | 46.1 KiB | 1.7 % |
| `libsqlite3_sys` | 39.8 KiB | 1.4 % |

> cargo-bloat prints: *"numbers above are a result of guesswork."* It also cannot attribute C — the 780 KiB `[Unknown]` is dominated by the bundled SQLite amalgamation plus the zstd C objects.

Raw C blobs before link-time GC: `libsqlite3.a` = **2.1 MB**; zstd objects include `zstd_fast.o` 140K, `huf_compress.o` 132K, `zstd_double_fast.o` 132K, `zstdmt_compress.o` 72K.

Embedded asset trees: `assets/` **3.0 MB** (of which `impeccable` 2.1 MB — `scripts/` alone 1.6 MB) + `web/dist` **1.9 MB**.

---

## 2. Finding: a compressor is shipped in a binary that only decompresses

```
$ cargo tree -i zstd-sys
zstd-sys v2.0.16+zstd.1.5.7
└── zstd-safe → zstd v0.13.3
    └── include-flate-compress v0.3.3
        ├── include-flate → rust-embed v8.11.0 → su-code
        └── include-flate-codegen (proc-macro) → include-flate
```

`include-flate-compress` is shared by the build-time proc-macro (which compresses) and the runtime crate (which decompresses). The **compressing** halves of `libflate` *and* `zstd` therefore link into the shipped binary. `zstd_sys` survives fat LTO at **58.4 KiB of `.text`** plus its `.rodata` tables — for functionality that can never execute at runtime.

---

## 3. Compiler knobs: exhausted (measured)

All variants built with an explicit `--target x86_64-unknown-linux-gnu` so `RUSTFLAGS` do not reach host proc-macros.

| Variant | Bytes | Δ | Verdict |
|---|---:|---:|---|
| baseline — `opt-level="z"`, `lto="fat"`, `codegen-units=1`, `strip`, `panic="abort"` | 6 406 696 | — | **keep** |
| `-C force-unwind-tables=no` | 6 405 992 | −704 B | reject — `.eh_frame` unchanged (482 772 B) |
| `CARGO_PROFILE_RELEASE_OPT_LEVEL=s` | 6 714 088 | **+307 392 B** | reject |
| `-C relocation-model=static` (no explicit target) | build error | — | invalid — proc-macros require PIC |

**Nothing further to win here.** Anyone proposing another flag should be asked for an A/B number.

---

## 4. Recommendations, ordered by measured payoff per unit of risk

### R1 — Introduce `[features]` and gate the three optional subsystems ★ highest payoff
No `[features]` section exists today. Add one, default to the lean set, and let the release build opt in:

```toml
[features]
default = []
web       = ["dep:axum", "dep:tokio", "dep:tower-http", "dep:scraper"]
toolstats = ["dep:rusqlite"]
full      = ["web", "toolstats"]
```
…with `optional = true` on those five dependencies and `#[cfg(feature = "…")]` on `verbs/harness/{web,marketplace,toolstats}.rs` plus their dispatch arms in `verbs/harness/mod.rs`.

Estimated removal from `.text`: web stack ≈ 354 KiB, scraping ≈ 124 KiB, SQLite (Rust bindings + its share of the C surface) ≈ 0.6–1.0 MB. **`[INFERENCE]` — projected ~1.0–1.5 MB total; confirm with `cargo bloat` on the gated build before claiming it.**

Ponytail note: gating is strictly cheaper than rewriting `toolstats` off SQL. Its schema is one table with `INSERT OR IGNORE` + `COUNT`/`GROUP BY` — genuinely a fit for SQLite. Gate first; only consider `redb` or a plain append-only file if `full` builds still miss the budget.

### R2 — Stop embedding what can be fetched ★ biggest single lever on `.rodata`
`web/dist` (1.9 MB) and `impeccable/scripts` (1.6 MB) are 3.5 MB of the 4.9 MB embedded raw. `web/dist` only matters under the `web` feature; `impeccable/scripts` is only read after the skill is deployed to `~/.omp/skills/`. Either gate them behind features or ship them as a release side-car the way `codegraph` is already installed. This also removes the pressure that justified the `compression` feature in the first place.

### R3 — Drop the dead compressor
Once R2 shrinks the embed set, re-evaluate `rust-embed`'s `compression` feature. Dropping it removes the `include-flate → zstd` chain entirely (58.4 KiB `.text` + tables) at the cost of larger `.rodata` — measure both ways. If compression must stay, pre-compress assets in `build.rs` and link only a decompressor.

### R4 — Zig in CI: `cargo-zigbuild` replaces Docker, and merges the macOS legs
Today `.github/workflows/release.yml` builds 5 assets and shells out to **`cross` (Docker)** for `aarch64-unknown-linux-musl`.

`cargo-zigbuild` uses Zig's bundled clang/lld as the cross-linker, so:
- `aarch64-unknown-linux-musl` builds on a plain `ubuntu-latest` runner — **no Docker layer**, and both Linux legs can share one runner.
- `universal2-apple-darwin` (Rust ≥ 1.64) collapses `x86_64-apple-darwin` + `aarch64-apple-darwin` into **one fat asset**.
- glibc-pinned triples (`aarch64-unknown-linux-gnu.2.17`) exist, but are **irrelevant here** — this project already ships musl-static Linux builds, which is the stronger portability guarantee.

**Hard caveat, quoted from the upstream README:** *"Currently only Linux and macOS targets are supported."* The Windows MSVC leg stays exactly as it is. Also `-C target-feature=+crt-static` against glibc is explicitly unsupported (musl-static is unaffected).

Net: 5 legs → 4, one Docker dependency removed. A CI simplification, **not** a binary-size change.

### R5 — Explicitly do **not** do these
- **Do not** replace the 9 `curl` shell-outs with `reqwest`/`ureq`. Shell-out costs **0 bytes**; `AGENTS.md` §8 bans the heavy dep by name; `curl` ships with Windows 10+, macOS, and every target distro. A TLS stack would *add* ~1 MB to fix a non-problem.
- **Do not** rewrite anything in Zig-the-language. There is no compute hot path — the binary is an IO/orchestration front-end that spends its life in `fork`/`exec` and file writes. A Zig module would add a second toolchain, break `cargo install`, and buy nothing measurable.
- **Do not** optimise startup. 11.6 ms per invocation, 10 ms for `help`.
- **Do not** touch `[profile.release]`. See §3.

---

## 5. Suggested order of work

1. R1 feature-gate (`web`, `toolstats`) → re-measure with `cargo bloat --release --crates`
2. R2 un-embed `web/dist` + `impeccable/scripts` → re-measure `.rodata`
3. R3 re-evaluate `compression` with the smaller embed set
4. R4 CI swap to `cargo-zigbuild` for the two Linux legs + `universal2` for macOS
5. Update the `AGENTS.md` §8 budget line to whatever the gated default build actually weighs — a budget nobody enforces is not a budget

---

## Open questions
- Does gating `web` let `web/dist` leave the embed set cleanly? `rust-embed` folder paths resolve at compile time, so the `#[derive(RustEmbed)]` site must be `cfg`-gated too.
- Does the installer's asset-naming scheme (`8sync-<version>-darwin-{x86_64,arm64}`) tolerate a single `universal2` asset, or do `install.sh`/`install.ps1` need matching changes first?

## Sources
See `outputs/.drafts/native-tooling-zig-rust-cited.md` for the full list — every number above is either a command run on this machine (recorded in `outputs/.drafts/native-tooling-zig-rust-research-direct.md`) or a quote from the fetched `cargo-zigbuild` README.
