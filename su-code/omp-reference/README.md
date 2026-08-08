# omp reference (distilled)

Compressed notes on how **omp** (`oh-my-pi` coding agent) discovers config, assembles its system
prompt, loads tools/MCP/skills, and can be steered from the outside. Written for `8sync harness`:
everything here is a knob 8sync may need to write, read, or avoid.

Source of truth = omp's bundled docs, readable inside omp via the `omp://` internal URI (125
files). Every claim cites its doc, e.g. `(settings.md)`. Nothing invented; where omp's docs are
silent, the note says so.

## Files

| File | Covers |
| --- | --- |
| [`LEVERS.md`](./LEVERS.md) | **THE deliverable.** Every documented way to force instructions, force tool choice, and trim omp defaults. Read this first. |
| [`system-prompt-and-rules.md`](./system-prompt-and-rules.md) | `SYSTEM.md` / `APPEND_SYSTEM.md` / `TITLE_SYSTEM.md`, context files, `@` imports, rulebook + always-apply + TTSR buckets, magic keywords |
| [`skills-and-commands.md`](./skills-and-commands.md) | `SKILL.md` layout, discovery precedence, `skill://`, `/skill:<name>`, slash-command discovery + expansion, task agents |
| [`mcp-and-tools.md`](./mcp-and-tools.md) | `mcp.json` shape, transports, precedence, `mcp__*` naming, built-in tool toggles, `loadMode`, `xd://` devices |
| [`hooks-extensions.md`](./hooks-extensions.md) | `ExtensionAPI` / `HookAPI`, `tool_call` blocking, `tool_result` rewriting, `context` rewriting, discovery paths, plugins/marketplace |
| [`settings-and-defaults.md`](./settings-and-defaults.md) | Settings layers + precedence + merge rules, full key catalog of what matters, env vars, profiles/XDG |
| [`session-memory-compaction.md`](./session-memory-compaction.md) | Compaction strategies + thresholds, what survives a compact, memory backends, session entry types |

## Reading rule

- Paths written `~/.omp/agent/...` mean **the active profile's agent dir**:
  `~/.omp/profiles/<name>/agent/...` under `omp --profile <name>`; `PI_CODING_AGENT_DIR`
  relocates the default profile only (`config-usage.md`).
- "project" means **the cwd omp was launched from**. Most project discovery does **not** walk
  ancestors — the exceptions are called out explicitly (`context-files.md`, `settings.md`).

## Re-generating

Inside an omp session in this repo:

```
read omp://                       # index of all 125 docs
read omp://settings.md            # any single doc
grep -n 'pattern' omp://          # omp:// expands to every embedded doc as a search root
```

Load-bearing docs behind this reference (re-read these when omp ships a new minor):

```
system-prompt-customization context-files rulebook-matching-pipeline ttsr-injection-lifecycle
magic-keywords skills skills/authoring-{extensions,hooks,marketplaces} slash-command-internals
custom-tools marketplace plugin-manager-installer-plumbing mcp-{config,runtime-lifecycle,
server-tool-authoring} hooks extensions extension-loading settings config-usage
environment-variables memory compaction session task-agent-discovery vibe-mode models providers
tools/{task,manage_skill,browser,grep,read,lsp,bash,hub,todo,learn,retain,recall,memory_edit}
```

## Budget

This directory stays **under 120 KB** (repo has a 5 MiB asset budget). Distill; never paste a
whole omp doc. Check with `du -sb reference/omp`.
