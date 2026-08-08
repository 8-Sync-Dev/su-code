# M0 + M1 — Verification

Executed on this box: **Fedora 44, dnf5 5.4.1.0, x86_64**, Rust 1.97.1.
Execution mode: the approved option — pull the Stage-A paru gate forward, then run M0 and M1
concurrently (6 parallel agents, contract-first).

## Headline

`8sync` now installs and configures Fedora. Before this phase `8sync setup` **aborted at step 3 of
8** on any non-Arch Linux, which is why `~/.omp/skills` was empty on this machine.

## Build + gates

| Metric | Value |
|---|---|
| `cargo build --release` | clean, **0 dead-code warnings** |
| `cargo test --release` | **48 passed, 0 failed** |
| `bash scripts/size-gate.sh` | **OK** |
| Binary | 4 992 776 B (baseline 4 923 384 B, **+69 392 B**) |
| Headroom to ceiling | 250 104 B |
| New clippy diagnostics | **0** — the 33 reported are pre-existing lines (verified by `git blame`: e862d2c, 4a2bc33, a06b8ac, 52e0b25) that only shifted position |

Note: the AGENTS.md baseline of 4 859 696 B is stale; the real pre-change build here is 4 923 384 B,
so actual headroom was 319 KB, not the 383 KB the plan assumed.

## M0 acceptance matrix

| AC | Criterion | Result | Evidence |
|---|---|---|---|
| AC-01 | No package manager invoked outside `pkg.rs` (incl. `sudo`-prefixed / `sh -c`) | **PASS** | grep for `Command::new`/`"sudo"` + `pacman\|paru\|yay\|makepkg\|paccache\|dnf` outside `src/pkg.rs` → empty |
| AC-02 | `parse_family` correct for fedora / cachyos+ID_LIKE / debian | **PASS** | `cargo test`; note Fedora 44 has **no `ID_LIKE`**, so `ID=fedora` alone must work — it does |
| AC-03 | No positional `pacman` param; `install_core_pkg` takes `CorePkg` | **PASS** | `platform.rs` — old signature deleted |
| AC-04 | Build clean, zero warnings from touched files | **PASS** | 0 dead-code warnings after wiring `undo` + deleting dead `pkg_manager()` |
| AC-05 | Under the size gate | **PASS** | size gate OK, 250 104 B headroom |
| AC-06 | `setup --dry-run --full` shows resolved dnf argv, zero pacman | **PASS** | prints `would install gh via dnf`, `would dnf install: coolercontrol … kitty`; **pacman mentions = 0** |
| AC-07 | `setup --no-profile` reaches ALL of Stage A, exit 0 | **PASS** | exit 0, "all steps succeeded"; `~/.omp/skills` = `00-force-load.md 8sync-cli codegraph image-routing karpathy-guidelines`; codegraph 1.5.0 installed |
| AC-08 | Every profile declares Fedora packages or is skipped with a reason; dry-run never errors on a missing AUR helper | **PASS** | all 10 profiles `--dry-run` **exit 0** (previously errored); `warp` prints "has no Fedora packages — it contributed nothing" |
| AC-09 | `doctor` prints `^distro: fedora \(dnf\)$` and exits 0 | **PASS** | exact anchor match = 1 |
| AC-10 | Arch unregressed vs a pre-refactor argv fixture | **PASS (fixture)** / **NEEDS-CONFIRM (hardware)** | pacman logic moved verbatim and `Pacman::install` now builds argv *from* `plan_argv`, so the fixture guards the real path. Cannot execute pacman on Fedora — confirm on an Arch box before release |
| AC-11 | `install.sh` temp path derived from `$BIN_DIR`, no cross-device copy | **PASS** | `install.sh:141` `tmp="$BIN_DIR/.8sync.new.$$"`; proven real: old temp `/tmp` dev=51 tmpfs vs dest dev=52 btrfs. Live cross-fs install → exit 0, `checksum: ok (sha256:93e5d4e8…)` |
| AC-12 | `Family::Other` still warns-and-continues (Debian/openSUSE unregressed) | **PASS** | `backend_for(Other)` → `None`; unit test on the notice path |
| AC-13 | `8sync --version` from a clean install path | **PASS** | `8sync 0.53.0` |
| AC-14 | A user profile cannot enable an unlisted COPR non-interactively | **PASS** | live probe: `copr_enable("attacker/backdoor", false)` → `Err("refusing COPR … not in the allowlist")` |
| AC-15 | This document exists with every AC resolved | **PASS** | — |

