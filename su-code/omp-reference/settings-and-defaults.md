# Settings: layers, precedence, and the keys that matter

## 1. Where settings live (settings.md, config-usage.md)

| Scope | Path | Read | Write |
| --- | --- | --- | --- |
| Global | `~/.omp/agent/config.yml` (or an existing `config.yaml`) | main persistent file | `/settings`, `omp config set`, `omp config reset` write **here** |
| Global legacy | `~/.omp/agent/settings.json` | migrated once, only when neither YAML name exists | renamed to `settings.json.bak` after migration |
| Project | `<cwd>/.omp/config.yml` (+ `.omp/settings.json`) | loaded when cwd `.omp/` is **non-empty** | settings commands do **not** write arbitrary project keys |
| CLI overlay | `--config <file>` (repeatable) | after global+project, process-local | never persisted |
| Env overlay | `PI_CONFIG_FILES` (`:` unix / `;` win) | loaded in order **before** `--config` | never persisted |
| Runtime | in-memory | dedicated CLI flags + feature env vars | never persisted |

**Native project settings do not walk ancestors** — only the cwd's `.omp/`.
The one supported project write path is a model-selector role assignment when
`modelRoleStorage: project`, which touches only `modelRoles` in `<cwd>/.omp/config.yml`.
Saves are debounced and re-read the file under a lock, so external edits made while a session is
open are preserved.

### Precedence (low → high)
```
built-in defaults ← global config ← project config ← PI_CONFIG_FILES ← --config ← runtime overrides
```
Runtime overrides = `--model`, `--smol`, `--slow`, `--plan`, `--approval-mode`,
`--auto-approve`/`--yolo`, `--hide-thinking`, `--advisor`, `--no-pty`, `--api-key`, protocol-mode
defaults.

### Merge rules — the one that bites
- **Objects deep-merge.**
- **Scalars and arrays are replaced wholesale.** A project `disabledProviders: [groq]` *replaces*
  the global list entirely. Same for `enabledModels`, `cycleOrder`, `extensions`, and every other
  array-typed setting.

### Settings-capability caveat (config-usage.md §8)
Settings capability items are **not** deduplicated. `Settings.#loadProjectSettings()` deep-merges
project items in returned order, and providers are visited highest → lowest priority, so a
**lower-priority provider's project settings can override a higher-priority one's**. Within the
native provider, project `config.yml` follows and overrides `settings.json`; native
`.omp/config.yml` model roles are then reapplied as the authoritative project model-role layer.

### File format & failure
Canonical global file is YAML `config.yml`; `config.yaml` accepted. The generic loader also accepts
`.yaml`, `.json`, `.jsonc` and migrates a sibling `.json` to `.yml` once per process when the
`.yml` form is requested. A settings YAML whose top level is not a mapping is invalid: on writable
startup omp moves it to `.broken-<timestamp>-<pid>-<uuid>` under a file lock and **exits** with the
original error and the backup path. `--config` / `PI_CONFIG_FILES` overlays are **strict** —
missing file, invalid YAML, or non-mapping root is a hard error and is *not* quarantined.

### CLI
```
omp config list [--json]      # every key with effective value + type (credentials masked)
omp config get <key> [--json] # single key, unmasked
omp config set <key> <value>  # parsed against the key's schema type; writes GLOBAL yml
omp config reset <key>        # writes the schema DEFAULT (does not delete the key)
omp config path               # active agent directory (honors PI_CODING_AGENT_DIR)
omp config init-xdg           # create XDG data/state/cache roots (Linux/macOS); moves nothing
```
Value parsing: boolean accepts `true/false/yes/no/on/off/1/0` (case-insensitive); number rejects
`Infinity`/`NaN`; array needs a JSON array; record needs a JSON object; enum must match exactly.
**Keys must match a schema path exactly — no shorthand** (`theme.dark`, never `theme`).

### Path-scoped arrays
Only `enabledModels` and `disabledProviders` accept scoped entries:
```yaml
disabledProviders:
  - ollama                 # everywhere
  - paths: [~/projects/sensitive]
    providers: [anthropic, openai]
```
Path keys: `path`, `paths`, `pathPrefix`, `pathPrefixes`. Value keys: `models`/`providers`, or
`values`/`items`. Applies when cwd **is** or is **under** the path. Resolved **after** layer merge.

