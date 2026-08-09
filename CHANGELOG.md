# Changelog

Mọi thay đổi đáng kể của `8sync` ghi vào đây. Format theo [Keep a Changelog](https://keepachangelog.com),
versioning theo [SemVer](https://semver.org). **8sync rule:** mỗi PR cập nhật mục `Unreleased`.

## [Unreleased]

## [0.54.0] - 2026-08-09

### Security
- **Command injection in the shipped `8sync-engine.ts`.** `engine_advance` built
  `git commit -m <msg>` as a string for `bash -lc`; `JSON.stringify` escapes `"` and `\` but a
  double-quoted bash word still performs command substitution, and the message is a model-supplied
  parameter routinely paraphrased from files the agent just read. Git and gitleaks now run through
  an argv-based `runArgv` with no shell. Shell evaluation is retained only for a task's `verify`
  commands, where it is the documented contract.
- **Argument injection into `sudo pacman` / `sudo dnf`.** Profile package lists were spliced into a
  privileged argv with no end-of-options separator, so an entry beginning with `-` became a flag —
  `--hookdir=` hands alpm attacker-written root hooks, `--setopt=reposdir=` repoints dnf. Package
  names starting with `-` are now refused outright at all three privileged entry points, and the
  planners emit `--` before the package list.
- The autonomous-commit gitleaks gate no longer fails open silently: `if command -v gitleaks` exits
  0 when gitleaks is absent, so fresh machines got unscanned commits from a gate that claimed to
  scan. Presence is probed separately and an unscanned commit says so.

### Fixed — profiles a teammate never asked for
- **`--full`/`--yall`/`-y` applied the maintainer's personal `alexdev` bundle**, so a teammate
  running "install everything" got Lian Li chassis drivers, a Vietnamese IME, DisplayLink DKMS and
  Bitwarden. `--full` now means every **community** profile, read from the same
  `offered_profiles()` the y/N prompt uses. Personal profiles are reachable only via
  `--profile <name>`; `--profile alexdev` restores the old behaviour in one flag.
- `--full` also skips the `warp` VPN: a flag meaning "don't ask me" is the wrong way to acquire
  something that rewrites DNS and routing, and `--community` already documented that opt-out.
- **Profiles installed packages for hardware the machine does not have.** `apply` installs packages
  before `post_install`, so the nvidia profile's own "no NVIDIA GPU detected" guard ran far too
  late. New `requires.detect` probe is evaluated in `resolve`, per bundle member, before any
  package work; `nvidia` probes sysfs PCI vendor `0x10de` (no `lspci` dependency) and fails closed.
- A profile skipped for missing hardware is no longer recorded as applied.

### Fixed
- `Cargo.toml` version now matches the release tag, enforced by a new CI gate. Shipping 0.53.0
  under a v0.54.0 tag would have told every up-to-date user "update available" forever and made
  `8sync up` re-download the binary on every run.
- **A stale `8sync-workflow.ts` kept loading after upgrade.** The extension was retired this
  release, but nothing deleted the copy an earlier 8sync had already written to
  `~/.omp/agent/extensions/` and `<root>/.omp/extensions/`, so omp went on registering its tools
  next to the engine's. `8sync harness` (any subcommand) now removes both copies; the retired
  deploy path and its six call sites are gone.
- **Two shipped skills never deployed.** `research-paper` and `remote-compute` were added to
  `assets/skills/` but never registered in the deploy list, so `~/.omp/skills/<name>/` was never
  created on any machine — while `00-force-load.md` told the agent to open those SKILL.md files.
  Both are registered now, and a new guard test walks the embedded assets in the opposite
  direction (asset → list) so an unregistered skill dir fails the build by name; `social-growth`
  stays the single documented opt-in exception.
- **The shipped skill registry pointed at a deleted asset dir.** `assets/configs/skills.toml`
  (deployed to `~/.config/8sync/skills.toml`) still declared `src = "builtin:karpathy"` after the
  rename to `karpathy-guidelines`, so `8sync skill update` warned and skipped it on every run.
- **User-facing docs advertised 15 skills that no longer ship.** `README.md`, `AGENTS.md` and the
  docs site claimed "37 skills bundled" (and, elsewhere, 17) and named `literature-review`,
  `autoresearch` and `paper-writing` among "18 research skills" — all three directories were
  deleted, and the research set is now `deep-research`, `research-paper`, `remote-compute`. Every
  count now reads 23, matching `assets/skills/*/SKILL.md`, every named skill resolves to a real
  directory, and the two-tier force-load order (4 CORE + 6 specialist always-on) is described as
  `00-force-load.md` actually implements it. The docs site also told users to run
  `8sync skill add builtin:karpathy`, which no longer resolves.

### Added — Fedora-first package core (8sync now works on Fedora/RHEL)
- `trait PkgBackend` with `Pacman` + `Dnf` impls, selected by `env_detect::distro_family()`
  (`Family::{Arch,Fedora,Other}`, reads `ID` **and** `ID_LIKE`; Fedora 44 ships no `ID_LIKE`, so
  `ID` alone resolves). `backend()` returns `Option`, so a distro with neither manager keeps
  today's warn-and-continue instead of failing — Debian/openSUSE users are unaffected.
- Every package-manager spawn now lives **only** in `pkg.rs`. `platform::install_core_pkg` takes a
  `CorePkg { arch, fedora, brew, winget }` instead of positional strings nobody could tell apart.
- Profiles gained an **additive** `[packages.fedora]` table (`dnf`/`copr`/`rpmfusion`/`swap`)
  beside the existing `pacman`/`aur`/`aur_yay` keys, plus `#[serde(deny_unknown_fields)]`. All 37
  Fedora package names were verified against the live repos with `dnf repoquery`; profiles with no
  real rpm equivalent declare none and are skipped with a printed reason rather than guessing.
- `vpn` and `harness browser` are no longer Arch-only (Cloudflare WARP / `chromium` on Fedora);
  `clean` gained a dnf arm (`dnf clean all`, `dnf autoremove`).
- COPR is treated as third-party root code: `copr_enable` refuses any `owner/project` outside a
  checked-in allowlist unless explicitly allowed, and `dnf swap --allowerasing` prints the erase
  set first.

### Fixed
- **`8sync setup` aborted at step 3 of 8 on every non-Arch Linux.** The AUR-helper step was gated
  on `Os::Linux` instead of the Arch family, so on Fedora it failed and `try_step`'s `?`
  propagated — `codegraph`, `path-bootstrap`, `configs`, `skills` and `codegraph-skill` never ran.
  This is why `~/.omp/skills` was empty on Fedora machines.
- **Skill paths written into `AGENTS.md` were absolute to the authoring machine**
  (`/home/alexdev/...`, `/home/alexng/...`), so on any other clone omp was told to read files that
  do not exist and silently skipped the skills. `skill/inject.rs` now emits repo-relative paths for
  project skills and `~/`-anchored paths for global ones. `harness audit` could never catch this
  because it skipped every `/`-prefixed token; it now flags `/home/`, `/Users/`, `/root/` while
  still ignoring `/etc`, `/usr`, `/tmp`.
- `Dnf::install` left a half-applied batch: its plan can hold two transactions (install, then
  upgrade), and a failed upgrade returned `Err` while the committed install stayed. It now reverts
  via `dnf history undo`.
- `install.sh` created its temp file in `/tmp` (tmpfs) while installing to `~/.local/bin` (btrfs),
  so the "atomic" `mv` crossed filesystems and degraded to a copy. The temp file is now a sibling
  of the destination, matching `selfup.rs`.
- `install.ps1` hardcoded the x86_64 asset, silently giving Windows ARM64 the wrong binary.

### Changed — enforced tool routing (omp stops ignoring the code-intel stack)
- Preferring codegraph/serena/codebase-memory was previously **prose**, which omp ignores under
  load. It is now enforced by mechanisms omp implements in code: a TTSR rule
  (`condition` + `scope: "tool:grep(*), tool:glob(*)"` + `interruptMode: tool-only`) that aborts the
  stream mid-tool-call, plus `bashInterceptor` patterns closing the `bash rg` escape.
- Both are **capability-gated**: with no code-intel tool installed the rule and the interceptor
  block are removed, so a bare machine is never dead-ended. TTSR `repeatMode: once` means the worst
  case is one restarted turn, not a hard veto.
- `APPEND_SYSTEM.md` shrank 9 198 → 3 260 B (−64.6 %); it is in every prompt on every turn, and the
  TTSR rule body costs zero prompt tokens.
- Downloads are now sha256-verified using the `digest` field the GitHub Releases API already
  returns — no new dependency, no release-plumbing change.
- First tests in the repo: 48 passing (`cargo test`), covering distro parsing, the pacman argv
  regression fixture, the COPR allowlist refusal, path portability, and audit path classification.

## [0.53.0] — 2026-08-08

### Added — `8sync .` named per-project sessions (run many features at once)
- `8sync .` grew a session layer so you can keep several concurrent lines of work in one repo,
  each an isolated omp conversation. Surface (namespaced under `.`, no new top-level verbs):
  `8sync . <name>` create-or-resume · `8sync . new <name> [--worktree]` · `8sync . ls`/`--list`
  (`--json`) · `8sync . mv <old> <new>` · `8sync . rm <name> [--force]` · `8sync . merge <name>...
  [--keep-worktree]`. `8sync .` with no name resumes the last-used session (omp's default store
  when none was ever named — unchanged legacy behavior).
- Mechanism (a thin layer over omp's existing session core — not a reinvention): one omp
  `--session-dir` per name (omp's own `--continue` resumes it — zero session-id bookkeeping),
  tracked in a machine-local registry `~/.config/8sync/sessions/<repo>/index.json` that stores
  only what omp lacks (human name, worktree binding, last-used). Every launch reuses `ModelConfig`
  so STEP-0 tool-routing + advisor survive.
- `--worktree` gives a session its own `git worktree add -b 8sync/<name>` so two features edit the
  same files concurrently without collision; `ls` shows branch + dirty state.
- `merge <name>...` lands session branches into the current branch, ECC-style (affaan-m/ECC, MIT):
  read-only `git merge-tree --write-tree` conflict preflight → `git merge --no-edit` → rebase the
  conflicting worktree onto the target to unblock (auto-abort + skip on true conflict) → clean up
  merged worktree + branch + session (`--keep-worktree` to preserve). Sequential, so branch-vs-branch
  conflicts surface as the target advances. Local-only — never pushes.
- Everything runs through `git`/`omp` shell-out — **no new dependencies** (ECC's rusqlite/git2/tokio
  stack was deliberately not adopted; it would bust the size budget). `8sync doctor` reports session
  count + dirty worktrees. New module `crates/cli/src/verbs/session.rs`; `here.rs` dispatches.
- Evaluated but NOT integrated (user asked to review): linshenkx/prompt-optimizer (AGPL-3.0 +
  optimizes human prose — wrong layer) and TypeScript 7 / typescript-go (transparent to `8sync run`,
  no code change).

### Fixed — `8sync up` was Linux-only and bricked itself on Windows
- Symptom (Windows): running `8sync up` left an un-runnable file so the next `8sync` invocation popped Windows' *"Select an app to open '8sync'"* picker instead of executing. Root cause in `crates/cli/src/verbs/selfup.rs`: the self-updater hard-coded `ASSET_SUFFIX = "-linux-x86_64"` (downloaded the **Linux** binary), wrote it to `~/.local/bin/8sync` with **no `.exe`**, and replaced it with a plain `std::fs::rename` (which Windows forbids on a running `.exe`). The release CI already publishes `8sync-<tag>-windows-x86_64.exe` and `install.ps1` installs to `%LOCALAPPDATA%\Programs\8sync\8sync.exe`, so the updater was simply never taught about non-Linux.
- Fix — self-update is now genuinely cross-platform:
  - **Correct asset per OS/arch:** `asset_label()` maps `platform::os()` + `std::env::consts::ARCH` to the CI labels (`linux-x86_64`, `linux-aarch64`, `darwin-x86_64`, `darwin-arm64`, `windows-x86_64`; macOS ARM is `arm64`, not `aarch64`), and `asset_filename()` appends `.exe` on Windows. Both `fetch_latest_release` (auto-check + `8sync up`) and `install_tag` (`8sync up --to`) use it.
  - **Install to the running exe** via `dest_path()` = `std::env::current_exe()` (canonicalized), so `8sync up` replaces 8sync wherever it actually lives — the Unix `~/.local/bin/8sync` OR the Windows `…\Programs\8sync\8sync.exe` — instead of a hard-coded Unix path. Falls back to the legacy path only if the current exe can't be resolved.
  - **Windows-safe replace:** `download_and_replace()` renames the live `.exe` aside to `.8sync.old.<pid>` (allowed while running) before sliding the new binary in, restoring the original if the install fails; stale `.old` files are swept on the next run. Unix keeps the same-dir inode-swap rename.
- Verified: native Linux release build is clean; the `#[cfg(windows)]` branch type-checks (it uses only cross-platform `std::fs` APIs — confirmed by compiling it with the gate temporarily forced on).

### Fixed — serena's web dashboard opened a browser tab on every session start
- Symptom: a Serena dashboard tab kept reappearing (showing `Error loading configuration` / `Error loading stats`, since it races the server it is reporting on), and RAM climbed. Cause: serena ships `web_dashboard: true` **and** `web_dashboard_open_on_launch: true`, so every `start-mcp-server` binds an HTTP dashboard and pops a browser tab. omp spawns one serena per session, so this multiplies — measured here: **16 live serena processes holding 878 MB**, one dashboard each.
- Fix: `deploy::ensure_serena_mcp` now registers serena with `--enable-web-dashboard False`. The flag is passed on the **command line** rather than by editing `~/.serena/serena_config.yml`, because serena owns that file and rewrites it — a config edit alone does not survive. `register_omp_mcp`'s self-heal rewrites the existing `mcp.json` entry in place, so the fix lands on `8sync harness global` with no manual step.
- The dashboard is pure observability: serena's MCP tools are unaffected. Verified live — no listener on `24282`, and `mcp__serena_get_current_config` still answers.
- Note (not a leak): a serena pair (`uv` wrapper + python child ≈ 145 MB) is held per **live omp session**, not per project. Closing unused omp sessions is what reclaims that memory.

### Added — STEP-0 tool-routing ENFORCEMENT (`step0`): the rule is now code, not prose
- Measured problem (this machine, `8sync harness toolstats`): the STEP-0 MCP stack was connected and callable yet effectively unused — `cbm 0 · serena 0 · headroom 0` agent calls; every code lookup fell back to the built-in `read`/`grep`. Prose directives in `APPEND_SYSTEM.md`/AGENTS.md had failed to change behaviour three times (see KNOWLEDGE). Zero-friction built-ins always beat an instruction.
- **Fix — remove the fallback instead of asking nicely.** `8sync ai` / `8sync .` now launch omp with `--tools` DROPping the two redundant searchers `grep` + `glob` (`STEP0_TOOLS` in `crates/cli/src/models.rs`), so code lookup MUST flow through codegraph (CLI) · codebase-memory-mcp · serena. `--tools` is an **allowlist** (omp has no deny-list — `tools.blocked` is only a telemetry counter), so it must name every tool to keep: anything omitted is silently disabled. The list is omp's validator list minus `grep`/`glob`, and minus `computer` to preserve its default-disabled state. `lsp` is deliberately KEPT (zero-friction; serena needs `activate_project` per session). MCP/xdev tools are orthogonal to `--tools` and survive it.
- **Shell escape closed:** `deploy::ensure_bash_interceptor` writes `bashInterceptor.patterns` into `~/.omp/agent/config.yml`. omp's rule shape is `{ pattern, tool, message }`, and its matcher **skips any rule whose `tool` is not in the session's tool list** — so a rule pointing at `grep`/`glob` disables itself precisely because STEP-0 removed them. Every rule therefore points at `lsp` (always present, and the honest suggestion). Blocks `rg`/`ag`/`ack`, `grep -r`, and `find -name`; single-file and log `grep` stay allowed. Setting the key replaces omp's default array. Wired into both `global_pass` and `harness_init`.
- **Verified end-to-end on omp 17.2.9** (the previous iteration was verified only as "the string is embedded in the binary", which is why it shipped broken): the captured provider request carries 18 tools with `grep`/`glob` **absent** and `recall`/`retain`/`reflect`/`memory_edit`/`hub`/`eval`/`ast_edit`/`lsp` **present**; `8sync ai` returns normally; `rg main main.rs` is refused with `Blocked: STEP-0: …`; `grep -r main .` is refused; plain `grep main main.rs` still runs.
- **Drift guard so the allowlist cannot rot silently again** (`8sync doctor`): an allowlist fails in two directions, and both have now happened — a name omp *drops* bricks every launch, a name omp *adds* disappears from the agent with no error at all. `models::step0_tool_drift` asks omp itself (`omp --tools __8sync_probe__ -p ""` — the validator runs before any provider call, so the probe is free and offline) and diffs the answer against `STEP0_TOOLS`, reporting `REJECTED by omp: …` or `missing omp tool(s): …`. Proven in both directions: injecting `bogus_tool` produced the REJECTED warning, reverting restored `✓ STEP-0 allowlist matches omp's tool list`.
- **`codegraph callers` demoted in the STEP-0 routing** (`assets/configs/omp/APPEND_SYSTEM.md`): on a clean, freshly rebuilt full index it reported `No callers found` for a function with two real call sites, and the misses cluster by *caller* (nothing inside `global_pass` resolves) rather than by callee — `let _ =` discards are not the cause. codegraph is a prebuilt external binary with no local source, so "who calls X" now routes to `mcp__serena_find_referencing_symbols`, with `codegraph callers` allowed only as a second opinion, never as proof of absence. The same note warns against `rm -rf .codegraph`, which deletes the exclusion config and makes the next index walk ~16k files instead of ~6k.
- **Toggle (never a trap):** default ON; `8sync ai --no-step0` for one run, or `step0 = false` in `~/.config/8sync/models.toml`.
- `APPEND_SYSTEM.md` RULE #0 rewritten to match reality: states that `grep`/`glob` are gone, gives a cheapest-first routing decision tree (cbm → serena → codegraph), and **drops the stale omp-16 `mcp.discoveryDefaultServers` claim** (omp ≥17 mounts MCP as `xd://` devices — that key no longer exists).

### Fixed — STEP-0 review pass: interceptor over-block + config-migration data loss
An independent reviewer (12 min) caught defects the happy-path tests missed; all verified live.
- **grep rule blocked single-file/log grep on hyphenated names** (`grep x-ray my-report.txt`, `grep bug build-Release.log`). The `.*(-[rR])` shape matched `-r`/`-R` *inside* words. The first repair (`\s-`) then over-corrected — `grep\s+` already consumed the space, so a first-token `grep -r` stopped matching and recursive grep was *allowed through*. Final pattern uses a negative lookbehind `(?<![A-Za-z0-9-])` so the dash must be a real token boundary, and excludes double-dash long flags from the short-flag matcher (`--color`/`--directories` contain an `r` but are not recursive). Verified 19/19 cases in Python before editing source, then live both directions: `grep -r` blocked, hyphenated-name grep + `--color=auto` allowed. Same lookbehind applied to the `find`/`fd` `-name`/`-type` rule.
- **config migration silently deleted user keys** — block-end detection used `is_ascii_alphabetic()`, so a following `# comment`, `_private:`, or `'quoted':` key was swallowed into the block and overwritten on the next `8sync harness`. Now ends at any column-0 line that is not an indented continuation, and the scan is no longer single-occurrence: every `bashInterceptor:` block is enumerated and only those carrying the `STEP-0` signature are removed (any ordering; duplicates collapse), so a user-authored block is never touched. Proven by replicating the exact Rust logic in Python against a config with comment + `_key` + quoted key + user block + ours.
- **doctor surfaced only one drift direction** — an omp upgrade that renames *and* adds reports both now (`rejected` and `silently_disabled` are no longer mutually exclusive arms).
- Trims the blank line that accumulated per `8sync harness` run.
- **Second review round — the `^` anchor made the guard mostly decorative.** `^\s*grep` only fires when the tool starts the command string, so `cd src && grep -r foo .`, `cd crates/cli && rg TODO`, `LC_ALL=C grep -r`, `sudo grep -r`, `time rg`, `cat x | grep -r y` and `; do grep -r …` all escaped. Rules now match at a **command position** — `(?:^|[;&|])` plus optional `\` escape, env assignments and wrapper words. `(` is deliberately not a separator and `do`/`then` are wrappers only, so quoted prose (`git commit -m '(rg removal)'`, `echo "do rg later"`) no longer trips the rule.
- **Substitutes closed:** blocking `rg` alone just moved the habit. `git grep` (recursive by default — the obvious fallback), `egrep`/`fgrep`, bare `fd '\.rs$'` and `fd -t f`, and `find -path`/`-regex` were all open. Tools with no single-file mode worth preserving (`rg`, `fd`, `git grep`) are now blocked outright; only `grep` gets flag analysis.
- **The grep rule is quote-aware.** It walks the option cluster (a run of shell words that are neither quoted nor containing `;&|`) instead of `.*`, so a flag-looking string inside the *search pattern* no longer counts: `grep " -r " f.txt`, `grep 'make -r' build.log`, `grep -F -- -r file.txt` and `grep pat /var/log/-r-rotated.log` are allowed again, and the scan can no longer cross into the next command (`grep foo a.txt; ls -r`). 48/48 cases verified against the patterns as actually written to disk, in omp's own JS engine.
- **P1 — a user-authored `bashInterceptor:` block was being silently voided.** The code appended its own block beside the user's, assuming a duplicate top-level key would make omp fail loudly. It does not: omp parses config.yml with `Bun.YAML.parse`, which keeps the **last** key, and 8sync always appends last — so every rule the user wrote disappeared on each `8sync harness`, with no error anywhere. The function now detects a surviving user block, leaves the file untouched, and warns instead of installing.
- Fresh-machine formatting: no stray leading blank line when `config.yml` did not exist.

### Fixed — omp 17 "Failed to load extension: mutable default value must be specified as a factory"
- omp 17.2.9 added a schema validator that rejects any zod `.default(<mutable>)` where the value is an array or object literal — it must be a **factory**: `.default(() => [])`. Primitives (`false`/`0`/`""`/`3`) are unaffected. Every `omp` session printed `Warning: Failed to load extension …/8sync-engine.ts: ParseError: A mutable default value must be specified as a factory`, so none of the `engine_*` tools registered.
- One site: `assets/extensions/8sync-engine.ts:146`, `verify: z.array(z.string()).default([])` → `.default(() => [])`. `8sync-workflow.ts` had no defaults. Verified: `omp -p` in the affected project loads the extension with zero warnings. The asset is rust-embedded, so this needs `cargo build` + `8sync harness` to redeploy; live project copies were refreshed directly for immediate relief.
- Note: the separate "`omp --continue` loses chat history" report is **not** caused by this — omp continues past extension-load failures. It is a distinct omp-17 symptom; retest `--continue` after this fix and diagnose separately if it persists.

### Changed — CI: `aarch64` builds without Docker, and the size budget is now enforced
- **`cross` → `cargo-zigbuild`** for `aarch64-unknown-linux-musl`. This fixes a correctness bug, not just speed: `cross` ran inside a Docker image with no JS toolchain, so `build.rs` could not build the Vite dashboard and **silently embedded the stub page** into that asset. zigbuild runs on the runner, where `npm` exists. Verified locally (zig 0.16.0 + cargo-zigbuild 0.23.0): 31.9 s → statically linked aarch64 ELF, **4 151 328 B — already under the 4 MiB goal**.
- **New `scripts/size-gate.sh`**, run on every release asset: hard-fails above a **5 MiB ceiling**, warns above the **4 MiB goal**. `AGENTS.md` §8 carried a budget nothing enforced, which is how the binary drifted 52 % over unnoticed. The ceiling sits above today's size on purpose — a gate that is already red gets ignored — and ratchets down as `size-report.sh` shows headroom. Both directions tested.
- **`universal2` rejected** (reversing the earlier proposal): a fat macOS binary makes every Mac user download both slices — the opposite of the goal — and would rename assets `install.sh` resolves by `${os}-${arch}`.

### Fixed — a directory merely *named* `su-code` made its parent look like a project
- `discover::detect_current_project_root` and `harness global`'s project test accepted any folder called `su-code` as proof of an omp project. Since this repo's own checkout is `~/Projects/tools/su-code`, an auto-stamp run from `~/Projects/tools` wrote a blank memory tree (`STATE.md`, `KNOWLEDGE.md`, `PLAYBOOKS.md`, …) **plus a 74-entry `skills/` tree into the repo root**.
- Both paths now share `discover::is_omp_project`, and the `su-code` marker requires the directory to actually contain memory (`skills/` or one of `STATE.md`/`KNOWLEDGE.md`/`PROJECT.md`/`PLAYBOOKS.md`/`skills.toml`). Verified three ways: a bare `su-code/` dir is left alone, an `AGENTS.md` repo is still stamped, and a memory tree without `AGENTS.md` is still detected.

### Changed — **binary is 24 % smaller**: 6 407 848 → 4 859 696 B, no feature removed (`lean-binary` M2)
- **Dropped `rusqlite` (−1 035 384 B).** `harness toolstats` never needed a database: its ingest opened with `DELETE FROM calls` and re-parsed every session JSONL on each run, so nothing ever persisted — 1 MB of embedded SQLite C answered `COUNT`/`GROUP BY` over rows the same process had just built. Now a single in-memory pass. Output is **byte-identical**, proven by running the old and new binaries against a frozen copy of the session tree; only the provenance line changed (`→ …/toolstats.db` became `← …/sessions/<slug>`). SQLite left `ORDER BY` ties in table-scan order, so ranking tie-breaks on first appearance to preserve it.
- **Replaced `elkjs` with `@dagrejs/dagre` (−512 768 B).** `elk.bundled.js` was 1 606 238 B of the 1 891 858 B dashboard chunk — 85 % of the frontend was a GWT-compiled Java layout engine serving two `layered` calls. New `web/src/layout.ts`; bundle down to 478 704 B (−75 %). A lazy `import()` was measured first and rejected: `rust-embed` embeds the whole `web/dist` tree, so chunk-splitting saves zero binary bytes. Verified headless — codegraph lays out in four clean LR ranks, workflow auto-layout chains top-down, zero page errors.
- **`toolstats` feature flag removed** — it guarded only `rusqlite`. `features` is now just `web`; a `--no-default-features` build *gains* the tracker and sits at 3 109 496 B, **25.86 % under the 4 MiB budget**.
- `AGENTS.md` / `README.md` size and tracker claims replaced with measured numbers.

### Added — `lean-binary` feature scope (GSD planning tree): M0 + M1 landed
- `su-code/planning/lean-binary/` — 4-phase roadmap to bring the binary back under the `AGENTS.md` §8 "< 4 MB" budget **without dropping a user-visible feature**: M0 land pending WIP · M1 feature gating + per-gate byte attribution · M2 eliminate rather than gate · M3 CI (`cargo-zigbuild` `aarch64-musl`, macOS `universal2`) + budget truth.
- **M0 PASS.** Baseline **6 407 848 B** vs the 4 194 304 B budget. Every commit in the range was replayed in a throwaway `git worktree` and built from scratch — 4/4 green — because `engine_verify` only ever sees the working tree.
- **M1 PASS (8/8 AC).** `crates/cli/Cargo.toml` gets its first `[features]` table: `default = ["web","toolstats"]`, `web` = axum + tokio + tower-http + scraper, `toolstats` = rusqlite. `marketplace` folds into `web` (single caller: `/api/marketplace`). A `--no-default-features` build now compiles **and runs**, warning-clean, with no JS toolchain — `build.rs` skips the Vite bundle when `CARGO_FEATURE_WEB` is unset. Gated subcommands say `rebuild with --features web` instead of failing as unknown.
- **New `scripts/size-report.sh`** — A/Bs four feature combinations into separate `--target-dir`s. Measured: full **6 407 144** · web-only 5 346 304 · toolstats-only **4 144 576** · minimal **3 081 416**. Per-gate cost: `web` **2 262 568 B**, `toolstats` **1 060 840 B**.
- **`cargo bloat` was wrong by ~26×** on SQLite (40 KiB claimed vs 1 060 840 B measured) — it attributes `.text` by symbol and is blind to `.rodata` and C blobs. Ranking tool only; every load-bearing number now comes from an A/B build.

### Added — `deep-research` §5 "Native & Binary-Weight Audits" + a measured audit of `8sync` itself
- `assets/skills/deep-research/SKILL.md` gains a 7-step protocol for native/binary-weight research: ground with `size -A`, attribute with `cargo bloat --crates` (and chase its `[Unknown]` C row through `build/*/out/*.a`), A/B every proposed flag into a scratch `--target-dir` with an explicit `--target`, record falsified knobs as `failure:` entries, trace surprise deps with `cargo tree -i`, prefer `[features]` gating over rewrites, and reach for Zig only as build tooling.
- Applied it to this repo — brief at `outputs/native-tooling-zig-rust.md` (+ plan/drafts/provenance). Headline: the binary is **6 406 696 B (6.11 MiB)** against the `AGENTS.md` §8 "< 4 MB" budget, and the cause is that `crates/cli/Cargo.toml` has **no `[features]` section** — `harness web` (axum/tokio/hyper), marketplace scraping (scraper/html5ever) and `harness toolstats` (bundled SQLite, 2.1 MB `.a`) link into every build, alongside 4.9 MB of embedded assets (`web/dist` 1.9 MB, `impeccable/scripts` 1.6 MB).
- Release-profile knobs proven exhausted by A/B: `opt-level="s"` is **+307 392 B** vs `"z"`; `-C force-unwind-tables=no` saves **704 B**; `-C relocation-model=static` breaks proc-macro builds. No profile change made.

### Added — `branch-sync` skill & automated script for zero-conflict multi-branch sync
- **`/sync-pr` slash command**: Deployed globally (`~/.omp/agent/commands/sync-pr.md`) and per-project (`.omp/commands/sync-pr.md`). Invoking `/sync-pr [<branch>]` in any omp session automatically loads the `branch-sync` skill, audits all local/remote branches, deep-previews and safely merges specified feature branches to `main`, and synchronizes all active branches to match latest `main` with zero conflicts.
- New skill `assets/skills/branch-sync/SKILL.md` + helper script `assets/skills/branch-sync/scripts/branch_sync.py`: provides complete audit, deep-preview (`git diff main...<branch>`, commit breakdown, conflict dry-run), safe merge to `main`, and zero-conflict multi-branch synchronization across all active local/remote branches without risk of merge conflict leaks or data loss.

### Enhanced — `8sync harness global` auto-detection for `su-code/` projects
- `8sync harness global` now automatically detects if the current working directory is an `omp` project (containing `su-code/` or `AGENTS.md`) and stamps its per-project harness layer (vendors global skills into `su-code/skills/`, injects `AGENTS.md`/`CLAUDE.md`, seeds memory, installs pre-commit hook, and initializes `codegraph`) without requiring manual `--sweep`.

### Enhanced — `deep-research` skill with AI Agent loop engineering & design patterns
- Deepened `assets/skills/deep-research/SKILL.md`: incorporates state-machine loop engineering (Plan → Execute → Verify → Advance), STEP-0 code intelligence (`codegraph`, `codebase-memory-mcp`, `serena`), multi-agent wave execution (`tasks[]`), headroom output compression, modality-fit vision routing, and ponytail YAGNI discipline for large real-world codebases.

### Added — `8sync omp update` verb (update omp + auto-repair a blocked install)
- New verb `crates/cli/src/verbs/omp.rs` (`8sync omp update`, `--force`): runs `omp update`
  and, when it fails with `npm error EEXIST … ~/.local/bin/omp` or bun `Fail extracting
  tarball`, auto-repairs — backs up the current binary, clears the standalone file squatting
  the symlink path, `npm install -g @oh-my-pi/pi-coding-agent@latest`, then re-resolves and
  reports `CURRENT → NEW`. Runs from the SHELL (not an omp `/command`) so it works even when
  omp is broken; native Rust, no `sudo`/system-pkg/git. `8sync up` note now points here.
- Verified live both paths: normal ("Already up to date" @ 17.0.7) and `--force` (backup → rm
  → npm reinstall → `~/.local/bin/omp` healthy `node_modules` symlink). Root cause: omp ships
  as a standalone binary that squats the bin path where the package manager wants a symlink.

### Added — Lark (larksuite) to the `apps-personal` profile
- `assets/profiles/apps-personal.toml` now installs Lark via AUR `larksuite-bin`
  (v7.66.11, URL larksuite.com) alongside Bitwarden. AUR bin package auto-fetches the
  official `.deb` at build time and is pacman-tracked — the 437 MB binary is NEVER
  committed to the repo/GitHub, only the download reference. Sets `requires.aur_helper`.
  China build (Feishu) is a separate package, `feishu-bin`. Opt-in; the `alexdev`
  bundle intentionally excludes `apps-personal`.

### Fixed — STEP-0 MCP tools were connected but NEVER called (0.09% usage) → always visible
- Root cause (measured over 29 omp sessions / 13,854 tool calls: serena 0 · headroom 0 ·
  cbm 10 · zai 3 calls; `search_tool_bm25` taken 2×): omp's default
  `tools.discoveryMode: auto` hides ALL MCP tools behind a `search_tool_bm25` discovery
  hop once the registry exceeds 40 tools (this stack registers 48), and every instruction
  surface taught BASE tool names (`search_graph`, `find_symbol`) that are not callable —
  the registered forms are `mcp__codebase_memory_mcp_search_graph`, `mcp__serena_find_symbol`, ….
- Fix: `8sync harness`/`harness global` now writes `mcp.discoveryDefaultServers:
  [codebase-memory-mcp, headroom, serena, zai-vision]` into `~/.omp/agent/config.yml`
  (`ensure_mcp_tools_visible` in `crates/cli/src/verbs/skill/deploy.rs`) — the four
  harness servers' full catalogs stay in the active tool set, zero discovery hops.
  Key-presence idempotent; never overrides a user-set value. NOTE:
  `tools.essentialOverride` does NOT work for MCP (omp filters entries to built-in
  tool names only — verified in omp 16.4.8 bundle); an inert pin block from that
  approach is auto-migrated away (byte-exact match only).
- Instruction surfaces now teach the REGISTERED `mcp__…` names + the
  `search_tool_bm25` escape hatch for other/new servers: `APPEND_SYSTEM.md` RULE #0,
  `00-force-load.md`, the AGENTS.md STEP-0 sentinel template (`inject.rs`), the
  capabilities catalog header, and the feature-skill R10 literals (assets + su-code
  mirror). Stale names dropped everywhere: `semantic_query` (never existed on cbm),
  `codegraph search/deps/defs` (real 1.1.2 verbs: `query/explore/node/callers/callees/impact`).
  Unfollowable headroom mandate ("compress before it enters context") rewritten to the
  followable form: compress what YOU re-emit (`mcp__headroom_compress`).
- `8sync doctor` now flags hidden MCP tools (`discoveryDefaultServers` missing) and a
  dead serena (mcp.json + uvx check). Live probe verified: `omp -p` calls
  `mcp__codebase_memory_mcp_search_graph` and `mcp__serena_find_symbol` directly — both OK,
  no discovery hop; toolstats records serena/cbm optimizer calls for the first time.

### Fixed — omp ≥17 dropped bm25 discovery; the recurring "MCP HIDDEN" was a phantom
- omp 17.x replaced the pre-17 tool-discovery model (the `search_tool_bm25` hop +
  `mcp.discoveryDefaultServers`) with `tools.xdev` (default on): MCP tools mount as
  `xd://mcp__…` device URLs, callable via read/write without shipping schemas every
  request. `discoveryDefaultServers` is gone from omp's settings schema — writing it is
  a no-op that omp strips on the next config rewrite. That churn is exactly why STEP-0
  looked like it "regressed" after every omp self-upgrade (upgrade resets `config.yml`
  → doctor's string-check screamed HIDDEN → re-harness → repeat), while the tools were
  callable all along (verified live via `xd://mcp__codebase_memory_mcp_*` this session).
- `ensure_mcp_tools_visible` (`deploy.rs`) now detects omp ≥17 and skips the dead key
  (nothing to configure — `tools.xdev` mounts them); pre-17 keeps the discoveryDefaultServers write.
- `8sync doctor` MCP check is omp-version-aware: omp ≥17 reports the xd:// mount (OK)
  instead of the false "HIDDEN behind search_tool_bm25" warning; the `harness global`
  summary bullet updated to match. New `env_detect::omp_major()` parses omp's major version.

### Added — omp `/push-now` command (cross-machine handoff + commit + push)
- New embedded command `assets/commands/push-now.md`, deployed by `8sync harness`
  (`ensure_engine` in `crates/cli/src/verbs/skill/deploy.rs`) to
  `~/.omp/agent/commands/push-now.md` (global) + `<repo>/.omp/commands/push-now.md`
  (project), alongside `/auto` and `/feature`. `/push-now [msg]` is the
  "I'm switching machines right now" verb: it rewrites `su-code/STATE.md` with a
  cold-resume handoff (branch/HEAD, what changed this session, done/next/blockers,
  new-machine runbook), updates `CHANGELOG.md`/`KNOWLEDGE.md` if code changed, then
  `git add -A` + commit (gitleaks-gated, no `--no-verify` past a real secret) +
  `git push` origin current branch. No PR, no branch switch, no tag bump, no
  force-push (that's `8sync ship` / a release). Since it's in `assets/`, a fresh
  machine gets it automatically after `git pull && bash scripts/bootstrap.sh && 8sync harness`.

### Added — omp `/pull-now` command (cold-resume: pull + orient + prepare)
- New embedded command `assets/commands/pull-now.md`, deployed by `8sync harness`
  the same way (`ensure_engine`) to `~/.omp/agent/commands/pull-now.md` +
  `<repo>/.omp/commands/pull-now.md`. `/pull-now [go]` is the **receiving end of
  `/push-now`**: safe `git pull` (ff-only → rebase on divergence; stop on conflict
  or dirty tree), then read `su-code/STATE.md` HANDOFF + recent `KNOWLEDGE.md`
  learnings + `CHANGELOG.md`/`git log` to understand exactly where the project is,
  prepare the workspace (rebuild + `8sync harness` if `crates/`/`assets/` changed,
  verify the per-machine gotchas the handoff listed, `8sync doctor`), and report
  current state + the single next concrete action. `go` = start that action;
  empty = orient + prepare, then STOP for the human. No push, no clobber.

## [0.52.0] — 2026-07-09

### Added — `8sync vpn`: SoftEther VPN Client + VPN Gate (study-through-another-region)
- New top-level verb `vpn [install|gui|list|on|off|status]` (`crates/cli/src/verbs/vpn.rs`).
  Connect through **VPN Gate** (University of Tsukuba academic public relays) the
  way the Windows client does. `install` pulls the native Linux engine
  `softethervpn` (the maintained RTM 4.44 build — **not** the `-git` 5.x dev
  edition) + the **Windows VPN Client Manager GUI under Wine**
  (`softethervpn-client-manager`, where the Windows-style region-switch plugin
  lives; `--no-gui` skips it) + `dhcpcd`, and enables the client service.
  `gui` opens that manager. SoftEther has **no native Linux GUI** and its Linux
  client **can't rewrite the routing table itself**, so the reliable region-switch
  is the CLI: `list [CC]` ranks relays from the VPN Gate CSV API (optional
  2-letter country); `on [CC|ip]` picks the best relay, connects via `vpncmd`
  (HUB `VPNGATE`, user/pass `vpn`), **pins the relay route to the physical uplink**,
  DHCPs the tap, full-tunnels the default route, swaps DNS to 1.1.1.1, and
  **auto-rolls-back if egress doesn't change** (egress checked via Cloudflare's
  IP-addressed trace so it survives the DNS swap); `off` restores routes/DNS.
  VPN Gate relays are volunteer-run and **logged** — a learning tunnel only.

## [0.51.0] — 2026-07-09

### Added — `8sync feynman auth-omp`: reuse omp's auth in Feynman
- New top-level verb `feynman [auth-omp|status|off]` (`crates/cli/src/verbs/feynman.rs`).
  Feynman (companion-inc/feynman) is a Pi research agent that expects its own
  `feynman model login`; omp is a fast-moving Pi fork with a fresh model catalog +
  a credential vault. Both read `<home>/agent/auth.json` in the SAME schema, so
  `auth-omp` mirrors omp's LIVE credentials into `~/.feynman/agent/auth.json`:
  OAuth providers (Claude Pro/Max) as `{type:oauth, access:<omp token>}` **without
  the refresh token** — omp stays the sole refresher (no dueling token rotation),
  re-run when it expires; API-key providers as `{type:api_key, key:"!omp token <p>
  --raw"}` so keys resolve live from omp (no secret copied). A sidecar
  (`.8sync-omp.json`) records managed providers so `off` removes only those and
  never touches Feynman's own logins. Verified live (feynman 0.3.5 + omp 16.4.6):
  after `auth-omp`, `feynman model list` shows `anthropic/claude-opus-4-8` + `zai/*`
  (31 authed models) reusing omp's Claude OAuth; `off` reverts to 0 authed.
  Note: Claude Pro/Max OAuth via a third-party harness draws subscription
  extra-usage (billed per token).

## [0.50.0] — 2026-07-09

### Added — `8sync harness browser`: omp browser control that reaches the internet
- New `harness browser [fix|status|off]` (`crates/cli/src/verbs/harness/browser.rs`).
  omp's Puppeteer browser control could render but **fail to reach the internet** on
  the bundled `chrome-headless-shell`. `fix` (default) ensures **ungoogled-chromium-bin**
  is installed (`/usr/bin/chromium`) and exports `PUPPETEER_EXECUTABLE_PATH` +
  `BUN_CHROME_PATH` (the vars omp/Bun honor, with `--no-sandbox`) in zsh/bash/fish so
  every omp launch — direct or via `8sync .`/`8sync ai` — uses it. Idempotent
  (sentinel-managed rc block); `off` reverts to the bundled chromium, `status` shows
  the wiring. Verified: `/usr/bin/chromium` fetches pages headless; interactive
  bash/zsh resolve the exported path.

### Fixed — omp `/new` no longer lands in the wrong project root
- omp's `/new` creates a child session that **inherits the launch root** (it does NOT
  re-detect cwd), so a drifting cwd made `/new` open in the wrong project. `8sync .`
  and `8sync ai` now pin omp to the detected project root via omp's `--cwd <root>`
  flag (+ `current_dir`), so the session — and every `/new` child — is correctly
  scoped. `8sync ai` previously launched omp in the ambient cwd with no root pin.

## [0.49.1] — 2026-07-09

### Fixed — `add-model --think` now exposes a model's FULL reasoning range
- `--think` defaulted to only the efforts the user typed and always wrote
  `mode: anthropic-budget-effort` — so a reasoning model like `xai/grok-4.5` got a
  truncated set in omp's `/model` thinking picker instead of the native
  `minimal · low · medium · high · xhigh`.
- **Bare `--think`** (or `full`/`all`/`max`) now emits the **complete canonical tier
  set** `[minimal, low, medium, high, xhigh]` — matching what a native grok/claude
  reasoning model exposes. A subset (`--think "min,high"`) is mapped to canonical
  tiers and reordered; `--think off` disables. `defaultLevel` = `high`.
- **`mode` now follows the API**: `effort` for `openai-completions` (the generic
  `reasoning_effort` wire param — correct for xAI/OpenAI/most), `anthropic-budget-effort`
  for `anthropic-messages`. (omp's valid modes: `effort|budget|google-level|
  anthropic-adaptive|anthropic-budget-effort`; the flat `thinking: [..]` list form is
  rejected as input — the nested `mode`/`efforts`/`defaultLevel` block is required.)

## [0.49.0] — 2026-07-09

### Added — `8sync harness add-model`: register a REMOTE model omp's catalog lacks
- **New `harness add-model <provider/model> --url <baseUrl>`** (`crates/cli/src/verbs/harness/custom_model.rs`).
  For the case where omp hasn't shipped a model yet — or lists it with **null metadata**
  (e.g. a brand-new `xai/grok-4.5` shows `context: -`, `max-out: -`): register it as a
  full custom provider in `~/.omp/agent/models.yml`, so it appears in `/model` and routes.
- **Empirically grounded** (`omp` 16.3.12): a metadata-only merge into a built-in provider is
  **rejected** (`"baseUrl" is required when defining custom models`), so `--url` is mandatory;
  the model selector omp exposes is `<providerKey>/<modelId>`.
- Flags: `--url` (required) · `--key` (else `$<PROVIDER>_API_KEY`, else a visible placeholder) ·
  `--api openai|anthropic` (default openai-completions) · `--ctx <N>` (default 256000) ·
  `--max <N>` (default 32000) · `--vision` (adds image input) · `--think "min,low,med,high"`
  (emits a valid `anthropic-budget-effort` thinking block + marks the model reasoning-capable).
  Sub-verbs `list` / `rm <provider/model>`.
- **Source of truth** = TSV registry `~/.config/8sync/custom-models.tsv`; regenerates a
  sentinel-managed block that **coexists** with the local-models block and the 9router gateway
  providers (each `sync` strips only its own block; `gateway apply` now re-attaches both).
  Models sharing a provider are grouped under one provider key (YAML forbids dup keys).
  After writing, `add` calls `omp models --json` to **verify the config still loads** (a bad
  `--think`/`--api` combo is a loud warning, not a silent broken file).
- `add-model` was previously an undocumented alias of `add-local-model`; it now means the
  remote path. GGUF stays on `add-local-model`.

## [0.48.0] — 2026-07-08

### Added — large-scope `feature` framework (GSD) + the `8sync feature` verb
- **`feature` skill + `/feature` command** (bundled, deployed by `harness`): ports a
  spec-driven GSD framework into su-code for LARGE multi-phase scopes — a planning
  tree at `su-code/planning/<slug>/` (`PROJECT`/`REQUIREMENTS`/`ROADMAP`/`STATE` +
  per-phase `M<n>-{CONTEXT,PLAN,VERIFICATION}`), an `ACTIVE` pointer to switch between
  features across sessions, and an acceptance-criteria (AC) contract. Phase execution
  (`/feature go`) delegates to the existing `engine_*` verify-gate loop — it adds no
  new execution engine. Small/single-concern work still uses `/auto`.
- **`8sync feature` verb** (deterministic, no model): `new <slug>` scaffolds the
  planning tree from bundled templates + activates it; `switch <slug>` flips `ACTIVE`;
  `status` prints the active STATE position; `list` shows features (★ active) +
  archived. `plan`/`go`/`ship` need model judgement → run via `/feature` in omp.

### Added — single-source CLI command name (`brand.rs`) — rebrand in one place
- New `crates/cli/src/brand.rs` is the single source of the CLI identity: `CMD` (the
  invoked command name) + `NS` (on-disk config namespace, AGENTS sentinels, deployed
  artifact filenames), both defaulting to `8sync` (or set `SC_CMD`/`SC_NS` at build
  time). A rename now propagates to clap `name`/`bin_name`, every help/EXAMPLES block
  (one `rebrand` pass over the clap command tree), all runtime `8sync <verb>` prose
  (routed through `brand::render` at the `ui::*` chokepoint + the cheatsheets), config
  paths (`~/.config/<NS>`), `kitty/<NS>.conf`, AGENTS.md sentinels, and deployed
  `<NS>-recall.ts` / `<NS>-engine.ts` / `<NS>-workflow.ts` / `<NS>-harness-up` names.
- The default build is **byte-identical** to before (`render` is an identity function
  when `CMD == NS == "8sync"`; `8sync {help,flow}` diff empty). A rebuilt
  `SC_CMD=sc SC_NS=sc` binary emits `sc …` everywhere with zero stray `8sync` command
  tokens and `Usage: sc`.
- Excluded from the rebrand (fixed identifiers): the `8-Sync-Dev` org + `github.com`
  self-update URLs, the bundled `8sync-cli` skill dir, and the `.cache/8sync/`
  namespace (derived/gitignored; the verbatim `8sync-engine.ts` extension couples to
  it — rendering `.ts` code is out of scope).
- A one-time migration shim (runs in `setup`/`harness`/`doctor`, only when rebranded)
  renames `~/.config/8sync` → `~/.config/<NS>` + `kitty/<NS>.conf`, removes stale
  `8sync-` deployed artifacts, and `skill::inject` recognises the legacy `8sync:`
  sentinels to self-heal existing `AGENTS.md` files in place.

### Added — dashboard Knowledge browser + Create-Project
- **Knowledge page** (`Discover → Knowledge`): auto-fetches `sindresorhus/awesome`
  (raw README via `curl`, cached `.cache/8sync/knowledge/` 6h TTL), parses it into
  categories → entries, and lets you browse/search, multi-select, and **Save to
  project** → appends curated links to `<project>/su-code/REFERENCES.md` (deduped
  by URL). Backend `crates/cli/src/verbs/harness/knowledge.rs` + `GET /api/knowledge`
  + `POST /api/knowledge/apply`.
- **New Project** button (`Projects → Workspaces`): a modal scaffolds a fresh
  8sync project — `mkdir` + `git init` + AGENTS.md/su-code memory/skills block —
  and applies the chosen extras: skills vendored into `su-code/skills/`, MCP servers
  into the project's `.omp/mcp.json`, and knowledge into `su-code/REFERENCES.md`,
  then activates it. Backend `here::scaffold_project` + `POST /api/projects/create`.

### Added — `harness model` two-model combo preset
- `8sync harness model <strong>+<cheap>` (or `model=claude+glm`) sets **every omp
  role** across two providers in one shot, the cost-optimal split: cheap model
  does the mechanical bulk (`default`/`task` high · `smol`/`tiny`/`commit` minimal ·
  `advisor`), strong model does the thinking (`vision`/`slow` high · **`plan`/
  `designer`/`reviewer` xhigh**). Writes omp `~/.omp/agent/config.yml` `modelRoles`
  + `task.agentModelOverrides.reviewer` (line-based, preserves every other key)
  and keeps `~/.config/8sync/models.toml` in sync.
- Aliases: `claude`/`opus` → `anthropic/claude-opus-4-8`, `sonnet` → `claude-sonnet-5`,
  `glm`/`zai` → `zai/glm-5.2`, `haiku` → `claude-haiku-4-5`; a bare `provider/model`
  passes through. `vision` routes to the strong model because `glm-5.2` is text-only.
- `xhigh` is valid on **direct** `anthropic/*` (the 9router gateway caps at `high`).
- General `key=value` shorthand added to `harness` dispatch (`compaction=50` etc.).
- `harness model` view now also prints omp's live `modelRoles` (not just models.toml).

## [0.47.0] — 2026-07-06

### Added — cross-platform builds: macOS + Windows (was Linux/Arch-only)
- **New `crate::platform` module** — the OS seam. `platform::os()` (compile-time
  constant per target), `os_name()`, `require_linux()` (clean no-op guard for
  Linux-only verbs), `pkg_manager()` (pacman ⁄ brew ⁄ winget), `install_core_pkg()`
  (per-manager package-id map), and a **cross-platform periodic timer** —
  `install_timer`/`remove_timer` backed by **systemd user timer (Linux) ⁄ launchd
  LaunchAgent (macOS) ⁄ Scheduled Task (Windows)**. Uses only cross-platform
  std/crate APIs (no `std::os::unix`) so one body compiles on every target.
- **Portable prebuilts:** dropped `target-cpu=native` from `.cargo/config.toml`
  (it baked the build host's ISA into the binary → SIGILL on older CPUs of the
  same arch). Release binaries now run on any CPU of the target arch.
- **`harness up --timer` + `clean --timer`** now route through `platform::*` —
  identical systemd behavior on Linux (incl. the 0.46.2 cgroup memory bounds),
  launchd `StartInterval` on macOS, `schtasks /SC MINUTE` on Windows.
- **`setup` Stage A is cross-platform:** `gh` installs via the native package
  manager (`github-cli` on pacman, `gh` on brew/winget); `paru`/AUR is skipped
  off-Linux; Arch-only Stage B profiles are skipped with a clear note on
  macOS/Windows. `omp`/`codegraph` keep their curl installers (POSIX shells).
- **`sec`, `bt`, `clean`** guard with `require_linux` — a clean "Linux-only"
  message + no-op on macOS/Windows instead of shelling out to absent tools.
- **CI (`.github/workflows/release.yml`):** on a `v*` tag, a matrix builds real
  binaries on native runners — musl-static Linux x86_64/aarch64 (aarch64 via
  `cross`), macOS x86_64 (macos-13) + arm64 (macos-14), Windows x86_64 (MSVC) —
  and publishes them to a GitHub Release. Assets keep the `8sync-<tag>-<os>-<arch>`
  scheme (`--locked`, `contents: write`).
- **Installers:** `install.sh` extended to the full os×arch matrix (linux
  x86_64/aarch64, darwin x86_64/arm64) keeping the source-build fallback; new
  **`install.ps1`** (Windows) — `irm …/install.ps1 | iex`, `-Uninstall`,
  `$env:SUSYNC_VERSION` pin, User-PATH wiring.
- Verified: Linux release build clean (0 warnings, 6.16 MB); Linux runtime
  smoke (sec/clean/help) unaffected. mac/Windows compilation + binaries are
  produced and verified by the CI native runners (a Linux host cannot build
  MSVC/Apple-SDK targets or the C deps `rusqlite`/`zstd-sys` without each
  platform's toolchain).

## [0.46.2] — 2026-07-06

### Fixed — `harness up --timer` OOM-killed the machine
- The generated `8sync-harness-up.service` had **no cgroup resource limits**, so
  the per-tick `codegraph index` (peak RSS ~5.3 GB on a large repo, e.g. `zus`)
  tripped the kernel OOM killer — reported as `Result: oom-kill, Mem peak: 5.3G`
  every 10 min — thrashing swap and taking down other apps.
- The service unit is now bounded to its own cgroup + de-prioritized:
  `MemoryHigh=2G` (reclaim throttle), `MemoryMax=4G` (hard ceiling — kills only
  this unit, never the machine), `MemorySwapMax=512M`, `OOMPolicy=stop`,
  `Nice=15` / `CPUWeight=10` / `IOWeight=10`, `TimeoutStartSec=900`.
- Verified: codegraph now held at ~2 GB by reclaim pressure instead of ballooning
  to 5.3 GB. Re-run `8sync harness up --timer <dur>` in a project to regenerate
  the bounded unit (already-installed unbounded units are overwritten).

## [0.46.1] — 2026-07-06

### Fixed — sweep now redeploys the project-level `/auto` command
- `8sync harness global --sweep` migrated a project's memory folder
  (`agents/` → `su-code/`) but left the project's `.omp/commands/auto.md`
  (and `8sync-engine.ts`) untouched — so `/auto` in a swept repo kept reading
  `agents/STATE.md` from a stale copy deployed by an older binary (project
  commands take precedence over the global one in omp).
- `stamp_project` (the per-repo sweep layer) now calls `deploy::ensure_engine`,
  refreshing both the `/auto` command and the engine extension in every swept
  project. Byte-identical writes stay quiet. Verified: all projects under
  `~/Projects` now have `su-code/`-only `/auto` commands (0 stale).

## [0.46.0] — 2026-07-06

### Changed — the agent-memory folder is now `su-code/` (was `agents/`), a distinctive project marker
- **BREAKING (auto-migrated):** every project's agent-memory dir is renamed
  `agents/` → **`su-code/`** so a su-code-managed repo is unambiguously
  identifiable (the old `agents/`/`AGENTS.md` markers are generic — every repo
  has them). The `AGENTS.md` anchor file stays (open-standard entry point); only
  the folder moves, and its links are rewritten to point at `su-code/`.
- **Detection now keys on `su-code/`.** `is_omp_project` (sweep) + the harness
  project-root detection recognise `su-code/` (with `agents/` kept only as a
  legacy migration trigger). `8sync harness global --sweep [DIR]` (the sweep
  command — **not** `harness all up`) migrates every legacy project it finds.
- **Auto-migration** (`memory::migrate_legacy_layout`, runs in
  `here`/`init`/`up`/bare-harness/sweep): renames `agents/` → `su-code/` and
  rewrites `agents/` → `su-code/` references in the anchor + live memory
  markdown. **Guarded:** only fires on a real 8sync memory dir (identified by
  `STATE.md`/`KNOWLEDGE.md`/`PROJECT.md`/`PLAYBOOKS.md`/`skills.toml`/`skills/`),
  so a source package literally named `agents/` is never touched. `.agents/`
  and `subagents/` are protected from the text rewrite. Idempotent.
- All 8sync-authored code, assets, skills, docs, and the recall hook (with an
  `agents/` fallback for un-migrated repos) updated to `su-code/`. Historical
  CHANGELOG entries left as-written (they document the `agents/` era).
  Verified E2E: legacy project migrated (folder + refs), guard skipped a
  non-memory `agents/` source dir, this repo dogfood-migrated, build clean.

## [0.45.0] — 2026-07-06

### Added — MCP `server.json` standard conformance (marketplace install) + the spec as a machine-wide default
- The marketplace MCP install now conforms to the official MCP registry
  `server.json` spec (schema `2025-12-11`, `modelcontextprotocol/registry`).
  `official_install` (`crates/cli/src/verbs/harness/marketplace.rs`) rewritten to honor:
  - **`registryType` → runtime**: `npm`→`npx -y` · `pypi`→`uvx` · `oci`→`docker run -i --rm`
    (+ `-e NAME` env forwarding) · `nuget`→`dnx` (was: everything defaulted to `npx`).
  - **`version` pinning** — `identifier@version` (or `identifier:version` for docker images).
  - **`runtimeArguments` + `packageArguments`** rendered into the command line
    (named/positional Argument shapes).
  - **`transport.type`** — a package/remote with `streamable-http`/`sse` becomes a
    remote (`{type,url,headers}`), not stdio.
  - **BUGFIX: `environmentVariables` → a `{NAME: value}` MAP**, not the array of
    descriptors it used to write (which produced an unusable `mcp.json` env for any
    server needing secrets). Required-but-empty vars are surfaced in the install note.
- `/api/mcp/add` (`McpAddBody`, `web.rs`) + the dashboard install flow (`api.ts`,
  `App.tsx`) thread `env`/`headers` maps end-to-end (were dropped before).
  Verified live against the registry: docker (`apithreshold` → `docker run … -e … img:0.1.0`),
  pypi (`armor-mcp` → `uvx armor-mcp@0.6.1` + `env:{ARMOR_API_KEY:""}`), npm — all via the UI, 0 console errors.
- **The standard is now a machine-local default that forces AI adherence.** A distilled
  spec reference ships in the binary (`assets/specs/mcp-server.md`) and `8sync harness
  global`/`init`/`up` deploy it to **`~/.omp/specs/mcp-server.md`** (`ensure_mcp_spec`).
  A short rule in the always-on `APPEND_SYSTEM.md` points every omp session at it:
  when writing/editing `mcp.json`, follow the on-disk standard — `env`/`headers` are
  maps, runtime from `registryType`, pin `version` — never invent fields.

### Added — `/auto` engine: gitleaks gate before every autonomous commit
- `engine_advance {commit:true}` (`assets/extensions/8sync-engine.ts`) now runs a
  secret gate (`gitleaks protect --staged`, matching the 8sync pre-commit hook)
  before committing — a finding aborts the commit and unstages, so an unattended
  `/auto` run can't leak a secret when the pre-commit hook is absent. No-op when
  gitleaks isn't installed (best-effort, no regression).
- `/auto` reviewed + independently functional-tested (Bun harness): all 6 `engine_*`
  tools register; verify-gate FAIL→WARN(2×)→BLOCK(3× doom-loop); `engine_advance`
  refuses an unverified task; pass→advance→done; trivial no-verify advance; commit path.

### Added — plan: Agent Terminal App (deep-researched, build later)
- `agents/plans/agent-terminal-app.md` — Tauri v2 + xterm.js/WebGL + portable-pty +
  Zellij-backend (resurrection sau reboot) + omp sidecar. Positioning: "cmux for every OS"
  (cmux: 22.3k stars/4 tháng, macOS-only → khe cross-platform đang mở). Gồm stack đã chốt,
  4 phase MVP, trend mechanics, rủi ro, metrics, next actions + sources.

### Fixed — omp startup error `providers: must be an object (was null)`
- `~/.omp/agent/models.yml` was left with a bare `providers:` key (YAML null) when the
  local-model registry became empty (e.g. after `add-local-model rm <last>`), making omp
  print a schema error + disable custom providers on every start. `insert_block` now
  finalizes the file: no real children ⇒ `providers: {}` (valid empty object); a later
  add reopens `{}` and inserts under it. Both branches proven live via the real binary
  and A/B'd against omp (`omp models list`): bare ⇒ error, `{}` ⇒ clean.

### Added — `harness help`: LOCAL GGUF MODEL real-flow example block
- Copy-paste flow: add from .gguf path / HF repo id / URL → `list` → use once via
  `8sync ai --model local/<name>` → set as `default`/`code` model → `rm`. Points at the
  TSV registry + sentinel-managed provider block.

## [0.44.0] — 2026-07-05

### Added — loop-engineering stop signals in the 8sync engine (doom-loop guard + real gate)
- Per the loop-engineering literature (Avi Chawla, "Prompt, Context, Harness & Loop
  Engineering", Jul 2026): an agent's own "done" is not a stop signal. Two code-enforced
  fixes in `assets/extensions/8sync-engine.ts`:
- **`engine_advance` now actually enforces the gate** — it REFUSES a task that has verify
  commands but no passing `engine_verify` run (new `verified` flag per task; previously the
  description claimed "code-enforced" but `advance` set `done` unconditionally). Tasks with
  zero verify commands keep the documented trivial-advance path.
- **No-progress detector (doom-loop guard)** — `engine_verify` fingerprints each failure
  output (FNV-1a); 2 consecutive identical failures WARN ("change the approach"), 3 BLOCK
  the task early even below `maxRetries` (a byte-identical failure means the retry did
  nothing but burn tokens). New per-task `failStreak`/`lastFailureHash` state; old
  `state.json` files load via zod defaults (backward compatible).
- `/auto` command updated: different-fix-per-retry rule, advance-refusal note, and
  unattended runs now require a hard token ceiling (omp budget `+Nk!`) as the third stop
  signal (turn/token cap). Verified end-to-end in Bun: refuse-unverified, warn-at-2,
  block-at-3 (retries 3/10), pass→advance, trivial-advance, old-state load.

### Added — `8sync harness global`: omp rules machine-wide, one key (Anthropic token-optimized)
- **`8sync harness global`** — applies the omp rule layer MACHINE-WIDE so every project that
  runs omp gets it without a per-project run: `~/.omp/skills` + `00-force-load.md`,
  `~/.omp/agent/APPEND_SYSTEM.md` (appended to EVERY omp system prompt), MCP servers
  (codebase-memory · headroom · serena · zai-vision), recall hook, capabilities snapshot,
  workflow extension + engine. CWD-independent — never touches the current project.
- **Anthropic token-optimizer defaults**: `compaction.thresholdPercent = 50` written only when
  unset (never overrides the user), headroom compression for >50-line outputs, and byte-stable
  `APPEND_SYSTEM.md` deploys (identical ⇒ skip) so the system prefix stays hot for Anthropic
  prompt caching. New `compaction::ensure_threshold_default` helper.
- **`--sweep [DIR]`** (default `~/Projects`) — stamps the per-project layer into every **omp
  project** under DIR (a git repo with `agents/` or `AGENTS.md`/`CLAUDE.md` — repos not using
  omp are skipped + reported): mirror skills (additive), inject force-load into
  AGENTS.md/CLAUDE.md, seed agents/ memory, install the gitleaks hook. Skips
  `node_modules`/`target`/hidden dirs, depth ≤ 4, found repos are not descended into.
  `--pull` re-pulls registered skills first.
- Dedup: bare `8sync harness`'s global block now calls the shared `global::global_pass()`
  (`crates/cli/src/verbs/harness/global.rs`) — one source of truth for the machine-wide layer.
- **Overwrite policy made explicit** (default = NEVER overwrite, only add what's missing):
  documented in `8sync harness help` (new OVERWRITE POLICY section) + AGENTS.md §8 as a
  repo-wide invariant. Audited: agents/*.md seed-if-missing, CHANGELOG created once, skills
  mirror additive (`--force` only), AGENTS.md sentinel-block only, gitleaks hook only-if-absent,
  config key-detect. Proven live: hand edits to a mirrored SKILL.md + STATE.md survive a
  sweep re-run byte-for-byte.

## [0.43.0] — 2026-07-05

### Added — codegraph canvas capture (`?shot=1`) + automatic locate for non-vision models
- **`/codegraph?shot=1`** on the dashboard renders ONLY the React-Flow package call graph,
  full-viewport (no sidebar/cards) — made for `8sync shot 'http://127.0.0.1:8731/codegraph?shot=1'`:
  one big, legible graph image (~2k vision tokens) instead of a full-page capture. Everything
  else on that page stays text via `/api/codegraph/overview|search|trace` — image for the
  layout, API text for the details.
- **Auto-rule baked into the always-read layer** (`APPEND_SYSTEM.md` + codegraph/image-routing/
  locate-anything skills): a non-vision model (GLM-5.2) that needs positions/layout/distribution
  from an image uses **`8sync locate`** (LocateAnything-3B, on-device ggml, CPU or CUDA)
  automatically — zai-vision answers *what it says*, locate answers *where it is*.

### Changed — README + GitHub Pages are English-first (100%)
- `README.md` fully rewritten in English with the dashboard demo: hero (State page) + Bench /
  Codegraph / Models / Marketplace screenshots (`docs/assets/dashboard-*.png`, freshly captured,
  leak-checked). `docs/index.html` (Pages) translated to English with a Dashboard gallery.

### Changed — bench now DRIVES optimization (breakdown + spine advisory, CLI/API/web)
- `BenchMetrics` (`/api/bench`) exposes the upfront breakdown — `core_tok`, `spine_tok`,
  `naive_tok` — plus `spine_advice`: a warning set when the memory spine (`agents/*.md`)
  eats **more than half the upfront budget** (the single lever bench exposes; prefix +
  CORE are fixed by design, the spine grows unbounded between consolidations).
- CLI `8sync harness bench` prints the same advisory as a `!` line with the concrete
  fix (trim `agents/STATE.md` / let `8sync harness` auto-archive KNOWLEDGE >200 lines).
- Dashboard **Bench page rebuilt**: auto-loads on mount (was an empty state until a
  manual "Run bench" click — the compute is 40 ms and deterministic), upfront breakdown
  meters (prefix / CORE / spine share of upfront), advisory callout, naive-baseline row.
  New `.meter-val-wide` for long meter values. Browser-verified: 0 console errors.
- Warning sweep: removed dead `assets::web_asset_iter`, `LocalModel` → `pub(crate)`
  (private-interface warning), unused `ctx` in `api_state`. `cargo build` is warning-free.

### Added — `8sync harness add-local-model`: local GGUF models for omp (Rust runtime)
- New subcommand `8sync harness add-local-model <path> [name]` loads a **GGUF** model
  through **mistral.rs** (pure-Rust, memory-safe inference — no C++ `llama.cpp`) and
  registers the served OpenAI endpoint as an omp provider `local/<name>`, so omp routes
  to on-device models exactly like a cloud one (`8sync ai --model local/<name>`).
- `<path>` auto-classifies: an existing `*.gguf` FILE, a HuggingFace repo id (`org/repo`),
  or a `*.gguf` URL (downloaded to `~/.cache/8sync/models/`). GGUF-only this version
  (validated by magic bytes); other formats auto-detect later. GGUF chosen for speed.
- Runner is auto-installed via the official mistral.rs `install.sh` (prebuilt per-GPU
  CUDA or CPU binary — no Rust/CUDA toolkit needed, just the NVIDIA driver for GPU).
- Each model runs as a systemd **user** service `8sync-llm-<name>.service`. For a local
  `.gguf` the served command is `mistralrs serve --host 127.0.0.1 --no-ui --format gguf
  -m <dir> -f <file>` (mistral.rs `--model-id` is a *directory*, not a file); an HF repo
  goes straight to `-m`. Port auto-allocated from 8770. A TSV registry
  (`~/.config/8sync/local-models.tsv`) is the source of truth; the omp provider lives in
  a managed sentinel block inside `~/.omp/agent/models.yml` that **`gateway apply`
  preserves** across re-deploys. `list` / `rm <name>` manage the set.
- The provider sends model **`id: default`** upstream (the only alias mistral.rs serves
  besides the model-dir path); the clean `local/<name>` handle lives in the `name:` field
  (omp fuzzy-matches both), so `--model local/<name>` still selects it.
- `doctor` reports the registered local-model count; `~/.omp/capabilities.md` lists them
  so the agent knows they exist.
- **E2E-validated** on this machine: mistral.rs 0.8.23 (auto-selected the prebuilt
  `cuda131-sm120` RTX-5080 binary) serves a 135M GGUF; `add-local-model` → systemd unit →
  `/v1/chat/completions` returns real text; `rm` tears down unit + block + registry clean.

### Added — `8sync locate`: visual grounding (NVIDIA LocateAnything-3B)
- New verb `8sync locate <image> "<prompt>"` returns labeled **bounding boxes +
  click-center coordinates** from an image — GUI element grounding (click points),
  OCR/text localization, open-set detection. Grounding, not captioning: complements
  zai-vision (describe) and the `browser` tool (act). Pipeline: `8sync shot` → `locate`
  → click. `--annotated out.png`, `--mode hybrid|slow|fast`.
- Runs **NVIDIA LocateAnything-3B** through `mudler/locate-anything.cpp` (MIT C++/ggml
  port, prebuilt GGUFs, no Python). `--setup` clones + cmake-builds the CLI (CUDA if
  the toolkit is present, else CPU) and downloads the q8_0 GGUF (~6.3 GB) to
  `~/.cache/8sync/locate-anything/`. Model license: NVIDIA research / non-commercial.
- New **always-on specialist** skill `locate-anything` (`inject.rs::always_on_rank`,
  rank 8 after `image-routing`): force-loaded into the AGENTS.md block every session and
  re-deployed/refreshed on every `8sync harness` run (bundled). Body read on trigger
  (token-lean); an APPEND_SYSTEM pointer surfaces it when exact coordinates are needed.

## [0.42.0] — 2026-07-04

### Added — modality routing: read STRUCTURE as an image, PRECISE things as text
- **`8sync shot` is now real** (was a no-op stub) — renders any URL / local HTML to
  PNG via system or omp's bundled Chromium (`~/.omp/puppeteer/chrome/…`), prints a
  vision-token estimate (`ceil(w/28)*ceil(h/28)`, 28×28 patch — no stale cap). Also
  fixes the `image-routing` skill which already pointed at it.
- New **Modality routing** directive forced across the harness: `APPEND_SYSTEM.md`
  (always-on), the `image-routing` skill (rewritten with honest economics), and the
  `capabilities.md` snapshot, enforced per-turn by `--advisor`. Rule: vision models
  render a codegraph / diagram / dashboard / big PDF to ONE image (modality-fit); code /
  exact config / line-numbered data stay TEXT (cheaper AND lossless).
- Grounded, not hyped: the 10×/90% token cut (DeepSeek-OCR, arXiv 2510.18234) needs a
  DEDICATED optical encoder — NOT a screenshot to Opus/GLM. Claude bills images per
  28×28 patch (pay-per-pixel on Opus 4.7+). Measured on this repo: STATE.md as image =
  0.87× (LOSES vs text); the 12k-edge codegraph as image ≈ 25× (structure win). The
  gate captures exactly that. OCR-Memory pattern (arXiv 2604.26622) documented: image to
  LOCATE, exact text to READ.
- **Dashboard deep-link** — `web/src/App.tsx` now reads `?page=<id>` (or `/<id>`) for
  the initial page (`pageFromUrl()` + `history.replaceState` on nav). Nav was in-memory
  only, so `8sync shot .../codegraph` used to render State; now
  `8sync shot http://127.0.0.1:8731/?page=codegraph` captures the real graph. `build.rs`
  already rebuilds the Vite bundle on `web/src` change, so a plain `cargo build` re-embeds.

## [0.41.0] — 2026-07-03

### Added — dashboard `Marketplace`: discover + install skills & MCP servers
- New **Marketplace** nav page (Discover group) in `8sync harness web`: browse,
  search, sort (Top by stars/uses · New by recency), and one-click install
  skills and MCP servers from public registries into the current project.
- **MCP sources (4):** the official registry (`registry.modelcontextprotocol.io`,
  REST API), Smithery (`registry.smithery.ai`), Glama (`glama.ai` JSON API), and
  **mcp.so scraped with the pure-Rust `scraper` crate** (HTML DOM via
  `a[href^="/server/"]`, fetched through `curl` — no reqwest, Rust-first). 135+
  merged/deduped entries; install writes a real `~/.omp/agent/mcp.json` stdio
  (`npx`/`uvx`) or remote (`http`/`sse`) entry.
- **Skills source:** GitHub repo search ranked by stars; install shells the
  existing collection-aware `8sync skill add <url>`.
- Catalog cached under `.cache/8sync/marketplace/*.json` (1h TTL — the MCP
  registry maintainers ask aggregators to poll infrequently + persist).
  (`crates/cli/src/verbs/harness/marketplace.rs`, `web.rs`, `web/src/*`)

### Added — import buttons across the dashboard (were plumbing-only)
- **Skills page**: `skillAdd`/`skillUpdate` were wired in the API client but had
  no UI — added an **Import** toolbar (github URL · `gh:owner/repo` ·
  `path:/abs/dir` folder · `builtin:name`) + **Update all**.
- **MCP page**: **Install-from-link** (`npx -y pkg`, `uvx pkg`, or an https
  remote URL → merged into `mcp.json`) + a per-server **Remove**.
- **Rules page**: **Import from a folder or GitHub repo** (`.md`/`.mdc`,
  recursively; prefers a `rules/`/`.cursor/rules`/… subdir), shallow-cloned to a
  RAII temp dir. Complements the existing inline text-add.
- New routes: `/api/marketplace`, `/api/mcp/{add,remove}`, `/api/rules/import`.
- New dep: `scraper 0.20` (pure-Rust html5ever + CSS selectors) for the mcp.so
  aggregator — HTTP still shells out to `curl`.

## [0.40.0] — 2026-07-03

### Changed — advisor default-ON (per-turn rule/tool-use reviewer)
- omp's `--advisor` (passive per-turn reviewer that checks each turn against the
  always-on rules — code-intel first, correct MCP tool names, open SKILL.md — and
  injects corrective notes) is now passed **by default** by `8sync ai` and the
  `8sync .` / resume session. Closes the last anti-forget gap: layers 1-4
  (APPEND_SYSTEM rules, recall-hook live context, Mnemopi memory, capabilities
  catalog) *declare + remind* but nothing *checked* whether the last turn actually
  obeyed — advisor is that live reviewer.
- Token-optimal gating: skipped for `trivial`-class prompts. Opt out per run with
  `8sync ai --no-advisor`, or globally via `advisor = false` in
  `~/.config/8sync/models.toml`. New `advisor` key in `ModelConfig` (default true).
- Docs: `models.toml`, `APPEND_SYSTEM.md`, and `8sync ai --help` document the
  toggle + tradeoff.

### Fixed — `8sync doctor` self-heals stale profile state
- `profile::mark_applied()` was append-only — a profile deleted from the repo
  (e.g. `caelestia.toml`, removed in `e761c31`) stayed in `~/.config/8sync/profile.toml`'s
  `applied` list forever, and `doctor` printed it back as a false positive. New
  `profile::prune_stale()` diffs `applied` against `load_all()`, drops entries that no
  longer resolve, rewrites state only if changed. Wired into `doctor` (warns once, then
  clean). Verified against a real stale state.

### Docs — README + GitHub Pages refreshed to current surface
- README + `docs/index.html`: TL;DR now leads with the one-liner install →
  `8sync harness` → `8sync harness web`; new **Dashboard** section (with screenshots)
  documents the CRUD control surface (models/skills/memory/rules/engines/Codegraph);
  full harness subcommand table (web/gateway/bench/audit/eval/toolstats); added the
  machine verbs (`bt`/`clean`/`theme`/`bg`) that were missing. Fixed stale numbers
  (binary ≈ 5.0 MB, **35** bundled skill). Landing page gains a Dashboard nav link +
  feature card + two live screenshots (`docs/assets/`).

## [0.39.0] — 2026-07-02

### Added — dashboard `Codegraph` page: visualize the codebase-memory-mcp knowledge graph
- The web dashboard (`8sync harness web`) had zero visibility into the codegraph/
  codebase-memory-mcp engines it lists on the Engines page — `search_graph`/
  `trace_path`/`get_architecture` were agent-only. New **Codegraph** nav item
  (Runtime group) renders the real graph: package call graph (elk auto-layout,
  box size ≈ node count, edges = call counts between packages), **Leiden
  cluster cards** (de-facto modules, cohesion %, top symbols — the actual
  architectural seams, not just folders), a BM25 **symbol search**, and a
  **caller/callee trace subgraph** for the selected symbol or a fan-in hotspot.
- Backend: 3 new routes (`/api/codegraph/{overview,search,trace}`) shell out to
  `codebase-memory-mcp cli <tool> <json>` (same binary+slug `harness up`
  already indexes against) — no MCP client embedded, stdout-only JSON parsing
  verified log-noise-free. Honest 404 ("not indexed yet — run `harness up`")
  when the project has no graph.
  (`crates/cli/src/verbs/harness/web.rs`, `web/src/App.tsx`, `web/src/api.ts`)

### Fixed — dashboard UI/UX audit (browser-verified against a real project)
- **Engines page**: the `codebase-memory-mcp 0.8.1` tile title rendered
  **one character per line** — `overflow-wrap: anywhere` collapsed the flex
  item's intrinsic width to zero next to a wide version tag. Fixed `.tile-head`
  to give the title `flex: 1 1 auto; min-width: 0` and wrap on word
  boundaries instead of mid-word.
- **Version tags** were inconsistent/redundant across tools (`on
  codebase-memory-mcp 0.8.1`, `on headroom, version 0.27.0` — duplicating the
  already-visible tool name). `api_engines` now extracts just the semver token.
- **Skills page** (67 skills, no way to find one): added a filter input +
  tier dropdown (`all/always/on-demand/off`) with a live "N of M" count.

### Added — `8sync harness gateway` — deploy/verify the omp model-gateway
- New subaction: `8sync harness gateway [apply|key <KEY>|verify|status]` — deploys
  `~/.omp/agent/models.yml` from a bundled template so the 9router gateway config
  (provider URL, models, API key, `thinking.mode = anthropic-budget-effort`) is
  reproducible by one command instead of hand-editing.
- `apply` is idempotent (backs up a differing file to `models.yml.bak`, preserves the
  existing key on refresh; key from `$NINE_ROUTER_KEY` or `gateway key <KEY>`).
- `verify` pings `cc/claude-sonnet-5` through the gateway — the exact path that 400'd
  before the thinking fix; HTTP 200 = healthy. `status` masks the key + flags a missing fix.
- Fixes recurring `400 thinking.enabled.budget_tokens: Field required` on claude-sonnet-5:
  omp's default `thinking:{type:adaptive}` is rejected by the gateway; the bundled template
  forces `enabled + budget_tokens`. (`crates/cli/src/verbs/harness/gateway.rs`, `assets/configs/omp/gateway-models.yml`)
### Added — 18 feynman research skills ported to omp-native (were unusable stubs)
- Audited `agents/skills.toml`'s 20 `companion-inc/feynman`-sourced skills
  (submodule-inspected at `reference/feynman`, then removed). Found 12 were
  12-line stubs pointing at feynman's OWN slash-commands (`/deepresearch`,
  `/lit`, `/recipe`, `/audit`, `/draft`, `/review`, `/compare`, `/watch`,
  `/replicate`, `/jobs`, `/log`, `/autoresearch`) — those commands only exist
  in feynman's own pi-coding-agent runtime (`prompts/*.md` +
  `extensions/research-tools.ts`), NOT in omp. Deployed as-is they were
  completely inert. 2 more (`session-search`, `preview`) had the same
  problem behind a documented fallback. Ported all 14 (deep-research,
  literature-review, autoresearch, ml-training-recipe, paper-code-audit,
  paper-writing, research-review, source-comparison, watch, replication,
  jobs, session-log, session-search, preview) into self-contained
  `assets/skills/<name>/SKILL.md` using omp's real tools (`task` in place of
  feynman's `subagent`, `web_search`/`read` in place of `fetch_content`,
  `ask` in place of `ask_user_question`, `job`/`retain` where feynman had no
  equivalent). Also re-bundled 4 genuinely-portable CLI skills (`eli5`,
  `docker`, `modal-compute`, `runpod-compute` — only cosmetic "Feynman"
  naming, no runtime dependency) as `builtin:` too. `alpha-research` is kept
  pointed at the real `companion-inc/feynman` source since it's a legitimate
  CLI wrapper (`feynman alpha ...`, needs `@companion-ai/feynman` installed
  via the existing `ensure_feynman_cli()`). Dropped `contributing`
  (feynman-repo-only, no value for su-code users).
- **Bug fixed in the process**: `agents/skills.toml` had `[peer-review]`
  pointing at feynman, but feynman renamed that skill upstream to
  `research-review` — the entry never resolved to anything on disk. Fixed
  to `[research-review]`.
- **Bug fixed in `update_skills` (`crates/cli/src/verbs/skill/update.rs`)**:
  registering ANY single skill from a git collection repo (e.g. just
  `alpha-research` from feynman's 20-skill repo) silently reinstalled
  EVERY sub-skill in that repo on every `8sync harness`/`skill update` run,
  regardless of registry membership — `contributing` kept reappearing after
  being deliberately dropped from the manifest, because the git-source loop
  treated `filter.is_none()` as "install everything found". Fixed: a
  sub-skill is only (re)installed when the URL/repo was explicitly targeted,
  the skill name was explicitly filtered, or (bulk run) it already has its
  own registry key — a collection repo no longer silently grows the
  registry.
- 18 new on-demand skills registered in `assets/skills/00-force-load.md`'s
  lookup table (55 on-demand total, up from 37).

### Fixed — kitty terminal zoom (`ctrl+shift+minus`) silently stolen by vsplit binding
- `8sync setup --profile terminal` mapped `ctrl+shift+minus` to
  `launch --location=vsplit` for the gsd-style 3-pane layout. That's kitty's
  DEFAULT font-zoom-out binding (`change_font_size all -2.0`) — user maps
  override defaults, so zoom-out silently stopped working with no error.
  Moved vsplit to `ctrl+shift+backslash` (unclaimed by any kitty default);
  `ctrl+shift+minus`/`+equal`/`+backspace` now behave stock. Re-ran
  `8sync setup --profile terminal` to regenerate the live
  `~/.config/kitty/8sync.conf` on this machine — user must reload/reopen
  kitty (font-zoom maps apply live via `kitty @ load-config` if remote
  control is on, no window-recreate needed unlike the earlier decoration fix).
  (`crates/cli/src/verbs/setup.rs:665-668`)

### Added — `~/.omp/capabilities.md` now embeds EXACT MCP/builtin/memory tool catalogs
- Previously the snapshot only said "`4` server(s) registered" — no tool names.
  Agents had to guess, which is exactly how the earlier "codegraph verb"
  hallucination bug happened (see KNOWLEDGE.md). Now `8sync harness` writes
  the FULL exact tool catalog for every registered MCP server:
  `codebase-memory-mcp` (14), `headroom` (3), `serena` (23), `zai-vision` (8)
  — plus omp's own built-in tools (parsed live from `omp --help`'s "Available
  Tools" block) and the Mnemopi memory tools (`recall`/`reflect`/`retain`/
  `memory_edit`, listed only when the backend is ON).
  (`crates/cli/src/verbs/skill/deploy.rs::known_mcp_tool_catalog` +
  `ensure_omp_capabilities_snapshot` rewrite.)
- `APPEND_SYSTEM.md` RULE #0 now names the 4 connected servers explicitly and
  points at `~/.omp/capabilities.md` as the ground truth for exact
  names/params — "never guess/invent an MCP tool name". The `8sync-recall.ts`
  hook (injected every `before_agent_start` + compaction) carries the same
  pointer so it survives past 50% context.

### Fixed — kitty lost its title bar/min-max-close/resize border on KDE (stacking WM)
- `8sync setup --profile terminal` unconditionally wrote
  `hide_window_decorations yes` into `~/.config/kitty/8sync.conf`. That's
  correct on a tiling Wayland compositor (Hyprland/HyDE — the project's
  primary target) which draws no chrome and expects clients to hide their own,
  but on a **stacking** desktop (KDE/kwin, GNOME/mutter, …) the compositor
  ALSO does not add server-side decorations for an undecorated client — the
  window ends up with no title bar, no traffic-light buttons, and no
  drag-to-resize border at all.
- New `env_detect::is_tiling_wm()` checks `is_hyde()` first, then
  `XDG_CURRENT_DESKTOP`/`DESKTOP_SESSION` against known tiling WMs
  (hyprland/sway/river/wayfire/qtile/i3/bspwm/awesome).
  `render_kitty_conf` now only emits `hide_window_decorations yes` when
  `is_tiling_wm()` is true; stacking desktops (verified live on KDE/Plasma/
  kwin/Wayland) keep normal window chrome. (`crates/cli/src/verbs/setup.rs`,
  `crates/cli/src/env_detect.rs`)
- Re-running `8sync setup --profile terminal` (idempotent) regenerates
  `~/.config/kitty/8sync.conf` with the fix; requires closing/reopening the
  kitty window (decorations are negotiated at window-creation time, not
  live-reloadable).

### Added — Z.AI vision MCP (`zai-vision`) bridges GLM-5.2's text-only gap + dedicated skill
- GLM-5.2 (omp's default model) is text-only. `8sync harness` now auto-installs
  `@z_ai/mcp-server` (npm, via `bun add -g`) and registers it as the `zai-vision`
  omp MCP server, exposing 8 GLM-5V tools (`ui_to_artifact`,
  `extract_text_from_screenshot`, `diagnose_error_screenshot`,
  `understand_technical_diagram`, `analyze_data_visualization`, `ui_diff_check`,
  `analyze_image`, `analyze_video`). Auth reuses the SAME Z.AI key already
  configured for `zai/glm-5.2` (pulled via `omp token zai`, no separate signup).
  (`crates/cli/src/verbs/skill/deploy.rs::ensure_zai_vision_mcp` +
  `resolve_zai_api_key`; wired into `harness auto`/`harness init`; reported by
  `doctor`.)
- **`register_omp_mcp` now supports per-server `env`** (only emitted when
  non-empty, so existing env-less entries stay self-heal-stable).
- **Verified end-to-end** (not illustrative): real browser screenshots run
  through the actual `zai-mcp-server` stdio process via JSON-RPC `tools/call`.
  Found and fixed a real gap — `8sync harness` now defaults
  `Z_AI_VISION_MODEL=glm-4.6v-flash`, the ONLY vision model that works on a
  stock Z.AI key with no vision resource package (paid models 400 with `1113
  insufficient balance`; verified against Z.AI's live pricing table).
- **New skill `zai-vision`** (`assets/skills/zai-vision/SKILL.md`, auto-deployed
  by `install_bundled_global`) documents the full combination matrix: browser
  screenshots, `8sync shot/pdf-img/diff-img`, codegraph/cbm diagrams, serena,
  headroom compression, `inspect_image` fallback, retain/recall, and advisor —
  plus the real verified tool-call output and a troubleshooting table for Z.AI
  error codes (1113/1211/1301/1305).
- **`~/.omp/capabilities.md`** — new live snapshot of omp's surface (advisor,
  thinking, inspect_image, adaptive model roles, retain/recall, registered MCP
  count, skill count), refreshed every `8sync harness` run, surfaced by
  `doctor` (`ensure_omp_capabilities_snapshot`).
- `APPEND_SYSTEM.md` and `image-routing` SKILL now point to `zai-vision` as the
  mandatory bridge step after routing to "image".

## [0.36.0] — 2026-06-30

### Added — `8sync bg search`: find wallpapers online (no API key) + pick with live preview
- New **`8sync bg search <query>`** sub-action. Searches **wallhaven.cc** via its public API
  (**no API key needed**, SFW, ≥1920×1080) — wallpaper-focused (incl. anime/dark), a good fit for
  the project's aesthetic without imposing an Unsplash/Pexels registration on the user.
- **Interactive (kitty)**: stages thumbnails, then opens `fzf` with a **live `kitten icat` preview
  pane** showing each candidate + its wallhaven **source link**. Enter downloads the full-res image,
  adds it to the collection, and sets it live; Esc cancels. Only the full image you pick is fetched.
- **Non-interactive**: prints the result list (id + resolution + source link) for scripting/agents.
- RAII temp cleanup; reuses the existing `add`+`set` path. No new Rust deps.
  (`crates/cli/src/verbs/bg.rs`)

## [0.35.0] — 2026-06-30

### Added — `8sync bg`: manage the kitty wallpaper at runtime (live swap + inline preview)
- New **`8sync bg`** verb: `show | get | set [file] | list | add <url|file>`. Brings back the
  wallpaper control that was removed in the slim-down — now without HyDE overlap (kitty's
  in-terminal `background_image` ≠ HyDE's desktop wallpaper).
- **Inline preview**: `bg show` renders the current wallpaper in the terminal via `kitten icat`
  (kitty graphics protocol — same mechanism omp uses); `bg list`/`bg set` (no arg) open an
  interactive **fzf picker with a live `kitten icat` preview pane** → scroll, see each image,
  Enter to set.
- **Live swap**: `bg set <file>` rewrites the `background_image` line in `8sync.conf` +
  SIGUSR1-reloads kitty (instant, no restart). The choice is recorded in
  `~/.config/8sync/wallpaper` and **`8sync setup` honors it** (re-setup no longer resets your
  wallpaper). Collection lives in `~/.config/8sync/wallpapers/` (`bg add <url>` populates it).
- Zero new Rust deps (shell-outs to `kitten`/`fzf`/`curl`) — binary stays lean.
  (`crates/cli/src/verbs/bg.rs`)

## [0.34.0] — 2026-06-30

### Added — `8sync theme`: switch kitty palettes live (readable on any wallpaper)
- New **`8sync theme`** verb: `list | set <name> | show [name]`. Six curated dark palettes
  (**tokyo-night** default · catppuccin-mocha · gruvbox-dark · nord · rose-pine · dracula), each
  a pure color fragment tuned for **wallpaper-overlay readability** (foreground + bright-black
  verified at WCAG-AA contrast ≥ 4.5:1 against the theme bg). Switching writes
  `~/.config/kitty/8sync-theme.conf` and **SIGUSR1-reloads kitty** — instant, no restart, no
  remote-control socket. `hydectl theme` still owns Hyprland/UI; this owns kitty (distinct surfaces).
  (`crates/cli/src/verbs/theme.rs`)

### Fixed — kitty config: readable text + restored `allow_remote_control` + structure/palette split
- **Readability root-cause**: deployed `8sync.conf` had `background_tint 0.55` (image 45% visible →
  bright wallpaper washed out the foreground). Raised to **0.86** (image subtle, text crisp).
- The glass **structure** (`background_opacity`/`blur`/font/splits/tabs) is now separated from the
  **palette** (`8sync-theme.conf`, swappable); `render_kitty_conf` no longer emits colors inline.
- **Restored `allow_remote_control yes`** in the managed config — it had been dropped in the
  slim-down, breaking `kitty @` live control. (`crates/cli/src/verbs/setup.rs`)
- `8sync setup --profile terminal` now deploys both files (structure + active palette); the active
  theme is recorded in `~/.config/8sync/kitty-theme` and survives re-runs.
## [0.33.0] — 2026-06-29

### Added — dashboard surfaces the live `/auto` engine run (real, not demo)
- New `/api/engine` reads the **real** gsd-pi state machine the engine drives at
  `<root>/.cache/8sync/engine/state.json`; the Engines page renders a live board — goal · progress
  bar · slice/task tree with ✓/▸/○/✗ status + retries · current task (4 s refresh, read-only mirror of
  the terminal board). Closes the gap where the dashboard showed the workflow *editor* + engine
  *binaries* but never the actual `/auto` run. `{active:false}` when none. Browser-verified, 0 console
  errors. (`crates/cli/src/verbs/harness/web.rs`, `web/src/{api.ts,App.tsx}`)

### Added — AFFiNE in the `alexdev` profile
- `affine-bin` — official prebuilt of the open-source Community Edition (AGPL/custom: free, self-hostable,
  no cloud lock-in). The from-source `affine` AUR pkg fails upstream (electron-packager zip step), so the
  prebuilt is used. (`assets/profiles/alexdev.toml`)

### Changed — always-on directives also prime recall/retain + browser
- `APPEND_SYSTEM.md` (every system prompt, never compacted) + the recall hook now explicitly prime
  **`recall`/`reflect` before · `retain` durable facts after** (Mnemopi) and **`browser` to verify any
  web/UI change for real** — on top of RULE #0 (code-intel MCPs) + skill ref-paths. Stays terse by design
  (the system prompt isn't headroom-compressed; headroom is for tool OUTPUTS).
  (`assets/configs/omp/APPEND_SYSTEM.md`, `assets/hooks/8sync-recall.ts`)
- **kitty tab bar moved to the bottom** (`tab_bar_edge bottom`) — easier tab switching. (`setup.rs` renderer)

### Fixed — `8sync harness up` now redeploys the recall hook
- `harness up` refreshed APPEND_SYSTEM/engine/workflow but not the recall hook (only init/bare-harness
  did), so hook changes never reached existing machines via `up`. Now it does.
  (`crates/cli/src/verbs/harness/up.rs`)

## [0.32.1] — 2026-06-29

### Fixed — `8sync harness` auto-installs the token-optimization MCPs (no startup error)

- `headroom` (and `serena`) were **registered in `~/.omp/agent/mcp.json` even when their binary
  wasn't installed** — so omp failed at startup with `Executable not found in $PATH: "headroom"`.
  Now `8sync harness` **bootstraps `uv`** (Astral, user-level curl install — no sudo), installs
  `headroom-ai[mcp]` through it, and **only registers an MCP whose executable actually exists** —
  a still-missing tool has its stale entry **purged** so omp never errors at startup. `uv` also
  ships the `uvx` serena needs, so both engines come up from one `8sync harness`, no manual steps.
  (`crates/cli/src/verbs/skill/deploy.rs`)

## [0.32.0] — 2026-06-29

### Performance — binary back under control (offsets bundled rusqlite)

- Enabled rust-embed's `compression` feature (transparent `include-flate` decompress on `.data` — both the
  `assets/` skills tree and the embedded `web/dist` FE shrink) and set the release profile to `opt-level = "z"`.
  Roughly halves the binary, offsetting the bundled `rusqlite` (toolstats) + impeccable + the Vite FE.
  (`crates/cli/Cargo.toml`, `Cargo.toml`)

### Fixed — wallpaper self-heal (no more kitty "render to RGB: EOF")

- `setup::deploy_wallpaper` trusted `exists()`, so a transient/blocked download left a **0-byte
  `wallpaper.png`** kitty can't render (blank background) — and the early `exists()` return meant it never
  re-tried. Now validates the file (size>0 + PNG/JPEG/WEBP magic via `is_valid_image`), adds a `Mozilla/5.0`
  UA + `--retry 2`, and **purges a corrupt file** so a re-run re-downloads. (`crates/cli/src/verbs/setup.rs`)

## [0.31.1] — 2026-06-29

### Changed — `toolstats` now reports the *actionable* ratio
- The headline is now **optimizer vs raw-search of code-lookup calls only** (optimizer = codegraph /
  cbm / serena; raw-search = grep / search / find / glob) — instead of "% of all calls", which was
  misleading (most calls are edit / bash / read-before-edit, not lookups). `read` is shown separately
  (often legitimate, not shamed) and `headroom` is labelled background/auto-compress (not an
  agent-called tool). Measured: su-code optimizer **34%** of lookups, agentic-cloudgo **25%** — vs the
  old "2% of all calls" framing. The DB is rebuilt from current sessions each run (re-categorizes).

## [0.31.0] — 2026-06-29

### Added — `8sync harness toolstats` (SQLite tool-call tracker)
- New verb that tracks how the agent **actually** uses tools, parsed from omp's own session
  JSONL, into a per-project SQLite DB (`.cache/8sync/toolstats.db`, gitignored). Reports the
  **optimizer** (codegraph / codebase-memory-mcp / serena / headroom) vs **fallback** (grep / read /
  search / find / glob) call ratio + per-tool failures, so you can see whether the STEP-0
  token-optimization stack is being used and catch failing calls (e.g. a dead MCP server).
- Idempotent (keyed on session+seq → re-run only adds new calls); inspectable with any SQLite tool.
- Motivation: across this machine's 68 omp sessions / 28k calls, the optimizer stack was **1.1%**
  of calls (serena/headroom **0**) vs **35%** raw fallback — `toolstats` makes that visible per project.

## [0.30.0] — 2026-06-29

### Changed — default `8sync setup` is AI-core only
- **Stage A now installs only the AI coding harness**: omp, codegraph, MCP servers + skills,
  github-cli, paru, PATH bootstrap, configs. The terminal/editor polish (kitty glass theme + helix
  + JetBrains Nerd font + wallpaper) is **no longer installed by default** — a fresh `8sync setup`
  is pure AI now.
- **New opt-in `terminal` stack**: `8sync setup --profile terminal` (also offered in the y/N menu
  and applied by `--full`). `docker` moved out of the terminal stack — it lives in `dev-stack`.
- **`doctor`** reports the terminal stack (kitty/helix/docker) as advisory/opt-in — no longer warns
  when it's absent.
- Personal/hardware profiles (vietnamese/unikey, warp, hardware-*, displaylink, …) stay opt-in as
  before. Nothing personal is installed unless you pick it.

## [0.29.3] — 2026-06-29

### Fixed — serena MCP "Transport closed"
- **serena's executable was renamed.** The registered command `uvx … serena-mcp-server` no longer
  exists (serena now ships `serena` with a `start-mcp-server` subcommand), so the MCP process exited
  instantly → omp reported `serena: Transport closed`. Now registers
  `uvx … serena start-mcp-server --context claude-code` (`ide-assistant` was also deprecated). Verified
  it launches (22 tools exposed, no error).
- **MCP registration now self-heals.** `register_omp_mcp` previously skipped any server already in
  `mcp.json`, so a stale entry never got corrected. It now updates in place when the command/args
  changed, and **`8sync harness up` also refreshes MCP servers** (was init/bare-harness only) — so
  `8sync harness up` fixes the stale serena entry on existing machines.

## [0.29.2] — 2026-06-29

### Fixed — Context page is now correct for ALL models (not just GLM)
- **Per-model context window.** `/api/context` hardcoded a 1,000,000-token window, so models with
  a smaller real window (e.g. `claude-haiku-4-5` 200k, `glm-4.x` 131–205k) showed an artificially
  low % and looked like they never hit the compaction threshold — while 1M models (glm-5.2,
  claude-opus) looked fine. Now the window is parsed per active model from `omp models` (cached via
  `LazyLock`), so the %, threshold marker, and "will compact" are accurate for every model. Falls
  back to an explicit `assumed` estimate only when the model isn't in omp's catalog.
- **Honest compaction copy.** omp's threshold compaction is **turn-triggered** (fires after a
  completed turn / safe mid-turn point once usage exceeds `thresholdPercent` of the real window) —
  not a hard cap, so a paused/ended session legitimately sits above the line until resumed. The page
  now says "compacts on next turn", flags idle/ended sessions (`stale`), surfaces the explanation,
  and only shows the "assumed window" badge when the window is truly unknown.
- **`build.rs` shipped stale FE.** It rebuilt the Vite bundle only when `web/dist` was *missing* and
  watched only `web/dist`, so edits to `web/src` were silently embedded stale. Now it rebuilds when
  any FE source is newer than dist and emits `rerun-if-changed` for `web/src` + configs.

## [0.29.1] — 2026-06-29

### Fixed — dashboard project switcher
- **Switching projects now actually switches the data.** `activate` only wrote an advisory
  `web-session.json`; every handler still read `detect_current_project_root()` (the launch cwd),
  so pages never changed. Now `apply_active_project` chdir's into the activated project (at startup
  + on activate) so all cwd-based handlers (State/Context/Skills/Memory/Rules/Submodules/Workflow)
  resolve to it. Verified in-browser: switch → State path + content + trigger label all update.
- **`/api/projects` cleanup** — dedup by resolved path; drop junk slugs (no session file / non-dir);
  widened the green-dot "active" window to 2h + added a `current` flag for the project being viewed
  (a project open but idle >30 min now shows correctly).

## [0.29.0] — 2026-06-29

### Added — `8sync harness web` dashboard: full redesign + Models/Projects
- **Models page** (`/api/models` get+post) — view/edit the adaptive model routing live: `[roles]`
  (default/plan/smol/slow) + `[tasks]` (plan/review/debug/code/trivial), inline selects write
  `~/.config/8sync/models.toml` immediately. Surfaces the routing philosophy: **thinking → Opus**
  (plan/review/debug/vision), **mechanical → GLM** (code/edit/default/trivial).
- **Project switcher** (`/api/projects`) — sidebar-top dropdown lists every omp project with a
  green (active) / gray (off) status dot; activate + refresh without `cd`.
- **Workflow templates** (`/api/workflows/templates`) — 3 starter graphs (research→plan→build,
  review, qa) loadable in the editor.
- **Markdown rendering** — new XSS-safe renderer (`web/src/markdown.tsx`); State/Memory/Context
  now render headings, lists, GFM checkboxes, code, emphasis (was raw text).

### Fixed
- **serena engine showed "off" wrongly** — detection now checks `mcpServers.serena` in
  `~/.omp/agent/mcp.json` + `uvx`/`uv` on PATH (serena is uvx-launched, no PATH binary), not
  `which serena`. Reports `{present,registered,runner}`.
- **Context window honesty** — `/api/context` now exposes `assumed:true`, `windowTok`,
  `thresholdPct`, `willCompact`; the FE labels the 1M window as an estimate (no false precision).
- **Workflow canvas** — react-flow viewport fixed (was a tiny broken box) to a usable 560px panel
  with fit/zoom.

### Changed
- Dashboard FE redesigned to a product-register design system (impeccable): solid surfaces,
  violet brand preserved, legible chips, grouped nav, dark + light. 14 pages, zero console errors.

## [0.28.0] — 2026-06-29

### Changed — ONE command: `/auto` (retired `/gs`)
- **Unified the autonomous entry to a single `/auto`** — removed `/gs` (command + skill +
  `ensure_gs_command` + all wiring + help/force-load refs). `/auto` (8sync-engine) is the only
  automation path; `deploy::cleanup_legacy_gs` removes the retired `/gs` from machines that had it.
- **`/auto` upgraded to gsd-pi-grade** (grounded in `reference/gsd-pi`): research INTEGRATED into
  planning (codegraph/cbm/serena scout + feynman/deep-research), fresh scoped context per task,
  mechanical verify-gate, hard **Closeout** (full suite + QA/UAT in a browser + independent re-review
  vs DoD + doc-hygiene), and a context-budget/handoff rule.
- **Verify UI for real**: web → `browser` at the dev URL; **Tauri v2 / WRY-WebKit desktop → run with
  its web-inspector/remote-debug port + point the same `browser` tool at it**.
- **`harness up` now deploys the full harness** (APPEND_SYSTEM + `/auto` engine + workflow), matching
  bare `8sync harness`.

### Added
- **`8sync harness model`** — view/edit `~/.config/8sync/models.toml` (single model-routing source):
  bare shows roles+tasks; `8sync harness model <key> <value>` sets one (e.g. `review opus`). omp
  resolves names fuzzily + falls back to an authenticated model.

## [0.27.0] — 2026-06-29

### Added — adaptive model routing

- **Per-prompt model selection** (no more single fixed model). `assets/configs/models.toml`
  (deployed → `~/.config/8sync/models.toml`) maps `[roles]` default/plan/smol/slow + `[tasks]`
  plan/review/debug/code/trivial → models (defaults: codex main · glm plan · opus review/debug ·
  haiku smol). New `crate::models` classifies the prompt heuristically and passes omp
  `--model/--plan/--smol/--slow` (omp resolves fuzzy: "glm","codex","opus"). Wired into
  `8sync ai` (+`--model` override) and `8sync .` (resume flags). omp owns the catalog — 8sync only steers.

### Added — gsd-pi-style automation engine (on omp core, no patch)

- **`8sync-engine` omp extension** (`~/.omp/agent/extensions/` + project) — durable slice/task
  state machine (`.cache/8sync/engine/state.json`) + model-callable tools `engine_plan/status/
  next/verify/advance/worktree`. **Code-enforced** verify-with-retry gate (counts attempts,
  BLOCKs at maxRetries — the agent can't skip it) and git worktree open/squash-merge/remove.
- **`/auto` command** orchestrates the engine to run a goal to DONE (right-sized, token-lean).
  Closes the gsd-pi gaps (verify/worktree as CODE, not prose). `/gs` stays a lighter skill.

### Added — context engineering (always-read + serena + tunable compaction)

- **`APPEND_SYSTEM.md`** deployed to `~/.omp/agent/` — RULE #0 (code-intel before grep/CRUD) +
  always-on skill manifest (name·purpose·ref-path) injected into EVERY system prompt (never
  compacts away) → fixes "skills/rules defined but ignored past 50%". Recall hook rewritten to
  the LIVE half (skill index + STATE Current/Next).
- **serena MCP** registered (`ensure_serena_mcp`, via `uvx`) — symbol-level code intel, prioritized
  over native search/file-CRUD. Surfaced on the dashboard Engines page + force-load RULE #0.
- **`8sync harness compaction [pct]`** — view/set `compaction.thresholdPercent` (auto-clean at 50%).

### Added — terminal: kitty glass + helix + docker (Stage A defaults)

- `8sync setup` now installs **kitty + helix + docker + docker-compose + JetBrains Nerd font** and
  deploys a **glass kitty theme** (`~/.config/kitty/8sync.conf`, included from kitty.conf — no clobber):
  transparency + blur + wallpaper + 3-pane split keymaps + violet accent. Wallpaper deployed to
  `~/.config/8sync/wallpaper.png` from `assets/wallpapers/default.png` (bundled) or `[ui].wallpaper_url`.
  Transparent helix config (`base16_transparent`) deployed if absent. `8sync doctor` checks hx/kitty/docker.

### Changed / Fixed

- **Web dashboard redesigned** to a dark glassmorphism / Hyprland aesthetic (translucent blurred panels,
  layered gradient, icon sidebar, refined type scale, light-mode + a11y fallbacks). 13 pages, react-flow
  workflow editor intact. Browser-verified: all pages render, zero console errors. (`web/src/{styles.css,App.tsx,icons.tsx}`)
- **`build.rs`** now builds the FE with bun → pnpm → npm (first available); on no toolchain it embeds a
  styled, instructive fallback page instead of a bare one-line stub.
- **Helix command fixed to `hx`** — dropped the dead `"helix"` fallback (Arch ships `/usr/bin/hx`, no
  `helix` binary); `note`/`find` now share one `pick_editor()` ($VISUAL→$EDITOR→hx→vi).

## [0.26.0] — 2026-06-27

### Added (dashboard FE enhancement)

- **Context tracker page** — live omp session token usage for the current project (reads the
  session JSONL's `contextSnapshot.promptTokens`, auto-refresh 4s). Gauge + 50% threshold marker +
  **compaction-observed badge** (detects the token drop = empirical proof auto-compact fired). `/api/context`.
  Verified real: tracks this very session 440k→447k live; detected last compact at 575k.
- **MCP servers page** — visualize `~/.omp/agent/mcp.json` (name/command/args/present). `/api/mcp`.
- **Rules CRUD page** — list/add/delete omp rule files (`.omp/rules/*` project + `~/.omp/agent/rules/*`
  global), add from pasted content (link/file/folder source). `/api/rules` (+add/delete).
- Dashboard now 12 pages (State · Context · Skills · Memory · Engines · Bench · Readiness · Workspaces ·
  Team · Submodules · MCP · Rules). Anti-slop per impeccable (no gradient text / glassmorphism / over-round;
  verb+object buttons). Browser-tested (Chromium): all pages render real data, Context live-tracking +
  Rules add-end-to-end verified.


## [0.25.0] — 2026-06-27

### Added (Phase A — anti-forget)

- **Anti-forget: compaction@50% + idle + recall hook.** `8sync harness` giờ ensure
  `~/.omp/agent/config.yml` có `compaction.thresholdPercent: 50` + `idleEnabled: true`
  (snapcompact vẫn là default), và deploy `~/.omp/hooks/pre/8sync-recall.ts` — hook inject
  lean ref bundle (skill index + live STATE) tại mỗi `before_agent_start` + vào mọi
  compaction summary → agent giữ index skills/rules/workflow qua 50% context & sau compact.
  `8sync doctor` báo "anti-forget ON/OFF". Key-based config detection (robust khi omp
  rewrite/strip comments config.yml — bỏ sentinel strategy). Verified: omp 16.2.1 load OK.

### Added (Phase B — `8sync harness web`)

- **`8sync harness web`** — dashboard Vite+React (embedded qua rust-embed) do axum serve tại
  `http://127.0.0.1:8731` (`--port`, `--no-open`). API: `/api/state` · `/api/skills` (list + toggle
  tier qua `agents/skills.toml`) · `/api/memory/:file` (get/set, allowlist) · `/api/engines`
  (codegraph/cbm/headroom/**serena** + mnemopi) · `/api/bench` · `/api/eval`. Refactor B5: tách
  `bench_metrics()`/`eval_project_data()` (home: &Path) cho cả CLI lẫn web reuse. Build.rs tự build
  FE qua pnpm khi thiếu + stub fallback. Deps: axum 0.7 + tokio + tower-http (override có chủ đích
  rule "tránh tokio" trong AGENTS.md §8, gated `harness web`). Verified real: 6 endpoint trả data
  sống (eval 96% 28/29, bench A1 PASS).

### Added (Phase C — full manage)

- **Workspace + team + submodule + skill install** qua dashboard: `/api/workspaces` (list omp
  profiles + project + activate ghi `web-session.json`), `/api/team` (subagent roster 8 loại +
  readiness reuse eval_project_data), `/api/submodules` (parse `.gitmodules` + add/pull/remove qua
  git shell-out), `/api/skills/add|update` (self-shell-out `8sync skill`). FE: 3 page mới (Workspaces,
  Team, Submodules) + nav. Verified real: workspaces/team/submodules trả data, skill add validate spec.

## [0.24.0] — 2026-06-25

### Added

- **`8sync harness eval --project` — agent-team readiness scorecard (% per vai).** Deterministic + offline:
  chấm capability coverage trên repo hiện tại theo dev · qa/testing · research · ba/po · fe · be · docs ·
  memory/learn · token-opt (engine on PATH + skill present + memory spine + stack signals). Honest READINESS
  (team được trang bị gì Ở ĐÂY), KHÔNG phải output-quality (đó là `harness eval` loop probe model+network).
  Run thật: su-code 89%, 8syncdev-pro-v2 79%.
- **`token-bench` skill (bundled) — chứng minh token-saving của code-intel trên repo thật.**
  `scripts/token_bench.py` (uv/PEP723, stdlib-only): mỗi symbol thật so codegraph-query+slice vs
  grep+read-whole-file, có def-kind correctness gate. Đo trên codebase lớn thật: 8syncdev-pro-v2 −96.6%,
  gsd-pi −78.6% (range 54–98%; symbol dùng rộng / file lớn → 95–98%), correctness gsd-pi 10/10. Cần
  ANSI-strip (codegraph tô màu cả khi pipe). Bundled qua `deploy.rs` (16 skills).
- **6 reference submodule** (inspect/track upstream; deinit, content gitignored): gstack · gsd-pi ·
  agent-reach · addyosmani/agent-skills · DietrichGebert/ponytail · **DeusData/codebase-memory-mcp**.
- **`outputs/agent-team-workflow-automation-plan.md`** — operating plan để vận hành su-code như một agent
  team: map sprint 23-specialist của gstack + loop slice/auto/worktree của gsd-pi lên `/gs` + skills +
  subagents, kèm **UI/UX Design Lane** riêng (impeccable + Clouds F + **Lighthouse 4-tiêu chí quality gate**).
- **`8sync` help dẫn đầu bằng AI TEAM (harness + `/gs`).** Cheatsheet (`8sync` / `8sync help` / `8sync flow`)
  trước đây mở đầu bằng install + vibe loop, **không hề** nhắc `8sync harness` (all-in-one) lẫn `/gs` (team
  lead) — giờ là section ĐẦU TIÊN. Fix dòng stale: `8sync skill sync` (đổi thành `skill update`; regen là
  `8sync harness`) và `8sync up` ("binary + omp" → chỉ 8sync; omp qua `omp update`).
- **`/gs <goal>` scope handshake (chỉ assisted).** Goal medium+/mơ hồ không dive thẳng: GS ground
  (codegraph/cbm) rồi đề xuất **2–4 phương án cụ thể** (scope · team size + roles/skills · effort · tradeoff,
  rút từ bench senior: impeccable+Lighthouse / senior-frontend / code-review-and-quality / senior-security /
  performance-optimization) kèm recommended default + 2–4 câu hỏi sắc qua `AskUserQuestion` — một vòng rồi
  chạy. `auto` vẫn unattended (no questions); trivial/small bỏ qua handshake. (`assets/commands/gs.md` §1b.)
- **`8sync harness eval` báo `%`** (`eval.rs:114`) — `score: N/M passed (X%)`. 3/3 = 100%.
- **`outputs/omp-customization-memory-platform-research.md`** — research grounded từ omp docs: cơ chế nhớ
  THẬT = **Mnemopi memory + cbm + spine**, dùng **model API (không local — máy yếu vẫn chạy)**, thay cho ngộ
  nhận GGUF/fine-tune (không khả thi); custom command/workflow trên ĐÚNG base omp (`.omp/commands` native,
  update không conflict); submodule auto-pull là ngộ nhận (skill đã auto-latest qua manifest+`harness up
  --pull`); agent-reach = capability layer (đọc internet), thêm làm skill.
- **Mnemopi memory wired vào `8sync harness`** (`deploy.rs::ensure_mnemopi_memory`) — `harness`/`init` bật
  `memory.backend: mnemopi` (+ `scoping: per-project-tagged`, `llmMode: smol` API, `noEmbeddings: true` FTS,
  `polyphonicRecall`) trong `~/.omp/agent/config.yml` (idempotent sentinel-block, KHÔNG clobber `memory:` của
  user). 0 local model → máy yếu chạy. `8sync doctor` báo memory ON/OFF (`doctor.rs`). Verified: omp 16.1.20
  load config OK, doctor "mnemopi memory ON". Tradeoff: recall inject token/phiên (user đã chốt bật).
- **5 reference repo = git submodule** (`reference/`, content gitignored, deinit mặc định): gstack · gsd-pi ·
  **agent-reach** · **addyosmani/agent-skills** · **DietrichGebert/ponytail**. Đăng ký để inspect/track upstream
  (`git submodule update --init --remote reference/<name>` để pull-latest đọc khi cần). Submodule = nguồn-tra-cứu;
  deploy auto-latest cho skill LIVE vẫn qua manifest + `harness up --pull`.

### Changed

- **Declutter skill registry — bỏ pack research `companion-inc/feynman` (20 skill on-demand).** Manifest
  (`agents/skills.toml` committed + `~/.config/8sync/skills.toml` machine-local) đăng ký 20 skill
  research/ML/academia (paper-writing, ml-training-recipe, literature-review, runpod/modal-compute,
  peer-review, jobs, eli5, …) — sai domain cho một coding harness + là prefix noise inject vào AGENTS.md
  mỗi phiên. Cắt cả 20 (collection re-pull là all-or-nothing theo URL — `update.rs:49`, giữ 1 cái là
  re-clone cả pack). Kết quả: on-demand 55 → 35, force-load prefix ~1998 → ~1717 tok, deferred −5k tok
  (`8sync harness bench`), A1 stable-prefix PASS. Giữ nguyên addyosmani coding-eng + design payload
  (impeccable/taste/assp) + bundled always-on.

## [0.23.0] — 2026-06-24

### Added

- **`8sync harness eval` — loop quality probe.** Runs a fixed task-suite through omp non-interactively
  (`omp -p --no-session --auto-approve`) and scores each task with a deterministic `verify.sh` (the
  verifier OWNS the assertion, so the agent can't game the check). Three bundled fixtures:
  `fix-failing-test` (correct a wrong impl until `cargo test` is green), `add-fn-with-test` (implement
  `slugify`; the verifier appends the assertions), `locate-symbol` (answer `path:line` for a symbol).
  Writes a JSON scorecard + a `--baseline` reference into the gitignored `.cache/8sync/eval/`; later
  runs print the pass-count delta vs baseline. Model + network, non-deterministic — a periodic quality
  SIGNAL, not a CI gate. Verified end-to-end: 3/3 on this machine.

### Changed

- **`/gs` L3 worktree isolation is now concrete.** The guardrail named "git-worktree isolation" with no
  mechanism; it now prescribes the exact flow — `git worktree add .gs/wt/<slug> -b gs/<slug>`, implement
  + verify + commit on that branch inside it, then `git worktree remove` (merge/PR only if asked); never
  edit `main`'s working tree directly. (`.gs/` is gitignored, v0.22.0.)

## [0.22.0] — 2026-06-24

### Added

- **`8sync harness audit` — code-backed doc-hygiene** (was prompt-only advice with zero code behind it).
  Scans committed docs (AGENTS.md/CLAUDE.md/README/CHANGELOG + `agents/*.md`) for **stale path references**
  (repo-relative paths in docs that no longer exist), **oversized docs** (>400 lines / >120-line force-load
  block), and **30-day churn hotspots** (history-awareness — docs near churned code are likeliest stale).
  Report-only: never auto-deletes (heuristic; illustrative paths flagged "review before editing"). Skips
  absolute / `~`-rooted / URL paths so the harness's own machine-generated refs don't false-positive.
  `8sync doctor` surfaces a one-line summary; `/gs` + the `gs` skill doc-hygiene step now run the audit
  instead of eyeballing.
- **`8sync doctor` AI-engine health check** — verifies the token-optimization stack is installed AND
  registered with omp ("luôn xài"): codegraph (local index) · codebase-memory-mcp (semantic graph) ·
  headroom (output compression). A missing or unregistered engine silently defeats STEP 0 token
  discipline, so doctor now flags it with the one-command fix (`8sync harness`).

### Fixed

- **codegraph STEP 0 verbs were wrong** in the force-load prefix, the subfolder-index block, and the
  KNOWLEDGE breadcrumb: they taught `codegraph search/deps/defs`, none of which exist. Corrected to the
  real CLI surface `codegraph query/callers/callees/impact` (verified against codegraph 0.9.6) so the
  agent's first explore call doesn't error out.
- **Duplicate always-on skill in the force-load list.** A stale/external `karpathy` dir alongside the
  canonical `karpathy-guidelines` (identical frontmatter `name`) made the skill appear twice — once in
  CORE, once in on-demand. `build_force_load` now dedups by frontmatter name, keeping the higher-ranked
  dir, so each logical skill is listed exactly once. Future-proof against any dir/name collision.
- **impeccable setup scripts couldn't run under 8sync's layout.** The bundled design skill referenced
  `.agents/skills/impeccable/scripts/*.mjs` (leading dot) but 8sync mirrors skills to `agents/skills/`
  (no dot). Fixed 28 references across SKILL.md + 4 reference docs → `agents/skills/`.

### Changed

- Managed `.gitignore` block now ignores `.gs/` (per-run worktree + `/gs stop` marker — machine-local).

## [0.21.0] — 2026-06-24

### Changed

- **`/gs` redesigned to right-size effort (fixes the post-`/gs` quality regression).** Eval +
  deep-research (`outputs/gs-eval-improve-research-brief.md`) found the drop was process
  over-engineering, not tokens (`harness bench`: ~8.5k upfront, 79% saved, KV-cache stable):
  the 93-line command forced a team + full Closeout on every task and `auto` "never asked".
  - **Right-size first** — classify trivial/small → **solo** (no team, no Closeout) · medium →
    solo + one verifier · large → full loop + roles + Closeout. A team is the exception you justify
    (Cognition/Anthropic: single-agent default; multi-agent only when it clears the bar).
  - **Solo-by-default delegation** — subagents only for parallel-independent / context-isolation /
    specialization; scoped objective + summary return (never free-form, never inline transcript).
  - **Autonomy confidence-gated** — strong `auto`, but a high-stakes hard-to-undo low-confidence call
    is now a blocker (Anthropic 2026: "agents learning when to ask"); prefer reversible, never compound.
  - **Doc-hygiene step** — detect stale paths / junk / superseded docs → fix or **delete** (no addition
    without the matching deletion); keep docs lean. Stale docs poison agent context.
  - **Codebase-history** — `git log/blame` + DECISIONS + cbm `detect_changes` before load-bearing edits.
  - **Leaner command** — `assets/commands/gs.md` 93 → 56 lines (lower constraint density → better
    instruction-following); full protocol stays in the `gs` skill (progressive disclosure).

## [0.20.1] — 2026-06-23

### Fixed

- **`/gs auto` actually runs unattended now.** Added an **Autonomy contract** to the `/gs` command +
  `gs` skill: in `auto`/L3 the agent NEVER calls `ask` or stops on ambiguity — it resolves unknowns by
  research (codegraph/cbm → `agents/*`/PLAYBOOKS → skills → `web_search`/`autoresearch`/`deep-research`),
  picks the boring/reversible option, logs it under a new `## Assumptions` section in `agents/STATE.md`,
  and proceeds. "Blocker" is tightened to ONLY missing credential / external approval / destructive-
  irreversible action; design choices, naming and scope are no longer stops. Note: a slash command
  cannot bypass omp's approval gate — keep `tools.approvalMode: yolo` (default) for true unattended runs.
- **`/gs` argument hint.** Added `argument-hint` frontmatter and front-loaded the description with
  `[auto | <goal> | status | next | stop]` so the autocomplete dropdown shows the modes when you type
  `/gs ` (omp renders per-argument hints only for built-ins; the description is what surfaces for
  file-based commands).
- **QA + test are now first-class gates in `/gs`.** Per-slice verify-gate explicitly runs tests + a QA
  pass and forbids skipping/weakening tests; added a mandatory **Closeout** step — full test suite +
  end-to-end QA + independent re-review against the Definition-of-Done + a handoff summary — that must
  pass before the loop reports "done". Never hands back unverified work.

### Added

- **Reference submodules `reference/gstack` + `reference/gsd-pi`** (git submodules, MIT) for studying
  the engineering-team + autonomous-loop patterns that informed `/gs`. Pointers are committed
  (reproducible) but the working trees are **deinitialized by default** so they never bloat the
  codegraph/cbm index (codegraph honors no exclude/ignore — populating them ballooned the index to
  ~3k files / 110 MB). Study on demand: `git submodule update --init reference/<name>`; re-shrink with
  `git submodule deinit -f reference/<name>`. `reference/` is also gitignored as a cbm-index guard.

## [0.20.0] — 2026-06-23

### Added

- **`/gs` — one-command autonomous engineering-team loop (omp slash command).** `/gs <goal>` plans +
  runs, bare `/gs` resumes, `/gs auto` runs unattended (L3), `/gs status|next|stop`. Drives the loop
  off `agents/STATE.md`: plan → delegate to specialist roles (`task` subagents / gstack roles if
  installed) → verify-gate → commit → record (KNOWLEDGE/PLAYBOOKS) → advance until Definition-of-Done
  or a blocker. Token-lean (codegraph + codebase-memory-mcp + headroom mandatory) and guardrailed
  (verify-gate before commit, worktree isolation + no push/PR at L3, hard-stop via `/gs stop`).
  Modeled on gsd-pi `/gsd auto`.
- **Deploy + team-sharing.** `8sync harness`/`init`/`up` write it to `~/.omp/agent/commands/gs.md`
  (global) and `<repo>/.omp/commands/gs.md` (committed → whole team gets `/gs`). New on-demand `gs`
  skill documents the protocol; `8sync harness up --timer` runs it 24/7.

## [0.19.0] — 2026-06-23

### Changed

- **Loop engineering v2 — Phase A (token & stable-prefix discipline).**
  - Force-load block (`inject.rs`) + master `00-force-load.md` split always-on skills into
    **CORE** (codegraph · karpathy · ponytail · 8sync-cli — đọc body upfront) và **SPECIALIST**
    (assp · impeccable · taste · image-routing — biết khả năng, đọc body khi task khớp /
    progressive disclosure). Thu nhỏ tập đọc-ngay; `impeccable` vẫn bắt buộc ngay khi có việc UI/design.
  - `headroom_compress` nâng từ khuyến nghị → **bắt buộc** cho output > ~50 dòng (STEP 0 + invariants).
  - KNOWLEDGE breadcrumb (`memory.rs`) bỏ timestamp `epoch:` volatile → byte-stable giữa các lần
    `harness` (thân thiện KV-cache, hết git churn). `now_stamp()` vẫn dùng cho tên file archive.
  - Plan + provenance: `outputs/harness-loop-engineering-v2-plan.md`.
- **Loop engineering v2 — Phase B (live memory & recitation).**
  - `agents/STATE.md` seeded as a structured **live plan** (Goal · DoD · Checklist · Current ·
    Next · Open-questions · Handoff) — recitation anchor (Manus todo.md pattern): read at session
    start, rewritten at each phase boundary to keep the plan in recent context.
  - Loop section (`00-force-load.md`) + generated block (`inject.rs`) gain **recitation**,
    **compaction** (near-limit → structured handoff to STATE + lessons to KNOWLEDGE → reinit, with
    `headroom_compress` as summarizer), and **budget-awareness** rules.
  - `harness bench` now counts the memory spine in the upfront budget (more honest accounting).
- **Loop engineering v2 — Phase C (maker/checker + Reflexion).**
  - Loop section + generated block: `task` implementer ↔ **independent verifier** (build/test in
    its own context, verify-gate before commit), explicit objective/boundaries/output per subagent,
    share-full-trace for dependent work, parallel only when subtasks are independent.
  - **Reflexion failure-capture**: a failed verify writes a `failure:` entry to KNOWLEDGE (symptom
    + cause + fix); recent failures are read at session start to avoid repeating them.
- **Loop engineering v2 — Phase D (procedural memory / playbooks).**
  - `agents/PLAYBOOKS.md` seeded (Voyager-style skill library): validated multi-step procedures
    distilled into reusable runbooks indexed by a `When:` line — retrieved + adapted, not re-derived.
  - Memory tiering: KNOWLEDGE = verbal lessons · PLAYBOOKS = verified procedures · DECISIONS = ADR.
    `harness bench` now counts PLAYBOOKS in the spine (6 files).
- **Loop engineering v2 — Phase E (phased autonomy + guardrails).**
  - L1 report · L2 assisted · L3 unattended defined, with guardrails (verify-gate before commit,
    gitleaks, commit scoped to `agents/`+docs, no auto `push`/PR at L3). `harness up --timer`
    per-tick job documented (read STATE → Next → verify → update spine → optional commit).

### Added

- **`8sync harness bench`** — deterministic loop-engineering benchmark (no model calls): upfront
  context budget (force-load prefix + CORE skill bodies) vs deferred (SPECIALIST + on-demand),
  the A2 progressive-disclosure saving, and an A1 KV-cache stable-prefix gate. Refactors a shared
  `inject::build_force_load()` (single source of truth for inject + bench). Baseline on this repo:
  upfront ~5.5k tok vs naive ~37.9k tok → **85% upfront cut**; A1 PASS.

## [0.18.1] — 2026-06-23

### Fixed

- **`8sync harness init` now pulls registered manifest skills** — `init` calls
  `skill update` against `agents/skills.toml` (git collections like `feynman`:
  deep-research, autoresearch, …) before mirroring, making it a true superset of
  bare `8sync harness`. Previously `init` only deployed the bundled skills + 2
  hardcoded external packs (ponytail, addyosmani), so manifest-only skills never
  reached `agents/skills/` via `init` — only bare `8sync harness` / `up --pull` did.

## [0.18.0] — 2026-06-21

### Added

- **Headroom context-compression wired as an omp MCP** — `8sync harness`/`init` auto-installs
  `headroom-ai[mcp]` (uv → pipx → pip fallback) and registers it in `~/.omp/agent/mcp.json`
  (`headroom mcp serve`, stdio). Tools `headroom_compress` / `headroom_retrieve` / `headroom_stats`
  compress long tool outputs / logs / diffs 60–95% before they reach the model. Force-injected into
  STEP 0 + `00-force-load.md`. Researched alongside PixelRAG + LocateAnything3D — **skipped**:
  PixelRAG (screenshot-RAG) overlaps `8sync shot`/`read`/`browser`; LocateAnything3D is a 3D-vision
  model (out of scope for a coding harness).

## [0.17.1] — 2026-06-21

### Fixed

- **Skills now propagate to other machines.** `8sync harness` / `skill update` write a
  committed project manifest `agents/skills.toml` (mirroring the machine-local registry) and
  read it back on any machine — so a fresh clone re-pulls the exact same skills. Previously only
  the machine-local `~/.config/8sync/skills.toml` recorded `skill add`-ed sources, so custom
  skills (e.g. git collections like feynman) never reached a second machine via harness — only
  the 15 binary-embedded skills + 2 hardcoded external packs did. (`agents/skills.toml` is a
  file, so it travels even when the `agents/skills/` directory is gitignored.)

## [0.17.0] — 2026-06-21

### Added

- **codebase-memory-mcp = first-class code-intelligence engine** — `8sync harness`/`init`
  auto-installs the binary (upstream installer, binary-only), sets `auto_index true`, and
  registers it as an omp MCP server in `~/.omp/agent/mcp.json` (idempotent, preserves other
  servers). `harness`/`up` index the repo. Mirrors `ensure_codegraph` — zero manual MCP config.
- **Code intelligence FIRST (STEP 0)** — the injected force-load block + `00-force-load.md`
  mandate codegraph + codebase-memory-mcp BEFORE grep/read for all code exploration
  (~99% token saving); raw `Read` only for read-before-edit.
- **Loop-engineering principles** (Addy Osmani / Boris Cherny) in `00-force-load.md`:
  STATE/KNOWLEDGE spine, maker/checker via `task` sub-agents, verify-gate, phased
  L1→L3 autonomy via `harness up --timer`.

## [0.16.0] — 2026-06-21

### Added

- **`8sync harness` (bare) = ONE command** — idempotent driver that makes a project
  agent-ready in a single pass: deploy/update skills + mirror (additive) + inject
  force-load + seed memory & gitleaks hook + consolidate learnings + re-index codegraph.
  `harness init` = explicit full bootstrap (progress UI); `harness up` = light refresh;
  `harness up --timer 30m` = background loop.
- **Additive skill mirror + `--force`** — `harness`/`harness init` never clobber an
  already-vendored (possibly edited) `agents/skills/<name>`; only missing skills are
  written. `harness init --force` re-mirrors everything. `harness up` now also seeds
  the gitleaks pre-commit hook.
- **`8sync skill update [name]`** — re-pull registered skills from their recorded
  source in `skills.toml` (git URL / `builtin:` / `path:`). Git sources are deduped
  per URL (a collection repo is cloned once, all sub-skills reinstalled); best-effort
  per source (offline / missing `git` warns + skips, exit 0). `name` updates just one.
- **`8sync harness up --pull`** — refresh AND re-pull every registered skill before
  re-injecting. Default `up` stays network-free + fast (timer/loop unaffected).
- **`8sync harness up --commit`** — close the self-learning loop: stage + `git commit`
  ONLY the refreshed agent memory (`agents/`, `AGENTS.md`, `CLAUDE.md`, `CHANGELOG.md`,
  `.gitignore`; never your code) so learnings persist to git in the same pass. No-op
  when nothing changed (no empty-commit spam on `--timer`); default off.
- **`8sync harness help`** — one-screen cheatsheet: commands, skill tiers, the
  commit-vs-ignore file taxonomy, and the new-machine runbook.
- **Portability**: `harness init`/`up` seed a managed `.gitignore` block (between
  `# >>> 8sync (managed) >>>` sentinels) — ignore derived (`.codegraph/`, `.cache/8sync/`)
  + secrets (`.env`, `.env.*`, keep `!.env.example`), keep agent memory + `agents/skills/`
  committed. `8sync doctor` now errors if any durable `agents/*.md` / `AGENTS.md` /
  `CHANGELOG.md` is gitignored (learnings wouldn't survive a move to a new machine).
- **`agents/KNOWLEDGE.md`** seeded with an append-only `## Learnings` zone below the
  managed breadcrumb block (overwritten each `harness up`) so learnings persist.

### Hardened (research-driven — see `outputs/harness-selfimprove-research-brief.md`)

- **Lean force-load context** — the injected on-demand skill list is now names+path
  only (one line each); full descriptions live in each `SKILL.md` (progressive
  disclosure). `8sync doctor` warns if the `AGENTS.md` force-load block exceeds 120
  lines. *Why:* Gloaguen et al. arXiv 2602.11988 (138 repos) — bloated/duplicative
  context files cut agent success and add >20% inference cost.
- **Skill version pinning (lockfile)** — `8sync skill add <url>@<ref>` pins a git
  commit/tag/branch; the resolved SHA is recorded as `rev` in `skills.toml` and
  `skill update` checks out exactly that rev (reproducible). Unpinned entries track
  latest. *Why:* mirrors Claude Code plugin marketplace (SHA pin = reproducible).
- **Secret-scanned auto-commit** — `harness up --commit` runs `gitleaks protect
  --staged` (if installed) and ABORTS on detection; `harness init` installs a
  gitleaks pre-commit hook (non-destructive); `8sync doctor` reports gitleaks.
  *Why:* GitGuardian 2026 — AI-assisted commits leak secrets ~2× baseline.
- **Bounded memory (anti context-rot)** — `harness up` consolidates the
  `## Learnings` zone past ~200 lines, archiving older entries to `agents/archive/`
  with a pointer. *Why:* 4-lever consolidation; "remember everything → remember nothing".
- **Verifier-gated learnings** — seeded `KNOWLEDGE.md` instructs prefixing entries
  `validated:` (test/build confirmed) vs `hypothesis:`. *Why:* Reflexion verifiability
  constraint — no reliable improvement beyond what's objectively verified.

## [0.15.1] — 2026-06-17

### Added

- **impeccable house design references** (`assets/skills/impeccable/references/house/`): bundled
  `frontend-agent-workflow.md` (senior coding-agent workflow) + `clouds-f.md` (senior front-end
  orchestration) + `clouds-f-rules/*.mdc` (design-redesign / responsive / performance / fix /
  refactor / security keyword routers). impeccable's SKILL.md auto-references them.

### Changed

- **Emphasised `impeccable` as THE house design system** across the force-load flow (AGENTS.md /
  CLAUDE.md block, `00-force-load.md`, sub-folder index, KNOWLEDGE breadcrumb): mandatory for any
  UI / design / redesign / audit, read with `references/house/*`.

## [0.15.0] — 2026-06-16

### Added

- **`8sync harness` verb** — one command to stand up the full agent harness.
  - `harness init`: deploy mọi bundled skill + codegraph binary + external skill
    packs (best-effort clone), mirror vào `agents/skills/`, `codegraph init`,
    seed `agents/*` memory + `CHANGELOG.md`, inject force-load vào AGENTS.md/CLAUDE.md
    + một index gọn vào **mọi sub-folder code** (progressive disclosure). Có progress
    UI `[i/N]` + thời gian.
  - `harness up`: refresh theo state hiện tại (re-inject + refresh `agents/KNOWLEDGE.md`
    breadcrumb + `codegraph index`). `--loop <dur>` chạy foreground; `--timer <dur>|off`
    cài/gỡ systemd **user timer** (đúng cách cho chạy nền, mirror `8sync clean --timer`).
- **6 bundled skill mới**: `ponytail` (always-on, lazy-senior YAGNI), `code-review-and-quality`,
  `senior-security`, `senior-frontend`, `full-flow`, `encore-deploy` (on-demand). Trước đó
  (0.14.x → nội bộ) đã thêm `assp-skill`, `impeccable`, `taste-skill`. Tổng **15 bundled**.
- **Always-on order** (đọc top-down, ưu tiên): codegraph → karpathy → ponytail → assp →
  impeccable → taste → 8sync-cli → image-routing. Inject block dạy rõ *cách tận dụng* từng skill.
- **Tech-gated skills**: `encore-deploy` chỉ hiện trong force-load block khi project dùng
  Encore (`encore.app` / `encore.dev`).
- **Opt-in skills**: `social-growth` (chiến dịch social/branding/lead-gen cho FB/YouTube/TikTok,
  page setup, insight, monthly plan + target) — KHÔNG auto-bật; bật bằng
  `8sync skill add builtin:social-growth`.
- **`8sync skill add` collection-aware**: clone repo rồi cài mọi `skills/<name>/SKILL.md`
  (vd `addyosmani/agent-skills` 24 skill, `ponytail` full); `builtin:<name>` deploy
  bundled skill từ embedded assets.
- **Sub-folder `AGENTS.md` index** + **`agents/KNOWLEDGE.md` breadcrumb** + **`CHANGELOG.md`**
  seeding tự động, để agent không bỏ sót rule và tự học theo state dự án.

### Changed

- **`8sync skill sync` → `8sync harness init`** (clean cutover, không giữ alias). `skill sync`
  in cảnh báo trỏ sang lệnh mới.
- `crates/cli/src/verbs/skill.rs` (~1340 dòng) tách thành module tree `verbs/skill/`
  (`mod` · `meta` · `discover` · `list` · `spec` · `add` · `gen` · `deploy` · `inject` · `index`),
  mỗi file < 500 dòng. Harness logic ở `verbs/harness/` (`mod` · `init` · `up` · `memory` · `external`).
- `8sync .` giờ cũng inject sub-folder index (nearest-AGENTS.md wins).
- Binary size target: < 4 MB (binary ~3.8 MB stripped, gồm 15 bundled skill).

## [0.14.2] — 2026-06-02

- fix(bt): Bluetooth vanishing after cold boot (USB autosuspend).

## [0.14.1] — 2026-05-31

- clean is project-safe: never touches models / Playwright / download caches.

## [0.14.0] — 2026-05-31

- `8sync clean`: disk/RAM reclaim + CPU/GPU report + periodic timer.

## [0.13.0] — 2026-05-31

- `8sync bt` bluetooth verb; Caelestia desktop install removed.

## [0.12.1] — 2026-05-30

- two-tier skill injection (always-on vs on-demand).
