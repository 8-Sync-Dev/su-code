# Skills, slash commands, task agents

## 1. Skills (skills.md)

### Layout — non-recursive, one level under `skills/`
```
<skills-root>/<skill-name>/SKILL.md      ✅ discovered
<skills-root>/group/<skill>/SKILL.md     ❌ NOT discovered by provider loaders
```
`skills.customDirectories` uses the same non-recursive `*/SKILL.md` scan. For nested taxonomy,
point `customDirectories` at the nested parent.

### `SKILL.md` frontmatter
`name?` (defaults to directory name) · `description?` · `globs?: string[]` · `alwaysApply?: boolean` ·
`hide?: boolean` · `disableModelInvocation?` (kebab `disable-model-invocation`, = Agent Skills'
equivalent of `hide`). Unknown keys are preserved as metadata.

`description` is **required** by: native `.omp` provider, `omp-plugins` extension-package skills,
`github` (`.github/skills/`), and `skills.customDirectories` scans. The
claude/codex/agents/opencode/claude-plugins providers accept a skill without one.

### Discovery pipeline (`loadSkills()`, 3 passes)
1. capability providers (`loadCapability("skills")`) — the `omp-managed` provider is skipped here
2. `skills.customDirectories` (`requireDescription: true`, one level). A custom-dir skill
   **overrides** a same-named default-provider skill; duplicate custom-dir names are first-wins
3. managed auto-learn skills (`omp-managed`) resolved **dead last**, always deferring to a
   same-named authored skill

`skills.enabled: false` → discovery returns nothing.

Provider priority: `native` 100 · `omp-plugins` 90 · `claude` 80 · (`claude-plugins`, `agents`,
`codex`) 70 in registration order · `opencode` 55 · `github` 30 · `omp-managed` 5.
Dedup key = skill **name**, first wins. Additionally: identical files deduped by `realpath`
(symlink-safe); later name collisions emit warnings.

Native skill roots (config-usage.md): `<ancestor>/.omp/skills/*/SKILL.md` for **every ancestor**
cwd → repo-root/home, plus `~/.omp/agent/skills/*/SKILL.md`. Unlike other native capabilities,
skills do **not** require the `.omp/` root to be non-empty.

### Source toggles & filters, applied in this order
1. not disabled by `disabledExtensions` entry `skill:<name>`
2. source enabled: `skills.enableCodexUser` · `enableClaudeUser` · `enableClaudeProject` ·
   `enablePiUser` · `enablePiProject` (legacy names, they gate `provider === "native"`) ·
   `enableAgentsUser` · `enableAgentsProject`
3. not matched by `skills.ignoredSkills` (glob patterns)
4. matched by `skills.includeSkills` (glob allowlist; empty = include all)

The `agents` provider (`.agent[s]/skills`) has its **own** toggles — disabling Claude/Codex/Pi does
not turn it off. Providers with no dedicated toggle (`claude-plugins`, `opencode`, `github`) are
enabled if **any** named third-party toggle is enabled.

### Prompt exposure
If the `read` tool is available, the discovered skill list (name + description) is included,
**excluding `hide: true`**. Without `read`, the list is omitted entirely. `hide: true` does *not*
disable a skill — it stays reachable via `skill://<name>` and `/skill:<name>`.

