# Hooks, extensions, custom tools, plugins

## 0. Which surface to use (extensions.md, custom-tools.md, hooks.md)

- **Extensions** (`ExtensionAPI`) — the unified, current API: events + tools + commands + shortcuts
  + flags + renderers + provider registration. **Strict superset of hooks.** Use this.
- **Hooks** (`HookAPI`) — legacy event-only API. Still accepted: `--hook` is an alias for
  `--extension`, and JS/TS hook factories discovered via `hookCapability` (e.g. `.omp/hooks/pre/*.ts`)
  are loaded **as extension modules** so their `pi.on(...)` handlers bind to the runtime event bus.
  Tools are wrapped by `ExtensionToolWrapper`, not `HookToolWrapper`.
- **Custom tools** — tool-focused modules; when loaded alongside extensions they are adapted and
  still pass through extension interception.
- **Skills** — passive content, not executable.

## 1. Module contract

```ts
import type { ExtensionAPI } from "@oh-my-pi/pi-coding-agent";
export default function (pi: ExtensionAPI) { /* register only */ }
```

Default export must be a **function**; may return a promise (awaited). Registration happens during
load; **runtime action methods throw `ExtensionRuntimeNotInitializedError` if called during load**.
Wiring happens later in `ExtensionRunner.initialize(...)`.

Legacy hook form is identical with `HookAPI` from
`@oh-my-pi/pi-coding-agent/extensibility/hooks` (the package root does not re-export `HookAPI`).

## 2. Discovery & load order (extension-loading.md)

`discoverAndLoadExtensions()` builds one ordered list, then imports each path:

1. **native auto-discovered modules** (provider `native` only)
   - `<cwd>/.omp/extensions/` (cwd-only, no ancestor walk; root must be non-empty)
   - active agent dir `extensions/` (default `~/.omp/agent/extensions`)
   - legacy `settings.json#extensions` string arrays at both scopes
2. **discovered JS/TS hook factories** from the `hook` capability
   (native roots: `.omp/hooks/pre/*`, `.omp/hooks/post/*`)
3. **installed plugin extension entries** (`package.json` `omp.extensions` / legacy `pi.extensions`)
4. **explicit configured paths**: CLI `--extension`/`-e`/`--hook` first, then the merged
   `extensions:` setting array

Dedup by **absolute path, first seen wins** — a module both auto-discovered and configured loads
once, at the auto-discovered position.

Path resolution for configured entries: normalize (`file://`, `@/abs`, stray leading `:`), expand
`~`, resolve relative against cwd, **reject `local://`**. A directory resolves to
`package.json#omp.extensions` → `index.ts` → `index.js` → one-level scan for `*.ts`/`*.js`,
`subdir/index.{ts,js}`, or `subdir/package.json` with a manifest. No recursion beyond one level;
`.ts` preferred over `.js`; symlinks are eligible.

Auto-scanning is limited to `.ts`/`.js`. Installed-plugin **manifest** entries additionally accept
`.mjs`/`.cjs` and directory `index.{ts,js,mjs,cjs}`.

Ignore behavior differs: native auto-discovery uses native glob with `gitignore: true`,
`hidden: false`; explicit configured directory scanning uses `readdir` and **does not** apply gitignore.

Import: realpath + dynamic import with an `?mtime` cache-buster (edited source reloads). A scoped
Bun `onLoad` hook rewrites legacy `@mariozechner/*`, `@earendil-works/*`, and bare
`@sinclair/typebox` specifiers onto host-bundled copies.

### Disabling
- `--no-extensions` (CLI) / `disableExtensionDiscovery` (SDK): ambient factories excluded; explicit
  `-e`/`--hook` paths still load, and only sibling capability roots of explicitly named packages
  remain eligible. Project/user `extensions:` settings and installed OMP extension packages are excluded.
  **Not a whole-process capability switch** — skills, MCP, tools, prompts, rules keep their own toggles.
- `disabledExtensions: ["extension-module:<derivedName>"]` — `derivedName` = filename stem, or the
  directory name for `index.ts`-style entries (`/x/foo.ts` → `foo`, `/x/bar/index.ts` → `bar`).
  Also accepts `skill:<name>` entries (skills.md).

Extension **packages** loaded through `extensions:` or `--extension` also get their sibling
capability dirs discovered by the `omp-plugins` provider: `skills/`, `hooks/pre|post/`, `tools/`,
`commands/`, `rules/`, `prompts/`, `.mcp.json`, and (via task discovery) `agents/`.

Failures are per-path (`{ path, error }`) and never abort the rest. Extensions are **not sandboxed** —
one process, one `EventBus`, one `ExtensionRuntime`.

## 3. Event catalog (extensions.md, hooks.md, skills/authoring-hooks.md)

### Tool lifecycle — the interception core
| Event | When | Return |
| --- | --- | --- |
| `tool_call` | before **every** tool execution | `{ block?: boolean; reason?: string; input?: Record<string,unknown> }` |
| `tool_result` | after execution | `{ content?; details?; isError? }` |
| `tool_execution_start` / `_update` / `_end` | observability | — |
| `tool_approval_requested` / `_resolved` | observability (only when a tool requires approval **and** an approval handler is registered) | — |

