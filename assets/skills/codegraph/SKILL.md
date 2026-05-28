---
name: codegraph
description: Use this skill on EVERY question that touches code in a project where `.codegraph/` exists. Triggers — "where is X defined", "what calls Y", "find references to Z", "explore <subsystem>", "understand the dependency graph", "outline this file", any keyword/symbol search. Saves ~35% tokens and ~70% tool calls vs raw grep/Read/Glob. The AI MUST prefer `codegraph` commands over `rg`/`fd`/`grep`/`Read`-whole-file whenever a semantic answer is wanted.
---

# codegraph — semantic code intelligence (use this BEFORE grep)

You are running inside an 8sync project that already has `codegraph` installed and a `.codegraph/` index. **Use it.** Dumping whole files into context is wasteful when the index can answer in one call.

## 1. When to use codegraph (always, when applicable)

| User intent | First tool |
|---|---|
| "Where is `<symbol>` defined?" | `codegraph query "<symbol>"` |
| "What calls `<function>`?" | `codegraph query "<function>"` then narrow with `context` |
| "Explore the auth subsystem" | `codegraph context "auth flow"` |
| "Outline this file before editing" | `codegraph query "<filename basename>"` |
| Any keyword / symbol / fuzzy search | `codegraph query "<term>"` |
| After bulk edits, before next question | `codegraph sync` |

If `.codegraph/` does NOT exist in cwd → `codegraph init .` then `codegraph index .`. `8sync skill sync` already does this for you when run inside a project; you only need to re-init if the directory has been wiped.

## 2. Canonical commands (memorise these four)

```bash
codegraph index .              # one-time per session — build/refresh semantic index
codegraph query "<term>"       # PRIMARY search — replaces grep / rg / fd / Glob
codegraph context "<task>"     # build a markdown context bundle for a task description
codegraph sync                 # incremental update after you (or the user) edited files
```

`query` returns symbol locations with `path:line` citations — copy them verbatim. `context` synthesises a multi-symbol bundle in markdown; prefer it for fan-out exploration ("how does feature X work end-to-end").

## 3. Secondary commands (one-liner each)

- `codegraph files` — project file structure from the index. Cheaper than `find` / `ls -R`.
- `codegraph affected <file>` — which test files cover a source file. Use before refactoring.
- `codegraph status` — index freshness + stats. Run when results look stale.
- `codegraph unlock` — remove a stale lock if indexing is blocked.
- `codegraph init -i` — init + initial index in one shot (8sync already does the init half).
- `codegraph serve` — MCP server mode for agents that speak MCP (rarely needed from inside 8sync).

## 4. Citation convention

`codegraph` already prints `path/to/file.rs:42` style locations. Echo them verbatim. **Never** translate to "in file.rs around line 42" — keep the machine-grep-able form so future tools can jump straight to the cursor.

## 5. When codegraph isn't enough

Use codegraph to **locate**, then `Read` (or 8sync's `read`) to fetch the narrow slice you actually need. The pattern is:

1. `codegraph query "thing"` → see 3 hits, each with `path:line`.
2. `Read <best hit's file>:<line-10>-<line+30>` → exact context, no waste.

Never `Read` an entire file > 200 lines without checking codegraph first.

## 6. After editing

When you change source files in a session, the index drifts. Before the next semantic query, run:

```bash
codegraph sync       # fast incremental
```

For batch refactors (many files): `codegraph index .` to rebuild from scratch.

---

For the full upstream CLI surface and installer details see `references/upstream-readme.md`. For a one-page command cheatsheet see `references/cli-cheatsheet.md`. **Do NOT read those during normal coding** — only when the user asks about install / uninstall / agent integration.
