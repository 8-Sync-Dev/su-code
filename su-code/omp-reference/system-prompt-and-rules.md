# System prompt, context files, rules

## 1. Prompt assembly inputs & precedence (system-prompt-customization.md)

| Input | Source | Effect |
| --- | --- | --- |
| `--system-prompt <text-or-file>` | CLI | Switches to bundled `custom-system-prompt.md` template. Highest precedence. |
| `SYSTEM.md` | discovered file | Same template switch; used only when the flag is absent. |
| `--append-system-prompt <text-or-file>` | CLI | Appends text to rendered prompt. Highest append precedence. |
| `APPEND_SYSTEM.md` | discovered file | Same as append flag; used when the flag is absent. |
| `TITLE_SYSTEM.md` | discovered file | Overrides **only** the auto session-title system prompt. |
| SDK `CreateAgentSessionOptions.systemPrompt` | SDK only | Replaces every rendered provider-facing block. CLI flags/files never set this. |

Discovery for all three `*.md` files: **project first, then user**; within each scope the config
bases are ordered `.omp`, `.claude`, `.codex`, `.gemini`:

1. `<cwd>/.omp/<file>`, `<cwd>/.claude/<file>`, `<cwd>/.codex/<file>`, `<cwd>/.gemini/<file>`
2. `~/.omp/agent/<file>`, `~/.claude/<file>`, `~/.codex/<file>`, `~/.gemini/<file>`

**No ancestor walk.** Launching in `<repo>/packages/api` does not find `<repo>/.omp/SYSTEM.md`.
A flag beats every discovered file; project beats user; first config base wins within a scope.

Text-or-file resolution: single-line values are first tried as a file path; ENOENT/too-long →
used literally. A value containing a newline is always literal. **Plain text contract** — contents
are inserted into bundled Handlebars templates but are *not* recursively compiled; `{{cwd}}` etc.
reach the model verbatim.

### What `SYSTEM.md` keeps vs drops
Keeps: custom text, append text, discovered context files, discovered skills, always-apply rules +
rulebook listing, secret-redaction guidance, and the separate project/environment footer
(workstation data, `<dir-context>` pointers, date/cwd, completion requirements).
Drops: the default template's role/personality text, tool inventory + tool policy, internal-URL
catalog, exploration/delegation/workflow rules, `xd://` guidance. Selective inheritance is **not**
supported.

### Append placement
- Without `SYSTEM.md`: append renders at the **end of `project-prompt.md`** (after everything).
- With `SYSTEM.md`: append renders **immediately after the custom text**; context/skills/rules
  follow, then the project footer.
- SDK-generated append content (memory/auto-learn/MCP guidance) is combined **before** user append text.

### Title normalization (still enforced with a custom `TITLE_SYSTEM.md`)
Only the first trimmed line; strips quotes, `<title>…</title>`, terminal punctuation; `none` /
`<title/>` = no title; >80 chars or >12 words is **rejected, not truncated**.

## 2. Context files (context-files.md)

Native files:

| File | Scope | Behavior |
| --- | --- | --- |
| `~/.omp/agent/AGENTS.md` | user | user context for every session |
| `<nearest-non-empty-ancestor>/.omp/AGENTS.md` | project | only from the **nearest non-empty `.omp/`** walking cwd → repo root; a missing file does *not* continue upward |
| `~/.omp/agent/RULES.md` | user | loaded as an **always-apply rule**, not a context file |
| `<nearest-non-empty-ancestor>/.omp/RULES.md` | project | same nearest-dir rule |

Empty `.omp/` dirs are skipped in the walk; empty `AGENTS.md`/`RULES.md` contribute nothing.

Other conventions (provider id → path → scope):
`native` `.omp/AGENTS.md` (user+project) · `claude` `.claude/CLAUDE.md` (user+project, **cwd only**) ·
`codex` `~/.codex/AGENTS.md` (user only) · `gemini` `.gemini/GEMINI.md` (user+project, cwd only) ·
`opencode` `~/.config/opencode/AGENTS.md` (user only) · `github` `.github/copilot-instructions.md`
(cwd only) + `~/.copilot/copilot-instructions.md` (`COPILOT_HOME`, `COPILOT_CUSTOM_INSTRUCTIONS_DIRS`) ·
`agents` `.agent/AGENTS.md` + `.agents/AGENTS.md` (walks up to repo root) ·
`agents-md` standalone `AGENTS.md` (walks up; dirs starting with `.` ignored) ·
`github` `.github/instructions/**/*.instructions.md` → **rules**, not context.

