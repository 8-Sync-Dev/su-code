# Roadmap — Fedora-First Core + Enforced omp Harness

Cut by dependency, not by wishlist. You cannot verify a harness with a binary that will not install
(**M0**). You cannot enforce tool routing while the skill paths it points at are dead (**M1**).
You cannot prune assets before there is one registry that says what an asset *is* (**M2**), and the
authoring commands are exactly that registry with a writer on top (**M3**). Engine dedup (**M4**)
needs the enforcement layer proven first, because it deletes the prose that currently substitutes
for it. Distribution polish (**M5**) is last because it ships whatever the other five produced.

| Phase | Name | Serves | Demo after this phase |
|---|---|---|---|
| **M0** | Fedora-first package core | UC-1,2,3,4 | Fresh Fedora → one-liner install → `8sync setup` completes with 0 pkg failures → `8sync doctor` green. Arch path byte-identical in behavior. |
| **M1** | Enforced tool routing + relative paths | UC-5,6,7,8,9,10 | In a live omp session, a `grep` for code structure is **vetoed** and redirected to codegraph; a repo cloned to a second machine still resolves every skill; `harness audit` fails on a planted absolute path. |
| **M2** | One registry + minimize | UC-11,12 | One dynamic asset registry; the four contradictory skill counts collapse to one number; binary shrinks measurably against `size-gate.sh`. |
| **M3** | Authoring commands | UC-13,14,15 | `/create-skill`, `/create-command`, `/auto-package` produce working artifacts with **no** Rust edit per artifact. |
| **M4** | Engine dedup on omp primitives | UC-16,17 | `8sync-workflow.ts` deleted, `8sync-engine.ts` reduced to the verify gate + gitleaks commit gate; a run-to-done loop still completes. |
| **M5** | Distribution + CI gates | UC-18,19,20,21 | Short install URL works; bare `8sync` never blocks on the network; `8sync up` no-ops when current; downloads are sha256-verified; CI typechecks the shipped `.ts`. |

## Integration contracts

- **M0 → M1:** M0 introduces `trait PkgBackend` — **defined once in `M0-CONTEXT.md` D-M0-1; this roadmap does not redeclare it** (an earlier revision carried a second, broken signature whose `install` returned `Result<()>`, so nothing could produce the `txn` that rollback consumes). Selection is `backend() -> Option<Box<dyn PkgBackend>>` keyed on `env_detect::distro_family()`; `None` on `Family::Other` preserves today's warn-and-continue. `platform.rs:93`'s positional `pacman: &str` dies here. M1 consumes only `Family`, never a package manager.
- **M1 → M2:** M1 defines the *deployment contract* for a managed asset — `{ asset_prefix, dest, scope: Global|Project, mode: Managed|SeedIfMissing }` plus the byte-compare write. M2 turns that struct into the single registry, replacing the hardcoded tables at `deploy.rs:17-38`, `deploy.rs:1160-1199` ✎, `setup.rs:715-720`. **Template = `profile::load_all()` (`profile.rs:82-115`)**, which already enumerates embedded `profiles/*.toml` *and* merges a user-override dir with last-wins dedup — a strictly better precedent than `harness/eval.rs:37` (an earlier revision wrongly called that the only dynamic loop; there are five).
- **M2 → M3:** the registry is the *read* side; `/create-skill` and `/create-command` are the *write* side, emitting into `assets/` (dev) or `~/.omp/` + `<repo>/.omp/` (user) in the same shape the registry already deploys. `/auto-package` composes both. Because the registry is dynamic, a new artifact needs zero Rust changes — that is the acceptance test.
- **M1 → M4:** M4 may delete `8sync-engine.ts`'s doom-loop guard **only because** M1 landed the TTSR `repeatMode`/`repeatGap` equivalent, and may delete the prose loop control in `commands/auto.md` **only because** M1 landed enforced routing. Deleting either earlier removes the guard with nothing behind it.
- **M0/M2 → M5:** M5's size ratchet lowers `size-gate.sh`'s ceiling to whatever M2 actually achieved; its installer changes assume M0's atomic-replace fix (same-filesystem temp) is already in.

## Dependency reasoning

**M0 first** — ✎ *corrected*: not because M0 technically blocks M1, but because `8sync setup`
currently **aborts at step 3 of 8 on Fedora**. `setup.rs:141-143` gates the AUR-helper step on
`Os::Linux` rather than Arch, and `try_step`'s error propagates for a non-`yolo` run, so
`codegraph`, `path-bootstrap`, `configs`, `skills` and `codegraph-skill` never execute — which is
exactly why `~/.omp/skills` is empty on this box. Until that one gate is fixed, no harness work can
be exercised end-to-end here. Scope is bounded: **23 package-manager call sites** outside `pkg.rs`
(platform ×1, setup ×8, profile ×5, vpn ×5, browser ×3, doctor ×1), 10 profile TOMLs, 1 script.

**M1 before M2** — M2 deletes skills. Deleting a skill whose path is already dead hides whether the
removal or the path broke it. Fix the path, prove skills load, *then* prune.

**M1 before M4** — `8sync-engine.ts`'s failStreak guard and `auto.md`'s prose loop control are the
current (weak) substitutes for TTSR `repeatMode` and enforced routing. M4 removes them; M1 must
have installed the real thing first.

**M2 before M3** — ✎ *confirmed real, not artificial*: `/create-command` cannot be "drop a file and
it works" while `deploy.rs:1160-1199` requires a hardcoded ~8-line Rust block per command. M2
removes that requirement; M3 then only writes files. (Caveat: a *user-scope* command could be
written straight to `~/.omp/commands/` with no registry — so M3's registry dependency is real only
for the bundled/global path.)

**M5 last** — the size ratchet needs M2's real number, and the installer ships M0's fix.

## Sequencing note

✎ *Corrected.* M0 and M1 touch disjoint files (`pkg.rs`/`platform.rs`/`env_detect.rs`/profiles vs
`skill/inject.rs`/`index.rs`/`audit.rs`/`assets/configs`). The earlier claim that M1 is blocked
because "`8sync setup` must be able to install codegraph on Fedora" was **wrong**:
`install_codegraph` (`setup.rs:377-392`) is a distro-neutral `curl … | sh` into `~/.local/bin` and
involves no package manager.

The single real coupling is M0-T5's Stage-A gate (`setup.rs:141-143`), which today aborts the run
before `install_codegraph` is reached. That is **one task**, not a phase. Therefore: M0 and M1 are
substantially independent and are sequenced by preference; if schedule pressure appears, pull T5
forward and run M1 concurrently with the rest of M0.
