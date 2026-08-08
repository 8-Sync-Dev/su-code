# MCP servers and the tool surface

## 1. MCP config locations (mcp-config.md)

OMP-owned, in order of preference:
- project `.omp/mcp.json` (also reads `.omp/.mcp.json` for compat)
- user `~/.omp/agent/mcp.json` (profile: `~/.omp/profiles/<name>/agent/mcp.json`; also `.mcp.json`)
- portable fallback at project root: `mcp.json`, then `.mcp.json`

**OMP writes only to the primary `mcp.json` paths.** Project `.omp/mcp.json` is keyed to the cwd,
so it applies under every profile; user MCP config is profile-isolated (a profile never sees the
default profile's `~/.omp/agent/mcp.json`).

Imported tool-native sources: Claude Code (`~/.claude.json`, `~/.claude/mcp.json`,
`.claude/.mcp.json`, `.claude/mcp.json`) · Codex (`config.toml` `[mcp_servers.*]`) · Gemini CLI
(`settings.json`) · OpenCode (`opencode.json`) · Cursor (`mcp.json`) · Windsurf
(`mcp_config.json`) · VS Code (`.vscode/mcp.json`, key `mcp.servers`, project-only) · installed
Claude marketplace plugins and OMP extension packages.

MCP is the one native capability that does **not** use the non-empty-`.omp/` admission helper —
it reads the four native paths directly (config-usage.md).

### Precedence (first definition wins; duplicates are never merged)
1. OMP native → 2. OMP extension packages → 3. Claude Code → 4. Claude marketplace plugins + Codex
→ 5. Gemini CLI → 6. OpenCode → 7. Cursor + Windsurf → 8. VS Code → 9. root `mcp.json`/`.mcp.json`.

Within OMP native: project `mcp.json` → project `.mcp.json` → user `mcp.json` → user `.mcp.json`.
A **differently named** definition is also shadowed when its transport, endpoint/command inputs,
auth, and request-id mode are equivalent to a higher-priority definition.

## 2. File shape

```json
{
  "$schema": "https://raw.githubusercontent.com/can1357/oh-my-pi/main/packages/coding-agent/src/config/mcp-schema.json",
  "mcpServers": {
    "codegraph": { "type": "stdio", "command": "codegraph", "args": ["mcp"] }
  },
  "disabledServers": ["unwanted"],
  "enabledServers": ["tool-owned-server"]
}
```

Top-level keys: `$schema` · `mcpServers` · `disabledServers` (active-profile **user** denylist,
highest precedence, hides a discovered server by name regardless of its `enabled`) ·
`enabledServers` (active-profile user allowlist; force-enables a source that says `enabled:false`,
but `disabledServers` still wins).

Server names: ≤100 chars, `[a-zA-Z0-9_.:-]`. The bundled JSON schema omits `:` from its
`propertyNames` pattern, so namespaced plugin names (`cloudflare:cloudflare-api`) are valid at
runtime but flagged by editors.

Shared fields: `enabled?` · `timeout?` (ms; `0` disables client-side timeout) ·
`requestIdFormat?: "number"|"string"` (default numeric; read **only** from OMP-native files, root
`mcp.json`/`.mcp.json`, and OMP extension packages) · `auth?` · `oauth?`.

`OMP_MCP_TIMEOUT_MS` has **process-wide precedence over every per-server `timeout`**; fallback
chain is env → server `timeout` → 30 000 ms.

Transports:
- `stdio` (default when `type` omitted): requires `command`; optional `args`, `env`, `cwd`
- `http`: requires `url`; optional `headers`
- `sse`: legacy protocol-revision 2024-11-05 HTTP+SSE; requires `url`; optional `headers`

Validation (`validateServerConfig`): stdio needs `command`; http/sse need `url`; both `command`
and `url` is an error; unknown `type` is rejected. Structural only — a valid URL can still fail at connect.

### Variable resolution (two distinct passes)
**Discovery-time** — OMP-native files and root fallback files expand `${VAR}` and `${VAR:-default}`
recursively across `command`, `args`, `env`, `cwd`, `url`, `headers`, `auth`, `oauth`. Unresolved
placeholders stay literal.

**Pre-connect** — for stdio `env` values and http/sse `headers` values, in order:
1. value starting with `!` → run the remainder as a shell command, 10 s timeout, use trimmed stdout
   (cached for the process). **Failed/timed-out/whitespace-only output omits that entry entirely.**
2. otherwise, if the whole value names a set, non-empty env var → use the env value
3. otherwise → use the string literally

So `"GITHUB_TOKEN": "GITHUB_TOKEN"` copies from the shell; a mistyped env-var name is sent literally.

## 3. Runtime lifecycle (mcp-runtime-lifecycle.md)

- Headless/SDK sessions **await** `discoverAndLoadMCPTools()`; interactive sessions build the
  manager up front and defer `discoverAndConnect()` to after the session exists.
- Filtering at load: `mcp.enableProjectConfig: false` removes every `level === "project"` entry
  before dedup (lets a same-named user entry survive); Exa servers always filtered
  (`filterExa: true`) with their API keys extracted for native Exa; browser-automation MCP servers
  filtered when `browser.enabled`.
- Handshake: MCP protocol version `2025-03-26`; advertises the `roots` capability; answers
  server→client `ping` and `roots/list`; unsupported methods return `-32601`.
- **Fast startup gate: 250 ms.** After that, still-pending servers contribute cached
  `DeferredMCPTool`s when `MCPToolCache` has them, otherwise nothing at startup — their tools
  register later through `#onToolsChanged`. A slow MCP server no longer blocks startup.
- Reconnect is `transport.onClose`-driven with backoff `500/1000/2000/4000 ms`; a circuit breaker
  suspends automatic reconnect after >5 attempts in 30 s (manual `/mcp reconnect` resets it).
  **There is no autonomous polling health monitor.**
- Notifications: `notifications/tools/list_changed`, `resources/list_changed`, `resources/updated`,
  `prompts/list_changed` are handled internally, then every notification (including custom ones) is
  fanned out to listeners. Buffered FIFO cap **100**, drop-oldest, drained into the first listener.
  `sdk.ts` bridges these into the extension event `mcp_notification` → `{ server, method, params }`.
- `/mcp reload` = `disconnectAll()` → `discoverAndConnect()` → `session.refreshMCPTools(...)`;
  changes apply without restarting the session. Config writes are atomic (temp file + rename).
- A subagent given `options.mcpManager` **borrows** the parent manager and never disconnects it.

## 4. MCP tools in the registry (mcp-server-tool-authoring.md)

Generated name: `mcp__<sanitized_server>_<sanitized_tool>` — lowercased, non-`[a-z_]` → `_`,
repeated underscores collapsed, a redundant `<server>_` prefix stripped once. Sanitization
collisions are resolved by lexicographic comparison of the original `<server>\0<tool>` origin key,
so reconnect/discovery order can never change ownership; the loser is logged and omitted.

**Outbound argument normalization** (before `tools/call`, both live and deferred):
1. non-object / `null` / array top level → `{}`
2. the harness-injected intent field **`i` is removed** unless the MCP tool's own
   `inputSchema.properties` declares `i`
3. a schema-declared but non-`required` property whose value is `undefined`, `""`, or an empty
   non-array object is **omitted**. Required props, undeclared props, `0`, `false`, `null`, and
   arrays (including empty) survive
4. strings are walked recursively; a resolvable `local://` file URL becomes the real filesystem path

Server authors must validate the **normalized** payload.

Errors map to `Error: …` (server `isError`) or `MCP error: …` (transport/runtime); abort becomes
`ToolAbortError`. Both tool classes attempt reconnect + one retry on retriable connection failures.

## 5. Built-in tool toggles (settings.md)

Per-tool boolean keys: `bash.enabled` (T) · `launch.enabled` (T) · `eval.py` (T) · `eval.js` (T) ·
`eval.rb` (F, `PI_RB`) · `eval.jl` (F, `PI_JL`) · `glob.enabled` · `grep.enabled` (T) ·
`fetch.enabled` · `browser.enabled` · `computer.enabled` (F) · `astEdit.enabled` (T) ·
`astGrep.enabled` (**F**) · `web_search.enabled` · `lsp.enabled` (T) · `debug.enabled` (T) ·
`checkpoint.enabled` (**F**, pairs `checkpoint`+`rewind`) · `inspect_image.mode`
(`auto`|`on`|`off`, default `auto`).

Session-scoped tool availability: CLI `--tools <names>` is an **allowlist and validates only
built-in tool names**; custom/MCP tool inclusion goes through discovery/registration and SDK
options (custom-tools.md). An allowlist rots silently — a name omp drops breaks every launch, a
name omp adds never reaches the agent.

Global tool settings: `tools.approvalMode` (default **`yolo`**) · `tools.approval` (per-tool
`allow|deny|prompt`) · `tools.maxTimeout` (s, `0` = no cap) · `tools.format` (wire dialect, default
`auto`) · `tools.intentTracing` (T) · `tools.outputMaxColumns` (768) ·
`tools.artifactSpillThreshold` (50 KB) · `tools.artifactHeadBytes` / `TailBytes` (20 KB) ·
`tools.artifactTailLines` (500).

### `loadMode`: `"essential"` vs `"discoverable"`
Every tool definition (built-in, custom, extension, MCP-bridged) carries `loadMode`
(custom-tools.md, extensions.md, rpc.md). Canonical **essential** built-ins:
`read`, `write`, `bash`, `edit`, `glob`, `computer`, `eval`, `task`, `hub`, `learn`, `manage_skill`
(also `browser`? — not documented as essential; `todo`, `checkpoint`, `rewind`, `recall`, `retain`,
`reflect`, `memory_edit`, `ast_edit`, `ast_grep`, `debug`, `ask`, `lsp` are documented
**discoverable**). Everything else defaults to `"discoverable"`; an explicit `loadMode` always wins.

**`tools.xdev`**: in an ordinary `tools.xdev` session, *discoverable* built-ins are presented as
`xd://<name>` tool devices instead of top-level function declarations; an **explicitly requested**
tool stays top-level (tools/checkpoint.md, tools/recall.md, tools/retain.md, tools/reflect.md,
tools/rewind.md, tools/memory_edit.md). This is the documented way to shrink the top-level tool
schema without disabling capabilities. (settings.md's published catalog does not itemize
`tools.xdev`; the tool docs are the citation.)

`xd://` mechanics (tools/write.md, tools/read.md, resolve-tool-runtime.md):
- `read xd://` lists mounted devices; `read xd://<name>` returns its generated input docs
- `write` with `path: "xd://<name>"` and `content` = one JSON args object dispatches it; the
  device's schema, updates, result blocks, error flag, renderer, and approval tier are preserved;
  `details.xdev` carries dispatch metadata
- `xd://resolve` / `xd://reject` finalize a staged preview (body = one-sentence reason);
  `xd://propose` submits a plan slug while plan mode is active
- unknown URI-like schemes are **refused** rather than silently creating a local file

## 6. Bash: policy vs routing (tools/bash.md, settings.md)

Two independent, differently-purposed layers:

| Setting | Purpose | Syntax | Effect |
| --- | --- | --- | --- |
| `bash.patterns` | may this command execute? | literal text + `*` | `allow` / `prompt` / `deny`, first match wins |
| `bashInterceptor.patterns` | which tool should do this? | JS regex + `tool` + `message` | returns a Bash **tool error** telling the model to call the named tool |

```yaml
bashInterceptor:
  enabled: true          # default false
  patterns:
    - pattern: '^\s*(cat|head|tail)\s+'
      tool: read
      message: "Use the read tool instead."
```

The named replacement tool **must be available in the session** or the interceptor does not block.
Matching runs against the original command and the `cd`-stripped command; for each, the complete
input first, then each flat command split on unquoted `&&`, `||`, `;`, `|`, `|&`, `&`, or newline
(excluding stages that consume piped stdin), then those fragments with leading `NAME=value`
assignments removed.

`bash.patterns` asymmetry: `deny`/`prompt` fire on the whole command **or any single segment**;
`allow` must match the **entire** command and never applies to a compound line. Critical commands
still require confirmation unless a rule explicitly denies them.

## 7. Built-in search/read behavior worth budgeting for

**`grep`** (tools/grep.md): defaults `case: true`, `gitignore: true`, `hidden: true` (hard-coded,
no model-facing flag). Context defaults `grep.contextBefore = 1`, `grep.contextAfter = 3`.
Caps: 20 files per page (`skip` = file-page offset), 20 matches/file multi-file, 200 single-file,
2 000 internal preselect, 512 chars/line, final output byte-capped at 50 KiB, 30 s native timeout,
4 MiB per-file size cap. Engine chain: Rust regex → PCRE2 (lookaround/backrefs) → literal recovery.
`omp://` as a path expands to every embedded doc.

**`read`** (tools/read.md): `read.defaultLimit` 300 lines (clamped to 3 000), `DEFAULT_MAX_BYTES`
50 KiB. `read.summarize.enabled` (T) produces structural summaries for parseable code ≤2 MiB and
≤20 000 lines and ≥`read.summarize.minTotalLines` (100); prose only with `read.summarize.prose`.
Handles archives, SQLite, PDFs/Office via markit, notebooks, images, profiler reports, and the
internal schemes `agent:// artifact:// history:// issue:// local:// mcp:// memory:// omp:// pr://
rule:// security:// skill:// ssh:// vault:// xd://`.

**`browser`** (tools/browser.md): `browser.enabled`, `browser.headless`, `browser.cdpUrl`,
`browser.relay` (+`browser.relayUrl`, default `http://127.0.0.1:9224`, env `PI_BROWSER_RELAY`),
`browser.cmux` (env `PI_BROWSER_CMUX`), `browser.screenshotDir`. `open` kind resolution:
`app.cdp_url` → `app.path` → relay → `browser.cdpUrl` → cmux → headless. Headless prefers a
detected system Chrome/Chromium, then `PUPPETEER_EXECUTABLE_PATH`, then downloads Chromium.
Timeout 30 s (clamp 1–300). Tabs are process-global by `name`; `open` before `run`; stealth
patches apply **only** in headless mode.
