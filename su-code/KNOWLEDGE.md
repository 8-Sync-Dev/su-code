<!-- 8sync:harness:begin -->
## 🧠 8sync harness

- **Always-on (đọc theo thứ tự; CORE đọc body ngay, SPECIALIST đọc khi task khớp):** codegraph → karpathy-guidelines → ponytail → assp-skill → impeccable → taste-skill → 8sync-cli → image-routing → locate-anything.
- **Cách tận dụng:** codegraph = explore code (query/callers/callees, không grep) · karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · impeccable = design CHUẨN, BẮT BUỘC khi UI/design (đọc body lúc đó) + taste chống slop.
- **Output lớn (>~50 dòng) → BẮT BUỘC `headroom_compress`** trước khi vào context.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (Unreleased) + ghi học được vào file này (prefix `validated:` nếu test/build xác nhận, `hypothesis:` nếu chưa).
<!-- 8sync:harness:end -->

# KNOWLEDGE (8sync managed — append-only)

## Learnings (append-only — ghi DƯỚI đây; KHÔNG sửa block `8sync:harness` ở trên)
_(consolidated 45 dòng cũ → su-code/archive/KNOWLEDGE-1786671697.md)_
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

## v0.56.0 — `8sync hz`, `8sync lcd`, and the duplicate-command upgrade bug (2026-08-09)

- **validated: a "content-gated" migration must compare against every version it ever shipped, not
  the current one.** v0.55.0's un-prefixed-command cleanup only deleted a file byte-identical to
  today's asset — but the same release rewrote every command body, so the compare could never match
  and all 8 `/auto` + `/sx-auto` pairs survived on every upgraded machine. Observed here: each stale
  file hashed exactly to the **v0.54.1** blob. Lesson: when the migrating release also changes the
  content, "is this ours?" needs a frozen digest set, not an equality check against HEAD.
- **validated: 3440x1440@180 was never a GNOME setting.** DRM filters modes by negotiated link
  bandwidth *before* userspace sees them, so a 180 Hz panel on `nouveau` (RTX 5080/Blackwell — no
  DSC, no UHBR) reads as "max 100 Hz" everywhere: `/sys/class/drm/*/modes`, Mutter's
  `GetCurrentState`, GNOME Settings. Ground truth is the panel's EDID range-limits descriptor
  (`0xFD`: vmin 48, **vmax 180**, max pixclk **970 MHz**), which is driver-independent. Comparing
  EDID against the compositor's mode list turns "your max is 100" into a named bottleneck.
  100 Hz at 3440x1440 8bpc ≈ 543 MHz pixclk — exactly the DP HBR2 ceiling, which corroborates it.
- **validated: Mutter mode-setting goes through `busctl`, not `gdbus`.** `busctl --user
  --json=short call … GetCurrentState` returns parseable JSON (gdbus returns GVariant prose), and
  `ApplyMonitorsConfig` takes the explicit signature `uua(iiduba(ssa{sv}))a{sv}` with every array
  count-prefixed — no introspection round-trip. Method `0` = VERIFY (validates, touches nothing),
  `2` = PERSISTENT. It replaces the **entire** layout, so position/scale/transform/primary must be
  echoed back or they reset. Proven live: 100 → 60 → 100 Hz with the layout intact.
- **validated: the fastest mode is often the smallest.** This panel offers 2560x1440@144 while
  running 3440x1440@100. Any "set max refresh" that sorts on Hz alone silently shrinks the desktop —
  filter to the current resolution first.
- **failure: `/sys/class/drm/<card>-<conn>/device` is the DRM card, not the PCI function.** The
  `driver` symlink lives one hop further up, so the first cut of the driver diagnosis printed
  nothing at all. Walk ancestors until one has `driver`; never hardcode the hop count.
