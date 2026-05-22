# Plan — Slim down `8sync` for HyDE-managed CachyOS (v2)

- **Branch:** `chore/slim-down-for-hyde`
- **Date:** 2026-05-18
- **Author / Driver:** alexdev
- **Status:** Draft v2 — supersedes v1 after deeper HyDE audit
- **Diff vs v1:** v1 assumed HyDE only provided Hyprland + wallbash. v2 discovers HyDE also ships `hydectl`, `hyde-shell`, and ~70 scripts that directly cover several `8sync` verbs. The recommended cut is therefore deeper and the **remaining surface delegates to HyDE** instead of competing with it.

---

## 1. Goal

Reduce `8sync` to **only what HyDE cannot already do**:

> `gh` + `forge` + an `agents/` memory harness + a few terminal verbs (`.`, `ai`, `end`, `ship`, `note`, `find`, `run`).

Everything HyDE already provides (wallpaper, theme, kitty, fzf wrappers, image tooling, fastfetch, starship, zsh, fish) must be **delegated**, not reimplemented.

---

## 2. HyDE audit (live host, 2026-05-18)

### 2.1 HyDE control surface (provided out of the box)

| Tool | Path | What it gives us |
|---|---|---|
| `hydectl` | `~/.local/bin/hydectl` (HyDE-Project official CLI, `r45.5b3a9cc`) | Subcommands: `wallpaper`, `theme`, `config`, `dispatch`, `reload`, `select`, `tabs`, `zoom`, `completion`, `version`. |
| `hyde-shell` | `~/.local/bin/hyde-shell` | `reload`, `wallbash <script>`, `validate`, `pyinit`, `uv <cmd>`, `init`, `completions`, plus runs any of the 70+ scripts in `~/.local/lib/hyde/`. |
| `hyprquery` | `/usr/bin/hyprquery` (`hyprquery-git`) | Read native Hyprland config values from Rust/CLI — useful if we ever need to read the current theme palette. |
| Wallbash | `~/.local/share/wallbash`, `~/.cache/hyde/wallbash` | Dynamically regenerates kitty / GTK / Qt / Helix-like themes from the active wallpaper. |
| HyDE source | `~/HyDE/` (clone of repo) | All install scripts, configs, themes. |

`hydectl wallpaper` alone covers: `get`, `list`, `set`, `next`, `previous`, `random`, `select` (rofi), `output`, with a `--backend` flag (swww / mpvpaper). **This is a strict superset of `8sync bg`.**

`hydectl theme` covers: `set`, `next`, `prev`, `select`, `import`. **Strict superset of `8sync look`** (and it propagates to kitty/GTK/Qt via wallbash automatically, which our presets cannot).

`hyde-shell -s` lists scripts including `fzf_preview`, `fzf_wrapper`, `parse.config`, `parse.json`, `swwwallbash`, `wallbash.print.colors`, `theme.{import,patch,select,switch}`, `screenshot`, `cliphist.image`, `notifications`. We can call these from 8sync if we ever need them.

### 2.2 Packages already installed by HyDE (`pkg_core.lst` + extras)

System pkgs (1170 total). Relevant to `8sync`:

| Already installed (HyDE/CachyOS) | Version on host |
|---|---|
| `kitty` | 0.46.2 |
| `git` | (system) |
| `github-cli` (`gh`) | (system) |
| `fzf` | (system) |
| `ripgrep`, `fd`, `jq`, `imagemagick`, `poppler`, `unzip`, `fastfetch`, `btop`, `ufw`, `curl`, `paru`, `protobuf`, `python` | all present |
| `starship` | 1.25.1 (HyDE prompt for zsh+fish) |
| `duf` | 0.9.1 (HyDE extra) |
| `hyprquery-git` | 0.6.8 |
| `rofi`, `waybar`, `dunst`, `code`, `firefox` | (HyDE core) |
| `vim` | (HyDE core) |
| `hyprland` + 16 hyprland-* packages | full Hyprland stack |
| `fcitx5` family | input method |

### 2.3 Packages **missing** on this host

`helix`, `lazygit`, `nodejs`, `npm`, `pnpm`, `bun`, `docker`, `docker-compose`, `eza`, `bat`, `zoxide`, `fish` (zsh is default), `abduco`, `zip`, plus all the mobile/db/warp extras.

