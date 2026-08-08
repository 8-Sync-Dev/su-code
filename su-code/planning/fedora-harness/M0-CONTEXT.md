# M0 — Fedora-first package core

> Revision 2 — amended after plan review (3 BLOCKER + 11 MAJOR findings). Every claim below was
> re-verified against the code on this box; corrected line numbers are marked ✎.

## 📌 Requirement scope

UC-1 (install + setup on Fedora) · UC-2 (distro-dispatched package layer) ·
UC-3 (correct distro detection) · UC-4 (per-distro profile package names).

## 🎯 Goal

`8sync` installs and fully configures a **Fedora** machine: one-liner install, `8sync setup`
completes with zero package-manager failures, `8sync doctor` reports the distro. Arch/CachyOS
behavior is unchanged, and Debian/openSUSE keep today's clean warn-and-continue.

## Structural findings that reshape the plan

1. **There is no abstraction to extend — only a seam to fix.** `pkg.rs` (✎ 209 lines) is literal
   `Command::new("pacman")` / `Command::new(helper)`. `platform.rs:81-87`'s Linux arm is
   `which::which("pacman")` and `install_core_pkg` takes a **positional parameter literally named
   `pacman`** (`platform.rs:93`). M0 introduces the trait rather than porting pacman logic.

2. **✎ CORRECTED — Stage A does *not* work on Fedora; `8sync setup` dies at step 3 of 8.**
   `setup.rs:141-143` gates the AUR-helper step on `platform::Os::Linux`, **not on Arch**:
   ```rust
   if platform::os() == platform::Os::Linux {
       try_step("paru", yolo, &mut failures, install_aur_helper)?;
   }
   ```
   `install_aur_helper` → `pacman_install_safe(["git","base-devel"])` → `sudo pacman` → fails.
   `yolo` is false for a plain `8sync setup` (`setup.rs:115`), so `try_step` **propagates** and the
   `?` aborts the run. **`codegraph`, `path-bootstrap`, `configs`, `skills`, `codegraph-skill`
   (`setup.rs:144-148`) never execute.** This is the direct cause of the empty `~/.omp/skills` on
   this box, and it means M0 must touch Stage A, not only Stage B. The earlier revision's claim
   that "Stage A already works" was false and is withdrawn.

3. **Fedora rollback is native but narrower than assumed.** `pkg.rs:79-91` / `:154-166` hand-roll
   snapshot + `sudo pacman -Rns` because pacman can partially apply a batch. RPM transactions are
   all-or-nothing, so that state does not arise — but `dnf install` also records **no transaction
   id when it fails during resolve/download**. The `Dnf` impl must define id capture (parse
   `dnf history list` after success; there is no `--print-id`) and the **no-id case**, and `undo`
   itself needs `sudo -y` and can fail if later transactions depend on the packages.

4. **The failure mode is a silent password prompt.** `setup.rs:151` gates Stage B on *Linux*, not
   *Arch*. Combined with (2), every profile path ends in a `sudo pacman` prompt. Detection must gate
   by **distro family**, and an unavailable profile must be *skipped with a reason*.

5. **`--dry-run` is already broken on any box without an AUR helper.** In `profile::apply` the
   lookup `env_detect::aur_helper().ok_or_else(…)?` (`profile.rs:215-221`) runs **before** the
   `if dry_run` branch (`profile.rs:222`), as does `pkg::ensure_yay()?` (`profile.rs:237`). So a dry
   run of any AUR-declaring profile errors instead of printing a plan. Verifying M0 "by dry-run"
   requires fixing this first.

6. **Stage A's dry-run output is seven hardcoded literals** (`setup.rs:128-136`) that never consult
   a backend — so any AC phrased as "dry-run mentions dnf" is satisfied by editing a string. Also, a
   bare `--dry-run` with no TTY exits at `setup.rs:244-248` before `profile::apply` is ever reached.

