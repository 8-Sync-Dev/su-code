# Roadmap — Lean Binary

Cut by dependency: you cannot measure a gate that does not exist, and you cannot
decide what to delete before you have measured. Foundation → measurement →
elimination → integration.

| Phase | Name | Serves | Demo after this phase |
|---|---|---|---|
| **M0** | Land pending WIP | UC-1 | `git status` clean; every finished deliverable is a verified commit on `main`. |
| **M1** | Feature gating + attribution | UC-2, UC-3 | `cargo build --no-default-features` produces a working lean `8sync`; `8sync harness bloat` prints the measured cost of each gate. |
| **M2** | Eliminate, don't just gate | UC-4 | The **default/full** binary shrinks — heavy deps replaced by in-repo equivalents where the data says it pays. |
| **M3** | CI + budget truth | UC-5, UC-6 | Release workflow builds `aarch64-musl` via `cargo-zigbuild` (no Docker) + a macOS `universal2` asset; `AGENTS.md` §8 states the real, enforced number. |

## Integration contracts

- **M0 → M1**: clean tree at a known commit; `outputs/native-tooling-zig-rust.md`
  is the measurement baseline (6 406 696 B).
- **M1 → M2**: `[features]` `web` / `marketplace` / `toolstats` exist and are
  cfg-honest (lean build compiles *and runs*); per-gate byte deltas recorded in
  `M1-VERIFICATION.md`. M2 only deletes a dep whose measured cost justifies it.
- **M2 → M3**: final default-build size is known, so §8's budget line and the CI
  size-gate threshold can be written as facts.

## Dependency reasoning

M1 before M2 because eliminating `rusqlite`/`scraper` is only worth the risk if
the gate measurement shows they actually cost what `cargo bloat`'s guesswork
suggests. M3 last because the CI size gate needs M2's final number.
