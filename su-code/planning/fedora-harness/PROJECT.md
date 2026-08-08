# Project — Fedora-First Core + Enforced omp Harness

## Core value

`8sync` becomes a **Fedora-first, minimal, Rust-fast harness that makes omp actually use its
strongest tools** — instead of politely asking it to in prose and being ignored.

Two failures are killing the current product, and both are now root-caused:

1. **It does not run on Fedora.** Zero Fedora support exists. `pkg.rs` is 208 lines of literal
   `pacman`; `platform.rs:93` takes a positional parameter literally named `pacman`;
   `env_detect.rs:17-23` reads only `ID=` from `/etc/os-release`. On this box every Stage-B
   profile fails. (`FedoraPortAudit`, ~117 port sites.)
2. **Every instruction is prose, so omp skips it.** `APPEND_SYSTEM.md` and `00-force-load.md`
   *ask* the model to prefer codegraph / serena / codebase-memory / browser / skills. omp ignores
   advisory text under load. Worse, the skill paths it is told to read are **absolute to another
   machine** — `inject.rs:174` `p.join(entry)` emits `/home/alexdev/...` into every project's
   AGENTS.md, so on any other machine the paths are dead and the skills silently never load.

## Stack

Rust workspace, single binary `8sync` (clap · anyhow · rust-embed brotli · serde · which).
No `reqwest` — HTTP is `curl` shell-out. Assets embedded from `assets/`.
omp is the runtime being steered; its docs are distilled in `su-code/omp-reference/`.

## Hard constraints

- **Binary ceiling 5 242 880 B (enforced by `scripts/size-gate.sh`); goal 4 194 304 B.**
  Current x86_64 = 4 859 696 B → only **383 184 B headroom (7.3 %)**. Any phase that adds bytes
  must remove more. No new crates (rules out `sha2`, `semver`, `self_update`, napi).
- **No Rust native omp addon.** omp's natives are internal-only: *"every `crates/*` entry is
  internal to `@oh-my-pi/pi-natives`"* (`user-facing-packages.md:10`); the loader only accepts its
  own version sentinel. The Rust lever is the `8sync` binary itself; TS extensions stay thin
  shell-outs.
- **Inherit, do not reimplement.** omp already ships the DAG layer (`task` batch, eval-kernel
  `agent()/parallel()/pipeline()`, `hub`, isolation PAL, `agent://` artifacts), a real **graph**
  memory backend (mnemopi: SQLite vector+FTS, episodic graph, `proactiveLinking`,
  4-voice `polyphonicRecall`), and ~10 loop/retry/watchdog mechanisms. su-code's extensions
  duplicate four of them and add exactly **two** things omp lacks: a code-enforced verify gate and
  a gitleaks-gated autonomous commit.
- **Default never overwrites user-owned files** (repo invariant). Managed files refresh by
  byte-compare — byte-stability is also what keeps the Anthropic prompt-cache prefix hot.
- Verb count ≤ 22 flat.

## The enforcement insight (why this project exists)

`su-code/omp-reference/LEVERS.md` (26 levers, distilled from 54 omp docs) identifies the
mechanisms that are **enforced in code** rather than advisory:

| Enforced lever | Why it beats prose |
|---|---|
| **TTSR rule** — `condition:` + `scope: "tool:grep(*)"` + `interruptMode: tool-only` | omp aborts the stream mid-tool-call, discards partial output, injects `<system-interrupt reason="rule_violation">`, retries. Zero prompt-token cost, and **compaction-proof** because rules re-evaluate every stream. |
| **`tool_call` hook veto** — `{ block: true, reason }` | Fail-closed. `reason` becomes the tool error the model reads, so it can name codegraph explicitly. |
| **`bashInterceptor.patterns[]`** | Closes the `bash rg` shell-escape the other two leave open. |
| **`tools.xdev` + per-tool `.enabled`** | Removes schema weight instead of adding prompt text. |

Prose (`APPEND_SYSTEM.md`) stays, but only as the cheap always-true layer beneath the enforced ones.

## Non-goals

- Not dropping Arch support — the package layer becomes distro-dispatched, not Fedora-only.
- No new orchestration engine. Delete duplication, keep the verify gate + commit gate.
- No `git push` / PR from any verb without an explicit ask.
