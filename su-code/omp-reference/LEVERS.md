# LEVERS — how an external tool steers omp

Everything below is documented behavior from `omp://` docs, cited inline. This is the design input
for `8sync harness`.

**Ground rule that decides most of this:** omp rebuilds the **system prompt on every provider
request**. Anything rendered into a prompt block therefore survives compaction *by construction*.
Anything injected as a conversation entry (magic-keyword notice, `/skill:` injection, TTSR
`<system-interrupt>`, `before_agent_start` message) is ordinary history and is discarded once the
compaction boundary passes it. TTSR is the exception that matters: its *rules* re-evaluate on every
stream, so its steering power is compaction-proof even though its injected text is not.

---

## The lever table

| # | Lever | Mechanism | Exact path / key | Survives compaction? | Risk |
|---|---|---|---|---|---|
| 1 | **Append to the default prompt** | text appended to the rendered prompt; default template fully retained | `<cwd>/.omp/APPEND_SYSTEM.md`, else `~/.omp/agent/APPEND_SYSTEM.md`; flag `--append-system-prompt` | **Yes** — prompt block, rebuilt every request | Cheapest safe injection. Unbounded growth costs every turn. No ancestor walk: launch from the dir that owns `.omp/`. |
| 2 | **Replace the base prompt** | switches to `custom-system-prompt.md` | `<cwd>/.omp/SYSTEM.md` / `~/.omp/agent/SYSTEM.md`; flag `--system-prompt` | **Yes** | **High.** Drops omp's tool inventory, tool policy, internal-URL catalog, exploration/delegation/workflow rules and `xd://` guidance. No selective inheritance — you must re-author them. (system-prompt-customization.md) |
| 3 | **Sticky always-apply rule** | `RULES.md` synthesized as rule name `RULES`, forced `alwaysApply: true`, re-attached near the current turn | `~/.omp/agent/RULES.md` and the **nearest non-empty** `<ancestor>/.omp/RULES.md` | **Yes** — always-apply body in `<generic-rules>` | User `RULES.md` **shadows** the project one (name-based dedup, they never concatenate). A regular `.omp/rules/RULES.md` shadows both. Keep it short — it costs every turn. (context-files.md) |
| 4 | **Project context files** | injected as `<repo-rules>` with one `<file path=…>` per surviving file | `<nearest-non-empty-ancestor>/.omp/AGENTS.md` (native, priority 100) | **Yes** | Only **one project file per directory depth** survives, and native shadows `.claude`/`.github`/`AGENTS.md` at that depth. `@path` imports expand inline, depth 5, cycles skipped. |
| 5 | **Named always-apply rule** | any `alwaysApply: true` rule file; full body auto-injected before the rulebook | `<cwd>/.omp/rules/<name>.md` (project) or `~/.omp/agent/rules/<name>.md` | **Yes** | Omitted from injection if its normalized content already appears in SYSTEM/APPEND/context files — silent dedup. Name collisions are first-wins across providers. |
| 6 | **Rulebook (lazy) rule** | name + `description` listed in `<domain-rules>`; body read on demand via `rule://<name>` | same paths as #5, with `description:` and **no** `alwaysApply` | **Yes** (the listing) | **Advisory only** — omp asks the model to read applicable rules; code never enforces glob applicability. Cheap, but skippable. |
| 7 | **TTSR — mid-stream veto** | rule with `condition` (regex) and/or `astCondition`; on match omp aborts the stream, optionally discards the partial output, injects `<system-interrupt reason="rule_violation">`, and retries | rule frontmatter `condition:` + `scope:` + `interruptMode:`; globals `ttsr.enabled` `ttsr.interruptMode` `ttsr.contextMode` `ttsr.repeatMode` `ttsr.repeatGap` | **Yes** — rules re-evaluate every stream (injected text does not persist) | **Highest-leverage, zero prompt cost.** Costs a turn restart when it fires. A `condition` that *looks like a glob* is silently rewritten to `tool:edit(...)`/`tool:write(...)` + `.*`. |
| 8 | **Hook veto of a tool call** | `tool_call` handler returns `{ block: true, reason }`; `reason` becomes the tool error text the model reads | `<cwd>/.omp/hooks/pre/*.ts` or `~/.omp/agent/extensions/*.ts`, `export default (pi) => pi.on("tool_call", …)` | **N/A** — runtime interception, unaffected by compaction | Fail-**closed**: a throwing handler also blocks. Extensions are **not sandboxed** and share the process. |
| 9 | **Hook rewrite of tool arguments** | non-blocking `tool_call` handler returns `{ input }`, replacing the raw execution arguments | same as #8 | **N/A** | Handlers don't see each other's revisions (last-wins); **ignored for `computer` calls**. Revised input *is* revalidated and seen by scheduling, persistence, and the approval gate. |
| 10 | **Hook rewrite of tool output** | `tool_result` handler returns `{ content, details }` | same as #8 | **N/A** | `isError` is typed but not propagated by the legacy `HookToolWrapper`; the original error is still rethrown on failure. |
| 11 | **Hook rewrite of the whole LLM message array** | `context` handler returns `{ messages }`, chained per handler, before **every** provider call | same as #8 | **N/A** — applies to whatever the context currently is | Nuclear option. You own correctness of tool-call/result pairing. |
| 12 | **Bash → dedicated-tool redirect** | matching command returns a Bash tool **error** naming the replacement tool | `bashInterceptor.enabled: true` + `bashInterceptor.patterns[]` (`pattern` JS regex, `tool`, `message`) in `config.yml` | **N/A** | The named tool **must exist in the session** or the interceptor does not block. Matches the whole command and each `&&`/`\|\|`/`;`/`\|`-split segment, with and without leading `NAME=value`. |
| 13 | **Shadow a built-in tool** | register a tool with a built-in's **name**; `ctx.invokeTool(params)` runs the native original | extension `pi.registerTool({ name: "grep", loadMode: "essential", … })` | **N/A** | Delegation is same-tool only (no escalation). `ctx.invokeTool` is `undefined` when nothing is shadowed. Breaks if omp renames the built-in. |
| 14 | **Shrink the top-level tool schema** | discoverable built-ins are presented as `xd://<name>` devices instead of top-level function declarations; explicitly requested tools stay top-level | `tools.xdev` (tools/checkpoint.md, tools/recall.md, tools/retain.md, tools/reflect.md, tools/rewind.md, tools/memory_edit.md) | **Yes** — schema-level, every request | The model must `read xd://<name>` then `write` JSON to it. Not itemized in settings.md's published catalog. |
| 15 | **Turn a built-in tool off** | per-tool boolean | `grep.enabled` `glob.enabled` `fetch.enabled` `browser.enabled` `astGrep.enabled` `astEdit.enabled` `web_search.enabled` `bash.enabled` `launch.enabled` `computer.enabled` `debug.enabled` `checkpoint.enabled` `lsp.enabled` `eval.py` `eval.js` | **Yes** | Disabling `grep` removes omp's fastest search; only do it once codegraph is proven reachable. `--tools` is an **allowlist that validates only built-in names** and rots silently. |
| 16 | **Force a tool's load mode** | `loadMode: "essential"` keeps a custom/MCP-bridged tool top-level instead of demoted to `xd://` | extension/custom-tool `ToolDefinition.loadMode` | **Yes** | Every `essential` tool you add is permanent schema weight. |
| 17 | **Drop a whole discovery source** | removes context files **and** its MCP servers, commands, skills, hooks, tools, prompts, settings | `disabledProviders: [claude, codex, gemini, opencode, github, agents-md, cursor]` | **Yes** | Arrays **replace** across layers — a project `disabledProviders` wipes the global list. Heavier than it looks. |
| 18 | **Trim skills** | discovery-level filters, applied in order: `disabledExtensions` → source toggle → `ignoredSkills` → `includeSkills` | `skills.enabled`, `skills.ignoredSkills[]`, `skills.includeSkills[]`, `skills.customDirectories[]`, `skills.enable{Claude,Codex,Pi,Agents}{User,Project}`, `disabledExtensions: ["skill:<name>"]` | **Yes** | `hide: true` in `SKILL.md` removes it from the prompt list but keeps it reachable via `skill://` and `/skill:` — that is the right knob for a large private library. |
| 19 | **Compaction threshold** | when omp compacts | `compaction.thresholdPercent` (`-1` = reserve-based), `compaction.thresholdTokens` (>0 wins), `compaction.keepRecentTokens` (20000), `compaction.reserveTokens`, `compaction.midTurnEnabled` | **Yes** | Lower threshold = more frequent compaction = more prompt-cache churn. Snapcompact needs a **vision-capable** model or it silently falls back to context-full. |
| 20 | **Model routing per role** | roles resolve model + thinking suffix; agents reference `@role` so routing changes without editing agent files | `modelRoles.{default,smol,slow,vision,plan,designer,commit,tiny,task,advisor,<custom>}`, `task.agentModelOverrides.<agent>`, `cycleOrder`, `enabledModels` (path-scoped) | **Yes** | `modelRoleStorage: project` is the **only** key `omp config set` will write into `<cwd>/.omp/config.yml`. |
| 21 | **MCP server registration** | project- or user-scope server map; `disabledServers`/`enabledServers` are user-scope cross-source overrides | `<cwd>/.omp/mcp.json`, `~/.omp/agent/mcp.json` | **Yes** | Duplicate names across sources **shadow**, never merge. So do differently-named definitions with equivalent transport+endpoint+auth+request-id. |
| 22 | **Drop project MCP wholesale** | removes every `level === "project"` entry before dedup, so a same-named user entry survives | `mcp.enableProjectConfig: false` | **Yes** | Kills legitimate per-repo servers too. |
| 23 | **Per-turn behavior words** | hidden user-attributed notice for that turn | `magicKeywords.enabled`, `.ultrathink`, `.orchestrate`, `.workflow` (all default `true`) | **No** — single-turn notice | `workflowz` only injects when **both** `eval` and `task` are active. Highlight gradient stays even when disabled. |
| 24 | **Second-opinion loop** | advisor model reviews each completed turn and can interrupt | `modelRoles.advisor` + `advisor.enabled`, `.subagents`, `.syncBacklog`, `.immuneTurns` (3), and `WATCHDOG.md` | **Yes** (config) | Doubles per-turn cost. `syncBacklog` can stall the primary up to 30 s. |
| 25 | **Bounded subagent fan-out** | worker policy | `task.maxConcurrency`, `task.maxRecursionDepth` (2), `task.maxRuntimeMs` (0), `task.agentIdleTtlMs` (420000), `task.disabledAgents`, `task.batch` (on) | **Yes** | Subagents force `tools.approvalMode: yolo` (no UI to confirm against) — never rely on approvals as a safety net inside a subagent. |
| 26 | **Byte-stable config writes** | preserve the Anthropic prompt-cache prefix | write `APPEND_SYSTEM.md` / `AGENTS.md` / `RULES.md` only when content differs | **Yes** | Any byte change to a prompt block invalidates the whole cached prefix for that session. |