### 2.4 Live environment signals

- `$SHELL` = `/usr/bin/zsh`.
- `$EDITOR` = `code`. → 8sync `find.rs` forcing `helix` open is wrong default.
- `~/.config/starship.toml` exists → don't write fish config or replace prompt.
- `~/.config/kitty/kitty.conf` chain: `kitty.conf → include hyde.conf → include theme.conf`. `allow_remote_control` is **NOT** set anywhere. → `8sync .` 3-pane via `kitty @ launch` would silently fail.
- HyDE source is cloned at `~/HyDE/` (user-customizable).

---

## 3. Reuse map (what `8sync` should delegate to HyDE)

| Current `8sync` feature | HyDE equivalent | Decision |
|---|---|---|
| `8sync bg <search>` (Wallhaven/etc) | `hydectl wallpaper set/select/random` | **Drop verb.** Optionally provide a thin alias `8sync bg → hydectl wallpaper` in phase 2; not in this PR. |
| `8sync bg opacity` | wallbash sets kitty opacity from wallpaper automatically | **Drop.** |
| `8sync look <preset>` | `hydectl theme set <name>` | **Drop verb.** Wallbash regenerates kitty / GTK / Qt; our 5 hard-coded `.conf` presets cannot reach GTK/Qt. |
| Custom kitty.conf install | HyDE owns `kitty.conf → hyde.conf → theme.conf` | **Stop writing kitty.conf.** Only write a kitty session file for `8sync .`. |
| fish `conf.d/8sync.fish` | HyDE uses zsh+starship; fish is only an optional HyDE extra | **Drop.** Provide a zsh drop-in instead (or simply rely on PATH for `~/.local/bin/8sync`). |
| `environment-im.conf` (fcitx5 env) | HyDE configures fcitx5 | **Drop.** |
| `8sync setup` installing kitty/fzf/rg/fd/jq/imagemagick/poppler/fastfetch/btop/git/gh/ufw/python | HyDE/CachyOS already installed | **Drop from install list.** Doctor warns if any vanish. |
| MCP systemd-user service | n/a — `mcp.rs` is a stub | **Drop until phase 2.** |
| `8sync find` → opens helix | `$EDITOR=code` (or vim, hx) | **Honor `$EDITOR`.** Fallback chain: `$EDITOR → hx → code → vim`. |
| `8sync shot` / `diff-img` / `pdf-img` | depends on `imagemagick` (HyDE has) + `poppler` (system has) | **Keep**, zero new packages needed. |
| `8sync .` 3-pane via `kitty @ launch` | needs `allow_remote_control yes`, which HyDE does not set | **Soft-mode**: 1 kitty window + `abduco` for detached forge. If user wants 3-pane, add a one-line opt-in (`hyde-shell` style snippet in `~/.config/kitty/kitty.conf`) and doctor reports it. No auto-edit. |

---

## 4. Final verb surface

12 verbs (down from 20):

```
setup     up        doctor    flow      help          (lifecycle, 5)
.         ai        end       ship      note          (vibe loop, 5)
find      run                                         (workflow, 2)
skill                                                  (forge config, 1)
shot      diff-img  pdf-img                            (optional, 3)  ← keep, no extra deps
```

Dropped: `bg`, `look`, `mcp`, `selfup` is internal helper not a verb (stays).

If `shot/diff-img/pdf-img` are also out of scope per user feedback, count drops to **9**.

---

## 5. Final setup footprint

### 5.1 Packages to install (`pacman_ensure`)

```
helix       — terminal-first editor for `8sync find` (optional but recommended)
lazygit     — TUI git for `8sync .`           (optional)
abduco      — detached forge session for `8sync .`  (REQUIRED)
```

That's it. `eza`, `zoxide`, `bat` are nice-to-have shell extras; HyDE lists them as *extras* (zsh/fish only). **Not 8sync's job.** Doctor reports availability.

