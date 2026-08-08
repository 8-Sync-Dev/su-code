# M0-01 — Plan: Fedora-first package core

> Revision 2 — amended after plan review. Adds T0 (toolchain + test harness), the Stage-A paru gate
> that the first revision missed entirely, the two forgotten Arch-only verbs (`vpn.rs`,
> `harness/browser.rs`), the dry-run hoist, and a fixture-based Arch regression proof.

Four waves. T0 is a real task, not a footnote — it gates 10 of 15 ACs. Wave A is a single-writer
refactor of the package layer. Wave B fans out across disjoint files. Wave C verifies.

## Wave 0 — prerequisites

| # | Task | AC | UC | skill | verify |
|---|---|---|---|---|---|
| T0 | Install the Rust toolchain via `rustup` (`rust-toolchain.toml` pins stable/minimal + rustfmt + clippy — `cargo` is **absent** on this box). Create the first `#[cfg(test)]` module in the repo and add a `cargo test` step to `.github/workflows/release.yml` (today it runs only checkout → toolchain → `cargo build` → `size-gate.sh`) | AC-02, AC-04 | UC-1 | 8sync-cli | `cargo --version && cargo test && cargo build --release` |

## Wave A — the seam (serial, one writer: `pkg.rs` + `platform.rs` + `env_detect.rs`)

