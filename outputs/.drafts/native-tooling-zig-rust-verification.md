# Verification pass — `native-tooling-zig-rust`

Reviewer: self-review against the cited draft. Categories: FATAL / MAJOR / MINOR.

## FATAL
1. **Arithmetic error on the budget overshoot.** Draft says "57 % over the 4 MB budget".
   - 6 406 696 B ÷ 4 194 304 B (4 MiB) = **1.527×** → 52.7 % over
   - 6 406 696 B ÷ 4 000 000 B (4 MB decimal) = **1.602×** → 60.2 % over
   Neither is 57 %. **Fix:** state absolute bytes and the multiplier, avoid the invented percentage.

## MAJOR
2. **Unit mixing.** Draft mixes MiB and MB (`2.19 MB .rodata` is 2 188 928 B = 2.09 MiB). **Fix:** quote raw bytes for every measured section, add the unit conversion once.
3. **Subsystem savings presented next to measured sections.** The ≈354 KiB / ≈124 KiB figures are *sums of cargo-bloat rows*, and cargo-bloat prints "numbers above are a result of guesswork". They also omit `.rodata`. **Fix:** label them explicitly as estimates and mark the projected total as `[INFERENCE]` until a gated build is measured.
4. **redb recommendation rests on unverified sizing.** **Fix:** already demoted in the cited draft — keep it out of the primary recommendation; feature-gating is the measured-cost-free move.

## MINOR
5. `[Unknown]` 780 KiB is attributed "overwhelmingly" to SQLite — zstd C objects also land there. Soften to "dominated by the bundled SQLite amalgamation plus the zstd C objects".
6. The draft doesn't give the concrete `[features]` stanza. Adding it makes the brief actionable rather than advisory.
7. Windows leg of CI: state plainly that `cargo-zigbuild` cannot cover it, so the matrix shrinks 5 → 4 legs at best, not to 1.

## Resolution
FATAL #1 and MAJOR #2–#4 fixed in `outputs/native-tooling-zig-rust.md`. MINOR #5–#7 also applied. Re-review: clean.