---

## (a) Every documented way to inject instructions — and their precedence

**Precedence, highest first** (system-prompt-customization.md):

```
--system-prompt / --append-system-prompt  (CLI flags beat every discovered file)
  └─ <cwd>/.omp/  →  <cwd>/.claude/  →  <cwd>/.codex/  →  <cwd>/.gemini/       (project scope)
       └─ ~/.omp/agent/ → ~/.claude/ → ~/.codex/ → ~/.gemini/                  (user scope)
```
No ancestor walk for `SYSTEM.md` / `APPEND_SYSTEM.md` / `TITLE_SYSTEM.md`.

**Render order inside the prompt:**

| Without `SYSTEM.md` | With `SYSTEM.md` |
|---|---|
| default instruction template | custom text |
| context files, skills, always-apply rules, rulebook | **append text (immediately after custom text)** |
| project/environment footer | context files, skills, always-apply rules, rulebook |
| **append text (very last)** | project/environment footer |

SDK-generated append content (memory / auto-learn / MCP guidance) is combined **before** your append text.

**Survival matrix:**

| Surface | Rendered as | Survives compaction |
|---|---|---|
| `APPEND_SYSTEM.md` | prompt block | ✅ |
| `SYSTEM.md` | prompt block | ✅ |
| `AGENTS.md` context files | `<repo-rules>` prompt block | ✅ |
| `RULES.md` / any `alwaysApply` rule | `<generic-rules>` prompt block | ✅ |
| rulebook rules | `<domain-rules>` listing | ✅ (listing; body is a `rule://` read = ordinary history) |
| skills | name+description list (needs `read` tool available) | ✅ (listing; body is a `read skill://` = ordinary history, **but never pruned**) |
| memory Memory Guidance | prompt block | ✅ |
| magic-keyword notice | hidden custom user message | ❌ single turn |
| `/skill:<name>` injection | custom message | ❌ |
| TTSR `<system-interrupt>` | custom message, `customType: "ttsr-injection"` | ❌ text; ✅ rule keeps firing |
| `before_agent_start` message | custom message | ❌ |
| extension `pi.sendMessage(..., "nextTurn")` | injected on next prompt | ❌ |