7. **✎ CORRECTED — this box runs dnf5.** `dnf5 5.4.1.0`; the `copr` subcommand comes from
   **`dnf5-plugins`**, not `dnf-plugins-core` (the dnf4-era package, present here only incidentally).
   Verified: `rpm -q dnf5-plugins` → `dnf5-plugins-5.4.1.0-1.fc44`; `dnf copr --help` → OK.
   **Also: Fedora 44 has no `ID_LIKE` line at all** — only `ID=fedora`. `ID_LIKE` still matters for
   RHEL/Alma/Rocky, but Fedora detection rests on `ID`.

8. **`install.sh`'s "atomic" replace is not atomic on Fedora.** `install.sh:71` `tmp=$(mktemp)`
   lands in `/tmp` (tmpfs) while `~/.local/bin` is btrfs → `install.sh:78`'s `mv -f` crosses
   filesystems (EXDEV) and degrades to copy. `selfup.rs:109` already creates the temp file as a
   **sibling of the destination**.

9. **✎ CORRECTED — port surface is 23 call sites outside `pkg.rs`, not ~74.** platform.rs ×1,
   setup.rs ×8, profile.rs ×5, **vpn.rs ×5**, **harness/browser.rs ×3**, doctor.rs ×1. The earlier
   ~74/~117 figures counted every textual occurrence of `pacman|paru|yay|makepkg` (169 in
   `crates/`). `vpn.rs:93-114` and `harness/browser.rs:51-57` were missing from the plan entirely
   and are Arch-only verbs that would still hard-fail on Fedora after M0 "completed".

10. **Real Fedora facts to encode** (observed on this machine): rpmfusion `ffmpeg` conflicts with
    Fedora's `ffmpeg-free` and needs `dnf swap … --allowerasing`; `dnf copr enable -y` is the
    idempotent form.

## Decisions

- **D-M0-1 — `trait PkgBackend`** in `pkg.rs` (**single source of truth**; ROADMAP references this,
  it does not redeclare it):
  `fn name(&self) -> &'static str` · `fn state(&self, pkg: &str) -> InstallState` ·
  `fn install(&self, pkgs: &[&str], noconfirm: bool) -> Result<Txn>` · `fn undo(&self, txn: &Txn) -> Result<()>`.
  Impls: `Pacman` (existing logic moved **verbatim**) and `Dnf`. `aur_install_safe` stays a
  `Pacman`-only capability; the `Dnf` analogue is `copr_enable()`.
- **D-M0-2 — ✎ `backend() -> Option<Box<dyn PkgBackend>>`.** `None` on `Family::Other` reproduces
  today's deliberate behavior (`platform.rs:103-110`: print "install `<label>` manually", return
  `Ok(())`). A total function would panic or silently pick a wrong backend, regressing
  Debian/Ubuntu/openSUSE users who work today.
- **D-M0-3 — `env_detect::distro_family()`** reads `ID` **and** `ID_LIKE` → `Family::{Arch, Fedora,
  Other}`. Exposed as a pure `fn parse_family(os_release: &str) -> Family` so it is testable;
  `distro_family()` is a thin reader. `is_cachyos_or_arch()` is re-expressed on top — no second
  detection path.
- **D-M0-4 — `platform.rs::install_core_pkg` loses its positional parameters** in favour of
  `CorePkg { arch, fedora, brew, winget }`.
- **D-M0-5 — ✎ Profile schema is ADDITIVE, never renamed.** `pacman` / `aur` / `aur_yay`
  (`profile.rs:52-63`) remain the canonical Arch keys; a new sibling `[packages.fedora]`
  (`dnf`, `copr`, `rpmfusion`, `swap`) is added. **`#[serde(deny_unknown_fields)]` on `Packages`.**
  Rationale: profiles are also loaded from the user-owned `~/.config/8sync/profiles/*.toml`
  (`profile.rs:99-112`) with `#[serde(default)]` everywhere — a rename would parse fine, yield an
  empty vector, install **nothing**, and exit 0. Silent no-op for existing Arch users.
- **D-M0-6 — A profile with no table for the detected family is skipped with a printed reason** and
  reported by `doctor` — never attempted.
- **D-M0-7 — Fedora rollback = `dnf history undo`,** with id captured from `dnf history list` after
  a successful transaction; if no id exists, `undo` is a no-op that says so. Do not port the pacman
  snapshot loop.
