<!-- 8sync:harness:begin -->
## 🧠 8sync harness

- **Always-on (đọc theo thứ tự; CORE đọc body ngay, SPECIALIST đọc khi task khớp):** codegraph → karpathy-guidelines → ponytail → assp-skill → impeccable → taste-skill → 8sync-cli → image-routing → locate-anything.
- **Cách tận dụng:** codegraph = explore code (query/callers/callees, không grep) · karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · impeccable = design CHUẨN, BẮT BUỘC khi UI/design (đọc body lúc đó) + taste chống slop.
- **Output lớn (>~50 dòng) → BẮT BUỘC `headroom_compress`** trước khi vào context.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (Unreleased) + ghi học được vào file này (prefix `validated:` nếu test/build xác nhận, `hypothesis:` nếu chưa).
<!-- 8sync:harness:end -->

# KNOWLEDGE (8sync managed — append-only)

## Learnings (append-only — ghi DƯỚI đây; KHÔNG sửa block `8sync:harness` ở trên)
_(consolidated 1 dòng cũ → su-code/archive/KNOWLEDGE-1786020546.md)_
  1 060 840 B**, both = 3 325 728 B. Cross-check `web-only + toolstats-only − minimal` =
  6 409 464 ≈ full (2 320 B of shared code double-counted) → the numbers are coherent.
  **A `minimal` build (3.08 MB) is already UNDER the 4 MiB budget, and so is toolstats-only
  (4.14 MB, −1.19 %).**
- failure (`cargo bloat` under-attributed SQLite by ~26×): it put `libsqlite3_sys` at **40 KiB**
  of `.text`; the A/B says the `toolstats` gate really costs **1 060 840 B**. `cargo bloat`
  attributes `.text` by symbol only — blind to `.rodata`, static tables and the true C-blob
  footprint — and prints *"numbers are a result of guesswork"*. Rule: `cargo bloat` may only
  RANK suspects; every load-bearing number MUST come from an A/B build.
- gotcha: `cargo build --release --no-default-features` WITHOUT `--target-dir` overwrites
  `target/release/8sync` with the lean binary — trivially installed onto `PATH` by a later
  `cp target/release/8sync $(command -v 8sync)`. Hit live in M1. Always give a variant build its
  own `--target-dir` (which `scripts/size-report.sh` does), and re-run the default build before
  installing.
- validated (feature gating is a MEASURING INSTRUMENT, not a diet): with `default = ["web",
  "toolstats"]` the shipped binary is byte-identical, so gating alone delivers **0** user-visible
  savings. Its payoff is that it makes elimination decisions data-driven. Next target chosen from
  the data: `toolstats` spends 1.01 MiB of bundled SQLite C on an append-only call log answering
  `COUNT`/`GROUP BY` over a few thousand rows — replaceable with a flat file + in-memory
  aggregation at zero feature loss. `web`'s 2.16 MiB is mostly the irreducible `web/dist` embed.
- validated (M2 `lean-binary` — the elimination that gating only pointed at): default binary
  **6 407 848 → 4 859 696 B (−1 548 152 B, −24.2 %)** with **zero feature loss**. Two deletions:
  (a) `rusqlite` −1 035 384 B, (b) `elkjs`→`@dagrejs/dagre` −512 768 B. Minimal build 3 109 496 B
  (−25.86 % vs budget); `web` gate now 1 750 136 B. Remaining overshoot 665 392 B lives in
  `assets/` (impeccable 2.1 MB) and the dashboard — no easy owner left.
- validated (`toolstats` never needed a database): `ingest` opened with `DELETE FROM calls` and
  re-parsed every session JSONL each run, so nothing ever persisted — the module's own
  "idempotent, keyed on (session, seq)" comment was FALSE and `INSERT OR IGNORE` was unreachable
  as a dedupe path. 1 MB of embedded SQLite C answered `COUNT`/`GROUP BY` over rows the same
  process had just built. Now one pass → four `HashMap`s. **Read what a dependency actually does
  before assuming its cost is earned.**
- gotcha (byte-identical output across a store swap): SQLite `ORDER BY <count> DESC` leaves ties
  in table-scan = insertion order, NOT alphabetical — the report printed `write×2,
  generate_image×2`. Reproduced by tie-breaking on first-appearance index (`ranked()` in
  `toolstats.rs`). Verify such a swap under FROZEN input: rebuild the old binary in a detached
  `git worktree`, copy the session tree to `/tmp/th/.omp/agent/sessions/<slug-for-that-HOME>`,
  run both with `HOME=/tmp/th`, `diff`. Two live runs differ only because the session grew.
