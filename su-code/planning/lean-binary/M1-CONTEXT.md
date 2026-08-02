# M1 — Feature gating + attribution

## 📌 Requirement scope

UC-2 (buildable lean binary) · UC-3 (one-command weight attribution).

## 🎯 Goal

`crates/cli/Cargo.toml` grows a `[features]` table, the three optional
subsystems become genuinely optional, and each gate's cost is a **measured byte
delta** rather than `cargo bloat` guesswork.

## Structural finding that reshapes the plan

`marketplace` is **not** a third subsystem. Symbol search shows exactly one
caller — `verbs/harness/web.rs:1452` (`super::marketplace::catalog`) behind the
`/api/marketplace` route. There is no `harness marketplace` verb. So `scraper`
(+ `html5ever`, `cssparser`, `selectors`) is dashboard-only and belongs inside
the `web` gate, not beside it.

Final gate shape — **two** features, not three:

| feature | pulls | covers |
|---|---|---|
| `web` | `axum`, `tokio`, `tower-http`, `scraper` | `verbs/harness/web.rs`, `verbs/harness/marketplace.rs`, `assets::WebAssets` + `assets::web_asset`, the `web/dist` Vite build in `build.rs` |
| `toolstats` | `rusqlite` (`bundled`) | `verbs/harness/toolstats.rs` |

## Decisions

- **D-M1-1** — `default = ["web", "toolstats"]`. The released binary keeps every
  feature (PROJECT constraint "no feature loss"); the gates exist so the cost is
  *measurable* and so a lean build is possible. Gating alone is therefore
  expected to ship **0 bytes** of user-visible win — that is M2's job. Saying so
  up front stops M1 from being mistaken for the fix.
- **D-M1-2** — `marketplace` folds into `web` (evidence above). No third flag.
- **D-M1-3** — gate at the **module + dispatch arm**, and cfg the clap `Sub`
  match arms, so a lean build reports `harness web: not built into this binary
  (build with --features web)` instead of silently accepting the subcommand.
- **D-M1-4** — attribution ships as `scripts/size-report.sh`, **not** a new verb.
  `AGENTS.md` §8 caps the flat verb count at 22 and a verb would add bytes to
  the thing being measured. A script costs zero binary.
- **D-M1-5** — `build.rs` skips the Vite build and the `web/dist` fallback when
  `CARGO_FEATURE_WEB` is unset, so a lean build needs no JS toolchain at all.

## ✅ Acceptance Criteria

| AC | Criterion | How verified |
|---|---|---|
| AC-01 | `[features]` exists with `default = ["web","toolstats"]`; `axum`/`tokio`/`tower-http`/`scraper`/`rusqlite` are `optional = true` | `cargo tree -e features` / manifest read |
| AC-02 | Default build is byte-comparable to the M0 baseline (no accidental regression) | `stat -c%s` within ±4 KiB of 6 407 848 B |
| AC-03 | `cargo build --release --no-default-features` **compiles** | exit 0 |
| AC-04 | The lean binary **runs**: `--version`, `help`, `doctor`, `harness -h` exit 0 | smoke |
| AC-05 | The lean binary rejects a gated subcommand with a clear message, not a panic or a silent no-op | `8sync harness web` output contains `--features web` |
| AC-06 | Per-gate byte deltas measured for all four combinations (`default`, `web` only, `toolstats` only, none), each into its own `--target-dir` with an explicit `--target` | table in `M1-VERIFICATION.md` |
| AC-07 | `scripts/size-report.sh` reproduces AC-06's table from a clean checkout | run it, output matches |
| AC-08 | No dependency is removed and no user-visible behaviour changes in a default build | `8sync harness web` still serves; `harness toolstats` still runs |
