<!-- 8sync:harness:begin -->
## 🧠 8sync harness

- **Always-on (đọc theo thứ tự; CORE đọc body ngay, SPECIALIST đọc khi task khớp):** codegraph → karpathy-guidelines → ponytail → assp-skill → impeccable → taste-skill → 8sync-cli → image-routing → locate-anything.
- **Cách tận dụng:** codegraph = explore code (query/callers/callees, không grep) · karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · impeccable = design CHUẨN, BẮT BUỘC khi UI/design (đọc body lúc đó) + taste chống slop.
- **Output lớn (>~50 dòng) → BẮT BUỘC `headroom_compress`** trước khi vào context.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (Unreleased) + ghi học được vào file này (prefix `validated:` nếu test/build xác nhận, `hypothesis:` nếu chưa).
<!-- 8sync:harness:end -->

# KNOWLEDGE (8sync managed — append-only)

## Learnings (append-only — ghi DƯỚI đây; KHÔNG sửa block `8sync:harness` ở trên)
_(consolidated 26 dòng cũ → su-code/archive/KNOWLEDGE-1783322297.md)_
  `windowTok`, `thresholdPct`, `willCompact` — 1M window is an estimate, not authoritative); **workflow
  canvas** fixed (was tiny broken box → 560px react-flow viewport). Model philosophy locked in
  `models.toml`+omp `config.yml`: **Opus = thinking** (plan/review/debug/vision), **GLM = mechanical**
  (code/edit/default/trivial). Verified: integrated `cargo build` (build.rs embeds FE) clean, all
  endpoints smoke-tested live, 14 pages browser-verified 0 console errors. Delegated FE↔backend to
  parallel subagents on disjoint files (web/src vs web.rs) — lead owned integrated build + verify + ship.
- **failure→fix: dashboard project switcher didn't switch data (v0.29.1).** `activate` only wrote
  advisory `web-session.json`; all handlers read `detect_current_project_root()` (launch cwd) → pages
  never changed (FE label changed locally, masking it). Fix: `apply_active_project` chdir's into the
  activated project at startup + on activate, so every cwd-based handler resolves to it (dashboard is
  single-user/local → process-global cwd is the simplest reliable switch). Also `/api/projects`: dedup
  by resolved path, drop junk slugs (mtime 0 / non-dir), widen green-dot window to 2h, add `current`
  flag. Lesson: a "switch" that only changes a label is a lie — verify the underlying data actually
  changes (curl /api/state before+after), and browser-test interactive flows before claiming done.
- **failure→fix: Context % wrong for non-1M models + stale-FE build (v0.29.2).** User: "GLM compacts,
  other vendors don't." Root cause was the dashboard, not omp: `/api/context` hardcoded window=1M, so
  smaller-window models (claude-haiku 200k, glm-4.x 131–205k) showed artificially low % and never
  looked over-threshold, while 1M models (glm-5.2, claude-opus) looked right. omp compaction IS
  model-agnostic — threshold = `thresholdPercent` (or default `window − max(15%,reserve)`) of the
  model's REAL `contextWindow` (omp/compaction.md). Fix: parse per-model window from `omp models`
  (cached `LazyLock`), fall back to assumed only when unknown. Also: omp threshold compaction is
  TURN-TRIGGERED (after a completed turn / safe mid-turn), not a hard cap → a paused session sits
  above threshold until resumed; copy now says "compacts on next turn" + flags `stale`.
  **build.rs trap:** it rebuilt the FE only when `web/dist` was MISSING and watched only `web/dist`,
  so `web/src` edits silently shipped stale (verify served bundle: curl `/assets/*.js` for your new
  strings). Fixed to rebuild when src newer than dist + `rerun-if-changed=web/src`. Lesson: after an
  FE edit, confirm the embedded bundle actually changed before claiming it shipped.
- **failure→fix: serena MCP "Transport closed" (v0.29.3).** Root cause: serena renamed its executable
  — `uvx … serena-mcp-server` no longer exists (now `serena start-mcp-server`); running it printed
  "An executable named `serena-mcp-server` is not provided" and exited → omp reported transport
  closed. Also `--context ide-assistant` is deprecated → `claude-code`. Fixed `deploy.rs::ensure_serena_mcp`
  args. Deeper fix: `register_omp_mcp` SKIPPED any already-present server, so stale entries never
  self-healed — now it UPDATES in place when command/args changed, and `harness up` now calls the MCP
  ensures too (was init/bare-harness only). Lesson: pin/verify external tool entrypoints — a renamed
  binary silently breaks an MCP; reproduce by running the exact `command + args` directly (stdin=EOF)
  and read stderr. Diagnose external-tool failures by running them, not guessing.