### `disabledProviders` — one namespace, two subsystems
- **model providers**: `anthropic`, `openai`, `google`, `groq`, `ollama`, `openrouter`, … →
  removed from model selection even with credentials present
- **discovery sources**: `native`, `claude`, `codex`, `gemini`, `github`, `opencode`, `cursor`,
  `agents-md` → the **entire config source** is removed: context files *and* MCP servers, slash
  commands, skills, hooks, tools, prompts, settings

`google` ≠ `gemini`. Disabling `claude` is far heavier than dropping `CLAUDE.md`.

## 2. Profiles, config dirs, XDG (config-usage.md)

Source priority for generic helpers: `.omp` → `.claude` → `.codex` → `.gemini`.
User bases `~/<PI_CONFIG_DIR>/agent` (normally `~/.omp/agent`), `~/.claude`, `~/.codex`, `~/.gemini`.
Project bases `<cwd>/.omp`, `<cwd>/.claude`, `<cwd>/.codex`, `<cwd>/.gemini`.

- `PI_CONFIG_DIR` changes the config-root **dirname** under home (default `.omp`) for generic helpers.
- `PI_CODING_AGENT_DIR` changes `getAgentDir()` (native discovery, settings, runtime state,
  `agent.db`) for the **default profile only**; named profiles ignore it, and it does **not** change
  the generic `getConfigDirs()`/`findConfigFile()` OMP base.
- `omp --profile <name>` / `OMP_PROFILE` / legacy `PI_PROFILE` relocate every OMP-native user path
  to `~/.omp/profiles/<name>/agent/…` — commands, rules, prompts, instructions, hooks, tools,
  extensions, settings, skills, MCP, `SYSTEM.md`/`RULES.md`/`AGENTS.md`, runtime state.
  `OMP_PROFILE` wins over `PI_PROFILE` even when explicitly empty. **Keybindings are the sole
  exception** — a named profile merges the default profile's `keybindings.*` under its own.
- XDG (Linux/macOS): an existing `$XDG_{DATA,STATE,CACHE}_HOME/omp` relocates that category; for a
  named profile only when it already contains `omp/profiles/<name>`. Run `omp config init-xdg` first.

Native `.omp` directory admission: slash commands, directory rules, prompts, instructions, hooks,
tools, extensions, extension modules, and settings require the root dir to **exist and be non-empty**.
Skills and MCP do **not**.

## 3. Key catalog — what a harness actually touches

### Prompt weight / latency
| Key | Default | Note |
| --- | --- | --- |
| `includeModelInPrompt` | `true` | active model name in the system prompt |
| `skills.enabled` | `true` | `false` ⇒ zero skills discovered |
| `skills.enableSkillCommands` | — | registers one `/skill:<name>` per skill |
| `skills.ignoredSkills` / `skills.includeSkills` | — | glob exclude / glob allowlist |
| `skills.customDirectories` | — | extra non-recursive `*/SKILL.md` roots |
| `skills.enable{Codex,Claude}User`, `enableClaudeProject`, `enablePi{User,Project}`, `enableAgents{User,Project}` | — | per-source gates |
| `commands.enableClaude{User,Project}`, `commands.enableOpencode{User,Project}` | — | per-source gates |
| `disabledExtensions` | `[]` | `extension-module:<name>`, `skill:<name>` |
| `disabledProviders` | `[]` | model backends **and** discovery sources |
| `mcp.enableProjectConfig` | — | `false` drops every project-level MCP source before dedup |
| `ttsr.enabled` / `ttsr.builtinRules` / `ttsr.disabledRules` | `true` / `true` / `[]` | |
| `magicKeywords.enabled` / `.ultrathink` / `.orchestrate` / `.workflow` | all `true` | |
| `tools.xdev` | — | discoverable built-ins presented as `xd://<name>` instead of top-level tools |

### Tool enable/disable
`bash.enabled` T · `launch.enabled` T · `eval.py` T · `eval.js` T · `eval.rb` F · `eval.jl` F ·
`glob.enabled` · `grep.enabled` T · `fetch.enabled` · `browser.enabled` · `computer.enabled` F ·
`astEdit.enabled` T · `astGrep.enabled` **F** · `web_search.enabled` · `lsp.enabled` T ·
`debug.enabled` T · `checkpoint.enabled` **F** · `inspect_image.mode` `auto`.