| # | Task | AC | UC | skill | verify |
|---|---|---|---|---|---|
| T1 | `env_detect.rs`: `Family::{Arch,Fedora,Other}` + pure `parse_family(&str) -> Family` (+ thin `distro_family()` reader) using `ID` **and** `ID_LIKE`; re-express `is_cachyos_or_arch()` on top. Unit tests for `ID=fedora` (no `ID_LIKE`), `ID=cachyos`+`ID_LIKE=arch`, `ID=debian` | AC-02 | UC-3 | 8sync-cli | `cargo test` |
| T2 | **Capture the Arch baseline first**: extract pure `plan_argv(pkgs, states, noconfirm) -> Vec<Vec<String>>` from the current pacman path and check in a fixture snapshot for install / rollback / AUR / `noconfirm=false`. Then introduce `trait PkgBackend` (D-M0-1) and move the pacman logic **verbatim** into `impl Pacman` | AC-01, AC-10 | UC-2 | ponytail | `cargo test` (fixture) |
| T3 | `impl Dnf`: `rpm -q` state; `dnf install -y` / `dnf upgrade -y`; **`dnf5-plugins`** as the `copr` prerequisite (✎ *not* `dnf-plugins-core`) with a `dnf copr --help` capability probe that degrades to a printed skip; `dnf copr enable -y` behind the D-M0-8 allowlist; `dnf swap --allowerasing` only via an explicit `swap` key printing what it would erase; `undo` via `dnf history undo <id>` with id captured from `dnf history list` and a defined no-id no-op | AC-01, AC-14 | UC-2 | senior-security | `cargo build --release` |
| T4 | `backend() -> Option<Box<dyn PkgBackend>>` selected by `distro_family()` (✎ `Option`, so `Family::Other` reproduces today's warn-and-continue at `platform.rs:103-110`); `platform.rs`: delete the positional `pacman:&str`, add `CorePkg { arch, fedora, brew, winget }`, route `pkg_manager()`/`install_core_pkg` through the trait | AC-01, AC-03, AC-12 | UC-2 | 8sync-cli | `cargo test && cargo build --release` |

## Wave B — callers, profiles, installer (parallel; disjoint files)

| # | Task | AC | UC | skill | verify |
|---|---|---|---|---|---|
| T5 | **✎ Gate the Stage-A AUR-helper step on `Family::Arch`** (`setup.rs:141-143` currently gates on `Os::Linux`, so `?` aborts the whole run on Fedora and `codegraph`/`configs`/`skills` never execute). Decide `install_core_pkg("gh", …)` behavior on Fedora (today silently warns via `platform.rs:104-107`; `gh` is available in Fedora repos, so give it a `CorePkg.fedora` name). Gate Stage B on family, not `Os::Linux` (`setup.rs:151`) | AC-06, AC-07 | UC-1 | 8sync-cli | `8sync setup --no-profile` |
| T6 | Migrate the **✎ grep-derived** callsite set to the trait — `platform.rs`, `setup.rs`, `profile.rs`, `doctor.rs`, **`vpn.rs:93-114`**, **`harness/browser.rs:51-57`** (23 sites total). The last two were missing from revision 1 and are Arch-only verbs; give each either Fedora package names or a printed skip reason | AC-01 | UC-2 | 8sync-cli | AC-01 grep + build |
| T7 | `assets/profiles/*.toml` (10 files): **add** `[packages.fedora]` (`dnf`/`copr`/`rpmfusion`/`swap`) as a sibling — ✎ **never rename `pacman`/`aur`/`aur_yay`**. Mark genuinely Arch-only profiles unavailable-on-Fedora rather than guessing rpm names | AC-08 | UC-4 | ponytail | `8sync setup --dry-run --profile <n>` |
| T8 | `profile.rs`: add `#[serde(deny_unknown_fields)]` to `Packages` (D-M0-5); resolve packages by family; skip-with-reason instead of the hard `?` at `profile.rs:210`; **✎ hoist the `dry_run` check above the AUR-helper lookup at `profile.rs:215-221` and above `pkg::ensure_yay()?` at `profile.rs:237`** so `--dry-run` is genuinely side-effect-free and never fails on a missing tool | AC-08 | UC-4 | 8sync-cli | dry-run per profile |
| T9 | `doctor.rs`: print `distro: <id> (<backend>)` and list skipped profiles with reasons. ✎ The "no native package manager on Linux" dead end lives at **`platform.rs:103-110`**, not in `doctor.rs` — retarget it there | AC-09 | UC-1 | 8sync-cli | `8sync doctor \| grep -E '^distro: fedora \(dnf\)$'` |
| T10 | Stage A dry-run (`setup.rs:128-136`): render the **resolved backend + argv** instead of seven hardcoded `ui::info` literals, so AC-06 cannot pass by editing a string | AC-06 | UC-1 | 8sync-cli | `8sync setup --dry-run --full` |
| T11 | `install.sh`: derive the temp path from `$BIN_DIR` (mirror `selfup.rs:109`) instead of `mktemp` in `/tmp`, fixing the EXDEV non-atomic replace on btrfs | AC-11 | UC-1 | 8sync-cli | install with `SUSYNC_BIN_DIR` on a different fs than `$TMPDIR` |
| T12 | Docs: `AGENTS.md` §2/§5b/§8 + `README.md` — Fedora support, the trait, per-distro profiles, dnf5. ✎ **Do not hand-edit the `8sync:skills:begin` sentinel block** in `AGENTS.md`/`CLAUDE.md`: it is regenerated from `inject.rs:174` and the fix belongs to M1; hand-editing it now is undone by the next `harness init` | — | UC-1 | — | doc read |

## Wave C — verification

| # | Task | AC | UC | skill | verify |
|---|---|---|---|---|---|
| T13 | Build + size gate; smoke `--version`, `help`, `doctor`, `setup --dry-run --full`, `setup --no-profile`; assert `~/.omp/skills` non-empty after AC-07 | AC-04,05,06,07,09,13 | UC-1 | 8sync-cli | `cargo build --release && bash scripts/size-gate.sh target/release/8sync` |
| T14 | Run the fixture tests: Arch argv regression (AC-10), `Family::Other` warn-and-continue (AC-12), COPR allowlist refusal (AC-14) | AC-10,12,14 | UC-2 | code-review-and-quality | `cargo test` |
| T15 | Write `M0-VERIFICATION.md` (AC matrix, every AC PASS or explicitly NEEDS-CONFIRM); update CHANGELOG + `su-code/KNOWLEDGE.md` + STATE; commit | AC-15 | — | — | `test -z "$(git status --porcelain)"` |

## Risk

- **Arch regression cannot be executed here** (no pacman; and `pacman_state` always returns
  `Missing` on Fedora, so a live argv dump is environment-dependent). Mitigated by T2 capturing the
  fixture **before** the refactor and by moving the logic verbatim. If the fixture cannot be
  captured faithfully, AC-10 lands as NEEDS-CONFIRM until run on an Arch box — stated up front.
- **`dnf` needs sudo.** AC-07 runs a real `setup --no-profile`; prime sudo interactively. Do **not**
  install a permanent NOPASSWD rule.
- **Profile package-name drift.** Several Arch names have no 1:1 rpm. T7 marks those unavailable
  rather than guessing — a wrong rpm name fails on a real user's machine, which is worse than an
  honest skip. With `--allowerasing` in play (D-M0-8) a wrong name could also erase system packages,
  which is why that flag is gated behind an explicit key.
- **COPR = third-party root code.** D-M0-8 exists because profiles are user-authored and shareable.
- **Size gate.** A trait object + second backend adds bytes against 383 184 B of headroom. AC-05 is
  a real gate.