`tool_call` contract:
- **any** handler returning `{ block: true }` stops execution immediately; `reason` becomes the tool
  error text the model sees
- a handler that **throws** also blocks (**fail-closed**)
- last non-blocking return wins; first `block: true` short-circuits
- a non-blocking handler may return `input` to **replace the raw execution arguments** — handlers do
  not observe each other's revisions; ignored when `block` is true; **not applied to `computer` calls**
- for model-issued calls it fires at arg-prep time in the agent loop, so a revision is revalidated
  and seen by concurrency scheduling, execution events, the persisted assistant message, and the
  approval gate

`tool_result` is middleware-style in the extension runner (each handler sees prior modifications);
in the legacy `HookRunner` each handler sees the original and **last override wins**. `isError` is
typed but **not propagated** by `HookToolWrapper`; on failure the original error is rethrown after
handlers run.

### Context / turn
`context` → `{ messages?: Message[] }`, **chained**: each handler receives the previous handler's
output. Runs before **each** LLM API call. This is the only documented way to rewrite the exact
message array sent to the provider.

Also: `input` · `before_agent_start` (`{ message? }`, first returned message wins) ·
`before_provider_request` (may replace the provider request payload) · `after_provider_response` ·
`agent_start` / `agent_end` (notification-only) · `turn_start` / `turn_end` ·
`message_start` / `_update` / `_end` · `session_stop` (main-session only, awaited before settle;
`{ continue: true, additionalContext }` or `{ decision: "block", reason }`; capped at **8**
consecutive continuations; never fires for subagents).

### Session lifecycle
`session_start` · `session_before_switch` (`{cancel?}`) / `session_switch` ·
`session_before_branch` (`{cancel?, skipConversationRestore?}`) / `session_branch` ·
`session_before_compact` (`{cancel?, compaction?}`) · `session.compacting`
(`{context?: string[], prompt?, preserveData?}`) · `session_compact` ·
`session_before_tree` (`{cancel?, summary?}`) / `session_tree` · `session_shutdown`.

### Reliability / MCP
`auto_compaction_start`/`_end` · `auto_retry_start`/`_end` · `ttsr_triggered` · `todo_reminder` ·
`goal_updated` · `credential_disabled` · `user_bash` / `user_python` (override with `{ result }`) ·
`mcp_notification` (`{ server, method, params }`, fires **after** the manager's own handling;
startup frames buffered FIFO cap 100, drop-oldest).

Ordering: capability providers priority-sorted, dedup key for hooks is `${type}:${tool}:${name}`.
Runtime handler order = hooks/extensions array order, then registration order per handler.

## 4. Registering tools and commands

```ts
pi.registerTool({
  name: "search_notes", label: "Search Notes", description: "…",
  parameters: pi.zod.object({ query: pi.zod.string() }),
  hidden: false, defaultInactive: false, deferrable: false,
  loadMode: "essential",          // "discoverable" by default
  approval: "exec",               // "read" | "write" | "exec"
  strict: true,
  async execute(toolCallId, params, signal, onUpdate, ctx) { … },
  onSession(event, ctx) {}, renderCall(){}, renderResult(){},
});
```

Schema builders injected on `pi`: `pi.zod` (Zod-compatible, omptype-backed), `pi.arktype`
(native `type(...)`), `pi.typebox` (legacy shim). Also `pi.logger`, `pi.pi` (package exports).

**`ctx.invokeTool`** — a tool that re-registers a built-in **name** (e.g. wrapping `write` or
`grep`) receives `ctx.invokeTool(params, { signal, onUpdate })`, which runs the **native** built-in
of the same name, including its side effects and bookkeeping. Delegation is same-tool only (cannot
reach another target or escalate past the granted approval); it is `undefined` for a net-new tool
that shadows nothing; self-recursion is guarded.

Message delivery: `pi.sendMessage(msg, { deliverAs })` — `"steer"` (default, interrupts the run) |
`"followUp"` (after the run) | `"nextTurn"` (injected on the next user prompt); `triggerTurn: true`
starts a turn when idle. `pi.sendUserMessage(content, { deliverAs })` always goes through prompt flow.