- **decided: default `8sync setup` = AI-core only (v0.30.0).** User wants a fresh install to pull
  ONLY the AI coding stack; everything personal/desktop (vietnamese/unikey, warp, LED/RGB, displaylink,
  kitty/helix/wallpaper) is opt-in. Stage A had crept to install kitty+helix+docker+Nerd-font+glass
  config always. Moved that into an opt-in `terminal` step (`--profile terminal`, y/N menu, `--full`);
  docker → `dev-stack` only. `doctor` terminal checks made advisory (was `check_cmd`→warn on missing).
  Safe because `8sync .` now just execs omp (no kitty panes) and `find` falls back to `vi`. Lesson:
  keep the DEFAULT install lean — personalization is opt-in profiles, not Stage A creep.
- **measured+built: `8sync harness toolstats` (v0.31.0) — the optimizer stack is barely used.** User
  observed the agent always grep/read, never codegraph/cbm/serena. Confirmed from omp session JSONL:
  across 68 sessions / 28,020 calls → optimizer **1.1%** (codegraph 302 via bash, cbm 5, serena 0,
  headroom 0) vs fallback **35.2%** (read 8250, search 1147, find 380, grep 77, glob 12). Built a
  SQLite tracker: parse `~/.omp/agent/sessions/<slug>/*.jsonl` (`type:message` → `message.content[]`
  `type:toolCall` {id,name,arguments}; `message.role:toolResult` {toolCallId, isError}); categorize
  optimizer/fallback/edit/other; store `.cache/8sync/toolstats.db` (gitignored), idempotent on
  (session,seq); report ratio + fails. NOTE: codegraph is a `bash` call (inspect `arguments.command`
  for "codegraph"); serena/cbm/headroom are MCP tools. rusqlite `bundled` (+~1.8MB binary, build ~2m
  first time). The tracker gives VISIBILITY; raising the ratio is a separate force-load/prompt fix.
- **validated: `8sync theme` verb + kitty readability fix shipped → v0.34.0.** Kitty transparency
  model (non-obvious, was a prior `fix(bg)` commit): `background_tint 0..1` blends the `background`
  color OVER the image (0=image fully visible→text washes out on bright wp; 1=solid bg, image gone).
  Readable-on-wallpaper sweet spot = **tint ≥ 0.85** + `background_opacity ~0.90` (higher opacity =
  less desktop double-bleed). Foreground + bright-black (color8) MUST pass WCAG-AA (≥4.5:1) vs bg —
  Tokyo Night `color8 #414868` fails at 2.9; brighten to `#7a85c4` (5.1). Architecture: **structure
  (8sync.conf: opacity/blur/font/splits) split from palette (8sync-theme.conf: bg+fg+16 ANSI+tabs)**;
  `include 8sync-theme.conf` at TOP so its `background` is the tint target. Live reload =
  `pkill -SIGUSR1 -x kitty` (re-reads kitty.conf+includes, instant, no socket). `8sync setup
  --profile terminal` regenerates both files (clean cutover from old inline-palette format);
  `8sync theme set <name>` swaps palette only. `hydectl theme` owns Hyprland; `8sync theme` owns
  kitty — distinct surfaces. Contrast verified in eval (WCAG formula), not eyeballed.
