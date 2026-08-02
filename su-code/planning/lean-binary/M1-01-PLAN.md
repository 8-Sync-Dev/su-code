# M1-01 — Plan: feature gating + attribution

Single wave; every task touches the same crate manifest or its cfg surface, so
fan-out would only create index contention.

| # | Task | AC | UC | skill | verify |
|---|---|---|---|---|---|
| T1 | `crates/cli/Cargo.toml`: add `[features]` (`default = ["web","toolstats"]`, `web = [dep:axum, dep:tokio, dep:tower-http, dep:scraper]`, `toolstats = [dep:rusqlite]`); mark those five deps `optional = true` | AC-01 | UC-2 | 8sync-cli | `cargo build --release` |
| T2 | cfg the `web` surface: `assets.rs` (`WebAssets`, `web_asset`), `harness/mod.rs` (`mod web`, `mod marketplace`, dispatch arm, help line), `build.rs` (skip Vite + fallback unless `CARGO_FEATURE_WEB`) | AC-03, AC-05 | UC-2 | 8sync-cli | `cargo build --release` |
| T3 | cfg the `toolstats` surface: `harness/mod.rs` (`mod toolstats`, dispatch arm, help line) | AC-03, AC-05 | UC-2 | 8sync-cli | `cargo build --release` |
| T4 | `scripts/size-report.sh`: build the four feature combinations into separate `--target-dir`s with an explicit `--target`, print a byte/delta table | AC-06, AC-07 | UC-3 | deep-research | `bash -n scripts/size-report.sh` |
| T5 | Run it; smoke the lean binary (AC-04, AC-05) and the default binary (AC-02, AC-08); write `M1-VERIFICATION.md` | AC-02, AC-04, AC-05, AC-06, AC-08 | UC-2, UC-3 | deep-research | `cargo build --release --no-default-features` |
| T6 | CHANGELOG + KNOWLEDGE + STATE; commit | — | — | — | `test -z "$(git status --porcelain)"` |

## Risk

`rust-embed`'s `#[folder]` is resolved at compile time — if `WebAssets` is cfg'd
out, `web/dist/` need not exist, which is exactly what makes D-M1-5 safe. If the
derive still probes the path, T2 falls back to keeping a 1-file stub dist.
