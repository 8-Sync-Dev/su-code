<!-- 8sync:harness:begin -->
## 🧠 8sync harness

- **Always-on (đọc theo thứ tự; CORE đọc body ngay, SPECIALIST đọc khi task khớp):** codegraph → karpathy-guidelines → ponytail → assp-skill → impeccable → taste-skill → 8sync-cli → image-routing → locate-anything.
- **Cách tận dụng:** codegraph = explore code (query/callers/callees, không grep) · karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · impeccable = design CHUẨN, BẮT BUỘC khi UI/design (đọc body lúc đó) + taste chống slop.
- **Output lớn (>~50 dòng) → BẮT BUỘC `headroom_compress`** trước khi vào context.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (Unreleased) + ghi học được vào file này (prefix `validated:` nếu test/build xác nhận, `hypothesis:` nếu chưa).
<!-- 8sync:harness:end -->

# KNOWLEDGE (8sync managed — append-only)

## Learnings (append-only — ghi DƯỚI đây; KHÔNG sửa block `8sync:harness` ở trên)
_(consolidated 1 dòng cũ → su-code/archive/KNOWLEDGE-1784595938.md)_
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
- failure: STEP-0 MCP stack (serena/cbm/headroom/zai) was connected but NEVER called — 13 of
  13,854 tool calls across 29 sessions (serena 0, headroom 0). Two causes, both verified:
  (1) omp `tools.discoveryMode: auto` hides ALL MCP tools behind `search_tool_bm25` once the
  registry exceeds 40 tools (this stack registers 48) — the discovery hop was taken 2× ever;
  (2) every instruction surface taught BASE names (`search_graph`, `find_symbol`) that are NOT
  callable — registered forms are `mcp__<server_underscored>_<tool>`
  (`mcp__codebase_memory_mcp_search_graph`, `mcp__serena_find_symbol`; exception
  `mcp__headroom_compress` — omp collapses a duplicated server prefix). Rules that name
  uncallable tools or demand the impossible ("compress output BEFORE it enters context")
  produce 0 usage AND teach the model to discount the whole rule block.
- validated: the fix knob is `mcp.discoveryDefaultServers` (array of SERVER names) — keeps those
  servers' full catalogs in the active tool set under discovery. `tools.essentialOverride` does
  NOT work for MCP: omp 16.4.8 filters its entries to BUILT-IN tool names (`k in BUILTIN_TOOLS`)
  and a non-empty list of only-MCP names CLOBBERS the builtin essential defaults to []. Written
  by `ensure_mcp_tools_visible` (deploy.rs), migrated the inert pin block away. Live-probed:
  `omp -p` calls `mcp__codebase_memory_mcp_search_graph` + `mcp__serena_find_symbol` directly →
  OK, no discovery hop; toolstats logged serena/cbm optimizer calls for the first time. Residual
  friction (out of scope, guided by tool errors): serena needs `activate_project` per session;
  cbm wants its own project slug (`list_projects` first).
- validated (SUPERSEDES the entry above on omp ≥17): omp 17.x DROPPED the bm25 discovery
  model entirely — no `search_tool_bm25`, no `mcp.discoveryDefaultServers` (absent from the
  settings schema). Replaced by `tools.xdev` (default on): MCP tools mount as `xd://mcp__…`
  device URLs, driven via read/write, schemas not shipped every request. So on omp ≥17 the
  STEP-0 tools are callable with ZERO config (proven live: called `xd://mcp__codebase_memory_mcp_*`
  all session with config.yml at 2 lines). The "MCP keeps regressing after every omp upgrade"
  saga was a PHANTOM: omp self-upgrade resets `~/.omp/agent/config.yml` (omp OWNS it — even
  modelRoles combo gets wiped to a default), doctor's `cfg.contains("discoveryDefaultServers")`
  string-check then screamed HIDDEN though tools were fine. Fix: `env_detect::omp_major()` gates
  both `ensure_mcp_tools_visible` (skip the dead key on ≥17) and doctor (report xd:// mount, not
  HIDDEN). **Lesson: a doctor check that proxies a vendor-internal key silently rots when the
  vendor changes mechanism — verify the ACTUAL capability (can I call the tool?), not a config string.**
- gotcha: omp auto-updates aggressively (16.5.2→17.0.1→17.0.6 across ~3 sessions) and each
  upgrade rewrites `~/.omp/agent/config.yml` to a minimal default, dropping 8sync's mnemopi/
  compaction/modelRoles additions. `8sync harness global` re-applies them (idempotent). Only
  the config.yml layer is affected; mcp.json, skills, hooks, APPEND_SYSTEM survive.