**Provider priority:** `native` 100 > `claude` 80 > `agents`/`codex` 70 > `gemini` 60 >
`opencode` 55 > `github` 30 > `agents-md` 10.

Dedup: exactly **one user context file** survives (native wins). **One project file per directory
depth** (cwd = depth 0; a `.claude/`/`.github/` subdir counts as its parent's depth). Across depths
multiple files survive. Byte-identical files collapse after ordering, closest-to-cwd surviving.
Injection order = **farthest ancestor first → nearest project → user file last** (later = more prominent).

Injection shape (default template):

```xml
<repo-rules>
You MUST follow the context files below for all tasks:
<file path="/abs/path/AGENTS.md">…expanded markdown…</file>
</repo-rules>
```

With `SYSTEM.md` active the same files land in the custom template's `<project>`/`<instructions>`
section. Deeper `AGENTS.md` files **below** cwd are surfaced only as pointers in `<dir-context>`.

### `@` imports (inside any context file)
`@path` expands inline before injection. Relative paths resolve **from the importing file's dir**.
`~/` = home. Tokens inside fenced blocks / inline code are left alone. `git@github.com:…` and
`user@example.com` are never imports (`@` must start a line or follow space/tab). Trailing
`. , ; : ! ? ) ] } " '` is trimmed off the path. **Recursion depth 5**, cycles skipped, missing
target leaves the literal `@token`.

## 3. Rules: shape, buckets, precedence (rulebook-matching-pipeline.md)

Canonical `Rule`:

```ts
interface Rule { name; path; content; globs?: string[]; alwaysApply?: boolean;
  description?: string; condition?: string[]; astCondition?: string[]; scope?: string[];
  interruptMode?: "never"|"prose-only"|"tool-only"|"always"; _source }
```

**Identity is `rule.name` only.** Dedup/precedence are name-based; two different files with the
same name are the same logical rule.

Rule providers (priority): `native` 100 · `omp-plugins` 90 · `agents` 70 · `cursor` 50 ·
`windsurf` 50 · `cline` 40 · `github` 30 · `builtin-defaults` 1. First-wins by name; shadowed items
stay in `all` marked `_shadowed`.

Native rule sources: `<cwd>/.omp/rules/*.{md,mdc}` (only when cwd `.omp/` is non-empty),
`~/.omp/agent/rules/*.{md,mdc}`, user `RULES.md`, project nearest-`.omp` `RULES.md`. Both sticky
files are synthesized as rule name **`RULES`** and forced `alwaysApply: true`; native append order is
project rules → user rules → user `RULES.md` → project `RULES.md`, so **a regular `rules/RULES.md`
shadows both sticky files**, and normally user sticky shadows project sticky.

### Bucketing (`bucketRules`, runs in `createAgentSession`)
1. Drop names in `ttsr.disabledRules`.
2. Drop `builtin-defaults` rules when `ttsr.builtinRules === false`.
3. Rules with non-empty `condition` or `astCondition` that `TtsrManager.addRule()` accepts → **TTSR-only**.
4. Remaining `alwaysApply === true` → **always-apply** (full content injected into system prompt).
5. Remaining rules **with a `description`** → **rulebook** (name + description listed; body read on demand).

A rule with both `alwaysApply` and `description` goes to always-apply only. A rule with no
description, no `alwaysApply`, and no accepted TTSR condition is **not addressable at all**.

Rendering: always-apply bodies render inside `<generic-rules>` (default template) before the
rulebook; rulebook entries render in `<domain-rules>` as `- <name> (<globs>): <description>`.
Custom template uses `<rules>` / `<rule name="…"><glob>…</glob>` with an explicit
"You MUST read `rule://<name>`" instruction. **This is advisory** — code does not enforce glob
applicability.

**Dedup against prompt sources:** an always-apply rule whose normalized content already appears in
the system/custom/append prompt or a loaded context file is **omitted** from auto-injection.

`rule://<name>` resolves against rulebook ∪ always-apply ∪ registered TTSR rules (exact name match;
raw body, frontmatter stripped, `text/markdown`).

### Frontmatter parsing (utils/frontmatter.ts)
Parsed only when content starts with `---` and has a closing `\n---`. On YAML failure: warn, then
fall back to line parsing `^([\w-]+):\s*(.*)$` with per-value YAML reparse. Multiline arrays /
nested objects are **not** reconstructed by the fallback. Hyphenated keys normalize to camelCase
(`thinking-level` → `thinkingLevel`); `ttsr_trigger` works in fallback.

### Caveat that bites
A `condition` value that *looks like a file glob* is converted into
`tool:edit(<glob>)` + `tool:write(<glob>)` scope entries plus catch-all condition `.*`.
`astCondition` never does this.

## 4. TTSR — Time Traveling Stream Rules (ttsr-injection-lifecycle.md)

The only documented mechanism that **interrupts the model mid-stream** and retries the turn.

Manager defaults when unset:

| Setting | Default |
| --- | --- |
| `ttsr.enabled` | `true` |
| `ttsr.contextMode` | `"discard"` |
| `ttsr.interruptMode` | `"always"` |
| `ttsr.repeatMode` | `"once"` |
| `ttsr.repeatGap` | `10` completed turns |
| `ttsr.builtinRules` | `true` |
| `ttsr.disabledRules` | `[]` |

`scope` tokens: `text`, `thinking`, `tool` (= `toolcall`), `tool:<name>(<path-glob>)`.
**Default scope = assistant prose + all tool arguments, not thinking.** Accepts a comma-separated
YAML string, a YAML sequence, or block sequence. A leading `(?i)`/`(?m)`/`(?s)` in `condition`
translates to JS RegExp flags. `globs` act as a global path gate: the match context must include a
matching file path.

Flow on match:
1. matched rules dedup into pending injections; abort-pending set; resume gate created
2. `agent.abort()` immediately (tool matches scope the reason to that tool-call id)
3. `ttsr_triggered` emitted fire-and-forget
4. retry scheduled +50 ms with prompt-generation + retry token
5. on retry: if `contextMode === "discard"`, the partial assistant output is dropped
   (`agent.replaceMessages(...slice(0, targetAssistantIndex))`); injection built from
   `ttsr-interrupt.md`; a hidden custom message + persisted `custom_message` entry with
   `customType: "ttsr-injection"` and `details.rules`; then `agent.continue()`

Injected payload:

```xml
<system-interrupt reason="rule_violation" rule="{{name}}" path="{{path}}">
{{content}}
</system-interrupt>
```

Per-rule `interruptMode` overrides the global one: `always` | `prose-only` | `tool-only` | `never`.

`astCondition` only evaluates on tool-argument streams that expose `matcherDigest` /
`matcherEntries` (built-in edit/write do) and only when a candidate path yields a file extension.
The snapshot is the **source-bearing payload**, not the whole prospective file.

Registration is skipped when: TTSR disabled · no compiling condition and no AST condition ·
duplicate `rule.name` in this manager · parsed scope excludes every monitored stream.

## 5. Magic keywords (magic-keywords.md)

Standalone lowercase prose words that inject hidden, user-attributed instructions for **that turn only**.

| Keyword | Effect |
| --- | --- |
| `ultrathink` | careful multi-step reasoning notice; with auto-thinking active also selects the model's highest supported effort for the turn |
| `orchestrate` | multi-agent orchestration contract (scope, delegate in parallel, verify each phase, continue to completion) |
| `workflowz` | deterministic multi-subagent contract around the `eval` kernel's `agent()`/`parallel()`/`pipeline()`/`completion()`. Injected **only when both `eval` and `task` are active** |

Matching: exact lowercase; standalone prose only (punctuation/quotes may touch it, but
`orchestrated`, `orchestrate.ts`, `foo::orchestrate`, `orchestrate()` do **not** match); fenced
code, inline code, HTML/XML comments/tags are ignored.

Config keys (all default `true`):
`magicKeywords.enabled` (global gate) · `magicKeywords.ultrathink` · `magicKeywords.orchestrate` ·
`magicKeywords.workflow` (note: the setting is `workflow`, the keyword is `workflowz`).
Disabling does **not** remove the TUI gradient highlight.

## 6. Where the "always-visible" budget actually goes

Injected into the system prompt every turn: context files (`<repo-rules>`), always-apply rule bodies
(`<generic-rules>`), rulebook name+description lines (`<domain-rules>`), skill name+description list,
append text, project/environment footer, and the active model name when
`includeModelInPrompt: true` (default).

Cost hierarchy, cheapest to most expensive per turn:
`rulebook entry (name+desc)` < `skill entry (name+desc)` < `TTSR rule (0 tokens until triggered)` <
`always-apply rule body` ≈ `context file body`.

**TTSR rules cost zero prompt tokens until they fire** — this is the cheapest steering surface omp
offers.