- **D-M0-8 — ✎ NEW (security). COPR is an unvetted third-party root-code source.** `dnf copr enable`
  is permitted only for `owner/project` values on a checked-in allowlist in `assets/`; anything else
  requires an interactive confirmation showing the full `owner/project`. `--allowerasing` is
  permitted **only** via an explicit per-profile `swap` key, and must print the packages it would
  erase. Without this, any user- or community-authored profile TOML gains arbitrary root execution.
- **D-M0-9 — Arch is the regression baseline,** proven by a checked-in argv fixture (see AC-10),
  not by prose. Pacman logic moves verbatim in T2.

## ✅ Acceptance Criteria

Note: AC-04 and AC-05 trace to PROJECT hard constraints, not to a UC — recorded deliberately.

| AC | Criterion | How verified |
|---|---|---|
| AC-01 | No package manager is invoked outside `pkg.rs` — including `sudo`-prefixed and `sh -c` forms | `grep -rnE '\b(pacman\|paru\|yay\|makepkg\|dnf\|rpm -q)\b' crates/ --include=*.rs \| grep -v 'src/pkg.rs'` → only the permitted exceptions enumerated in `M0-VERIFICATION.md` |
| AC-02 | `parse_family()` returns `Fedora` for `ID=fedora` (no `ID_LIKE`), `Arch` for `ID=cachyos`+`ID_LIKE=arch`, `Other` for `ID=debian` | `cargo test` — new `#[cfg(test)]` module (repo has **zero** tests today; T0 adds the harness + CI step) |
| AC-03 | `platform.rs` has no positional parameter named `pacman`; `install_core_pkg` takes `CorePkg` | read + `cargo build --release` |
| AC-04 | `cargo build --release` clean, zero warnings from touched files *(PROJECT constraint, no UC)* | build output |
| AC-05 | Binary under the gate *(PROJECT constraint, no UC)* | `bash scripts/size-gate.sh target/release/8sync` exit 0 |
| AC-06 | `8sync setup --dry-run --full` on Fedora prints the **resolved backend name and argv** per step, showing `dnf` and zero pacman argv | run it; Stage A dry-run must render resolved argv, not the literals at `setup.rs:128-136` |
| AC-07 | `8sync setup --no-profile` on this Fedora box reaches **all** of Stage A — `codegraph`, `path-bootstrap`, `configs`, `skills`, `codegraph-skill` — and exits 0 | run it; assert `~/.omp/skills` is non-empty afterwards |
| AC-08 | Every bundled profile either declares Fedora packages or is skipped with a printed reason; `--dry-run` never errors on a missing AUR helper | `8sync setup --dry-run --profile <name>` for all 10 |
| AC-09 | `8sync doctor` prints a line matching `^distro: fedora \(dnf\)$` and exits 0 | `8sync doctor \| grep -E '^distro: fedora \(dnf\)$'` |
| AC-10 | Arch unregressed: `plan_argv()` output for install / rollback / AUR / `noconfirm=false` matches a fixture captured **before** the refactor | `cargo test` against the checked-in fixture |
| AC-11 | `install.sh` derives its temp path from `$BIN_DIR`, so no cross-device copy occurs | `grep` that the temp path derives from `$BIN_DIR`, **plus** an install with `SUSYNC_BIN_DIR` on a *different* filesystem than `$TMPDIR` (e.g. `$HOME/.cache/…` on btrfs) |
| AC-12 | On `Family::Other` (mocked), `8sync setup` still completes with the manual-install notice rather than an error | `cargo test` on the `None`-backend path |
| AC-13 | `8sync --version` runs from a clean install path on this box | end-to-end smoke |
| AC-14 | A user-supplied profile cannot enable a COPR non-interactively unless allowlisted | `cargo test` / dry-run with a synthetic profile |
| AC-15 | `M0-VERIFICATION.md` records every AC as PASS or explicitly NEEDS-CONFIRM | file exists, matrix complete |

## Out of scope for M0

Enforcement/TTSR (M1), asset registry + pruning (M2), authoring commands (M3), engine dedup (M4),
short install URL + update-notify + digest verification (M5).