`forge` keeps its curl installer (HyDE doesn't ship it).

### 5.2 Configs written (`assets/configs/`)

| Keep | Target | Why |
|---|---|---|
| `global.toml` | `~/.config/8sync/global.toml` | 8sync's own runtime config. |
| `skills.toml` | `~/.config/8sync/skills.toml` | 8sync's own. |
| `kitty.session` | `~/.config/kitty/8sync.session` | Read by `kitty --session 8sync.session` from `8sync .`. Does NOT touch `kitty.conf`. |
| `helix-config.toml` | `~/.config/helix/config.toml` (only if user lacks one) | Idempotent skip if file exists. |

| Delete | Reason |
|---|---|
| `assets/configs/kitty.conf` | Conflicts with HyDE include chain. |
| `assets/configs/helix-glass_black.toml` | Paired with `look`. |
| `assets/configs/fish-config.fish` | Shell is zsh; HyDE owns prompt. |
| `assets/configs/environment-im.conf` | HyDE owns fcitx5. |
| `assets/configs/8sync-mcp.service` | Stub. |
| `assets/presets/*.conf` (5 files) | Wallbash supersedes. |
| `assets/wallpapers/wallpapers.toml` | `hydectl wallpaper` supersedes. |

### 5.3 Services / system changes — DROP ALL

- No `systemctl --user enable 8sync-mcp.service`.
- No `ufw enable`, no `docker enable`, no `usermod -aG docker`.
- No WARP block.

### 5.4 Setup flags

Keep: `--dry-run`, `-u/--update`.
Drop: `--minimal`, `--no-mobile`, `--no-db`, `--no-warp`.

---

## 6. File-by-file changes

| File | Action | Notes |
|---|---|---|
| `crates/cli/src/main.rs` | edit | Remove `Cmd::Bg`, `Cmd::Look`, `Cmd::Mcp` variants + matches; update `HELP_AFTER`. |
| `crates/cli/src/verbs/mod.rs` | edit | Drop `pub mod bg; pub mod look; pub mod mcp;`. |
| `crates/cli/src/verbs/bg.rs` | delete | |
| `crates/cli/src/verbs/look.rs` | delete | |
| `crates/cli/src/verbs/mcp.rs` | delete | |
| `crates/cli/src/verbs/setup.rs` | rewrite | Per §5. Add a `hyde_detected()` block at the top printing "HyDE detected → skipping kitty/fish/wallpaper". |
| `crates/cli/src/verbs/doctor.rs` | edit | Trim checks. Add HyDE awareness: detect `hydectl`, `hyde-shell`, `~/.config/hyde/wallbash`, report. Drop fish / docker / node / pnpm / bun / warp-cli / mcp checks. Honor `$EDITOR`. Add explicit "kitty remote control = ON/OFF" check with a fix hint. |
| `crates/cli/src/verbs/here.rs` | edit | Default mode: single kitty window + abduco (no `kitty @`). If `allow_remote_control` is detected on, optionally do 3-pane. Never auto-edit kitty.conf. |
| `crates/cli/src/verbs/find.rs` | edit | Editor fallback: `$EDITOR → hx → helix → code → vim`. |
| `crates/cli/src/verbs/flow.rs` | edit | Drop `bg`, `look` from listing. Add a "HyDE delegation" hint. |
| `crates/cli/src/verbs/root.rs` | edit | Update cheatsheet. |
| `crates/cli/src/verbs/up.rs` | review | Should only manage `forge` + binary self-update. Drop refs to removed verbs. |
| `crates/cli/src/env_detect.rs` | edit | Add `is_hyde() -> bool` (checks for `hydectl` in PATH or `~/.config/hyde/wallbash` dir). |
| `assets/presets/*.conf` | delete | 5 files. |
| `assets/wallpapers/wallpapers.toml` | delete | |
| `assets/configs/{kitty.conf, fish-config.fish, environment-im.conf, helix-glass_black.toml, 8sync-mcp.service}` | delete | |
| `AGENTS.md` | edit | Rewrite §2 install steps (HyDE prereq), §5 verb count (12), §8 add "delegate to HyDE" rule. |
| `README.md` | edit | Clear: "Designed for HyDE on CachyOS. Non-HyDE users see appendix." Provide minimal non-HyDE install snippet. |

---

## 7. Implementation steps

Tracked as todos. Each step ends with `cargo check`.

1. **Commit this plan v2.**
2. **Drop verb code & assets** (`bg`, `look`, `mcp`, presets, wallpapers, conflicting configs).
3. **Add `env_detect::is_hyde()`** + small helper for kitty remote-control detection.
4. **Rewrite `setup.rs`** per §5.
5. **Trim `doctor.rs`** per §6.
6. **Adjust `here.rs`** to soft 1-pane abduco mode + optional 3-pane.
7. **Adjust `find.rs`** for `$EDITOR` honor.
8. **Update `flow.rs` / `root.rs` / `main.rs::HELP_AFTER`.**
9. **Docs** — `AGENTS.md`, `README.md`.
10. **Build + smoke test (§8).**
11. **Commit + open PR**.

---

## 8. Verification (smoke tests)

```bash
cargo build --release
B=./target/release/8sync

$B --version
$B help                    # verb count = 12 (or 9 if image trio dropped)
$B flow                    # no bg/look references
$B doctor                  # reports: HyDE=YES, kitty=OK (remote=OFF, hint), forge=OK, helix=missing
$B setup --dry-run         # plan = 3 pkgs + 4 configs + forge curl + skills. No wallpaper/service/warp/docker.
$B . -h                    # subactions intact
$B find --no-open --type rs "fn run"
```

Manual:
- `~/.config/kitty/kitty.conf` still starts with `include hyde.conf` (we never touch it).
- `~/.config/fish/conf.d/8sync.fish` is absent.
- `~/.config/8sync/{global,skills}.toml` exist after real `setup`.
- `~/.forge/skills/{karpathy-guidelines,image-routing,8sync-cli}/SKILL.md` + `00-force-load.md` exist.
- `ls -l target/release/8sync` → expect < 1.0 MB stripped (currently ~1.3 MB).

---

## 9. Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `here.rs` 3-pane breaks on HyDE because remote control off. | High | Medium | Already in plan: soft-mode default; doctor reports; never auto-edit kitty.conf. |
| Non-HyDE users who relied on `8sync setup` for kitty/fish/wallpaper regress. | Medium | Medium | README appendix: one-liner pacman install for non-HyDE. Doctor messaging is friendly. |
| `here.rs` or other modules reference dropped assets at compile time. | Medium | High | `fs_search` for every asset path before commit; `cargo check` after each step. |
| External integrations call `8sync mcp` / `8sync bg` / `8sync look`. | Low | Low | Print a stable "moved to HyDE: use `hydectl wallpaper` / `hydectl theme`" message if such an arg is matched as `_` in clap? Optional — by default clap will error. |
| HyDE renames `hydectl` subcommands in future. | Low | Low | We don't link against HyDE; we just delegate via docs. |

---

## 10. Out-of-scope follow-ups (phase 2)

- Optional thin wrappers: `8sync bg <args>` → `hydectl wallpaper <args>`, `8sync look <name>` → `hydectl theme set <name>`. Only if users complain about muscle memory.
- HyDE-palette-aware helix theme generator (read `hyprquery` / wallbash output → emit helix theme).
- Real MCP server.
- zsh drop-in (`~/.zshrc.d/8sync.zsh`) for tab-completion + aliases.
- Non-HyDE installer profile (`8sync setup --no-hyde`) that installs kitty/fzf/etc. for vanilla Arch users.

---

## 11. Audit evidence

- HyDE pkg list: https://github.com/HyDE-Project/HyDE/blob/master/Scripts/pkg_core.lst and `pkg_extra.lst`.
- Local `hydectl --help` output captured 2026-05-18 (`r45.5b3a9cc`).
- Local `hyde-shell -s` output captured 2026-05-18 (70+ scripts).
- Live system: 1170 packages installed; missing only `helix`, `lazygit`, `abduco`, plus opinionated extras.
- Kitty `allow_remote_control` not set in `~/.config/kitty/{kitty,hyde}.conf`.
- Code citations: `crates/cli/src/verbs/setup.rs:81-122`, `:154-163`, `:172-190`, `:213-242`; `crates/cli/src/verbs/doctor.rs:30-65`; `crates/cli/src/verbs/find.rs` (editor open); `crates/cli/src/main.rs:65-89` (verbs to remove).