Other API: `registerCommand`, `registerShortcut`, `registerFlag`, `registerMessageRenderer`,
`registerAssistantThinkingRenderer`, `registerProvider`, `setLabel`/`getFlag`,
`getActiveTools`/`getAllTools`/**`setActiveTools`**, `getCommands`, `getSessionName`/`setSessionName`,
`setModel`, `get/setThinkingLevel`, `get/setServiceTiers`, `appendEntry`, `exec`, `events`.

`ctx` (handlers + tool `execute`): `ui`, `hasUI`, `cwd`, read-only `sessionManager`,
`modelRegistry`, `model`, `models` (`list/current/resolve/family`), `localProtocolOptions`,
`getContextUsage()`, `getAsyncJobSnapshot()`, `compact(...)`, `isIdle()`, `hasPendingMessages()`,
`abort()`, `shutdown()`, `getSystemPrompt()`, `memory`, and **managed timers**
`ctx.setInterval` / `ctx.setTimeout` / `ctx.clearTimer`.

> **Background-work hazard.** A raw `setInterval`/`setTimeout`/detached-promise callback that throws
> escapes handler dispatch, surfaces as `uncaughtException`, and the global postmortem handler
> **tears down the whole session**. `ctx.setInterval`/`ctx.setTimeout` contain the throw, are
> `unref`'d, and auto-clear on `session_shutdown`.

Command context adds `waitForIdle()`, `newSession()`, `switchSession()`, `branch()`,
`navigateTree()`, `reload()`, `compact()`.

Reserved shortcuts (silently ignored): `ctrl+c ctrl+d ctrl+z ctrl+k ctrl+p ctrl+l ctrl+o ctrl+t
ctrl+g ctrl+q alt+m shift+tab shift+ctrl+p alt+enter escape enter`.
Command names clashing with built-ins are skipped with a diagnostic.

## 5. Custom tools (custom-tools.md)

Two integration paths: SDK `options.customTools`, and filesystem discovery via
`discoverAndLoadCustomTools(configuredPaths, cwd, builtInToolNames)`.

Discovery merges: capability providers (`~/.omp/agent/tools`, `.omp/tools`, `~/.claude/tools`,
`.claude/tools`, `~/.codex/tools`, `.codex/tools`, Claude marketplace plugin cache) + installed
plugin manifests (`~/.omp/plugins/node_modules/*`) + explicit configured paths.
Native tool files: `tools/*.{json,md,ts,js,sh,bash,py}` and `tools/<name>/index.ts` — **`.md`/`.json`
are metadata only**, the executable loader rejects them.

Tool names must be globally unique; conflicts against built-ins and already-loaded custom tools are
rejected. Restricted sessions (`restrictToolNames: true`) exclude SDK custom tools unless
`allowRestrictedCustomTools: true`, and then only names present in `toolNames`.

`CustomToolAPI` gives `cwd`, `exec`, `ui`, `hasUI`, `logger`, `arktype`, `typebox`, `pi`, and
`pushPendingAction(action)` (stages a preview finalized by writing to `xd://resolve` / `xd://reject`).

## 6. Persisting extension state (extensions.md, session.md)

`pi.appendEntry("<customType>", data)` writes a `custom` session entry; rebuild on `session_start`,
`session_branch`, `session_tree` by scanning `ctx.sessionManager.getBranch()`.

> **`customType` is a global namespace.** Core reserves values such as `tool_execution_start` and
> `session_exit`. Extensions MUST use a reverse-domain / package-qualified id
> (`com.example.my-extension.state`); a collision makes core replay logic interpret extension data
> as lifecycle state.

## 7. Plugins & marketplace (plugin-manager-installer-plumbing.md, marketplace.md)

On-disk (user data root `~/.omp/plugins`, or `$XDG_DATA_HOME/omp/plugins` after
`omp config init-xdg` + XDG vars):

```
~/.omp/marketplaces.json                     # configured catalogs
~/.omp/plugins/{package.json,node_modules/}  # bun manifest + installs/link/marketplace symlinks
~/.omp/plugins/omp-plugins.lock.json         # enabled / features / settings
~/.omp/plugins/installed_plugins.json        # user-scope marketplace installs (version 2)
~/.omp/plugins/cache/{marketplaces,plugins}/
<project>/.omp/plugins/{node_modules,omp-plugins.lock.json,installed_plugins.json}
<project>/.omp/plugin-overrides.json         # read-only to omp: disable plugins, override features
```

Manifest resolution: `package.json.omp` → `package.json.pi` → `{ version }`. No strict schema
validation; runtime discovery **skips** packages with neither key. `manifest.version` is always
overwritten from `package.version`.

Install specs: `pkg`, `pkg[*]`, `pkg[]`, `pkg[a,b]`, `@scope/pkg@1.2.3[feat]`; git
(`github:user/repo#ref`, `gitlab:`, `bitbucket:`, `codeberg:`, `sourcehut:`/`srht:`, full URLs).
Install runs `bun install`, validates every declared extension entry imports to a factory, and
**rolls back** `package.json` + `bun.lock` + package tree on any post-install failure. Effective
enablement = `runtimeEnabled && !projectDisabled`; enabled project scope shadows enabled user
scope. **No cross-process lock on the lockfile — concurrent writers overwrite each other.**

Marketplace catalog: `.omp-plugin/marketplace.json` (preferred) or `.claude-plugin/marketplace.json`
(fallback). Required `name`, `owner.name`, `plugins[]`; each plugin needs `name` + `source`.
Sources: `./path` (inside the marketplace root, after optional `metadata.pluginRoot`),
`{source:"github",repo,ref,sha}`, `{source:"url",url,sha}`, `{source:"git-subdir",url,path,ref,sha}`.
**npm sources parse but the installer rejects them.** Names: lowercase alnum + `-`/`.`, start/end
alnum, ≤64 chars; `name@marketplace` ≤128.

TUI marketplace mutations update disk and invalidate discovery caches but **do not refresh the
active session** — `/reload-plugins` for skills/commands/MCP; restart for new tools, hooks, or
extension modules.
