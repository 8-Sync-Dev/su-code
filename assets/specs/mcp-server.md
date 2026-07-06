# MCP `server.json` — the standard spec (bundled, machine-local ground truth)

> Deployed by `8sync harness global` to `~/.omp/specs/mcp-server.md`. This is the
> **authoritative, on-disk copy** of the Model Context Protocol server metadata
> standard (registry schema `2025-12-11`). When you write, edit, or reason about
> an MCP server config (`mcp.json`) or a registry `server.json`, follow THIS —
> do not invent fields or guess shapes. Canonical source:
> `https://github.com/modelcontextprotocol/registry` ·
> schema: `https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json`

## Two shapes — don't confuse them

1. **`server.json`** — registry *metadata* about a server (what a registry/API
   returns). Rich, descriptive: packages, remotes, env-var *descriptors*.
2. **`mcp.json`** — a *client* config (`~/.omp/agent/mcp.json`, Claude Desktop,
   etc.) that actually launches servers. Compact, executable.

Installing = **projecting** a `server.json` entry into an `mcp.json` server object.

## `server.json` (registry metadata)

Top-level: `$schema`, `name` (reverse-DNS id, e.g. `io.github.user/srv`),
`title` (display), `description`, `version`, `repository`, `websiteUrl`,
`icons`, `packages[]`, `remotes[]`, `_meta`.

### `packages[]` — a locally-launched server
| field | meaning |
|---|---|
| `registryType` | how to fetch: `npm` · `pypi` · `oci` · `nuget` · `mcpb` |
| `identifier` | package name (e.g. `@scope/pkg`, `pypi-pkg`, `org/image`) |
| `version` | package version — **pin it** |
| `runtimeHint` | explicit runtime: `npx` · `uvx` · `docker` · `dnx` (present when `runtimeArguments` are) |
| `transport` | `{ type }`: `stdio` (default) · `streamable-http` · `sse` (+ `url`, `headers[]`) |
| `runtimeArguments[]` | args for the runtime (docker/npx flags) — `Argument` |
| `packageArguments[]` | args for the server binary — `Argument` |
| `environmentVariables[]` | `KeyValueInput[]` — env-var *descriptors* |
| `registryBaseUrl` | non-default registry base |

### `remotes[]` — a hosted server (RemoteTransport)
`{ type: "streamable-http" | "sse", url, headers[] }` where `headers[]` is
`KeyValueInput[]`.

### `Argument` (named | positional)
- **named**: `{ type:"named", name:"--flag", value?|default?, isRepeated? }`
- **positional**: `{ type:"positional", value?|default?|valueHint?, isRepeated? }`
- both extend `Input`: `value`, `default`, `isRequired`, `isSecret`, `format`
  (`string`|`number`|`boolean`|`filepath`), `choices`, `placeholder`.

### `KeyValueInput` (env var / header)
`Input` + `{ name }`. i.e. `{ name, value?, default?, isRequired?, isSecret? }`.

## Projection → `mcp.json` (the rules that MUST hold)

**Runtime** — derive from `registryType` (or `runtimeHint` when set):
| registryType | command | leading args | package token |
|---|---|---|---|
| `npm` | `npx` | `-y` | `identifier@version` |
| `pypi` | `uvx` | — | `identifier@version` |
| `oci` | `docker` | `run -i --rm` + `-e NAME`… | `identifier:version` (image tag) |
| `nuget` | `dnx` | — | `identifier@version` |
| `mcpb` | (bundle) | — | not directly runnable |

Full arg order: `command` + leading + `runtimeArguments` + (`-e NAME`… for docker)
+ package-token + `packageArguments`.

**stdio server object:**
```json
{ "type": "stdio", "command": "npx", "args": ["-y", "pkg@1.2.3"], "env": { "API_KEY": "" } }
```
**remote server object:**
```json
{ "type": "http", "url": "https://…/mcp", "headers": { "Authorization": "" } }
```
(`streamable-http` → client key `"http"`; `sse` → `"sse"`.)

### Invariants — never violate
- **`env` and `headers` are MAPS `{NAME: value}` — NEVER arrays of descriptors.**
  Project each `KeyValueInput` to `name → (value ?? default ?? "")`. An empty
  value = a placeholder the user fills; keep `isRequired`/`isSecret` only as UI
  hints, never inside the map.
- **Pin `version`** (`@version`, or `:version` for docker images) for reproducibility.
- **Runtime from `registryType`** — do not default everything to `npx`.
- **Honor `transport.type`** — a package with `streamable-http`/`sse` transport is
  a remote (url + headers), not stdio.
- **Only spec fields.** No invented keys. Unknown → omit, don't guess.