**Dedup trap:** an always-apply rule whose normalized content already appears in
SYSTEM/custom/append text or a loaded context file is **silently omitted** from injection
(rulebook-matching-pipeline.md §7). Do not duplicate the same paragraph across `APPEND_SYSTEM.md`
and `RULES.md` expecting it twice — you get it once, and possibly not from the file you expected.

**Ordering trap:** context-file injection order is farthest ancestor → nearest project → user file.
Later is more prominent. The **user** file therefore sits last (most prominent) among context files.

---

## (b) Making codegraph / serena / codebase-memory / browser / skills actually get used

omp documents four escalating mechanisms. Use them together; each alone leaks.

### B1 — Make the alternative *cheaper to reach* than the built-in
Register the MCP servers so their tools are top-level, not buried:

```json
// ~/.omp/agent/mcp.json
{ "$schema": "https://raw.githubusercontent.com/can1357/oh-my-pi/main/packages/coding-agent/src/config/mcp-schema.json",
  "mcpServers": {
    "codegraph": { "type": "stdio", "command": "codegraph", "args": ["mcp"] },
    "serena":    { "type": "stdio", "command": "uvx", "args": ["--from","git+https://github.com/oraios/serena","serena","start-mcp-server"] }
  } }
```
MCP tools register as `mcp__<server>_<tool>` (lowercased, non-`[a-z_]` → `_`, collapsed
underscores, redundant `<server>_` prefix stripped once). Keep server names short and distinct
**after sanitization** — collisions are resolved deterministically by origin key and the loser is
dropped (mcp-server-tool-authoring.md).

