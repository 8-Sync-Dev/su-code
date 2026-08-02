# M3 — CI + budget truth

## 📌 Requirement scope

UC-5 (documented budget matches reality and is enforced) · UC-6 (aarch64 without
Docker; sane macOS assets).

## 🎯 Goal

Make the size budget a **gate** rather than a comment, and remove the one CI leg
that cannot build the product correctly.

## Finding that changes the CI plan

The `aarch64-unknown-linux-musl` leg builds with `cross`, i.e. inside a Docker
image. `build.rs` builds the Vite dashboard by shelling out to bun/pnpm/npm and
**silently embeds a stub page when none is found** — the container has no JS
toolchain, so that asset most likely ships a placeholder dashboard. This is a
correctness bug, not just a speed problem.

`cargo-zigbuild` runs on the runner itself, where `npm` exists, so replacing
`cross` fixes the stub *and* deletes the Docker leg.

**Verified locally before planning** (`cargo-zigbuild` 0.23.0, zig 0.16.0):

```
cargo zigbuild --release --locked --target aarch64-unknown-linux-musl
→ Finished in 31.91s
→ ELF 64-bit LSB executable, ARM aarch64, statically linked, stripped
→ 4 151 328 B  (under the 4 194 304 B budget)
```

## Decision reversal — `universal2` is REJECTED

M0's brief proposed collapsing `darwin-x86_64` + `darwin-arm64` into one
`universal2` asset. **In a feature whose entire purpose is a smaller download,
that is backwards**: a universal binary contains both slices, so every Mac user
downloads ~2× the bytes to save the project one CI leg. It would also rename
assets that `install.sh` resolves by `${os}-${arch}`.

Two separate assets stay. Recorded here because the brief says otherwise, and a
plan that is wrong should be corrected in the open rather than quietly dropped.

## Decisions

- **D-M3-1** — `cross` → `cargo-zigbuild` for `aarch64-unknown-linux-musl`; no
  other leg changes. Upstream supports Linux and macOS targets only, so the
  Windows MSVC leg is untouched.
- **D-M3-2** — reject `universal2` (above).
- **D-M3-3** — add a **size gate** to CI: every built asset is measured against
  a ceiling declared in one place, and the job fails above it. A budget nobody
  enforces is how this repo drifted 52 % over in the first place.
- **D-M3-4** — set the ceiling at **5 MiB (5 242 880 B)**, above today's
  4 859 696 B x86_64 build. Not the aspirational 4 MiB: a gate that is already
  red teaches people to ignore it. `AGENTS.md` §8 states both the ceiling and the
  4 MiB goal, with the measured numbers.
- **D-M3-5** — clear the doc-hygiene warning carried from M0
  (`8sync doctor`: 17 stale paths / 2 oversized).

## ✅ Acceptance Criteria

| AC | Criterion | How verified |
|---|---|---|
| AC-01 | `release.yml` builds `aarch64-unknown-linux-musl` with `cargo-zigbuild`; no `cross`/Docker remains | file inspection; `grep -c cross` → 0 |
| AC-02 | The zig cross-build is proven, not assumed | done above: 4 151 328 B static aarch64 ELF |
| AC-03 | Workflow YAML parses and the matrix still yields the same 5 asset names | YAML parse + asset-name derivation check |
| AC-04 | CI fails a build whose asset exceeds the ceiling | gate script tested locally both ways (pass + fail) |
| AC-05 | `AGENTS.md` §8 states ceiling, goal and measured sizes | inspection |
| AC-06 | `8sync doctor` reports no stale/oversized docs | run it |
| AC-07 | Local smoke still green after all edits | `8sync --version/help/doctor` |