- validated (`8sync omp update` verb — auto-fix `omp update` EEXIST): omp gets installed as a
  standalone binary — a real 173 MB file at `~/.local/bin/omp` (`readlink -f` = itself, NOT a
  symlink; nothing in `~/.local/lib/node_modules`). `omp update` shells out to npm/bun
  `install -g @oh-my-pi/pi-coding-agent`, which wants a bin SYMLINK at that path → `npm error
  EEXIST` (or bun `Fail extracting tarball`). Fix (native Rust, `crates/cli/src/verbs/omp.rs`,
  verified live BOTH paths): run `omp update`; on a block, back up the bin → `rm` the squatter →
  `npm install -g @oh-my-pi/pi-coding-agent@latest` (path now free → creates the symlink) →
  re-resolve + report. `--force` skips straight to reinstall. Shipped as the `8sync omp`
  verb, NOT an omp `/command`: it runs from the SHELL so it works even when omp itself is
  broken (exactly when you need it); `8sync up` stays decoupled (8sync binary only) and now
  points at `8sync omp update`. Gotchas: (a) two installs coexist — bun global `~/.bun/bin/omp`
  + npm `~/.local/bin/omp`, `~/.bun/bin` is FIRST on PATH so `command -v omp` is what runs;
  `rm`-ing the PATH-first bin makes resolution fall to the npm one (still healthy). (b) 8sync is
  NOT omp, so removing omp's file mid-run is safe (no in-memory-process worry the old
  `/omp-update` slash-command had). (c) `npm config get prefix` = `~/.local`; bun's updater threw
  `Fail extracting tarball` even on partial success — npm direct install is the reliable path.