The MCP tool's **`description` is the server's own `tools/list` description** — omp passes the
`inputSchema` through `normalizeSchemaForMCP()` and does not rewrite descriptions. So the highest-
leverage text you control is on **your** MCP server, not in omp.

⚠️ omp strips the harness-injected intent field **`i`** from outbound MCP args unless the server's
own `inputSchema.properties` declares `i`. If your server wants intent strings, declare `i`.

### B2 — Make the built-in *refuse* and name the alternative
This is the only documented mechanism that returns a **tool error naming a replacement tool**:

```yaml
# ~/.omp/agent/config.yml
bashInterceptor:
  enabled: true
  patterns:
    - pattern: '^\s*(rg|grep|ag|ack)\s+'
      tool: mcp__codegraph_search
      message: "Symbol/reference lookup goes through codegraph, not shell grep."
```
**Scope limit:** `bashInterceptor` only intercepts the **`bash` tool**. It cannot veto a direct
`grep` tool call. The replacement tool must be present in the session or nothing is blocked.

### B3 — Veto the built-in tool call itself (hook)
`bashInterceptor` cannot reach `grep`; a `tool_call` hook can. **Yes — a hook can veto a `grep` call
and tell the model to use codegraph instead.** The blocked call's `reason` is what the model reads
as the tool error:

```ts
// ~/.omp/agent/extensions/prefer-codegraph.ts
import type { ExtensionAPI } from "@oh-my-pi/pi-coding-agent";

export default function (pi: ExtensionAPI) {
  pi.on("tool_call", async (event) => {
    if (event.toolName !== "grep") return;
    const pattern = String(event.input.pattern ?? "");
    // only symbol-shaped queries; leave literal text search to grep
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(pattern)) return;
    return {
      block: true,
      reason:
        `Symbol lookup must use codegraph. Call mcp__codegraph_search { symbol: "${pattern}" }. ` +
        `Re-run grep only if codegraph returns nothing.`,
    };
  });
}
```
Contract (hooks.md, skills/authoring-hooks.md): **any** handler returning `{ block: true }` stops
execution; `reason` becomes the tool error text; a **throwing** handler also blocks (fail-closed);
first block short-circuits, last non-blocking return wins.

Softer variant — **redirect instead of veto** by rewriting arguments, e.g. narrow a repo-wide grep
to a scoped path (`{ input: { ...event.input, path: "src" } }`). Not applied to `computer` calls.

### B4 — Interrupt the model *mid-stream* before the call completes (TTSR)
Zero prompt-token cost, and it fires while the tool arguments are still streaming:

```md
---
name: codegraph-first
description: Symbol lookups go through codegraph.
condition: '(?i)\b(class|fn|func|def|struct|impl)\s+\w+'
scope: "tool:grep(*)"
interruptMode: tool-only
---
Do NOT grep for symbols. Call `mcp__codegraph_search` first; fall back to `grep` only when
codegraph returns no hit. Cite the codegraph result path.
```
Place at `<cwd>/.omp/rules/codegraph-first.md` or `~/.omp/agent/rules/`.
Scope tokens: `text`, `thinking`, `tool` / `toolcall`, `tool:<name>(<path-glob>)`. Default scope is
prose + **all tool arguments** but *not* thinking. `interruptMode` per rule overrides
`ttsr.interruptMode`; `ttsr.contextMode: "discard"` (default) drops the partial assistant output
before retrying.