- validated (`elkjs` = 85 % of the dashboard bundle): `elk.bundled.js` is 1 606 238 B of a
  1 891 858 B chunk — a GWT-compiled Java layout engine for two `elk.algorithm: layered` calls.
  `@dagrejs/dagre` covers `rankdir` LR/TB + `nodesep` identically → bundle **478 704 B (−75 %)**.
  Porting notes: dagre reports node CENTRES (elk = top-left, so subtract half w/h), and dagre
  INVENTS a node for an unknown edge endpoint (elk ignored those) — filter edges to known ids.
- failure (splitting a lazy chunk does NOT shrink the binary): `await import("elkjs/…")` makes
  Vite emit a separate chunk, but `rust-embed` embeds the whole `web/dist` tree, so embedded
  bytes are unchanged — and top-level `await` breaks the Vite build against its browser targets
  anyway. Only a SMALLER dependency helps when the whole output directory is embedded.
- failure (project detection stamped THIS repo's root — found live, fixed same session): a
  directory merely NAMED `su-code` satisfied the `su-code` marker in
  `discover::detect_current_project_root` and `global::is_omp_project`, so its PARENT looked like
  an omp project. This checkout is `~/Projects/tools/su-code`, so any `8sync harness global` run
  with cwd `~/Projects/tools` stamped `<parent>/su-code/` — i.e. **the repo root** — with a blank
  `STATE.md`/`KNOWLEDGE.md`/`PLAYBOOKS.md`/… and a 74-entry `skills/` tree. Caught only because
  `git add -A` swept them in and the gitleaks pre-commit hook fired on the `senior-security`
  skill's regex literals. Fix: `is_memory_dir()` requires the dir to actually CONTAIN memory
  (`skills/` or one of `STATE.md`, `KNOWLEDGE.md`, `PROJECT.md`, `PLAYBOOKS.md`, `skills.toml`); `is_omp_project` moved to
  `discover` so both paths share it. Verified 3 ways: bare `su-code/` → untouched · `AGENTS.md`
  repo → stamped · memory tree without `AGENTS.md` → still stamped.
- gotcha: **`brand::NS` is `"8sync"`, NOT the memory-dir name.** `NS` is the config/artifact
  namespace (`~/.config/8sync`, `8sync-engine.ts`, AGENTS sentinels); the project memory dir is
  the separate literal `"su-code"` (hardcoded in `deploy::mirror_global_to_local`,
  `memory::migrate_legacy_layout`). Writing `is_memory_dir(&p, brand::NS)` compiles, passes the
  "bad case" test, and silently breaks detection for every real project — it looks for a dir
  named `8sync`. Now a named constant `discover::MEMORY_DIR`. Test the POSITIVE case too; a fix
  that only proves the bug is gone can have deleted the feature.
- validated (`cargo-zigbuild` replaces `cross` for `aarch64-unknown-linux-musl`): zig 0.16.0 +
  cargo-zigbuild 0.23.0 → **31.91 s**, `ELF 64-bit LSB executable, ARM aarch64, statically
  linked, stripped`, **4 151 328 B** (already UNDER the 4 MiB goal; musl-static beats the glibc
  host build). Local install path when there is no distro zig: `uv tool install ziglang` →
  `python-zig`, symlinked as `zig` on PATH. CI uses `mlugg/setup-zig@**v2**` (v1 is stale — the
  README's own example is `@v2`) pinned to `version: 0.16.0`, plus `taiki-e/install-action` for
  cargo-zigbuild.
- failure (the `cross` leg shipped a PLACEHOLDER dashboard): `build.rs` builds the Vite FE by
  shelling out to bun/pnpm/npm and, finding none, **silently writes `FALLBACK_HTML` and keeps
  going** (`cargo:warning` only). `cross` runs inside a Docker image with no JS toolchain, so the
  released `linux-aarch64` asset embedded the stub. Moving to `cargo-zigbuild` (runs on the
  runner, npm present) fixes it. Lesson: a build step that degrades to a stub on missing tooling
  must be checked on EVERY leg, not just the one you build locally.
- validated (a budget must be a GATE): `AGENTS.md` §8 carried "< 4 MB stripped" that nothing
  enforced, and the binary drifted to 6 407 848 B — **52 % over** — unnoticed. `scripts/size-gate.sh`
  now runs per asset in `release.yml`: hard-fail above the 5 MiB **ceiling**, warn above the
  4 MiB **goal**. Ceiling deliberately sits ABOVE today's size — a gate that is already red gets
  ignored; ratchet it down as `size-report.sh` shows headroom. Test both directions
  (`CEILING=4000000` must exit 1) or you have shipped a gate that cannot fail.
- gotcha (`8sync harness audit` is heuristic, and chasing it to zero is WRONG): of 18 stale-path
  findings only 2 were real. The other 16 were `CHANGELOG.md` entries naming paths that were
  correct when written (`agents/STATE.md` pre-rename, `verbs/skill.rs` pre-directory), omp's own
  docs (`docs/providers.md`), and source-layout blocks with paths relative to `crates/cli/src/`.
  Rewriting a changelog to satisfy a path scanner falsifies history. The tool prints
  *"report-only — illustrative paths can false-positive"*: fix real errors, record the rest as
  reviewed, and NEVER report a green that required faking.
- decision (REJECTED `universal2` for macOS, reversing the M0 brief): a fat binary carries both
  slices, so every Mac user downloads ~2× to save the project one CI leg — backwards for a
  size-reduction effort — and it renames assets `install.sh` resolves by `${os}-${arch}`. Keep
  `darwin-x86_64` + `darwin-arm64` separate.
- failure (omp 17 rejects mutable zod defaults — `8sync-engine.ts` ParseError): after `omp update` to
  17.x, every `omp` load printed `Warning: Failed to load extension … ParseError: A mutable default
  value must be specified as a factory`. omp's schema validator (`HF0` in cli.js) throws when a zod
  `.default(value)` has `typeof value === "object"` (array/object literal) and is not a Date — so
  `.default([])` and `.default({})` are banned; the value MUST be a factory: `.default(() => [])`.
  Rule: primitive defaults (`false`/`0`/`""`/`3`) are fine; any array/object default must be a thunk.
  Fix was one site: `assets/extensions/8sync-engine.ts:146` (`verify: z.array(z.string()).default([])`
  → `.default(() => [])`). `8sync-workflow.ts` had no defaults. The asset is rust-embedded, so the
  fix needs `cargo build` + `8sync harness` to redeploy; copied directly to the live projects for
  immediate relief. Verified: `omp -p "ok"` in the failing project loads with ZERO extension warnings.
- gotcha (a failing extension does NOT explain lost `--continue` history): omp loads each extension
  in its own try/catch and prints `Warning: Failed to load extension` then CONTINUES — session
  restore is a later phase. The `8sync-engine` extension only registers tools (no `session_start`
  handler touching the branch), so its parse failure could not drop chat history. Treat the warning
  and the history loss as TWO symptoms of the `omp update`; omp's changelog shows `--continue` has
  its own bug class (resume-into-subagent-transcript, session-resume hang, auto-thinking dropped —
  all fixed by 17.2.9, so a remaining loss is a fresh regression). Don't claim the factory fix
  resolves the history loss; ask the user to retest `--continue` now and diagnose separately.
- **validated (STEP-0 enforcement — prose lost 3×, so the fallback was REMOVED):** measured on this
  machine with `8sync harness toolstats`: STEP-0 was connected and genuinely callable (`xd://mcp__…`
  probed live: cbm `list_projects` returned 5 indexed projects, headroom `compress` returned a hash)
  and STILL unused — `cbm 0 · serena 0 · headroom 0` agent calls, every lookup falling to `read`/`grep`.
  **Lesson: a zero-friction built-in always beats an instruction; if a rule keeps losing, delete the
  thing it competes with rather than writing the rule louder.** omp's enforcement surface, mapped from
  the binary: (1) `--tools=<list>` CLI allowlist — **BUILT-INS ONLY**, verified by capturing a real
  provider request (`omp -p ""` 400s and logs the full body to `~/.omp/logs/http-400-requests/`): under
  `--tools=read,bash,todo` the request still carried **48 `mcp__*` tools + `engine_*` + `wf_state_*`** —
  so dropping `grep`/`glob` costs zero MCP. There is NO persistent `tools.enabled` key; only the launch
  flag. (2) `bashInterceptor.patterns` — user-configurable, shape `{ pattern: <regex>, reason: <string> }`
  (from omp's own `explicitExclusions` schema + runtime `Blocked by bash pattern: ${match}`); omp itself
  uses this pattern to redirect raw `git` to its structured git tool, which is exactly the raw→structured
  redirect we needed. (3) Hooks are **message/session-scoped only** (`before_agent_start`,
  `session.compacting`, post-hoc `on_tool_execution_success/failed`) — there is **no pre-tool-call guard
  event**, so "block `read` until codegraph ran" is NOT implementable as a hook; the allowlist is the
  only pre-execution lever. (4) `--advisor` is passive (notes, never blocks).
  Trick worth reusing: **a rejected request still logs its full tool array** — the cheapest way to prove
  what omp actually ships to the model, with no successful model call needed.
- failure (STEP-0 v1 shipped BROKEN — `8sync ai` bricked; "verified" checked the wrong thing):
  the `--tools` allowlist listed `python` + `notebook`, taken from omp's `--help` "Available
  Tools" section. That section is STALE. omp 17.2.9's validator rejects both and **exits**:
  `Error: Unknown tools in --tools: python, notebook`, so EVERY `8sync ai` / `8sync .` launch
  died before omp started. The commit claimed "verified: allowlist embedded in binary ✓" — it
  verified the string was in the binary, never that omp ACCEPTS it. Rule: a flag is verified only
  by running the program end-to-end and observing the effect, never by grepping the binary for
  the value. Authoritative tool list = the one omp prints in that error, not `--help`:
  read, bash, edit, ast_grep, ast_edit, ask, debug, eval, github, glob, grep, lsp, inspect_image,
  browser, computer, checkpoint, rewind, security_scan, task, hub, todo, web_search, write,
  memory_edit, retain, recall, reflect, learn, manage_skill, yield, goal.
- gotcha (`--tools` is an ALLOWLIST and omp has NO deny-list): `tools.blocked` in the schema is a
  telemetry counter, not config. So `--tools` must enumerate everything you want to KEEP; every
  omitted name is silently disabled. STEP-0 v1 omitted 17 real tools including `recall`/`retain`/
  `reflect`/`memory_edit` (the whole mnemopi memory stack), `hub`, `eval`, `ast_grep`/`ast_edit` —
  fixing only `python`/`notebook` would have silently killed memory. Correct list = validator list
  − `grep` − `glob` − `computer` (omit `computer` so it keeps its default-disabled state).
  Verify with the real provider request, not the model's self-report: omp logs a rejected request
  body to `~/.omp/logs/http-400-requests/` (`omp --tools <list> -p ""`), which shows the exact tool
  array. NOTE the names there are `_`-prefixed (`_read`, `_recall`) — matching bare names gives
  false negatives.
- failure (bashInterceptor silently blocked NOTHING — wrong rule shape + a self-disabling gate):
  8sync wrote `{pattern, reason}`. omp's real shape is `{pattern, tool, message}` (+ optional
  `flags`), and its matcher is:
  `for ({rule:p, regex:o} of rules) { if (!toolNames.includes(p.tool)) continue; ... }`
  A rule with no `tool` key hits `includes(undefined)` → false → **skipped unconditionally**, so
  the interceptor was inert; verified live, `rg main main.rs` ran fine. Worse, the obvious repair
  is also wrong: omp's own default rule for `grep|rg` carries `tool:"grep"`, and STEP-0 removes
  `grep` from the allowlist — so the stock rule disables itself exactly when it is needed
  (catch-22). Fix: point every rule's `tool` at something guaranteed PRESENT — `lsp`. Verified
  live after the fix: `rg` and `grep -r` are refused with `Blocked: STEP-0: …`, single-file
  `grep main main.rs` still runs.
- gotcha (omp REWRITES `~/.omp/agent/config.yml` in its own style): it re-quotes scalars and adds
  trailing spaces, so a byte-exact `s.find(LAST_BLOCK_WE_WROTE)` migration never matches after a
  single omp run — the deploy then mistakes its own stale block for a user-authored one and skips
  forever. Identify an 8sync-owned block by a content SIGNATURE (`STEP-0`) and replace the whole
  block (start of key → next top-level key); skip only when the signature is absent.
- validated (`--continue` history loss is NOT reproducible — the likely cause was the bricked
  `--tools` flag): tested end-to-end on omp 17.2.9 — `omp -p "remember BANANA47"` then
  `omp --continue -p "what codeword?"` returns `BANANA47`, and the `8sync ai` resume path returns
  it too. So omp's session restore is fine. The reported symptom almost certainly came from
  STEP-0 v1's invalid `--tools`: `8sync ai` / `8sync .` DIED at launch, the user fell back to a
  bare `omp`, got a fresh session, and it looked exactly like "--continue lost my history".
  Lesson: when a wrapper dies at launch, the user experiences it as the wrapped tool misbehaving —
  always test the wrapper's own exit path before blaming upstream.
- failure (`codegraph callers` gives FALSE NEGATIVES — do not trust it alone): on a clean, freshly
  rebuilt full index it reported `No callers found for "ensure_bash_interceptor"` while two real
  call sites existed (`harness/global.rs:42`, `harness/init.rs:66`). Not staleness — reproduced
  after a full rebuild. Narrowing: `ensure_recall_hook` is called from BOTH `up.rs::refresh_once`
  and `global.rs::global_pass`, and codegraph reported only the `up.rs` one, so the misses cluster
  by CALLER (nothing inside `global_pass` resolves), not by callee. `let _ =` discard bindings are
  NOT the cause — the plainly-called `ensure_codebase_memory_mcp` is missed too. codegraph is a
  prebuilt external binary (`~/.local/bin/codegraph`, no local source), so this is upstream.
  Workaround now in APPEND_SYSTEM: answer "who calls X" with `mcp__serena_find_referencing_symbols`
  (it found both); treat `codegraph callers` as a second opinion only, never as proof of absence.
- failure (never `rm -rf .codegraph` to force a rebuild — self-inflicted, cost ~5 min): the
  directory holds the exclusion config, so deleting it makes the next `codegraph index .` walk the
  whole tree (16,465 files incl. `target/`) instead of the ~6k it normally indexes; it blew a
  300 s timeout, got interrupted, and left NO index at all (`codegraph query` returned nothing).
  Use `codegraph index --force` ("rebuild the full index from scratch") or just `8sync harness`,
  which re-inits with the right exclusions. Recovery: `8sync harness` in the background.
- validated (a guard you never fired is theatre — test the NEGATIVE case): `8sync doctor` now
  probes omp's live validator list and diffs it against `STEP0_TOOLS`. Proving it prints
  `✓ matches` is NOT enough, since a probe that silently fails to parse also prints nothing bad.
  Proof required injecting real drift: adding `bogus_tool` made doctor warn
  `STEP-0 allowlist REJECTED by omp: bogus_tool — every 8sync ai / 8sync . will fail to launch`,
  then reverting restored `✓ matches`. The probe is free and offline — omp validates `--tools`
  before contacting any provider, so `omp --tools __8sync_probe__ -p ""` returns the authoritative
  list instantly.
- validated (an interceptor regex for "block recursive grep" has THREE traps, not two):
  the first attempt `.*(-[rR])` matched `-r` inside words (`my-report`, `build-Release`),
  over-blocking the single-file/log grep the rule promises to allow. The second attempt
  (`\s-`, "require a space before the dash") over-corrected: `grep\s+` ALREADY consumes the
  space, so a first-token `grep -r` has no remaining `\s-` and the recursive form was ALLOWED
  through — a regression caught by live testing before commit, not by the unit cases (which
  used clean inputs). The double-dash long flags (`--color`, `--directories`) contain an `r`
  and the short-flag matcher naively matched them too. Correct shape: negative lookbehind
  `(?<![A-Za-z0-9-])` (the dash must NOT be preceded by a word char OR another dash — that
  also kills matching at the SECOND dash of `--color`), plus a separate literal alt for the
  real long recursive flags (`--recursive`, `--dereference-recursive`). Verified 19/19 in
  Python BEFORE touching source. Lesson: a "token boundary" is a lookbehind, not a `\s`
  requirement — and the `\s` you anchored on has already been eaten by the prefix.
- validated (config-block migration must be scan-all, not find-first): finding the FIRST
  `bashInterceptor:` key and acting on it leaves a later copy (e.g. a user block at byte-0
  shadowing our stale block lower) in place as a duplicate YAML key — omp then rejects the
  file. The safe shape: enumerate EVERY `bashInterceptor:` block, classify each by the `STEP-0`
  signature (ours) vs absent (user's), remove ALL owned blocks from the END backwards (so
  earlier offsets stay valid), then append one fresh. This is invariant to ordering and
  dedups correctly. A user-authored block is never touched by construction.
- failure (happy-path self-review has the same blind spot as happy-path tests): the grep
  over-block shipped because my self-test inputs (`grep main main.rs`) had no hyphens — the
  exact class the bug affected. An independent reviewer + LIVE two-direction testing (does
  the allowed case still run? does the blocked case still refuse?) is what caught both the
  original defect and my first-fix regression. Cost: ~12 min reviewer. Verdict: earned.

- failure (a `^`-anchored shell interceptor is barely an interceptor): `^\s*grep` only fires when
  the tool is the FIRST thing in the command string, so `cd src && grep -r foo .` and
  `cd crates/cli && rg TODO` — the single most natural shape an agent writes — sailed straight
  through, as did `LC_ALL=C grep -r`, `sudo grep -r`, `time rg`, `cat x | grep -r y`, and
  `; do grep -r …`. Fix: match at a COMMAND POSITION — `(?:^|[;&|])` plus optional `\` escape,
  env assignments, and wrapper words (`sudo|time|command|nohup|env|xargs|do|then|else`).
  Deliberately do NOT put `(` in the separator class and do NOT treat `do`/`then` as separators:
  that is what made `git commit -m '(rg removal)'` and `echo "do rg later"` get blocked. Quote
  parity is NOT expressible in a lookbehind (`(?<!["'][^"']*)` wrongly allows `echo "x" && rg y`),
  so shrinking the separator set beats trying to detect quotes.
- validated (scan the OPTION CLUSTER, never `.*`, when a regex must find a flag): to decide
  "is this grep recursive", walking `.*` is wrong twice over — it reads flag-looking text inside
  the search pattern (`grep " -r " f.txt`, `grep 'make -r' build.log` were blocked, contradicting
  the rule's own message) and it crosses command boundaries (`grep foo a.txt; ls -r` blocked
  because it found `ls -r`). Correct shape: after the tool name, consume a run of shell words that
  are neither quoted nor containing `;&|`
  (`(?:-(?!-\s)[^\s;&|]+|[^\s;&|'"-][^\s;&|]*)\s+`), then require the flag. Quote-safety then
  falls out for free — a token starting with `'` or `"` simply cannot be consumed. 48/48 cases.
- failure (blocking `rg` just moves the habit to `git grep`): an allowlist-style shell guard must
  enumerate the SUBSTITUTES, not one tool. `git grep -rn TODO` (recursive by default, the obvious
  fallback once rg is gone), `egrep -r`/`fgrep -r`, bare `fd '\.rs$'` and `fd -t f` (that is fd's
  normal invocation and it is a full recursive walk), and `find -path`/`-regex` were all open.
  Tools with no single-file mode worth preserving (`rg`, `fd`, `git grep`) are blocked outright;
  only `grep` needs flag analysis.
- failure (`Bun.YAML.parse` does NOT reject duplicate mapping keys — it takes the LAST): the code
  assumed a duplicate top-level key would make omp fail loudly, so appending an 8sync block next
  to a user-authored `bashInterceptor:` was thought to be self-announcing. In reality omp
  (17.2.10, `var {YAML:jQf}=globalThis.Bun`) silently keeps the last key, and 8sync always appends
  last — so every rule the user wrote was voided on each `8sync harness`, with no error anywhere.
  Rule: when a managed writer cannot merge, it must DETECT the user's block and BAIL OUT with a
  warning, never append-and-hope. Never assume a parser is strict; verify what it actually does.
- validated (`8sync up` was Linux-only — hard-coded `-linux-x86_64` asset + extension-less
  `~/.local/bin/8sync` dest + plain rename): on Windows this downloaded the Linux binary, wrote a
  file with no `.exe`, and tried to `rename` over a running `.exe` (forbidden). The extension-less
  file then made Windows pop the "Select an app to open '8sync'" picker. Fix in `selfup.rs`: derive
  the asset label from `platform::os()`+`std::env::consts::ARCH` (mind macOS arm = `arm64`, not
  `aarch64`), install to `std::env::current_exe()` (replace 8sync wherever it lives, all OSes), and
  do a Windows-safe replace (rename the live `.exe` aside to `.8sync.old.<pid>`, then move the new
  one in). Rule: any self-updater must key the asset name, install path, AND replace strategy off
  the target OS — a `#[cfg(unix)]`-only path silently ships a broken updater to every other OS.

- validated: **Encore Go apps compile với `go` thuần trong podman — không cần Docker/encore codegen** (2026-08-07, feature ai-router-hub M0). `go mod tidy && go vet ./... && go build ./... && go test ./...` trong `docker.io/library/golang:1.24` (mount source + named volume `/go` cache) PASS trên app có `//encore:api`/`//encore:authhandler` — các directive chỉ là comment với `go` thuần, hàm là Go thường. `encore.dev` resolve qua go mod (v1.57.13). ⇒ verify **Go-level** (compile/type/logic) được ở máy rootless-podman-no-Docker; chỉ `encore run/build/test` mới cần Docker (dựng runtime + codegen). Cách rẻ để verify Encore mà không dựng Docker.
- failure: **`errs.Code(err)` panic dưới `go test` thuần** (encore.dev/beta/errs builder.go:121 `doPanic` — helper cần Encore runtime). Fix: soi field trực tiếp `err.(*errs.Error).Code == errs.Unauthenticated` (construct + đọc field KHÔNG cần runtime). Rule: test Encore package bằng `go test` thuần chỉ chạm hàm/logic thuần; mọi helper `errs.*`/`rlog.*`/`auth.UserID()` cần runtime → dùng `encore test` (Docker) hoặc né bằng field access.
- failure: **grep-based endpoint-lister tự match comment của chính nó** (verify-public-endpoints.sh: regex `//encore:api public...path=` quét cả `*.md`/`*.sh` bắt phải path ví dụ `/x` trong comment). Fix: `grep --include='*.go'` — endpoint chỉ định nghĩa trong Go source. Rule: script "liệt kê directive từ source" phải giới hạn theo đuôi file, không quét doc/self.
- validated: **Encore API type risk — `interface{}`/`any` trong request/response có thể bị Encore schema-parser từ chối** (go build pass nhưng `encore run` fail). Chưa xác nhận (defer Docker box). Nếu envelope dùng `Result interface{}` → sẵn sàng đổi `json.RawMessage`/type cụ thể. Kiểm TRƯỚC khi tin `encore run`.
- validated (multi-session, 2026-08-08): **omp exposes exactly the session levers 8sync needs — build a thin layer, don't reinvent.** `omp --session-dir <dir>` scopes storage+lookup; `--continue` then resumes the latest conversation *in that dir* → one dir per named session = isolated conversations with ZERO uuid tracking (verified: captured omp argv shows `--session-dir …/<name> … --continue`). Fresh session = launch WITHOUT `--continue`; resume = WITH. Always route through `models::ModelConfig::resume_flags()` so the STEP-0 `--tools` allowlist + `--advisor` survive (a bare `Command::new("omp")` would silently drop them).
- validated (worktree-per-session merge, ECC blueprint, MIT): the whole isolation+merge engine is `git` CLI shell-out, no crate. `git worktree add -b 8sync/<name> <path> HEAD` per session; land via read-only `git merge-tree --write-tree <target> <branch>` (exit code = conflict truth; parse conflicted files by taking non-blank lines after the tree-oid and dropping any containing a space to skip "Auto-merging …"/"CONFLICT …" info lines) → `git merge --no-edit` → on conflict `git rebase <target>` in the worktree (auto `rebase --abort` on fail) → `git worktree remove` + `git branch -d`. Sequential merges make the target advance so branch-vs-branch conflicts surface naturally. `git branch -d` (not -D) keeps unmerged work safe by default.
- gotcha (multi-session): `8sync . new` runs `seed_project_context` on the MAIN repo, writing untracked `AGENTS.md`/`su-code/`/`CLAUDE.md` when missing → a fresh test repo then looks "dirty" and `merge` (which refuses a dirty main tree) balks until you commit them. Real repos already track these → fresh-repo artifact, not a bug. Smoke tests must `git add -A && commit` after the first `new`.
- validated (testing an omp-launching verb headlessly): stub `omp` with `printf '#!/bin/sh\nexit 0\n'` on `PATH` + isolated `XDG_CONFIG_HOME` → exercises the real registry + git engine without a TTY hang; assert on recorded argv for launch-flag correctness. `git init -qb main` for a deterministic base-branch name.
- failure (fedora-harness, 2026-08-08): **`8sync setup` aborted at step 3 of 8 on every non-Arch Linux.** `setup.rs` gated the AUR-helper step on `platform::Os::Linux` instead of the Arch family; on Fedora `pacman_install_safe` failed, and `try_step` PROPAGATES in strict (non-`yolo`) mode, so the `?` killed the run before `codegraph`/`configs`/`skills`/`codegraph-skill`. Symptom looked like "skills never install on Fedora"; cause was one wrong gate five steps earlier. Rule: an OS check is not a distro check — gate package work by distro family, and remember `try_step` only swallows errors when `yolo` (`--full`/`--profile`/`--community`).
- failure (fedora-harness): **8sync wrote machine-absolute skill paths into every project's `AGENTS.md`.** `skill/inject.rs` built `p.join(entry)` from a cwd-derived PathBuf, emitting `/home/alexdev/...` / `/home/alexng/...` into the sentinel block. On any other clone those files do not exist, so omp was told to read nothing and silently skipped the skills — the real reason "omp hay bỏ qua skills". The on-demand tier 15 lines below already built relative strings, so the correct pattern was sitting next to the bug. `harness audit` structurally could not catch it: it `continue`d on every `/`-prefixed token. Rule: anything written into a COMMITTED file must be repo-relative (or `~/`-anchored for genuinely-global paths), and the audit that guards it must flag `/home/`,`/Users/`,`/root/` while still ignoring `/etc`,`/usr`,`/tmp`.
- validated (fedora-harness): **prose does not steer omp; enforced levers do.** `APPEND_SYSTEM.md` asking for codegraph/serena/cbm is advisory and gets ignored under load. The mechanisms omp implements in CODE are: a TTSR rule (`condition:` + `scope: "tool:grep(*)"` + `interruptMode: tool-only`) that aborts the stream mid-tool-call and re-injects — zero prompt tokens and compaction-proof because rules re-evaluate every stream; a `tool_call` hook returning `{block:true, reason}` (fail-closed); and `bashInterceptor.patterns[]` to close the `bash rg` escape. Distilled reference lives in `su-code/omp-reference/LEVERS.md` (26 levers). Gotcha: an interceptor whose `tool:` names an MCP server is SILENTLY SKIPPED when that tool is absent — point `tool:` at something always present (`lsp`) and name the real replacement in `message`.
- validated (fedora-harness): **capability-gate any enforcement you deploy.** Blocking `grep` on a box with no codegraph/serena/cbm dead-ends the session. Gate at deploy time (install the rule/interceptor only when a replacement is actually reachable, delete both when it is not) because omp cannot express a capability predicate inside `condition:`/`pattern`. TTSR `repeatMode: once` also caps the blast radius at one restarted turn.
- gotcha (fedora-harness): **`dnf` is not `pacman` and needs LESS code.** RPM transactions are atomic, so the hand-rolled pacman snapshot+`-Rns` rollback must NOT be ported; use `dnf history undo <id>`. But a PLAN of two dnf transactions (install missing, then upgrade outdated) is not atomic as a whole — if the upgrade fails after the install committed, you must undo the committed one. Also: on dnf5 the `copr` subcommand comes from **`dnf5-plugins`**, not the dnf4-era `dnf-plugins-core`, and Fedora 44's `/etc/os-release` has **no `ID_LIKE`** line, so distro detection cannot rely on it.
- validated (fedora-harness): **6 agents editing one Rust crate in parallel compiled on the FIRST try** because the exact trait/struct signatures were fixed in the batch `context` up front and every agent was told to skip `cargo build`. Contract-first + no mid-flight validation is what makes wide fan-out safe; agents then self-corrected over IRC (one caught that `load_all()` returns `HashMap`, not a slice, and refused my wrong signature).
- gotcha (fedora-harness): **`.gitignore` bare `reference/` silently swallows any `reference/` dir at any depth.** Distilled omp docs written to `su-code/reference/omp/` vanished from `git status`; relocated to `su-code/omp-reference/`. `assets/skills/impeccable/reference/` survives only because it was tracked BEFORE the rule existed. Verify a new doc dir is actually tracked (`git check-ignore -v <file>`) before assuming it is committed.
- gotcha: a "dead code" warning can be a real bug in disguise — `undo` was unused because nothing wired it to the partial-failure path, and deleting the warning properly meant fixing the rollback. Prefer wiring over `#[allow(dead_code)]`.

## v0.54.0 release closeout (2026-08-09)

- **validated: an independent reviewer pass before a tag pays for itself.** Two `task` reviewers
  (`reviewer` + `security-reviewer`) on `git diff origin/main..HEAD` found 28 issues the author
  (me) had missed, including one that would have broken the release outright and two exploitable
  bugs. Cost ~18 min wall-clock, run in parallel with the test suite. Do this for every tag.
- **failure: `--full`/`--yall` hardcoded the maintainer's personal bundle** (`"alexdev"`) as the
  meaning of "install everything". Any teammate running it got Lian Li chassis drivers, a
  Vietnamese IME and DisplayLink DKMS. Root cause: three separate hardcoded profile lists that
  could disagree with the `Visibility` enum. Fix = one `offered_profiles()` source of truth, and
  `Visibility` now defaults to `Personal` so a forgotten marker fails CLOSED.
- **failure: a profile's own `post_install` guard is too late to stop a bad install.**
  `profile::apply` installs packages first, so `nvidia`'s "no NVIDIA GPU detected" check ran after
  the driver stack was already on an AMD box. Gates must live in `resolve`'s walk, per bundle
  member, before any package work.
- **failure: `std::thread::spawn` cannot do background work in a CLI.** The M5 "non-blocking"
  update check spawned a detached thread; the runtime kills it when `main` returns, and
  `touch_check()` had already burned the 6-hour window — so the notice could never print. A CLI
  that needs work to outlive the command must re-exec itself as a detached child process and read
  the result from cache on the next run.
- **failure: `JSON.stringify` is not shell quoting.** It escapes `"` and `\` but a double-quoted
  bash word still performs `$()`/backtick substitution. `git commit -m ${JSON.stringify(msg)}` via
  `bash -lc`, with `msg` model-supplied, was arbitrary code execution. Use argv (`spawnSync(file,
  args)`); never build a shell string from data.
- **failure: package names spliced into a `sudo` argv with no `--`.** A profile entry starting
  with `-` becomes a package-manager FLAG: `--hookdir=` gives alpm attacker-written root hooks.
  Validate AND emit `--`.
- **validated: `if command -v X; then X …; fi` is a fail-OPEN security gate** — it exits 0 when X
  is absent. Probe presence separately and say so when a check did not run.
- **validated: test the negative direction too.** The existing guard only checked
  list→asset, so two new skills silently never deployed. Every registry needs both directions.