- **validated: `8sync bg` verb (live wallpaper swap + inline preview) shipped → v0.35.0.** Inline
  image render in terminal = **`kitten icat <file>`** (kitty graphics protocol; omp uses the same).
  Requires a real kitty TTY — from a detached shell it errors `open /dev/tty: no such device` (can't
  visually verify from agent context; verify path/record/reload mechanics instead). Live wallpaper
  swap = rewrite the `background_image <path>` LINE in `~/.config/kitty/8sync.conf` (in-place) +
  `pkill -SIGUSR1 -x kitty` — changing the PATH string forces a definite image reload (same-path-
  content-swap is unreliable). Persist choice in `~/.config/8sync/wallpaper` (path text); taught
  `install_terminal_config` to honor it so re-setup doesn't clobber. fzf image picker =
  `fzf --preview "kitten icat '{}'"` with paths piped to stdin (fzf reads list via pipe, interacts
  via /dev/tty — spawn with Stdio::piped stdin, take+drop stdin to close, wait_with_output). Image
  validity = magic bytes (PNG 89 50 4E 47 / JPEG FF D8 FF / RIFF…WEBP / GIF8) — guards 0-byte HTML
  downloads. Zero new Rust deps (kitten/fzf/curl shell-outs). `hydectl wallpaper` = desktop, `8sync
  bg` = kitty background_image — distinct. **Online search shipped v0.36.0 via `8sync bg search
  <q>`** — wallhaven.cc public API (NO key needed, SFW, ≥1920×1080); JSON `data[].path`=full,
  `thumbs.small`=thumb, `url`=page link. Stage thumbs in temp, fzf `--preview 'kitten icat {};
  cat {}.url'` (id+res baked into filename so no --delimiter needed), pick → read `{thumb}.full`
  sidecar → `add`+`set`. Sidecar pattern (`<file>.url`/`.full`) maps a chosen path to its metadata
  without a TSV/delimiter. RAII `TmpGuard` cleans staged thumbs. Non-TTY → print list with links.
- **validated: dashboard `Codegraph` page (visualize codebase-memory-mcp graph) shipped.**
  Browser-audited `8sync harness web` end-to-end (real project, all 14 pages) per user request —
  confirmed 0 route existed for codegraph/memory graph viz despite `@xyflow/react`+`elkjs` already
  bundled (only powered the manual Workflow builder). Fix: shell `codebase-memory-mcp cli <tool>
  <json-args>` (NOT the MCP protocol — a plain subprocess call) from 3 new axum routes
  (`/api/codegraph/{overview,search,trace}`); stdout is pure JSON, all progress/log lines go to
  stderr (verified separately) so no scraping needed. Project slug = path with leading `/` stripped
  + remaining `/`→`-` (matches `list_projects` output exactly, e.g.
  `/home/alexdev/Projects/tools/su-code` → `home-alexdev-Projects-tools-su-code`) — **passing the
  raw absolute path as `project` triggers `store.auto_clean` and DELETES a phantom index entry**,
  always slug it first. FE: elk `layered`/`RIGHT` layout for the package boundary graph (nodes =
  packages sized by node_count, edges = `boundaries[].call_count`), Leiden `clusters[]` rendered as
  plain cards (cohesion/top_nodes/packages — no fake inter-cluster edges fabricated, the API doesn't
  give them), trace subgraph uses depth=1 only so every rendered edge is a real direct caller/callee
  (no fabricated multi-hop edges from a flat hop-list). failure(caught in review, fixed same session):
  initial `SWAP` on the route-registration line accidentally deleted the pre-existing
  `/api/workflows/:name` CRUD route (get/post/delete) — `cargo build` warnings
  (`api_workflow_get`/`save`/`delete` never used) caught it immediately; always re-check route list
  after a route-block edit, not just "does it compile". Also fixed pre-existing bug found during the
  audit: `.tile-head strong` with `overflow-wrap:anywhere` + no `flex-basis` rendered
  `codebase-memory-mcp` one-character-per-line next to a wide version tag (`flex:1 1 auto;
  min-width:0` + `overflow-wrap:break-word` fixes it) — different tools' `--version` output format
  differs (`0.9.2` vs `codebase-memory-mcp 0.8.1` vs `headroom, version 0.27.0`), extract just the
  semver token instead of trusting raw output verbatim.
