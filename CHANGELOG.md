# Changelog

Mọi thay đổi đáng kể của `8sync` ghi vào đây. Format theo [Keep a Changelog](https://keepachangelog.com),
versioning theo [SemVer](https://semver.org). **8sync rule:** mỗi PR cập nhật mục `Unreleased`.

## [Unreleased]

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
