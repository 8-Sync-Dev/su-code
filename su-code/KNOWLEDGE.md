<!-- 8sync:harness:begin -->
## 🧠 8sync harness

- **Always-on (đọc theo thứ tự; CORE đọc body ngay, SPECIALIST đọc khi task khớp):** codegraph → karpathy-guidelines → ponytail → assp-skill → impeccable → taste-skill → 8sync-cli → image-routing → locate-anything.
- **Cách tận dụng:** codegraph = explore code (query/callers/callees, không grep) · karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · impeccable = design CHUẨN, BẮT BUỘC khi UI/design (đọc body lúc đó) + taste chống slop.
- **Output lớn (>~50 dòng) → BẮT BUỘC `headroom_compress`** trước khi vào context.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (Unreleased) + ghi học được vào file này (prefix `validated:` nếu test/build xác nhận, `hypothesis:` nếu chưa).
<!-- 8sync:harness:end -->

# KNOWLEDGE (8sync managed — append-only)

## Learnings (append-only — ghi DƯỚI đây; KHÔNG sửa block `8sync:harness` ở trên)
_(consolidated 160 dòng cũ → su-code/archive/KNOWLEDGE-1783900247.md)_
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
- **validated: `harness model <strong>+<cheap>` combo preset writes omp roles directly.** There are TWO
  model layers: 8sync's `~/.config/8sync/models.toml` (its own routing for `8sync ai`/`/auto`) and omp's
  `~/.omp/agent/config.yml` `modelRoles` (the `/model` picker — what actually drives every omp session).
  `8sync harness model` used to only touch the former; the user's pain ("set sai") was the latter pointing
  at `9router-cc/*` + reviewer `9router-cx/cx/gpt-5.5` (providers they'd stopped using). The combo
  (`model=claude+glm`, `=`-shorthand normalized in harness dispatch) now writes BOTH: it rewrites the omp
  `modelRoles` block + `task.agentModelOverrides.reviewer` **line-based** (find the top-level `modelRoles:`
  line, splice until the next non-indented line; preserves every other key — verified: memory/mnemopi/
  compaction/setupVersion untouched) and syncs models.toml. Optimal split: cheap=mechanical
  (default/task high · smol/tiny/commit minimal · advisor), strong=thinking (vision/slow high ·
  plan/designer/reviewer **xhigh**). `vision`→strong because glm-5.2 is `images:no` (text-only).
- **correction: `xhigh` IS valid on DIRECT `anthropic/*`, but NOT on the 9router gateway (`cc/*`).** The
  earlier blanket "NO xhigh" rule was 9router-specific: `omp models` shows `cc/claude-opus-4-8` (9router)
  efforts = `minimal,low,medium,high` (no xhigh), while `anthropic/claude-opus-4-8` (direct) =
  `minimal,low,medium,high,xhigh`. omp's `ReasoningEffort` enum includes `xhigh`. So design/plan/review on
  direct anthropic opus can use `:xhigh` (user's explicit ask); the gateway-models.yml "NO xhigh" comment
  stays correct for the 9router path. **Verify a thinking level exists before setting it: `omp models`
  prints the per-model efforts list.**
- **validated: dashboard Knowledge browser + Create-Project (this session, engine-built A–E).** Reuse map that
  paid off: `marketplace.rs` curl+cache pattern → `knowledge.rs` (raw `sindresorhus/awesome` README via
  `curl`, 6h cache, markdown `##`/`###` + `- [n](u) - d` parse → 679 resources/26 cats, browser-verified);
  `here::seed_project_context` → extracted `pub(crate) fn scaffold_project` (mkdir+git init+seed, headless,
  no omp exec) for `POST /api/projects/create`; `deploy::copy_dir_recursive` to vendor skills. FE: new
  `Page` id + `NAV_GROUPS` entry + render arm + `icons.tsx` glyph + `api.ts` method are the 5 touch-points
  to add a dashboard page.
- **failure→fix: `8sync skill add builtin:<name>` does NOT vendor an already-global skill into a project**
  (prints "already installed", no-op for the project's `su-code/skills/`). To vendor a bundled skill into a
  new project, COPY the dir `~/.omp/skills/<name>` → `<proj>/su-code/skills/<name>` (via
  `deploy::copy_dir_recursive`), don't shell `skill add`. Caught in browser QA (skill dir stayed empty).
- **finding: `/api/skills` lists `00-force-load.md` as a "skill"** (it's the force-load index file, not a
  skill). Any UI offering a skill picker must filter `*.md` entries. The dashboard create-modal now does.
- **note: rust-embed (`WebAssets`) embeds `web/dist` at COMPILE time** — after `bun run build`, `touch
  crates/cli/src/assets.rs` before `cargo build --release` or the binary keeps serving the stale FE.
- **validated (v0.47.0 — cross-platform ship, option B):** the v0.46.2 finding held — porting to
  macOS/Windows needed NO `std::os::unix` removal. Pattern that worked: a single `crate::platform`
  module with `pub const fn os()` (cfg-selected variant per target) + runtime `match os()` dispatch,
  so ONE code body compiles on every target and the wrong-OS branch just never runs (add
  `#[allow(dead_code)]` on the `Os` enum — only one variant is constructed per compiled target, so the
  others read as dead code on any given build). Timer abstraction: systemd user unit (Linux) / launchd
  `StartInterval` plist (macOS) / `schtasks /SC MINUTE /MO <min>` (Windows) — schtasks has no per-task
  cwd so wrap `cmd /c cd /d "<wd>" && "<exe>" <args>`; launchd/schtasks have no cgroup memory cap, so
  the OOM-bound is Linux-only (fine — it was a Linux-only bug). Linux-only verbs (`sec`/`bt`/`clean`)
  gated with a `require_linux()` no-op guard rather than `#[cfg]` stubs (keeps one binary, honest msg).
- **validated (release engineering):** portable Linux prebuilts = **musl-static** (`x86_64/aarch64-unknown-linux-musl`)
  not gnu — dodges `GLIBC_2.xx not found`. `musl-tools` covers x86_64 native; aarch64-musl + the bundled
  C deps build cleanly via **`cross`** (dockerized toolchain) on ubuntu. mac/Win = native runners
  (macos-13 x86_64, macos-14 arm64, windows-latest MSVC) — the ONLY way (Linux can't emit Apple-SDK/MSVC).
- **failure (local cross-verify):** `cargo check --target x86_64-pc-windows-gnu` from Linux ABORTS on
  `libsqlite3-sys` build.rs (needs a Windows C compiler / mingw). Without passwordless sudo to install
  mingw-w64, local win/mac compile-verification is impossible — CI native runners are authoritative, and
  that's not a shortcut, it's the standard. Don't burn time trying to cross-build C-FFI crates from Linux.
- **validated (0.49.0 — omp custom models):** to add a model omp's fetched catalog lacks (or lists with
  null metadata, e.g. new `xai-oauth/grok-4.5` shows context/max `-`), write a FULL custom provider under
  `providers:` in `~/.omp/agent/models.yml`. Empirically (omp 16.3.12): a metadata-ONLY merge
  under a built-in provider is REJECTED — `Validate(models): Provider X: "baseUrl" is required when
  defining custom models`. So baseUrl is mandatory; selector omp exposes = `<providerKey>/<modelId>`
  (e.g. provider key `xai` + id `grok-4.5` → `xai/grok-4.5`). A bad thinking/api combo makes omp reject
  the WHOLE file (all custom models vanish) → always re-validate with `omp models --json` after writing.
  `8sync harness add-model` does exactly this; registry `~/.config/<NS>/custom-models.tsv`, sentinel
  block coexists with local-models + gateway (strip-only-own-block pattern from local_model.rs).
- **validated (windows portability):** any `std::os::unix::*` (e.g. `PermissionsExt`/`from_mode` chmod)
  MUST be `#[cfg(unix)]`-gated — the module is ABSENT on Windows and breaks MSVC compile. selfup.rs shipped
  ungated in 0.47.0 and only CI's windows-x86_64 job caught it (fixed 7f50c59). grep gate before shipping:
  `std::os::unix|PermissionsExt|set_mode|from_mode|CommandExt|signal::unix`.

- **validated (0.49.1 — omp thinking config):** omp's valid `thinking.mode` enum =
  `effort | budget | google-level | anthropic-adaptive | anthropic-budget-effort` (found in the binary:
  `"effort" | "budget" | "google-level" | "anthropic-adaptive" | "anthropic-budget-effort"`). For a custom
  model, pick mode by API: **`effort`** for `openai-completions` (generic `reasoning_effort` wire param —
  correct for xAI/OpenAI), **`anthropic-budget-effort`** for `anthropic-messages` (sends `budget_tokens`).
  The config block MUST be nested `{mode, efforts, defaultLevel}` — the flat `thinking: [minimal,...]` list
  (what `omp models --json` OUTPUTS) is REJECTED as input, and `mode` is required. Canonical effort tiers
  low→high = `minimal, low, medium, high, xhigh` (picker abbreviates minimal→"min", adds meta inherit/off/auto).
  Full native range for grok-4.5/claude = all 5 tiers; `add-model --think` (bare) now emits the full set.

- **validated (0.50.0 — omp /new root):** omp's `/new` slash-command = `newSession({parentSession})` — the
  child session INHERITS the launch project root; it does NOT re-detect from cwd. So if omp was launched in
  the wrong dir, every `/new` perpetuates it. omp has a `--cwd <dir>` flag ("Directory to start in,
  overrides launch cwd") + scopes sessions per-cwd (`gc.retainNewestPerCwd`, `mnemopi.scoping=per-project-tagged`).
  Fix: `8sync .` and `8sync ai` now pass `--cwd <detected-root>` (+ current_dir). `8sync ai` used to launch
  omp in ambient cwd with no root pin — that was the drift source.
- **validated (0.50.0 — omp browser internet):** omp's Puppeteer browser can render but fail to reach the
  internet on the bundled `~/.omp/puppeteer/chrome-headless-shell`. omp runs under Bun and honors
  `PUPPETEER_EXECUTABLE_PATH` + `BUN_CHROME_PATH` (with `--no-sandbox`) to use a real system Chromium.
  `ungoogled-chromium-bin` (cachyos repo on CachyOS, else AUR) installs `/usr/bin/chromium` which fetches
  pages headless fine. `8sync harness browser` exports those vars in zsh/bash/fish (interactive shells pick
  them up — .bashrc's non-interactive `*i*` guard means a `bash -c 'source ~/.bashrc'` test won't show them,
  use `bash -ic`). Do NOT force the env at launch-time or `browser off` becomes leaky — rc export is the
  single source of truth.

- **validated (0.51.0 — feynman↔omp auth bridge):** Feynman (companion-inc/feynman) and omp are BOTH Pi
  (earendil-works/pi; feynman=base pi-ai 0.3.5, omp=@oh-my-pi/pi-ai fork) → both read `<home>/agent/auth.json`
  in the SAME schema: `{ "<provider>": {"type":"api_key","key":"..."} }` or `{"type":"oauth","access":"...",
  ...} }`. Pi keys per provider: anthropic→`anthropic`, zai→`zai`, xai→`xai`, openai→`openai`, google→`google`
  (see pi docs/providers.md). `key` supports `"!command"` (exec, stdout; auth.json = cached per-process,
  models.json = per-request) + `"$ENV"`. Resolution order: CLI --api-key > auth.json > env > models.json.
  omp stores creds in SQLite `~/.omp/agent/agent.db` table `auth_credentials(provider,credential_type,data,
  disabled_cause,identity_key)`; anthropic oauth data = `{access,refresh,expires,accountId,email}`. `omp token
  <p> --raw` mints/refreshes the current access token (NOT the full record). VERIFIED: a minimal
  `{type:oauth, access:<omp token>}` (no refresh, no expires) authenticates feynman fine (25 anthropic models,
  default claude-opus-4-8). `8sync feynman auth-omp` bridges: oauth→access-only (omit refresh so feynman never
  rotates omp's token = no dueling refresher, omp sole refresher, re-run on expiry); api_key→`!omp token <p>`.
  DUELING-REFRESH is the key gotcha: copying the refresh token would let both omp+feynman refresh → Anthropic
  rotates refresh-token on use → they invalidate each other. Omitting refresh avoids it. omp auth-gateway
  (forward proxy) is the alternative but REQUIRES a broker (`OMP_AUTH_BROKER_URL`) = 2 daemons, too heavy.
  feynman `feynman chat` needs `feynman setup` (installs Pi npm packages) — auth resolution works without it
  (feynman model list / doctor read auth.json directly).

- **validated (0.52.0 — 8sync vpn / SoftEther + VPN Gate):** SoftEther on Linux, grounded in official docs:
  (1) native Linux VPN Client has **NO GUI** ("cannot be operated using a GUI") — only the Windows VPN
  Client Manager exists; on Arch the AUR `softethervpn-client-manager` packages that Windows `vpncmgr.exe`
  to run under **Wine** (+ `.desktop`), which is where the Windows-style VPN Gate region-switch plugin lives.
  (2) The Linux client **does not auto-rewrite the routing table** — you must manually pin a static route to
  the VPN server via the physical uplink, then set the tap as default. So the reliable region-switch on Linux
  is the CLI, not the GUI. Package: `softethervpn` = maintained RTM **4.44** (vpnclient+vpncmd+client service);
  `softethervpn-git` = unstable 5.x dev — use 4.44. Client mgmt is non-interactive via
  `vpncmd localhost /CLIENT /CMD <cmd>` (NicCreate se → tap `vpn_se`; AccountCreate /SERVER:ip:443 /HUB:VPNGATE
  /USERNAME:vpn, AccountPasswordSet /PASSWORD:vpn /TYPE:standard, AccountConnect). VPN Gate server list =
  CSV API `https://www.vpngate.net/api/iphone/` (cols HostName,IP,Score,Ping,Speed,CountryLong,CountryShort,…).
  This box had **no DHCP client** (NetworkManager only) → `8sync vpn install` also pulls `dhcpcd` for the tap.
  Egress check uses Cloudflare's IP-addressed trace (`https://1.1.1.1/cdn-cgi/trace`) so it survives the DNS
  swap to 1.1.1.1; `on` auto-rolls-back (routes+DNS) if egress doesn't change. VPN Gate = academic + LOGGED.
- **validated: `8sync feynman auth-omp` succeeds but `feynman` REPL crashes = broken pnpm `npm` shim, NOT the bridge.** feynman shells out `npm install @companion-ai/alpha-hub --prefix ~/.feynman/agent/npm --legacy-peer-deps` on interactive launch (`feynman chat`). If `npm` on PATH is a pnpm shim reached via a **symlink from another dir** (`~/.local/bin/npm -> ~/.local/share/pnpm/npm`), the shim's `basedir=$(dirname "$0")` resolves to the symlink's dir (`~/.local/bin`) and it looks for `~/.local/bin/global/5/.pnpm/npm@…/npm-cli.js` → `MODULE_NOT_FOUND` (real tree lives under `~/.local/share/pnpm/global/…`). Running the shim by its real path works. Fix = replace the `npm`+`npx` symlinks in `~/.local/bin` with wrapper scripts `exec /home/<u>/.local/share/pnpm/{npm,npx} "$@"` so `$0` inside the shim points at the real install dir. Diagnose: bridge is fine if `feynman model list`/`feynman doctor` show the omp-authed providers (anthropic+zai); the crash is purely the npm subprocess. `pi_key` passes unknown omp ids through harmlessly (`xai-oauth`, `llama.cpp` bridged but not counted authenticated).