- **validated: dashboard `Marketplace` (discover + install skills/MCP) shipped → v0.41.0.**
  Browser-verified end-to-end: MCP catalog merges 4 sources (official
  `registry.modelcontextprotocol.io` REST, Smithery `registry.smithery.ai`, Glama
  `glama.ai` JSON, mcp.so **scraped with pure-Rust `scraper` crate** — HTML DOM
  `a[href^="/server/"]`, fetched via `curl` so no reqwest), 135+ deduped; install
  writes real `mcp.json` stdio/remote entries (verified United States Weather →
  `npx -y @smithery/cli run …`, then Remove cleaned it). Skills = GitHub star-ranked
  search → `skill add`. **Rust-first scraping rule:** prefer JSON APIs (official/
  smithery/glama all have them → robust); only mcp.so needed DOM scraping. pulsemcp
  API was dead (410) — dropped. Also wired import buttons that were plumbing-only:
  Skills Import (github/gh:/path:/builtin:), MCP install-from-link + Remove, Rules
  import-from-folder/github (prefers a `rules/` subdir, RAII temp clone). failure:
  `tab.click('text/MCP')`/`text/Skills` matches BOTH the nav link AND marketplace
  seg tab — target `.seg button` via `tab.evaluate` or the nav via `observe` role.
- **validated: bench-driven optimization → dashboard Bench rebuilt + spine advisory.** Full review
  (browser, 15 pages × 0 console errors; project switcher verified end-to-end: `/api/state`+bench+
  codegraph all follow the switched project via web-session.json + process chdir) found ONE real
  gap: bench *measured* but didn't *drive* — the memory spine (su-code/*.md) hit **55% of the
  upfront budget** (7.5k/13.6k tok, bigger than prefix+CORE combined) and nothing warned. The
  200-line KNOWLEDGE budget doesn't bound tokens (200 dense lines ≈ 5k tok). Fix:
  `bench.rs::spine_advice` (warn at spine >50% upfront) in CLI + `BenchMetrics` (+`core_tok`/
  `spine_tok`/`naive_tok`) + Bench page (auto-load on mount — deterministic 40ms compute never
  deserved an empty-state gate; breakdown meters; advisory card). STATE trim −71% (8.4→2.4 KB)
  dropped upfront ~13.6k→~12.2k tok. Pattern: a metric page must surface the *lever*, not just
  totals; and cheap deterministic queries should `enabled: true`, not hide behind a button.
- **validated: v0.43.0 shipped — canvas capture + auto-locate E2E + English docs.** (1) `/codegraph?shot=1`
  renders ONLY the React-Flow graph full-viewport (`position:fixed inset:0 z-index:var(--z-overlay)` —
  nav sits at `--z-sticky:20`, overlay must beat it). Full non-vision chain proven on this box:
  `8sync shot '?shot=1'` (~2k vision tok) → `8sync locate <img> "the box labeled skills"` →
  box [1176,624,1502,735] click≈(1339,679) — EXACTLY on the node (annotated PNG cross-checked by
  vision). CPU build (GCC 16, -march=native): 44s wall / 20min CPU on 9950X3D; GGUF q8_0 = 6.3GB.
  (2) **Editing `assets/*` requires `cargo build` BEFORE `8sync harness` deploys them** — harness
  deploys from the EMBEDDED (rust-embed) copy; "already deployed" = content-equal to the embed,
  not the repo file. Stale binary → silent no-op deploy (hit this live).
  (3) `harness web` restores the last web-session project — **activate the target project via
  `/api/workspaces/activate` BEFORE screenshots** or you leak another repo (hit: content-post-agency).
  (4) `su-code/skills/` is fully gitignored (machine-local mirror); `git check-ignore` echoes the
  path — don't misread it as `ls-files` output. Source of truth = `assets/skills/` (committed).
- validated: `8sync harness global` (0.44 dev) — machine-wide rule layer extracted into
  `harness/global.rs::global_pass()` (shared with bare harness; auto.rs step-1 block deleted).
  Key facts: (1) `~/.omp/agent/APPEND_SYSTEM.md` is what makes rules GLOBAL — every omp system
  prompt, any project, no per-project run needed; byte-stable writes keep the Anthropic prompt
  cache hot. (2) `mirror_global_to_local` count = skills PROCESSED (skips included), not newly
  copied — label as "synced", never "+N mirrored". (3) `compaction::ensure_threshold_default`
  writes 50 only when the key is absent (never override user config). (4) `--sweep` scanner:
  repos are dirs containing `.git`, depth ≤ 4, found repos not descended, skips
  node_modules/target/hidden; gitleaks hook still requires the gitleaks binary.