- validated (omp 17.0.1 plan-mode "Enter không ăn" — diagnosed, upstream bug, NOT keyboard):
  plan-review dialog key handling is CORRECT — verified live with all Enter encodings (legacy
  `\r`, kitty CSI-u `\x1b[13u`, numpad `\x1b[57414u`) in bare PTY AND real kitty: every variant
  picks. Freeze mode (bundle decompile): `handlePlanApproval` keeps the overlay MOUNTED while
  awaiting the whole approval flow (`#eA`: clear/compact = full model call over the entire
  context + `session.prompt`), and the pick resolver is single-shot (`if (O) return`) — after the
  FIRST Enter, arrows still move ❯ but Enter/esc are dead until the async flow finishes; a stalled
  provider call (no timeout) freezes it indefinitely. "Approve and compact context" on a ~229k
  session = minutes of frozen-looking dialog. Recovery: plan file survives at
  `local://<slug>-plan.md` — Ctrl+C / restart omp, resume, approve again (prefer "Approve and
  execute" on huge contexts). Upstream fix = hide overlay (or busy state) right after pick, before
  `#eA`. Report to can1357/oh-my-pi (blocked this session: gh keyring auth broken).
- failure: omp hub daemon launcher breaks under fish as $SHELL — generated wrapper uses
  `printf '%s' "$$"` which fish rejects (`$$ is not the pid`) → every `hub start` exits 127
  before exec; SHELL env override on the start call does NOT change the wrapper shell (broker
  resolves shell at its own start). Workaround: drive PTYs via python `pty` module or
  `kitty --listen-on unix:... @ send-key/get-text`. Also upstream-reportable.
- validated (`branch-sync` skill & `8sync harness global` auto-detection): (a) `assets/skills/branch-sync/SKILL.md` + `assets/skills/branch-sync/scripts/branch_sync.py` provides automated multi-branch audit, deep preview (commit breakdown + `git merge-tree` conflict check), safe merge to main, and zero-conflict sync across all branches. (b) `8sync harness global` now auto-detects `su-code/` projects in cwd and automatically updates their local harness (skills mirror, AGENTS/CLAUDE injection, memory, commands, gitleaks hook, codegraph init) without requiring explicit `--sweep`. (c) `deep-research` skill enhanced with loop engineering state machines, STEP-0 code intelligence (`codegraph`/`codebase-memory-mcp`/`serena`), multi-agent wave execution, headroom compression, and ponytail YAGNI discipline.
- validated (binary-size audit 2026-08-02, brief: `outputs/native-tooling-zig-rust.md`): `8sync`
  stripped = **6 406 696 B (6.11 MiB)** vs the `AGENTS.md` §8 budget "< 4 MB" → ~1.5–1.6× over.
  Sections: `.text` 2 854 517 · `.rodata` 2 188 928 (embedded assets) · `.eh_frame` 482 684 ·
  `.rela.dyn` 419 856. `cargo bloat --crates`: `[Unknown]` C 780 KiB · std 571 · **our code only
  405 (14.5 %)** · axum 217 · clap_builder 122 · scraper 104 · zstd_sys 58 · tokio 46 ·
  libsqlite3_sys 40. Raw blobs pre-GC: `libsqlite3.a` 2.1 MB; embedded `assets/` 3.0 MB
  (impeccable 2.1 MB, its `scripts/` 1.6 MB) + `web/dist` 1.9 MB. Root cause is NOT the language
  or the flags: `crates/cli/Cargo.toml` has **no `[features]` section**, so `harness web`
  (axum/tokio/hyper/tower), marketplace scraping (scraper/html5ever) and `harness toolstats`
  (bundled SQLite) link into every build. Fix order: feature-gate → un-embed `web/dist` +
  `impeccable/scripts` → re-evaluate `rust-embed`'s `compression`.
- failure (release-profile knobs are EXHAUSTED — stop re-litigating them; all A/B'd with an
  explicit `--target x86_64-unknown-linux-gnu` so RUSTFLAGS skip host proc-macros):
  (a) `-C force-unwind-tables=no` under `panic="abort"` saves **704 bytes**; `.eh_frame` stays
  482 KB (std + the C blobs still emit tables). (b) `opt-level="s"` is **307 392 B BIGGER** than
  `"z"` → keep `"z"`. (c) `-C relocation-model=static` without an explicit `--target` **breaks the
  build** — proc-macros (`indoc`) must be PIC. Lesson: demand an A/B byte count before accepting
  any new size flag.
- gotcha: `rust-embed`'s `compression` feature pulls `include-flate` → `include-flate-compress`,
  which is shared by the build-time proc-macro AND the runtime crate — so the **compressor** half
  of both `libflate` and `zstd` links into a binary that only ever decompresses (`zstd_sys` =
  58.4 KiB `.text` surviving fat LTO). Verify with `cargo tree -i zstd-sys`.
- validated (Zig's real role for this repo): use it as **build tooling, never as a language** —
  there is no compute hot path (`8sync --version` ≈ 11.6 ms incl. fork+exec; `help` 10 ms).
  `cargo-zigbuild` can replace the `cross`/Docker leg for `aarch64-unknown-linux-musl` in
  `.github/workflows/release.yml` and collapse the two macOS assets via `universal2-apple-darwin`
  (5 legs → 4). Hard caveat from the upstream README: *"Currently only Linux and macOS targets
  are supported"* → the Windows MSVC leg is untouched; and `-C target-feature=+crt-static`
  against glibc is unsupported (musl-static unaffected). glibc-pinned triples are irrelevant here
  because CI already ships musl-static Linux builds. Also keep the 9 `curl` shell-outs: 0 bytes,
  `AGENTS.md` §8 bans the heavy HTTP dep, and a TLS stack would ADD ~1 MB to fix a non-problem.
- failure (`engine_advance {commit:true}` CANNOT make atomic commits): the extension runs a bare
  **`git add -A`** before committing (`.omp/extensions/8sync-engine.ts:287`), so it sweeps the
  ENTIRE working tree regardless of what you staged — a per-task message then lies about a
  whole-tree commit. Hit live in M0 of `lean-binary`: "T1 add omp verb" captured 29 files.
  Rule: when splitting a dirty tree into deliverable-shaped commits, `git add <paths>` +
  `git commit` by hand and call `engine_advance {commit:false}` — the engine still enforces the
  verify-gate, which is the part that matters. `commit:true` is only safe when the tree contains
  exactly one task's work (the greenfield task-by-task case it was written for).
- gotcha (verify-gate scope): `engine_verify` runs its commands against the **working tree**, not
  against the staged index or the resulting commit. Splitting one dirty tree into N commits
  therefore proves only that the final tree builds. To make "every commit compiles" a real claim,
  replay the range in a throwaway worktree (`git worktree add`, `git checkout <sha>`,
  `cargo build`) instead of trusting N green `engine_verify` calls.
- validated (M1 `lean-binary` — REAL per-gate cost, supersedes every `cargo bloat` figure for
  this repo): `bash scripts/size-report.sh` A/B's four builds, each in its own `--target-dir`
  with an explicit `--target`. full **6 407 144** · web-only 5 346 304 · toolstats-only
  **4 144 576** · minimal **3 081 416**. Gate cost: **`web` = 2 262 568 B**, **`toolstats` =
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