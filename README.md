# su-code

> `8sync` — vibe coding harness for **CachyOS + Kitty + Helix**. Built in Rust. Tiny binary, low RAM, fast.

## What it is

A thin harness that wraps **forge** (AI engine) and orchestrates **kitty** + **helix** + **fish** + **WARP** + project context. Not another AI agent. It composes the tools you already use — and persists project knowledge so the AI gets smarter at *your* project over time.

## Install

```bash
git clone https://github.com/8-Sync-Dev/su-code.git
cd su-code
cargo install --path crates/cli --locked
# binary: ~/.cargo/bin/8sync
```

Then bootstrap your machine (idempotent — skips what's installed):

```bash
8sync setup
```

## Daily verbs (10)

| Verb       | What it does                                                   |
|------------|----------------------------------------------------------------|
| `setup`    | Install everything (system packages, WARP DoH, forge, configs) |
| `up`       | Update tools (only if newer version available)                 |
| `doctor`   | Health-check; report what's installed and what's missing       |
| `.` (`here`) | Open project session — kitty layout + forge resume           |
| `ai`       | AI prompt / resume forge                                       |
| `ship`     | `git add -A && commit && push && gh pr create`                 |
| `run`      | `dev / build / test / fmt / lint` per recipe                   |
| `bg`       | Wallpaper — `<keywords | path | url | 0..1 | + | - | off>`     |
| `look`     | Style preset — `neon / ice / mint / dark / dim`                |
| `end`      | Capture session knowledge to `agents/`, close panes              |

## Skill + context verbs

| Verb        | What it does                                                |
|-------------|-------------------------------------------------------------|
| `skill`     | `list / add / sync` skills                                  |
| `shot`      | Render web/file to PNG (for image-routing skill)            |
| `diff-img`  | Render git diff to PNG                                      |
| `pdf-img`   | Render PDF pages to PNG                                     |
| `mcp`       | Run MCP server for forge/cursor/opencode (phase 2)          |

Every verb has `-h` with examples.

## What `8sync setup` does

1. **pacman**: kitty, helix, git, gh, lazygit, node, pnpm, bun, docker, ripgrep, fd, fzf, eza, bat, jq, fastfetch, btop, zoxide, ufw, fish, poppler, imagemagick, jdk-openjdk, android-tools, postgresql, valkey, protobuf, zip/unzip.
2. **paru** (auto-builds from AUR if missing) + `cloudflare-warp-bin`.
3. **forge** via `curl forgecode.dev/cli | sh`.
4. **WARP** auto-on-boot: DoH + MASQUE + malware filter.
5. **UFW** + **Docker** enabled on boot.
6. **Configs** written: kitty.conf (opacity + bg image), helix config + `glass_black` theme, fish aliases, fcitx5 IME.
7. **Wallpaper**: downloads a default 4K image to `~/.local/share/8sync/wallpapers/`.
8. **Skills** copied to `~/.forge/skills/`: `karpathy-guidelines`, `image-routing`, `8sync-cli`.
9. **systemd-user** service for `8sync mcp` (auto-start on login).
10. **Docker group** for current user.

Skip flags: `--no-mobile --no-db --no-warp --minimal --dry-run`.
Re-run is safe — only newer versions get installed, configs are backed up before overwrite.

## Project session model

`8sync .` inside a project:

1. Detects project root (`.git` / `Cargo.toml` / `package.json` / …).
2. Auto-seeds `AGENTS.md` + `agents/PROJECT.md` if missing.
3. Splits kitty into 3 panes: `hx .` | `forge` | `fish`.
4. Forge auto-loads `agents/*.md` via AGENTS.md anchor → has full prior context.

`8sync end` captures structured knowledge back into `agents/*.md` for next time.

## License

MIT. See `LICENSE`.