### Approvals & routing
`tools.approvalMode` **`yolo`** (`always-ask` | `write` | `yolo`) · `tools.approval` record
(`allow`/`deny`/`prompt` per tool) · `tools.maxTimeout` `0` · `tools.format` `auto` ·
`tools.intentTracing` T · `tools.outputMaxColumns` `768` · `tools.artifactSpillThreshold` `50` KB ·
`tools.artifactHeadBytes`/`TailBytes` `20` KB · `tools.artifactTailLines` `500`.
`bash.patterns[]` (`match` + `approval`) · `bashInterceptor.enabled` **F** + `bashInterceptor.patterns[]`
(`pattern` regex, `tool`, `message`).

### Files
`edit.mode` `hashline` (`apply_patch`|`hashline`|`patch`|`replace`) · `edit.fuzzyMatch` T ·
`edit.fuzzyThreshold` `0.95` · `edit.blockAutoGenerated` T · `edit.streamingAbort` F ·
`read.defaultLimit` `300` · `read.summarize.enabled` T · `read.summarize.prose` F ·
`read.summarize.minTotalLines` `100` · `read.toolResultPreview` F · `readLineNumbers` F ·
`grep.contextBefore` `1` · `grep.contextAfter` `3`.

### LSP
`lsp.enabled` T · `lsp.lazy` T · `lsp.shared` T · `lsp.diagnosticsOnWrite` T ·
`lsp.diagnosticsOnEdit` F · `lsp.formatOnWrite` F · `lsp.diagnosticsDeduplicate` T.

### Models & thinking
`modelRoles` record — built-in roles `default, smol, slow, vision, plan, designer, commit, tiny,
task, advisor` (values may carry `:minimal|:low|:medium|:high|:xhigh|:max`) ·
`modelRoleStorage` `global`|`project` · `modelTags` · `modelProviderOrder` ·
`cycleOrder` `["smol","default","slow"]` · `enabledModels` `[]`.
`defaultThinkingLevel` `high` (+`auto`) · `hideThinkingBlock` F ·
`thinkingBudgets.{minimal,low,medium,high,xhigh,max}` = `1024/2048/8192/16384/32768/32768` ·
`providers.autoThinkingMaxEffort` `xhigh`.
Sampling `temperature/topP/topK/minP/presencePenalty/repetitionPenalty` all `-1` (= provider
default, parameter omitted) · `textVerbosity` `medium` · `tier.{openai,anthropic,google,subagent,advisor}` ·
`personality` `default`.

### Retry / fallback
`retry.enabled` T · `retry.maxRetries` `10` · `retry.baseDelayMs` `500` · `retry.maxDelayMs` `300000` ·
`retry.modelFallback` T · `retry.fallbackRevertPolicy` `cooldown-expiry` · `retry.fallbackChains`
(keys: role name, `provider/model-id`, or `provider/*`; model-oriented keys win over roles;
`default` covers every role without its own chain).

### Task / subagents
`task.batch` T · `task.enableEffort` **F** · `task.maxEffort` `max` · `task.maxConcurrency` ·
`task.maxRecursionDepth` `2` · `task.maxRuntimeMs` `0` · `task.agentIdleTtlMs` `420000` ·
`task.disabledAgents` · `task.agentModelOverrides` · `task.isolation.mode`
(`none|auto|apfs|btrfs|zfs|reflink|overlayfs|projfs|block-clone|rcopy`) · `task.prewalk` F ·
`task.agentPrewalk` record.

### Advisor
`advisor.enabled` F · `advisor.subagents` F · `advisor.syncBacklog` `off` · `advisor.immuneTurns` `3`.
Needs `modelRoles.advisor` to resolve. `WATCHDOG.md` drives it (advisor-watchdog.md).

### Interaction / UI
`steeringMode` / `followUpMode` `one-at-a-time` · `interruptMode` `immediate` ·
`doubleEscapeAction` `tree` · `autoResume` F · `ask.timeout` `0` · `ask.notify` `on` ·
`theme.dark` `titanium` / `theme.light` `light` · `symbolPreset` `unicode` · `statusLine.*` ·
`startup.showSplash` / `startup.quiet` · `tui.hyperlinks` `auto`.

### Marketplace / update
`marketplace.autoUpdate`: `off` | **`notify` (default)** | `auto`. Catalogs older than 24 h are
refreshed best-effort before version checks. **Despite the name, `notify` only writes availability
to the debug log — there is no user-facing notification** (marketplace.md).