Task subagents receive the session's discovered skills list; there is **no per-task skill pinning
override** (but agent frontmatter `autoloadSkills` injects named parent-session skills before the
child's first prompt — see §3).

### `skill://` URLs (internal-urls/skill-protocol.ts)
- `skill://<name>` → that skill's `SKILL.md`
- `skill://<name>/<relative-path>` → file inside the skill dir
Guards: exact name match; URL-decoded relative paths; absolute paths rejected; `..` rejected;
resolved path must stay inside `baseDir`; missing file → explicit `File not found`. No fallback
search. `.md` → `text/markdown`, else `text/plain`.
`read` sets `ignoreResultLimits: true` for `skill://`, so a skill body is paginated only by an
explicit selector, not the default 300-line limit (tools/read.md).

Compaction never prunes `skill` tool results or `read` results of `skill://` paths
(compaction.md).

### `/skill:<name>` commands
Registered per discovered skill when `skills.enableSkillCommands` is true. Recognizes both the
leading form and a whitespace-delimited `/skill:<name>` token embedded in prose (token is removed,
surrounding prose becomes the args). Reads `filePath` directly, strips frontmatter, wraps the body
with skill name + base directory + optional user args, injects as a custom message.
Delivery mode follows the **submission keybinding**: Enter → `steer` queue while streaming;
Ctrl+Enter (`app.message.followUp`) → `followUp` queue. No flag or frontmatter overrides this.

### Managed skills (tools/manage_skill.md)
`manage_skill` (`approval="write"`, `strict`, `loadMode="essential"`) is registered only when
`autolearn.enabled = true` (default `false`); independent of `memory.backend`. Subagents don't
auto-receive it. Writes `<agent-dir>/managed-skills/<name>/SKILL.md`. Names must match
`[a-z0-9][a-z0-9-]{0,63}`; final file capped at **64 000 UTF-8 bytes**; body must not contain
frontmatter (it is generated). `create` on a name owned by an active authored skill returns
`isError: true, details.shadowed = true` and writes nothing.

## 2. Slash commands (slash-command-internals.md)

Capability id `slash-commands`, key = command **name**, first-wins by provider priority:
`native` 100 · `omp-plugins` 90 · `claude` 80 · `claude-plugins` 70 · `agents` 70 · `codex` 70 ·
`opencode` 55. Equal priority keeps registration order (`claude-plugins` before `agents` before
`codex`).

Source paths:
- `native`: `<cwd>/.omp/commands/*.md` then `~/.omp/agent/commands/*.md` → **project beats user**
- `omp-plugins`: `commands/*.md` in extension-package roots (CLI → project settings → user settings
  → installed plugins). Marketplace roots excluded here.
- `claude`: `~/.claude/commands/**/*.md` then `<cwd>/.claude/commands/**/*.md` (recursive) →
  **user beats project**; subdir `foo/bar.md` also aliases as `foo:bar`. Gated by
  `commands.enableClaudeUser` / `commands.enableClaudeProject`.
- `codex`: `~/.codex/commands/*.md` then `<cwd>/.codex/commands/*.md` → **user beats project**
- `opencode`: `~/.config/opencode/commands/*.md` then `<cwd>/.opencode/commands/*.md` →
  user beats project. Gated by `commands.enableOpencodeUser` / `commands.enableOpencodeProject`.
- `claude-plugins`: `<pluginRoot>/commands/*.md`, names prefixed `<plugin>:<command>`. Roots merge
  `--plugin-dir` → project registry → user registry; the OMP registry
  (`~/.omp/plugins/installed_plugins.json`) is authoritative over Claude's for the same plugin id.
- `agents`: non-recursive `commands/*.md` under `.agent/` and `.agents/` cwd → repo root, then
  `~/.agent/commands` and `~/.agents/commands`. Nearest project first; `.agent` before `.agents`.

File scanning (`loadFilesFromDir`) is non-recursive `*.md`, native glob with `gitignore: true`,
`hidden: false`, files-only. **Hidden and gitignored command files never load.**

Description source: `frontmatter.description`, else the first non-empty body line (max 60 chars + `...`).
Frontmatter parse severity: discovered user/project commands = warning-level with fallback parsing;
`native`-marked capability items and bundled templates = fatal.

### Routing order (`AgentSession.prompt`, when `expandPromptTemplates !== false`)
Built-in registry runs **before** `prompt()` in TUI and ACP/RPC, and reserves its names + aliases.
Then:
1. extension-registered commands (`#tryExecuteExtensionCommand`) — execute immediately, even while streaming
2. TypeScript custom commands and MCP prompt commands — may return a `string` (replaces prompt text)
   or `void` (handled, no LLM turn)
3. file-based slash commands (`expandSlashCommand`)
4. prompt templates (`expandPromptTemplate`)
5. delivery: idle → sent; streaming → `steer` or `followUp` per `streamingBehavior` (omitting it throws)

Expansion placeholders: `$1`, `$2`, … · `$@[start]` / `$@[start:length]` (1-based) · `$ARGUMENTS` ·
`$@` · then `prompt.render` with `{ args, ARGUMENTS, arguments }`; an inline-argument fallback
appends args when the template used no placeholder. `parseCommandArgs` is quote-aware
(`'…'`, `"…"`) but has **no backslash escaping** and tolerates unmatched quotes.

**Unknown `/…` input is not rejected** — it falls through as literal prompt text.

Refresh points: interactive init · after `/move` changes cwd · editor component swap ·
`/reload-plugins`. **There is no file watcher on command directories.**

## 3. Task agents (task-agent-discovery.md, tools/task.md)

`AgentDefinition`: required `name`, `description`, `systemPrompt`; optional `tools`, `spawns`,
prioritized `model` list, `thinkingLevel`, `output`, `blocking`, `autoloadSkills`, `readSummarize`,
`prewalk`; `source: bundled|user|project`.

Frontmatter notes:
- missing `name` or `description` → invalid, file skipped (warning)
- `tools` CSV or array; when provided, `yield` is auto-added
- `spawns` accepts `*`, CSV, or array; missing `spawns` + `tools` including `task` ⇒ `spawns: *`
- `read-summarize: false` → subagent `read` returns verbatim content (`read.summarize.enabled:false`
  on its isolated settings). `scout` and `librarian` ship with it disabled.
- `model` accepts one selector, CSV, or array — tried in order after role aliases expand
- `autoloadSkills` names parent-session skills injected before the first child prompt; unknown names ignored
- `prewalk: true|"@smol"|"provider/model"` hands off to a cheaper model at the first edit/write

Discovery roots (first-wins by exact, case-sensitive `name`):
1. nearest project `.omp/agents/*.md`
2. user `~/.omp/agent/agents/*.md`
3. `<extension-root>/agents` for enabled OMP extension packages (CLI `--extension` → project
   `extensions:` → user `extensions:` → installed npm/link plugins)
4. Claude marketplace plugin `agents/` roots (only when `claude-plugins` provider enabled;
   project before user)
5. bundled: `scout`, `designer`, `reviewer`, `security-reviewer`, `librarian`, `task`, `sonic`

`.claude/agents`, `.codex/agents`, `.gemini/agents` are **intentionally skipped** —
`TASK_AGENT_CONFIG_SOURCE = ".omp"`. Within one directory, files load in lexicographic order.
Bundled parsing is `level: "fatal"` — malformed bundled frontmatter fails discovery entirely.

Role-backed routing example:

```md
---
name: reviewer
description: Review a change for correctness.
model: "@review"
---
```
```yaml
# ~/.omp/agent/config.yml
modelRoles:
  review: openai/gpt-5.4:high
```

Model precedence: `task.agentModelOverrides[name]` → agent frontmatter `model` list → parent active
model → configured/default fallback.
Output-schema precedence: task item `outputSchema` → agent frontmatter `output` → parent session schema.

Availability gates beyond discovery: `task.disabledAgents` · parent `spawns` policy
(`*`/`true`/absent = any; `""`/`false` = none; CSV = allowlist, omitted `agent` defaults to first) ·
`PI_BLOCKED_AGENT` env self-recursion guard · `task.maxRecursionDepth` (default `2`; negative
disables; at the cap the `task` tool is hidden and stripped from the child).

Wire shape is swapped by `task.batch` (default **on**): `{ context, tasks: item[] }` where `context`
is required shared background rendered into every child's `CONTEXT` section. Off → one spawn per call.
`effort` field exists only with `task.enableEffort = true` (default `false`), clamped by
`task.maxEffort` (default `max`). `isolated` exists only when `task.isolation.mode !== none` **and**
plan mode is off.

Subagent settings snapshot: parent settings inherited (`async.enabled`, `bash.autoBackground.enabled`
inherited, **not** force-disabled); tiers re-resolved via `tier.subagent`;
**`tools.approvalMode` is forced to `yolo`** (headless children have no UI). Child tool list:
explicit `agent.tools` if given; `task` auto-added when `spawns` set and depth allows; `hub` always
ensured in explicit lists; `exec` expands to `eval` + `bash`; parent-owned `todo` stripped (unless
prewalk-armed).

Limits: `task.maxConcurrency` (live-resized semaphore) · `task.agentIdleTtlMs` default `420000`
(≤0 disables parking) · `task.maxRuntimeMs` default `0` (off) · output truncation
`PI_TASK_MAX_OUTPUT_BYTES` 500 000 / `PI_TASK_MAX_OUTPUT_LINES` 5 000 · inline summary preview
threshold 5 000 chars, full artifact at `agent://<id>`.

Plan mode rewrites an `effectiveAgent`: prepends the plan-mode prompt, restricts tools to
`read`/`grep`/`glob`/`web_search` (+`ast_grep` if declared), clears `spawns` and `prewalk`, and
rejects per-spawn isolation/apply/merge.

## 4. Vibe mode (vibe-mode.md)

`/vibe` turns the top-level session into a **director**: active tools reduce to `read`, optional
parent-owned `todo`, and `vibe_spawn` / `vibe_send` / `vibe_wait` / `vibe_kill` / `vibe_list`.
Tiers: `fast` → bundled `sonic` (`@smol`), `good` → bundled `task` (`@task`) — always the bundled
definition, never a same-named custom agent. Routing still goes through
`task.agentModelOverrides.sonic` / `.task`, then `modelRoles`.
Mutually exclusive with active **and paused** plan/goal modes. Fork/move/handoff are rejected while on.
Exiting kills every worker in the scope.
