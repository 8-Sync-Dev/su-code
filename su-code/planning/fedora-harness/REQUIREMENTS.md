# Requirements — Fedora-First Core + Enforced omp Harness

Use cases (UC). Every ROADMAP AC maps back to ≥1 UC.

## v1 (in scope)

### Fedora
- **UC-1 — Install on Fedora with one command.** A fresh Fedora box runs a short one-liner and gets
  a working `8sync`; `8sync setup` completes with **zero** package-manager failures; `8sync doctor`
  is green. Today Stage B fails on every profile.
- **UC-2 — Distro-dispatched package layer.** One abstraction over `pacman`/`paru` and `dnf`/`copr`
  so no verb calls a package manager directly. Fedora uses `dnf history undo` for rollback rather
  than porting the hand-rolled pacman snapshot logic. Arch behavior is unchanged.
- **UC-3 — Distro detection that is actually correct.** `ID` **and** `ID_LIKE` from
  `/etc/os-release`, so RHEL-likes and Arch-likes both resolve. A profile that cannot run on the
  detected distro is skipped with a clear reason, never a failed `sudo` prompt.
- **UC-4 — Profiles carry per-distro package names.** Each profile declares its Arch and Fedora
  package names (or declares itself unavailable), including COPR/rpmfusion prerequisites.

### Enforcement (the "omp ignores my tools" fix)
- **UC-5 — Skill paths work on every machine.** ✎ *Reworded:* nothing 8sync writes into `AGENTS.md`
  may contain a **machine-specific** absolute path (`/home/<user>/…`, `/Users/<user>/…`,
  `/root/…`). Project skills are emitted repo-relative (`su-code/skills/<dir>/<entry>`);
  **`~/`-anchored paths remain legal** because global skills genuinely live in `$HOME`
  (`~/.omp/skills/…`) and no relativisation can move them — the on-demand tier at
  `inject.rs:190-193` already does exactly this and is the reference implementation. Acceptance is
  therefore: a repo cloned to a second machine resolves every skill **after `8sync setup`**.
- **UC-6 — Code-intel preference is enforced, not requested.** A `grep`/`rg`/`find` attempt for
  code structure is intercepted and redirected to codegraph/serena/codebase-memory by a mechanism
  omp enforces (TTSR rule, `tool_call` hook veto, `bashInterceptor`), not by prose.
- **UC-7 — Enforcement degrades safely.** If codegraph/serena/cbm are absent, nothing is blocked —
  the interceptor must never dead-end a session on a machine where the replacement tool is missing.
- **UC-8 — Lean prompt.** Unused omp defaults are turned off (`tools.xdev`, per-tool `.enabled`,
  skill trimming) so the harness costs less context, not more.
- **UC-9 — Byte-stable writes.** Managed files are rewritten only when content differs, preserving
  the prompt-cache prefix.
- **UC-10 — `harness audit` catches the defects it currently skips.** ✎ *Scoped:* flag only
  machine-specific prefixes (`/home/`, `/Users/`, `/root/`), not every absolute path.
  `audit.rs:55-57` ✎ deliberately `continue`s on any `/`-prefixed token, and its comment is right
  that generic absolutes carry no doc-rot signal — a naive "flag all `/`" fix would flag every
  legitimate `/etc/os-release`, `/tmp`, `/usr/bin` mention and make the audit unusably noisy.

### Minimize
- **UC-11 — One source of truth for skills and commands.** A single dynamic registry replaces the
  four contradictory hardcoded lists (`deploy.rs:17` 20 · `setup.rs:715` 4 · `skills.toml` 4 ·
  AGENTS.md 8/17/37). Adding an asset must not require editing a Rust array.
- **UC-12 — Cut dead weight.** Drop/merge the redundant skill clusters and move pre-bundled browser
  JS out of the embedded payload. Measured against the size gate, not guessed.
- **UC-12b — Fix the `.gitignore` footgun.** `.gitignore:29` is a bare `reference/`, which silently
  ignores **any** `reference/` directory at any depth. It was discovered live: the distilled omp
  docs written to `su-code/reference/omp/` vanished from `git status` and had to be relocated to
  `su-code/omp-reference/`. `assets/skills/impeccable/reference/` (27 files) survives only because
  it was tracked *before* the rule existed — a new contributor re-adding it would lose it. Narrow
  the pattern to the directory it was meant for. Note `.gitignore` is harness-managed, so the fix
  belongs in the generator, not only in the file.

### Authoring (the missing commands)
- **UC-13 — `/create-skill <description>`** scaffolds a spec-compliant skill (dir + `SKILL.md`
  frontmatter), registers it, and deploys it. Nothing today scaffolds a skill: `skill add` only
  installs from git/path/builtin, `skill gen` only fuses existing ones.
- **UC-14 — `/create-command <description>`** scaffolds a slash command into `assets/commands/`
  and deploys it to both global and project scope, with no hardcoded Rust block per command.
- **UC-15 — `/auto-package`** captures a workflow that was just performed and packages it into one
  reusable command (+ skill when warranted), so a proven procedure becomes a single verb.

### Engine
- **UC-16 — Inherit omp's DAG/graph/loop instead of duplicating it.** Delete
  `8sync-workflow.ts` (100 % duplicate of `todo` + the engine state file + `goal_updated`);
  reduce `8sync-engine.ts` to the two genuinely additive gates (code-enforced verify; gitleaks-
  gated autonomous commit) and lean on omp's `task`/eval DAG, isolation PAL, and mnemopi graph
  memory for the rest.
- **UC-17 — Promote the recall hook.** `8sync-recall.ts` is the correct compaction-surviving
  re-injection pattern; move it off the legacy `HookAPI` onto `ExtensionAPI`.

### Distribution
- **UC-18 — Short install URL** comparable to `curl -fsSL https://omp.sh/install | sh`.
- **UC-19 — Update notification that never blocks.** Today `main.rs:145-148` calls
  `auto_check_notice()` synchronously before dispatch, so bare `8sync` can stall up to 5 s on
  `api.github.com`. It must be non-blocking, TTY/CI/NO_COLOR-aware, and correct (string equality
  currently flags dev builds as outdated).
- **UC-20 — `8sync up` must not re-download when current.** `up.rs:32-33` hardcodes `force=true`,
  making the `remote == local` skip unreachable; every run pulls ~5 MB.
- **UC-21 — Verify the download.** The Releases API already returns a per-asset
  `digest: "sha256:…"`; verify it with `sha256sum` (no new crate, no release-plumbing change).

## v2 (recorded, out of scope)

- **UC-22 — Graph RAG over the project's own memory** using mnemopi's `polyphonicRecall` instead of
  8sync's flat `su-code/*.md` files. Real, but depends on UC-16 landing first.
- **UC-23 — `harness web` project isolation.** A real leak already happened (screenshots of
  `content-post-agency` served from another project's session). Needs its own security phase.
- **UC-24 — Bounded codegraph indexing.** The watch timer OOM-killed a 5.3 GB Node process on
  `zus`. Needs a cgroup/ulimit ceiling.
- **UC-25 — TypeScript 7 API adoption** once TS 7.1 ships the compiler API (~Oct 2026).

## Explicitly NOT doing

- No Rust native omp addon (natives are internal-only; loader rejects third-party `.node`).
- No new crates — 383 KB of headroom forbids `sha2`/`semver`/`self_update`/napi.
- No replacing omp's DAG, memory, or retry machinery with our own.
- No dropping Arch/CachyOS support.
- No `tsgo` in the runtime (Bun strips types; TS 7 API is `not ready` until 7.1) — CI typecheck only.