### Other groups exposed by `omp config list`
`github.*`, `async.*`, `goal.*`, `loop.*`, `todo.*` (`tasks.todoClearDelay` `60` s, display-only),
`display.*`, `share.*`, `collab.*`, `stt.*`/`tts.*`, `memories.*`/`hindsight.*`/`mnemopi.*`,
`secrets.enabled` F, `contextPromotion.enabled` F, `snapcompact.*`, `branchSummary.*`.

## 4. Environment variables that override settings (settings.md, environment-variables.md)

Env vars are **not a settings layer** — each is read by the owning feature and never written back.

| Env | Overrides |
| --- | --- |
| `PI_SMOL_MODEL` / `PI_SLOW_MODEL` / `PI_PLAN_MODEL` | `modelRoles.smol` / `.slow` / `.plan` |
| `PI_PY` / `PI_JS` / `PI_RB` / `PI_JL` | `eval.py` / `.js` / `.rb` / `.jl` (`0` disables) |
| `PI_NO_PTY=1` | disables PTY bash (= `--no-pty`) |
| `PI_TINY_DEVICE` / `PI_TINY_DTYPE` | `providers.tinyModelDevice` / `.tinyModelDtype` |
| `OMP_AUTH_BROKER_URL` / `_TOKEN` | `auth.broker.url` / `.token` (env wins) |
| `OMP_MCP_TIMEOUT_MS` | **every** per-server MCP `timeout`; `0` disables |
| `PI_CODING_AGENT_DIR` | relocates the agent dir (default profile only) |
| `PI_CONFIG_DIR` | config-root dirname under home |
| `OMP_PROFILE` / `PI_PROFILE` | active profile |
| `PI_CONFIG_FILES` | settings overlays, loaded before `--config` |
| `PI_INTENT_TRACING` | `tools.intentTracing` |
| `PI_EDIT_VARIANT` | forces `edit.mode` (`patch`/`replace`/`hashline`/`apply_patch`) |
| `PI_STRICT_EDIT_MODE=1` | disables model-specific edit-mode fallbacks |
| `PI_BROWSER_RELAY` / `PI_BROWSER_CMUX` | `browser.relay` / `browser.cmux` |
| `PUPPETEER_EXECUTABLE_PATH` | browser Chromium binary |
| `PI_DISABLE_LSPMUX=1` | forces direct LSP spawning |
| `PI_BLOCKED_AGENT` | blocks one subagent type |
| `PI_TASK_MAX_OUTPUT_BYTES` / `_LINES` | `500000` / `5000` |
| `PI_NO_TITLE` | disables auto session titles |
| `OMP_SKIP_SETUP` | skips interactive setup scenes |
| `NULL_PROMPT=true` | **system prompt builder returns an empty string** |
| `PI_TIMING`, `PI_DEBUG_STARTUP` | startup span tree / phase markers to stderr |
| `PI_BASH_NO_CI`, `PI_BASH_NO_LOGIN`, `PI_SHELL_PREFIX` | shell env shaping (`CLAUDE_*` aliases as fallback) |

`.env` loading order (`$env` in `packages/utils/src/env.ts`), each filling only empty/unset keys:
process env → project `.env` (launch cwd) → `~/.omp/agent/.env` → `~/.omp/.env` → `~/.env`.
Inside each `.env`, **every `OMP_*` key is mirrored to its `PI_*` alias and replaces a same-file
`PI_*` value.** Names must be shell identifiers.

## 5. Legacy migrations already applied by omp

`inspect_image.enabled`→`inspect_image.mode` · `queueMode`→`steeringMode` · `ask.timeout` ms→s
(when `>1000`) · flat `theme`→`theme.dark`/`.light` · `task.isolation.enabled`→`.mode` ·
`task.simple` removed · `worktree`/`fuse-overlay`/`fuse-projfs`→`rcopy`/`overlayfs`/`projfs` ·
`lastChangelogVersion` moved to a **marker file** and stripped from `config.yml` ·
`memories.enabled:true`→`memory.backend:local` · `providers.webSearch`/`providers.image` enums →
head of `providers.webSearchOrder`/`.imageOrder`.

Startup migration to `config.yml` runs only when neither `config.yml` nor `config.yaml` exists:
`~/.omp/agent/settings.json` (renamed `.bak`) merged with legacy `agent.db` settings (DB wins).
