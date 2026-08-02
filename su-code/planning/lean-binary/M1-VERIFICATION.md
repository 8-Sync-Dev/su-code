# M1 — Verification

Phase: M1 Feature gating + attribution · 2026-08-02 · host `x86_64-unknown-linux-gnu`

## Measured attribution (`bash scripts/size-report.sh`)

| combination | bytes | vs full | vs 4 MiB budget |
|---|---:|---:|---:|
| `full` (default) | **6 407 144** | 0 | **+52.76 %** |
| `web` only | 5 346 304 | −1 060 840 | +27.47 % |
| `toolstats` only | 4 144 576 | −2 262 568 | **−1.19 %** |
| `minimal` | **3 081 416** | −3 325 728 | **−26.53 %** |

**Gate cost** (full minus the build without it):

| gate | cost |
|---|---:|
| `web` (axum + tokio + tower-http + scraper + `web/dist` embed) | **2 262 568 B (2.16 MiB)** |
| `toolstats` (`rusqlite` bundled) | **1 060 840 B (1.01 MiB)** |
| both | 3 325 728 B (3.17 MiB) |

Internal consistency check: `web-only + toolstats-only − minimal` = 6 409 464 vs
the measured full 6 407 144 — a 2 320 B gap, i.e. shared code counted twice.
The measurement is coherent.

## The headline result: `cargo bloat` under-attributed SQLite by ~26×

M0's brief, quoting `cargo bloat --crates`, put `libsqlite3_sys` at **40 KiB**
and treated the 780 KiB `[Unknown]` row as unattributed C. The A/B says the
`toolstats` gate really costs **1 060 840 B**. `cargo bloat` measures `.text`
attribution by symbol and prints *"numbers are a result of guesswork"* — it
cannot see `.rodata`, static tables, or the C blob's true footprint. **Only the
A/B is load-bearing.** This is exactly the failure mode `deep-research` §5 step 3
exists to prevent, now confirmed on a real binary.

## AC matrix

| AC | Criterion | Result |
|---|---|---|
| AC-01 | `[features]` with `default = ["web","toolstats"]`; five deps `optional = true` | **PASS** — `crates/cli/Cargo.toml:14-23`, deps at `:40-51` |
| AC-02 | Default build within ±4 KiB of the 6 407 848 B baseline | **PASS** — 6 407 848 B in-tree (identical); 6 407 144 B under an explicit `--target` (−704 B, different target dir) |
| AC-03 | `--no-default-features` compiles | **PASS**, and with **zero warnings** |
| AC-04 | Lean binary runs | **PASS** — `--version`, `help`, `flow`, `harness -h`, `feature list`, `doctor` all exit 0 |
| AC-05 | Gated subcommand explains itself | **PASS** — `Error: \`harness web\` is not built into this binary — rebuild with \`--features web\`` (same for `toolstats`); `harness help` omits both lines in a lean build, prints both in a default build |
| AC-06 | Four combinations measured, each own `--target-dir` + explicit `--target` | **PASS** — table above |
| AC-07 | `scripts/size-report.sh` reproduces it | **PASS** |
| AC-08 | No behaviour change in a default build | **PASS** — dashboard `HTTP 200`, `/api/bench` `200`, `/api/marketplace` `200` (the `scraper` path), `harness toolstats` prints its report |

## Deviations

- **T3 folded into T2's commit** (`95a4a26`). The `toolstats` module/dispatch/help
  gating lives in the same `harness/mod.rs` hunks as the `web` gating; splitting
  one file's cfg block across two commits would have been noise, not history.
- **Scope grew by five items.** The first lean build emitted 11 dead-code
  warnings, which was the compiler correctly reporting that
  `harness/knowledge.rs` (whole module), `bench::BenchMetrics`,
  `bench::bench_metrics` and `here::scaffold_project` are *also* dashboard-only.
  Reference search confirmed a single caller each in `web.rs`; all are now gated.
  A warning-clean lean build was worth the extra edits.
- **Live slip, caught and fixed:** `cargo build --release --no-default-features`
  *without* `--target-dir` overwrites `target/release/8sync`, and that lean
  binary was briefly installed onto `PATH`. Recorded as a `gotcha:` in
  `su-code/KNOWLEDGE.md`; `scripts/size-report.sh` always passes `--target-dir`
  precisely to avoid this.

## What this means for M2 (the important part)

**Gating shipped 0 user-visible bytes, as D-M1-1 predicted.** The value of M1 is
the measurement, and it points at one unambiguous target:

- `toolstats` costs **1.01 MiB** to store a flat call log and answer `COUNT` /
  `GROUP BY` over a few thousand rows. Replacing the bundled SQLite C
  amalgamation with an append-only file + in-memory aggregation removes ~1 MB
  **with no feature loss**. Highest value/risk ratio in the whole project.
- `web`'s 2.16 MiB is mostly the `web/dist` embed, which is irreducible while a
  dashboard ships; `axum` + `tokio` are the only compressible part, and
  hand-rolling HTTP is a footgun, not a saving.
- Even after M2 removes ~1 MB, the full build lands near **5.35 MB** — still over
  §8's 4 MB. M3 must therefore either trim the Vite bundle or amend the budget
  to a number this table can justify. Deciding that on data is M3's job.

## Verdict

**M1 PASS** — 8/8 AC. Proceed to M2.