- validated: loop-engineering audit vs Avi Chawla's 4-layer article (Jul 2026) — su-code already
  covers prompt (APPEND_SYSTEM/skills), context (headroom/compaction-50/codegraph/STATE handoff),
  harness (engine_* + worktree + MCP). The 2 REAL gaps were in the loop layer, both fixed
  code-enforced in `assets/extensions/8sync-engine.ts`:
  (1) `engine_advance` never checked verification — "code-enforced gate" was prompt-ware; now a
  per-task `verified` flag makes advance REFUSE unverified tasks (agent say-so ≠ stop signal).
  (2) no-progress detector: FNV-1a fingerprint of verify-failure output; identical ×2 warns,
  ×3 blocks early below maxRetries — doom-loop guard. Old state.json loads via zod defaults.
  Testing recipe: Bun.Transpiler + stub `pi` {zod, registerTool} + chdir to tmp → call
  tools[name].execute directly; zod lives at ~/.bun/install/global/node_modules/zod.
  Remember: assets are rust-embed'd — REBUILD the binary before `8sync harness` deploys them.
- validated: `--sweep` detection = omp project ⇔ repo has `su-code/` dir OR AGENTS.md/CLAUDE.md
  (`global.rs::is_omp_project`) — sweep never injects into non-omp repos (skip + report).
  Live run 2026-07-05: 8/8 omp projects under ~/Projects stamped, 0 failed, 0 foreign repos touched.