### B5 — Standing instruction (the cheap always-on backstop)
`RULES.md` (sticky always-apply) or `APPEND_SYSTEM.md`. Keep it to a handful of lines — it costs
every turn. Combine with B3/B4 which cost nothing until violated.

### B6 — Skills: make them *listed* and *reachable*
- The skill list is included in the prompt **only when the `read` tool is available**
  (skills.md). If a session restricts tools and drops `read`, the entire skills list silently
  disappears. Never strip `read`.
- `description` is required by the native/omp-plugins/github providers and is the **only** text
  the model sees before opening a skill. Write it as a trigger condition
  ("Use when …"), not a topic label.
- `skill://` reads bypass the default 300-line limit (`ignoreResultLimits: true`) and are **never
  pruned by compaction** — a skill body, once read, is durable context. This makes
  "read the skill once, keep it" a real strategy.
- Layout is **non-recursive**: `<root>/<skill>/SKILL.md` only. `<root>/group/<skill>/SKILL.md` is
  invisible unless `skills.customDirectories` points at `<root>/group`.
- Native skills scan **every ancestor** `.omp/skills` cwd → repo root, plus
  `~/.omp/agent/skills` — and unlike other native capabilities they do **not** require a non-empty
  `.omp/`.
- `hide: true` keeps a skill loaded and reachable but out of the prompt list. Use it for a large
  private library plus a small always-listed index skill.

### B7 — Browser
`browser.enabled` must be on; with it on, omp **filters browser-automation MCP servers** out of
discovery (mcp-runtime-lifecycle.md) — so you cannot run Playwright-MCP and the built-in browser
side by side. Steer usage with a rulebook rule scoped to web work, and note the built-in
guidance already says "use `read` for static URLs; `browser` only when JS execution,
authentication, or interaction is required" (tools/browser.md). If the model is under-using it,
the fix is a TTSR rule on `tool:read(*)` for `http` URLs that need JS, not a prompt paragraph.

### B8 — Verify, don't assume
There is **no** documented "tool usage telemetry" surface. What you can observe:
`tools.intentTracing` (default on) records per-call intent strings, and extensions can subscribe to
`tool_execution_start` / `_end`. Build the harness's own usage audit on those events — omp will
not tell you which tools were ignored.

---

## (c) Which defaults to turn off

Ordered by prompt-weight/latency payoff per unit of risk.

| Turn off | Key | Why | Cost of doing it |
|---|---|---|---|
| Foreign discovery sources | `disabledProviders: [claude, codex, gemini, opencode, github, agents-md, cursor]` | Removes their context files, MCP servers, commands, skills, hooks, tools, prompts **and** settings in one key | Loses `CLAUDE.md` etc. Arrays **replace** across layers — restate the full list in every layer that sets it |
| Third-party skill sources | `skills.enableClaudeUser/Project`, `enableCodexUser`, `enableAgentsUser/Project` = `false` | Skill list is per-turn prompt weight | The `agents` provider has its **own** toggles; disabling Claude/Codex/Pi does not cover it |
| Unused skills | `skills.ignoredSkills` / `skills.includeSkills` (globs), or `hide: true` per skill | Same | `includeSkills` is an allowlist — a new skill is invisible until listed |
| Discoverable built-ins as top-level tools | `tools.xdev` | Moves discoverable built-ins behind `xd://<name>` — big schema reduction | Adds one `read xd://<name>` hop before first use |
| `ast_grep` | `astGrep.enabled` | **Already `false` by default** — do not enable unless used | — |
| `computer` | `computer.enabled` | **Already `false`** | — |
| `checkpoint`/`rewind` | `checkpoint.enabled` | **Already `false`**; enabling one auto-registers both | — |
| `debug` | `debug.enabled: false` | On by default, discoverable, exclusive concurrency | Loses DAP access |
| `web_search`, `fetch` | `web_search.enabled`, `fetch.enabled` | Drop when the harness supplies its own research path | Loses the whole provider chain |
| Ruby/Julia eval | already off (`eval.rb`, `eval.jl` default `false`) | — | — |
| Python or JS eval | `eval.py: false` / `eval.js: false` (or `PI_PY=0` / `PI_JS=0`) | Removes a heavy essential tool | `workflowz` magic keyword requires `eval` **and** `task`; `browser`'s `run` uses the same JS runtime |
| LSP diagnostics churn | keep `lsp.lazy: true` (default); `lsp.diagnosticsOnEdit` is already `false` | Latency on every write | Turning `lsp.enabled: false` loses definitions/references entirely |
| Magic keywords you don't want | `magicKeywords.orchestrate: false` etc. | Per-turn hidden notice tokens | TUI gradient remains |
| Advisor | `advisor.enabled` already `false` | Would double per-turn model cost | — |
| Auto-learn | `autolearn.enabled` already `false` | Enabling registers `learn` + `manage_skill` as **essential** tools (permanent schema weight) and adds a capture turn if `autoContinue` | — |
| Auto session titles | `PI_NO_TITLE=1` | Removes a tiny-model call on the first message | Sessions stay unnamed |
| Project MCP sources | `mcp.enableProjectConfig: false` | Drops every project-level MCP entry pre-dedup | Kills legitimate per-repo servers |
| Startup chrome | `startup.quiet: true` | Startup latency | — |
| Built-in TTSR rules | `ttsr.builtinRules: false`, or `ttsr.disabledRules: [name…]` | Drops omp's embedded `builtin-defaults` rules | You lose omp's own guardrails; user/project rules still load |
| Extension discovery | `--no-extensions` / SDK `disableExtensionDiscovery` | Explicit-only extension loading | **Not** a capability-isolation switch: skills, MCP, tools, prompts, rules keep their own toggles |