**Bonus beyond AC:** the digest check was implementable at zero cost — the Releases API already
returns a per-asset `digest: "sha256:…"` (verified live on v0.53.0), so `install.sh` now verifies
downloads with `sha256sum`/`shasum`, fatal on mismatch, skip-with-notice when unavailable.

## M1 (enforcement) — verified

| Item | Result | Evidence |
|---|---|---|
| Dead machine-absolute paths removed | **PASS** | `grep -rn '/home/alex' AGENTS.md CLAUDE.md` → empty. Root + 3 sub-folder `AGENTS.md` regenerated to `su-code/skills/<dir>/SKILL.md` |
| Root cause fixed | **PASS** | `inject.rs` `p.join(entry)` replaced by `skill_ref()`: local → repo-relative, global → `~/`-anchored (global skills genuinely live in `$HOME` and cannot be relativised) |
| `harness audit` can now see it | **PASS** | skip narrowed to generic absolutes only; `/home/`,`/Users/`,`/root/` are flagged; `/etc/os-release` still ignored |
| TTSR rule deployed | **PASS** | `~/.omp/agent/rules/8sync-code-intel-first.md` (2 768 B) with LEVERS.md row-7 keys verbatim: `condition:`, `scope: "tool:grep(*), tool:glob(*)"`, `interruptMode: tool-only` |
| bashInterceptor deployed | **PASS** | `~/.omp/agent/config.yml` — 3 patterns (rg/ag/fd/git-grep, recursive grep, `find -name`), each matching `sudo`/`env`/`xargs`/`VAR=x` prefixes |
| Prompt weight reduced | **PASS** | `APPEND_SYSTEM.md` 9 198 → **3 260 B (−64.6 %)**; the TTSR rule body costs **zero** prompt tokens (rules with a `condition` never enter the rulebook) |
| UC-7 safe degradation | **PASS** | three layers: deploy-time capability gate (rule + interceptor are removed when no code-intel tool is present); TTSR `repeatMode: once` means worst case is one restarted turn, not a hard veto; interceptor keeps `tool: lsp` because naming an MCP tool would make omp **silently skip** the rule |

## Deliberate behaviour changes (not defects)

1. **On Fedora, a profile with no `[packages.fedora]` contributes nothing** — not its `services`,
   not its `post_install` — because those configure software that will not be installed. Arch
   behaviour is byte-identical to before.
2. **`platform::pkg_manager()` deleted.** `pkg::backend().name()` answers the same question, and
   250 KB of headroom does not fund a dead public API.
3. **`Dnf::install` now rolls back a partial plan.** A real bug found while eliminating the "unused
   `undo`" warning: the plan can hold two transactions (install, then upgrade). If the upgrade
   failed after the install committed, the code returned `Err` while leaving the install applied.
   It now undoes the committed transaction via `dnf history undo`.

## Known gaps / follow-ups

- **AC-10 hardware confirmation** on a real Arch box (cannot run pacman here).
- `su-code/skills/` is gitignored and absent, so `harness audit` lists ~50 `su-code/skills/...`
  refs as stale until `8sync skill sync` mirrors them. Pre-existing (the on-demand tier always
  emitted repo-relative), not a regression.
- Fedora has **no** SoftEther and no evdi/DisplayLink in Fedora+RPMFusion; `vpn` offers Cloudflare
  WARP instead and `displaylink` relies on COPR `crashdummy/Displaylink`.
- Not ported to Fedora (no rpm exists, documented in-file rather than guessed): AFFiNE,
  larksuite-bin, bitwarden (Flatpak-only), bun, typescript-language-server.
- `.gitignore:29`'s bare `reference/` silently swallowed the distilled omp docs during this session
  (relocated to `su-code/omp-reference/`). Recorded as UC-12b for M2.