- **validated: a broken GUI does not mean broken hardware support.** `lianli-gui` died instantly
  with `Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display` while
  `lianli-daemon` had been up for three hours with all 13 devices enumerated. The whole feature was
  reachable over its IPC socket; the fix for the GUI itself is one env var,
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` (survived 12 s vs 2.7 s to crash). Check the daemon before
  believing the app.
- **validated: `lian-li-linux` IPC.** Newline-delimited JSON on
  `$XDG_RUNTIME_DIR/lianli-daemon.sock`; requests are `{"method":"X","params":{…}}`
  (`#[serde(tag="method", content="params")]`), replies `{"status":"ok","data":…}`. LCD media is
  `SetLcdMedia{device_id, config}` where `config.serial` must equal the device's `device_id`
  (`hid:…`) **and** `device_id` must be `serial:<that id>` — upstream matches on
  `LcdConfig::device_id()`, so any other spelling appends a duplicate instead of replacing.
  `type` is one of `image|gif|video|color|sensor|doublegauge|cooler|custom`; paths must be
- validated: (Kernel default boot mismatch root-causes system driver losses): On Fedora 44 with dual kernels (e.g. 7.1.7 testing vs 6.19.10 official), GRUB defaults to the higher lexical version (7.1.7). If 7.1.7 lacks specific modules like `btusb.ko.xz`, Bluetooth service fails completely (`bluetooth.service` inactive). Fix: `grubby --set-default=/boot/vmlinuz-6.19.10-300.fc44.x86_64` permanently forces the official kernel with full driver stack (`btusb`, `nvidia`, 180Hz) on all reboots.
- validated: (Lian Li TLV2 Wireless LCD config lock vs image pipeline): In Lian Li daemon `config.json`, setting a fan entry to `"type": "color"` with `"rgb": [255, 0, 0]` locks the fan to red and rejects IPC image frames. Updating `config.json` to `"type": "image"` unlocks the fan for realtime LCD sensor dashboard streaming (`/tmp/lcd_m_*.png`).
  absolute (the daemon has its own cwd).
## STEP-0 must be a deny-list, not an allowlist (2026-08-14)

- **failure: mirroring another program's tool list bricks the launcher.** STEP-0 shipped omp a
  `--tools` ALLOWLIST, so 8sync had to name all ~29 built-ins to drop 2. omp 17.3 renamed
  `ast_grep` → `ast_edit` and dropped `github`/`checkpoint`/`rewind`/`security_scan`, and every
  `8sync .` / `8sync ai` died at argv-parse time with `CliUsageError: Unknown tools in --tools`.
  Rule: when you only want N things GONE, use the API that names those N. `omp` has one —
  `grep.enabled`/`glob.enabled` (settings.md §"Individual built-in tools are toggled by their own
  keys") — passed per-launch as a `--config` overlay so `--no-step0` and bare `omp` stay clean.
- **failure: omp's "Valid tools:" error list is NOT a stable inventory.** It is a snapshot of
  whatever is registered at validation time, and MCP/xdev registration is async: two runs of the
  same binary in the same cwd returned 64 vs 65 names, and a real project returned 35 built-ins
  with zero `mcp__*` while `/tmp` returned 17 built-ins plus 47 `mcp__*`. Any allowlist derived
  from that probe — and the old `step0_tool_drift()` doctor check built on it — is unsound.
  Verify enforcement instead: with the overlay, `omp --tools grep,glob -p ""` must be REJECTED.
  `--tools` is validated before any provider call, so the check is offline and free.
- **validated: a crashing launcher reads as a data-loss bug.** `8sync . core` printed
  `→ resume session 'core'`, omp exited before drawing a frame, and the user typed
  `omp --continue` — which reads omp's DEFAULT per-cwd store (`~/.omp/agent/sessions/<key>/`),
  not the named store (`~/.config/8sync/sessions/<key>/<name>/`). The named session looked lost.
  The two stores are correctly isolated (proven: `--session-dir` on an empty dir starts fresh and
  does not leak into the default store), so the cure was fixing the launch, plus printing the
  `omp --session-dir … --continue` line that reopens a named session without 8sync.
- **validated: the both-directions registry test earns its keep again.** `every_asset_skill_is_
  registered_or_explicitly_opt_in` caught 4 foundation skills (tauri-v2, nextjs-app,
  encore-eino-go, ai-microservice-design) embedded in the binary and advertised in AGENTS.md but
  absent from `BUNDLED_SKILLS`, so `8sync harness` never deployed them to `~/.omp/skills/`.