**Latency-specific:** MCP has a hard **250 ms fast-startup gate** — slow servers no longer block
startup and register later via `#onToolsChanged`, so MCP costs startup time only on a cold
`MCPToolCache`. There is **no autonomous MCP health poller**: reconnect is `transport.onClose`-driven
with `500/1000/2000/4000 ms` backoff, circuit-broken after >5 attempts in 30 s.

**Do not turn off** without a proven replacement: `read` (kills the skills list), `grep`
(codegraph must be verified reachable first), `task` (`workflowz`, delegation), `hub`
(subagent coordination — `runSubprocess` re-adds it to explicit tool lists anyway).

---

## (d) What an external tool may safely write

| Path | Does omp write it? | Safe for 8sync? |
|---|---|---|
| `~/.omp/agent/APPEND_SYSTEM.md` | **No** | ✅ **Own it.** Best injection point. |
| `~/.omp/agent/RULES.md`, `<proj>/.omp/RULES.md` | No | ✅ Own it (short). |
| `<proj>/.omp/AGENTS.md` | No | ✅ Own it. |
| `<proj>/.omp/rules/*.md`, `~/.omp/agent/rules/*.md` | No | ✅ Own it. Rule identity is the **name** — namespace your filenames. |
| `<proj>/.omp/skills/<name>/SKILL.md`, `~/.omp/agent/skills/…` | No (except `managed-skills/`) | ✅ Own it. Never write into `<agent-dir>/managed-skills/` — `manage_skill` owns that. |
| `<proj>/.omp/commands/*.md`, `~/.omp/agent/commands/*.md` | No | ✅ Own it. |
| `<proj>/.omp/agents/*.md`, `~/.omp/agent/agents/*.md` | No | ✅ Own it. |
| `<proj>/.omp/hooks/pre/*.ts`, `~/.omp/agent/extensions/*.ts` | No | ✅ Own it. |
| `<proj>/.omp/mcp.json`, `~/.omp/agent/mcp.json` | **Yes** — `/mcp add|enable|disable|reauth` write here, atomically (temp+rename), and inject `$schema` | ⚠️ **Shared.** Merge, never overwrite. Prefer flipping `enabled` / `disabledServers` / `enabledServers` over rewriting `mcpServers`. |
| `~/.omp/agent/config.yml` | **Yes** — `omp config set`, `omp config reset`, `/settings`, and ordinary runtime changes. Saves are debounced and **re-read the file under a lock**, so external edits during a live session are preserved | ⚠️ **Shared.** Read-modify-write; only add keys the user has not set. |
| `<proj>/.omp/config.yml` | **Almost never** — the only supported write is a `modelRoles` role assignment under `modelRoleStorage: project` | ✅ Effectively yours. This is the right home for per-repo `compaction.*`, `tools.*`, `bashInterceptor.*`, `skills.*`. |
| `~/.omp/agent/models.yml` | Generic loader may migrate a sibling `.json` → `.yml` once | ⚠️ Use a fenced sentinel block. |
| `~/.omp/plugins/omp-plugins.lock.json`, `installed_plugins.json`, `marketplaces.json` | **Yes**, and there is **no cross-process locking or merge** — concurrent writers overwrite each other | ❌ Do not write. Shell out to `omp plugin …`. |
| `~/.omp/agent/agent.db` | Yes (auth store) | ❌ Never. |
| `<proj>/.omp/plugin-overrides.json` | No — read-only from omp's perspective | ✅ Own it (disable plugins, override features/settings). |
| An overlay file passed via `--config` / `PI_CONFIG_FILES` | Never persisted by omp | ✅ **Zero-conflict.** Cleanest way to force process-local settings. |

