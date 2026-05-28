# codegraph CLI cheatsheet

One-page reference. For day-to-day usage rules see `../SKILL.md`.

| Command | When to use | Example |
|---|---|---|
| `codegraph init [path]` | Bootstrap `.codegraph/` in a project. Non-interactive. | `codegraph init .` |
| `codegraph init -i [path]` | Init + run an initial index in one shot. | `codegraph init -i` |
| `codegraph uninit [path]` | Delete `.codegraph/` from a project. | `codegraph uninit .` |
| `codegraph index [path]` | Full (re)index of the project. | `codegraph index .` |
| `codegraph sync [path]` | Incremental update — only changed files since last index. | `codegraph sync` |
| `codegraph status [path]` | Show index stats + freshness. | `codegraph status` |
| `codegraph query <search>` | Semantic symbol/keyword search. **Primary tool — replaces grep/rg/fd.** | `codegraph query "handle_login"` |
| `codegraph context <task>` | Build a markdown context bundle for an open-ended task. | `codegraph context "auth flow"` |
| `codegraph files` | List indexed file structure. Cheaper than `find`. | `codegraph files` |
| `codegraph affected <files...>` | Find test files impacted by source changes. | `codegraph affected src/auth.rs` |
| `codegraph unlock [path]` | Remove a stale lock if indexing is blocked. | `codegraph unlock` |
| `codegraph serve` | Run as an MCP server for agents that speak MCP. | `codegraph serve` |
| `codegraph install` | Install codegraph MCP into Claude Code / Cursor / etc. | `codegraph install` |

## Flags worth remembering

- `-v, --verbose` (on `init` / `index` / `sync` / `status`): show worker lifecycle + memory.
- `-i, --index` (on `init`): chain an initial index after init.

## Output format

`query` and `context` emit `path:line` citations. Pipe directly into editor jump commands or echo back verbatim in your reply.