- validated: no-overwrite contract audited end-to-end (2026-07-05) — user-owned files
  (su-code/*.md seed-if-missing memory.rs:129 · CHANGELOG once :146 · skills mirror additive
  deploy.rs:105 · AGENTS.md sentinel-only · hook only-if-absent :239 · config key-detect) are
  NEVER clobbered by default; proven live: custom edits to su-code/skills/*/SKILL.md + STATE.md
  survive a sweep re-run. Overwrite = explicit `--force` only. Managed layer (~/.omp bundled
  skills, 00-force-load, APPEND_SYSTEM, extensions) refreshes byte-compare on binary update —
  customize the PROJECT copy, not ~/.omp. Policy now printed in `harness help` + AGENTS.md §8.
- failure: omp `Schema error: providers: must be an object (was null)` = 8sync wrote
  `~/.omp/agent/models.yml` with a bare `providers:` key (empty local-model registry after
  `add-local-model rm`). YAML: key with no children parses as null, NOT {}. Fix: single choke
  point `local_model::insert_block` finalizes → `providers: {}` when no real (non-comment)
  children; `ensure_providers` reopens `providers: {}` for later inserts. Rule: any managed
  YAML map key must never be emitted bare.
- **validated: MCP marketplace install now conforms to `server.json` spec (2025-12-11) → v0.45.0.**
  `official_install` (marketplace.rs) projects registry `server.json` → `mcp.json`: `registryType`→runtime
  (npm→`npx -y` · pypi→`uvx` · oci→`docker run -i --rm`+`-e NAME` · nuget→`dnx`), version pin
  (`id@ver`/`img:ver`), `runtimeArguments`+`packageArguments`, `transport.type` streamable-http/sse→remote.
  **BUGFIX + failure lesson: `env`/`headers` MUST be `{NAME:value}` maps, NEVER arrays of descriptors** —
  the old code wrote `env:[{name,required,desc}]` which is unusable in mcp.json. Threaded env/headers
  end-to-end (McpAddBody + api.ts + App.tsx were dropping them). E2E via UI on live registry: docker
  `apithreshold` (run…-e KEY…img:0.1.0) + pypi `armor-mcp@0.6.1`+env map, 0 console errors.
- **validated: an open spec becomes "machine default + AI-forced" via the harness global layer.** Bundle
  the distilled spec as an asset (`assets/specs/mcp-server.md`) → `ensure_mcp_spec` deploys to
  `~/.omp/specs/` in global_pass/init/up (byte-stable skip) → a SHORT rule in `APPEND_SYSTEM.md` points
  every omp session at the on-disk file. Keep the full spec OUT of APPEND_SYSTEM (prompt stays cache-hot);
  APPEND holds only the pointer + invariants. Pattern reusable for any standard (skills/AGENTS.md/…).
- **validated: `/auto` engine reviewed + functional-tested (Bun harness, v0.45).** All 6 `engine_*` register;
  verify-gate FAIL→WARN(2×)→BLOCK(3× doom-loop even at maxRetries=10, so it's the FNV-1a no-progress guard,
  not maxRetries); `engine_advance` REFUSES a task with verify cmds but no passing run; pass→advance→done;
  trivial no-verify advance; commit path creates a real commit. **Gap fixed:** `engine_advance {commit:true}`
  did `git add -A` + `git commit` with NO secret scan (doctor: gitleaks absent) → added a gitleaks gate
  (`if command -v gitleaks; then gitleaks protect --staged; fi` — no-op when absent, aborts+resets on a finding).
- **failure→fix: `harness up --timer` OOM-killed the whole machine (v0.46.2).** The generated
  `8sync-harness-up.service` was a `Type=oneshot` timer unit with **no cgroup resource limits**.
  Per tick (`--timer 10m`) it ran `codegraph index`, whose Node process (`~/.codegraph/versions/v0.9.2/node`)
  hit ~5.3 GB RSS on a big repo (`zus`) → kernel OOM killer fired (`Result: oom-kill, Mem peak 5.3G`),
  thrashing swap and killing other apps, every 10 min. **Not a slow leak — a periodic memory spike with
  no ceiling.** Fix: bound the generated unit to its own cgroup + de-prioritize it — `MemoryHigh=2G`
  (reclaim throttle, slows instead of exploding), `MemoryMax=4G` (hard cgroup ceiling — kills only THIS
  unit, never the box), `MemorySwapMax=512M`, `OOMPolicy=stop`, `Nice=15`/`CPUWeight=10`/`IOWeight=10`,
  `TimeoutStartSec=900`. cgroup v2 `memory` controller is delegated to the user slice on CachyOS so
  `systemctl --user` units honor these. Verified live: codegraph held ~2.05 GB by `MemoryHigh` reclaim
  pressure (was 5.3 GB). **Lesson: any unattended background unit that shells out to a memory-hungry
  indexer MUST be cgroup-bounded** — scope the danger to the timer (unattended); manual/`--loop` runs stay
  unbounded (user-visible, interruptible).
- **validated: `--sweep` must redeploy PROJECT-level `/auto`, not just migrate the folder (v0.46.1).**
  omp resolves slash commands with **project `.omp/commands/*.md` taking precedence over global**
  `~/.omp/agent/commands/*.md`. After the `agents/`→`su-code/` rename, sweep migrated the memory folder
  but `stamp_project` never refreshed the project's `.omp/commands/auto.md` (+ `8sync-engine.ts`), so
  `/auto` in a swept repo kept executing a stale copy pointing at `agents/STATE.md`. Fix: `stamp_project`
  now calls `deploy::ensure_engine(&env.home, Some(root))` (byte-identical writes stay quiet). **Lesson:
  a rename/migration must chase every deployed COPY of a config, especially higher-precedence project-local ones.**
- **failure (tooling): embedded-shell `grep '\|'` BRE alternation silently returns nothing (false negative).**
  Verified "clean" migration state twice with `grep "agents/\|su-code/"` and got 0 hits → wrongly concluded
  no `agents/` refs remained. The bundled shell doesn't honor GNU BRE `\|`; must use `grep -E 'a|b'` (or the
  built-in grep tool, Rust regex). **Lesson: never trust `\|` alternation in the embedded shell — a false
  negative reads as "verified clean".**
- **finding (cross-platform build, v0.46.2 investigation):** code compiles cross-platform as-is — 0
  `std::os::unix`/`PermissionsExt`, 0 `#[cfg]` gating; `cargo check --target x86_64-pc-windows-gnu` passes
  all Rust code + pure-Rust deps. Two gotchas for portable/multi-OS release: (1) `.cargo/config.toml`
  `rustflags = target-cpu=native` tunes the binary to the BUILD CPU → prebuilts can SIGILL on older CPUs
  (affects the CURRENT Linux prebuilt too) — drop it for release builds; (2) C-FFI deps `libsqlite3-sys`
  (rusqlite `bundled`, for `harness toolstats`) + `zstd-sys` (via `include-flate`) compile bundled C in
  `build.rs`, so cross-from-Linux needs mingw-w64/osxcross — **native CI runners (macos-14, windows-latest)
  build them cleanly**, which is the recommended release path.