**Byte-stability rules for a harness:**
1. **Write only on change.** Any byte diff in a prompt block invalidates the Anthropic prompt-cache
   prefix for that session. Hash-compare and skip identical writes.
2. **Use sentinel blocks** for shared files. `su-code` already does this
   (`# >>> 8sync:custom-models … # <<< 8sync:custom-models`) — apply the same discipline to
   `config.yml` and `mcp.json`.
3. **Never clobber an array.** Higher-precedence layers *replace* arrays wholesale, so a written
   `disabledProviders` / `enabledModels` / `cycleOrder` / `extensions` must be the **complete**
   desired set for that layer.
4. **Do not write a broken YAML mapping.** On writable startup omp moves an invalid persistent
   settings file to `.broken-<timestamp>-<pid>-<uuid>` and **exits**. A malformed `--config`
   overlay is a hard error and is *not* quarantined.
5. **Prefer a project layer over the global layer** — the global file is what `omp config set` and
   `/settings` fight you over; `<proj>/.omp/config.yml` is not.
6. **`omp config reset <key>` writes the schema default, it does not delete the key.** To stop
   overriding, remove the key by hand.
7. **`customType` is a global namespace** for session entries — use a reverse-domain id
   (`dev.8sync.harness.state`); core reserves values like `tool_execution_start` and `session_exit`.
8. **Non-empty-directory admission:** commands, rules, prompts, instructions, hooks, tools,
   extensions, extension modules and settings only load from a `.omp/` root that **exists and is
   non-empty**. Skills and MCP do not require that. Never leave an empty `.omp/`.

---

## (e) Update checking — what omp actually does (precedent for `8sync update`)

Honest summary: **omp documents no self-update or version-notification mechanism for the omp binary
itself.** Searching the full doc set surfaces only these three facts.

1. **Marketplace plugin update checks** (marketplace.md) — the only documented startup check:
   - `marketplace.autoUpdate`: `off` | **`notify` (default)** | `auto`
   - catalogs older than **24 hours** are refreshed best-effort before version comparison
   - comparison uses only catalog entries that declare `version`; semver must be strictly newer,
     non-semver counts as changed when unequal; per-plugin failures are skipped so an all-plugin
     upgrade can partially succeed
   - **"Despite its name, current `notify` mode writes update availability only to the debug log;
     it does not show a user-facing notification."**
2. **npm/git plugins have no update check at all** (plugin-manager-installer-plumbing.md):
   *"No separate npm-plugin 'check updates' or migration action exists."* Update = re-run
   `omp plugin install pkg@newVersion`.
3. **Install one-liner** (macos-signing-notarization.md): `curl https://omp.sh/install | sh`.
   Called out because `curl` sets no quarantine bit, so Gatekeeper is not consulted — the same
   reason a `curl | sh` installer is the frictionless path on macOS. Homebrew **formula** installs
   likewise skip Gatekeeper (casks do not).

omp also **migrated `lastChangelogVersion` out of `config.yml` into a marker file**
(settings.md legacy-migration table) and shows a startup changelog in the TUI
(tui-runtime-internals.md) — a "seen version" marker pattern, but no documented remote check.

**Takeaway for `8sync update`:** there is no omp behavior to mirror, so 8sync should not wait for
one. The documented-adjacent design worth copying is the marketplace shape:

- a **24-hour cached** remote version probe, refreshed best-effort and never blocking startup
- a tri-state setting `off` | `notify` | `auto`
- **semver-strict comparison**, non-semver treated as "changed when unequal"
- a persisted **marker file** (not a config key) for "version last shown", matching omp's own
  `lastChangelogVersion` migration
- **fail-open**: a failed probe must never delay or break a launch

And do the thing omp explicitly does *not*: make `notify` actually print to the user.

---

## Highest-impact levers for the 8sync harness

Ranked by (steering power × durability) ÷ prompt cost:

1. **TTSR rules scoped to tool arguments** (#7) — the only zero-prompt-cost, compaction-proof way
   to make omp stop ignoring codegraph/serena/cbm. `~/.omp/agent/rules/*.md` with
   `condition:` + `scope: "tool:grep(*)"` + `interruptMode: tool-only`.
2. **`tool_call` hook veto with a naming `reason`** (#8) — the only mechanism that can block a
   direct `grep`/`read` call and hand the model the exact replacement call to make. Deterministic
   where a prompt paragraph is probabilistic.
3. **`tools.xdev`** (#14) — the single largest documented reduction in top-level tool-schema weight
   without losing capability. su-code's `doctor` already checks for it on omp ≥ 17.
4. **`bashInterceptor.patterns`** (#12) — closes the shell-escape hole (`bash rg`) that #1/#2 leave
   open. Already deployed by `8sync harness`.
5. **Byte-stable `APPEND_SYSTEM.md`** (#1 + #26) — durable standing instructions at the lowest risk;
   keep it short and write only on change so the Anthropic cache prefix stays hot.
6. **`<proj>/.omp/config.yml` as the harness's settings home** (§d) — omp almost never writes it, so
   8sync gets a conflict-free layer for `compaction.*`, `bashInterceptor.*`, `skills.*`, `tools.*`.
7. **Skill `description` as a trigger sentence + `hide: true` for the long tail** (#18) — the list is
   the only thing the model sees; the bodies are free until read and never pruned once read.
8. **`disabledProviders` for foreign discovery sources** (#17) — one key removes a whole tool's
   config surface, which is the biggest single source of unwanted context and MCP entries.

## Where the docs contradict what `8sync harness` currently does

Verified against `crates/cli/src/verbs/harness/*` and `verbs/doctor.rs`.

1. **`8sync harness compaction` writes `compaction.thresholdPercent` into
   `~/.omp/agent/config.yml`, the file `omp config set` / `/settings` also own.**
   Contradiction is mild — `ensure_threshold_default()` correctly writes only when the key is
   absent, and settings.md confirms omp's saves are debounced and re-read the file under a lock, so
   external edits during a live session survive. Still, `<proj>/.omp/config.yml` is the
   documented-safe home for a per-repo threshold and would remove the shared-file risk entirely.
2. **`doctor.rs` warns "MCP tools HIDDEN behind `search_tool_bm25`" as the failure mode for
   serena/cbm never being called.** No omp doc mentions `search_tool_bm25`. The documented
   mechanism for demoting discoverable tools is `tools.xdev` → `xd://<name>`, and the documented
   remedies are `loadMode: "essential"` on the definition or an explicit tool request. If
   `search_tool_bm25` is a real omp ≥17 behavior it is **undocumented**, so the harness's check
   should not be the only guard — pair it with a `tool_call` hook (#B3) that does not depend on
   omp's internal tool-presentation strategy.
3. **`doctor.rs` treats a `--tools` allowlist as load-bearing and notes it "rots silently."**
   The docs agree and go further: *"CLI `--tools` currently validates only built-in tool names;
   custom tool inclusion is handled through discovery/registration paths and SDK options"*
   (custom-tools.md). So a `--tools` list can never admit an MCP or custom tool — it can only
   restrict built-ins. Any harness logic that expects `--tools` to *enable* `mcp__codegraph_*` is
   wrong by construction.
4. **`harness browser` exports `PUPPETEER_EXECUTABLE_PATH` to force a system Chromium.**
   Consistent with tools/browser.md (headless launch prefers detected system Chrome/Chromium, then
   `PUPPETEER_EXECUTABLE_PATH`, then downloads). But note the doc adds that **proxy env vars are
   baked into the shared broker-owned daemon at first launch** and only take effect after its next
   cold start — a `harness browser` change may not apply until the `omp.browser.headless` daemon is
   restarted. The current implementation only tells the user to open a new shell.
5. **The harness registers browser-related MCP servers alongside the built-in browser tool.**
   mcp-runtime-lifecycle.md: browser-automation MCP servers are **filtered out of discovery when
   `browser.enabled`**. Registering both is a silent no-op for the MCP side.
6. **`harness global` writes `APPEND_SYSTEM.md` byte-stably and calls that an Anthropic
   prompt-cache optimization** — this is exactly right and matches system-prompt-customization.md's
   render model. No contradiction; worth keeping as the template for every other file the harness
   writes.
7. **`8sync harness` mirrors skills into a project `su-code/skills/` tree.** Provider discovery is
   **non-recursive one level under `skills/`** (`<root>/<skill>/SKILL.md`); any nested grouping is
   invisible unless `skills.customDirectories` points at each nested parent. Worth asserting in
   `doctor`.
