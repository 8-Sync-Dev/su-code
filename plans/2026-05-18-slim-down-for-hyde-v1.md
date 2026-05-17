# Plan — Slim down `8sync` for HyDE-managed CachyOS

- **Branch:** `chore/slim-down-for-hyde`
- **Date:** 2026-05-18
- **Author / Driver:** alexdev
- **Status:** Draft v1 (awaiting approval before execution)

---

## 1. Context

`su-code` ships a binary `8sync` whose `setup` verb installs ~35 Arch packages and overwrites configs for kitty / fish / fcitx5 / wallpaper. The current target host (and the project's primary audience) installs **HyDE** (https://github.com/Hyde-project/hyde) on CachyOS first.

HyDE already provides:

- Hyprland stack (compositor, hyprpaper-equivalent, hyprlock, hypridle, hyprsunset, hyprpicker, xdg-desktop-portal-hyprland).
- Wallbash theme engine that dynamically generates `~/.config/kitty/theme.conf`, GTK, Qt, etc. from the active wallpaper.
- Kitty with `kitty.conf → hyde.conf → theme.conf` include chain.
- fcitx5 fully configured, fastfetch, btop, ripgrep, fd, fzf, jq, ufw, paru, poppler, imagemagick, curl, github-cli, git, python.
- Default shell `zsh` (not fish).

Observed conflicts / redundancy on the live host (see audit at the bottom of this plan):

| Area | Conflict |
|---|---|
| `8sync bg` | Bypasses Hyprland + wallbash → broken sync between wallpaper and kitty/gtk/qt themes. |
| `8sync look` | Overwrites `kitty.conf`, breaks `include hyde.conf` chain. |
| `fish` config | Default shell is `zsh`; the dropped `~/.config/fish/conf.d/8sync.fish` is never loaded. |
| `environment-im.conf` | Redundant — HyDE already configures fcitx5. |
| WARP / mobile / db / docker blocks | Out of scope for a coding harness. |
| MCP systemd service | Verb `mcp` is a stub; service runs an empty daemon. |

Goal of this plan: shrink `8sync` to its true value (**`gh` + `forge` + agents harness**), avoid stepping on HyDE, and reduce verb count + install surface.

---

## 2. Objectives & Non-Goals

### Objectives
1. Remove every feature that duplicates or fights HyDE's wallbash / kitty / wallpaper / IM stack.
2. Reduce `setup` to install only what HyDE does *not* already give us.
3. Keep the AI harness surface intact: `.`, `ai`, `end`, `ship`, `note`, `find`, `run`, `skill`, `up`, `doctor`, `flow`, `help`, `setup`, `selfup`.
4. Keep binary < 1 MB stripped after cleanup (currently ~1.3 MB).
5. Ship one PR with passing smoke tests.

### Non-Goals
- Re-implementing wallpaper / theming via HyDE-aware adapters (deferred — phase 2 if ever needed).
- Adding new features.
- Multi-distro support beyond CachyOS/Arch + HyDE.
- Migrating shell from zsh to fish (we drop the fish drop-in entirely).

---

## 3. Target Verb Surface (post-cleanup)

| Keep | Drop |
|---|---|
| `setup` (slim) | `bg` |
| `up`, `selfup` | `look` |
| `doctor` (check-only) | `mcp` (stub, defer to phase 2) |
| `flow`, `help`, root | `shot`, `diff-img`, `pdf-img` (optional — keep only if user agrees; default: keep, no extra deps required since `poppler` + `imagemagick` already installed by HyDE) |
| `.` / `here` (+ sub-actions) | |
| `ai`, `end`, `ship`, `note`, `run`, `find`, `skill` | |

Verb count: **20 → 14** (or 11 if image-routing trio also dropped).

---

## 4. Target Setup Footprint

### 4.1 Packages

Before:
```
kitty helix git github-cli lazygit nodejs npm pnpm bun docker docker-compose
ripgrep fd fzf eza bat jq fastfetch btop zoxide protobuf unzip zip ufw fish
python python-pip poppler imagemagick abduco curl
[+ jdk-openjdk android-tools android-udev postgresql valkey cloudflare-warp-bin]
```

After:
```
helix lazygit abduco eza zoxide bat
```

Everything else is either already provided by HyDE or out of scope. Doctor will *warn* (not auto-install) if any of HyDE's tools go missing.

### 4.2 Configs written

Before (`setup.rs:154-163`): 8 pairs.

After: 4 pairs.
```
configs/helix-config.toml          → ~/.config/helix/config.toml
configs/global.toml                → ~/.config/8sync/global.toml
configs/skills.toml                → ~/.config/8sync/skills.toml
configs/kitty.session              → ~/.config/kitty/8sync.session   (used by `8sync .`, not by kitty itself on boot)
```

Dropped pairs:
- `configs/kitty.conf` (would clobber HyDE include chain).
- `configs/helix-glass_black.toml` (paired with `look`).
- `configs/fish-config.fish` (zsh host).
- `configs/environment-im.conf` (HyDE owns fcitx5).

### 4.3 Wallpaper

Section removed entirely. Hyprland + wallbash own this.

### 4.4 Services

- Remove `8sync-mcp.service` install + `systemctl --user enable` (`setup.rs:217-221`).
- Remove `ufw enable`, `docker enable`, `usermod -aG docker` (`setup.rs:122, 224-228`). These are sysadmin opinions, not harness concerns.
- Remove WARP service block (`setup.rs:230-240`).

### 4.5 Setup flags

Remove `--minimal`, `--no-mobile`, `--no-db`, `--no-warp`. Keep `--dry-run` and `-u/--update`.

---

## 5. File-by-file Changes

| File | Action | Notes |
|---|---|---|
| `crates/cli/src/main.rs` | edit | Remove `Cmd::Bg`, `Cmd::Look`, `Cmd::Mcp` variants + match arms. Update `HELP_AFTER` to drop references. |
| `crates/cli/src/verbs/mod.rs` | edit | Drop `pub mod bg; pub mod look; pub mod mcp;`. |
| `crates/cli/src/verbs/bg.rs` | delete | — |
| `crates/cli/src/verbs/look.rs` | delete | — |
| `crates/cli/src/verbs/mcp.rs` | delete | — |
| `crates/cli/src/verbs/setup.rs` | rewrite | New shape per §4. |
| `crates/cli/src/verbs/doctor.rs` | edit | Trim checks: drop fish, docker, node, pnpm, bun, warp-cli, ufw, mcp, wallpaper. Keep helix/hx, gh, lazygit, forge, abduco, eza, zoxide, bat, plus `gh auth status`. Report `wallbash` presence as informational HyDE detection. |
| `crates/cli/src/verbs/flow.rs` | edit | Remove `bg` / `look` from workflow listing. |
| `crates/cli/src/verbs/root.rs` | edit | Update cheatsheet to drop `bg` / `look` / `mcp`. |
| `crates/cli/src/verbs/up.rs` | review | Confirm it only targets tools we still ship. |
| `crates/cli/src/verbs/here.rs` | review | Confirm it does not read `assets/presets/` or `wallpapers/`. If it does, replace with a no-op or HyDE detection. |
| `assets/presets/*.conf` (5 files) | delete | — |
| `assets/wallpapers/wallpapers.toml` | delete | — |
| `assets/configs/kitty.conf` | delete | — |
| `assets/configs/fish-config.fish` | delete | — |
| `assets/configs/environment-im.conf` | delete | — |
| `assets/configs/helix-glass_black.toml` | delete | — |
| `assets/configs/8sync-mcp.service` | delete | — |
| `AGENTS.md` | edit | Update §2 (no `8sync setup` mass install), §5 verb count (14), §7 skills (unchanged), and add HyDE awareness paragraph. |
| `README.md` | edit | Update install instructions; clearly state HyDE prerequisite or compatibility. |

---

## 6. Implementation Steps

Tracked as todos during execution. Each step ends with `cargo check` (and at the end with full `cargo build --release` + smoke).

1. **Branch hygiene** — Already on `chore/slim-down-for-hyde`. Commit this plan first (`docs: plan slim-down for HyDE`).
2. **Drop verbs (code)** — Remove `bg.rs`, `look.rs`, `mcp.rs`; update `mod.rs`, `main.rs` (`Cmd` enum + match), `root.rs`, `flow.rs`. Run `cargo check`.
3. **Drop verb assets** — Delete `assets/presets/`, `assets/wallpapers/`, `assets/configs/{kitty.conf,fish-config.fish,environment-im.conf,helix-glass_black.toml,8sync-mcp.service}`.
4. **Rewrite `setup.rs`** — Per §4. Keep `--dry-run`, `-u`. Remove mobile/db/warp/docker branches and helpers.
5. **Trim `doctor.rs`** — Per §5. Add HyDE detection (look for `~/.config/hyde/wallbash` and report).
6. **Trim `here.rs`** — Audit: ensure it does not depend on dropped assets. If it referenced wallpaper/preset, replace with a comment pointing to HyDE.
7. **Docs** — Update `AGENTS.md` and `README.md`.
8. **Build & verify** — `cargo build --release`, run smoke tests (§7).
9. **Commit & open PR** — Atomic commits per phase if reviewers prefer; otherwise a single squashable commit.

---

## 7. Verification (smoke tests)

```bash
cargo build --release
./target/release/8sync --version
./target/release/8sync help            # verb list shows 14 (or 11)
./target/release/8sync flow
./target/release/8sync doctor          # warns nothing missing on this host
./target/release/8sync setup --dry-run # plan shows ≤6 pkgs + 4 configs, no wallpaper / warp / mcp
./target/release/8sync . -h
./target/release/8sync find --no-open --type rs "fn run"
```

Manual checks:
- `~/.config/kitty/kitty.conf` must still contain `include hyde.conf` (i.e. we did **not** touch it).
- `~/.config/fish/conf.d/8sync.fish` is **absent** (we no longer drop it).
- `~/.config/8sync/{global,skills}.toml` and `~/.forge/skills/00-force-load.md` exist after a real run of `setup`.
- Binary size: `ls -l target/release/8sync` → expect < 1.0 MB stripped.

---

## 8. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `here.rs` silently depends on a dropped asset (preset path) and panics at runtime. | Medium | High | Read `here.rs` end-to-end during step 6; smoke test `8sync .` in a scratch dir. |
| Users on non-HyDE Arch boxes regress (they relied on `8sync setup` to install kitty/fish from scratch). | Medium | Medium | Doctor warns clearly; README states HyDE/Hyprland prerequisite. Provide one-liner `pacman -S kitty fish ...` for non-HyDE folks in README. |
| Removing `Cmd::Mcp` breaks any external integration calling `8sync mcp`. | Low | Low | Stub only; no real consumers. Document in CHANGELOG section of PR. |
| Skill-loading paths shift after asset reshuffle. | Low | Medium | Keep `assets/skills/` untouched; only configs/presets/wallpapers are touched. |
| Binary embedded paths (rust-embed) referenced by name elsewhere. | Low | Medium | `fs_search` for every deleted asset path before commit. |

---

## 9. Audit Snapshot (2026-05-18)

System state captured before starting work:

- Already installed (HyDE): `kitty git github-cli ripgrep fd fzf jq fastfetch btop poppler imagemagick paru ufw curl protobuf python unzip` + Hyprland stack + fcitx5 + wallbash.
- Missing on host: `helix lazygit abduco eza zoxide fish nodejs npm pnpm bun docker docker-compose zip jdk-openjdk android-tools android-udev postgresql valkey cloudflare-warp-bin python-pip`.
- Shell: `/usr/bin/zsh`.
- Kitty configs: `kitty.conf`, `hyde.conf`, `theme.conf` (wallbash-managed).

Cited evidence: `crates/cli/src/verbs/setup.rs:81-122`, `crates/cli/src/verbs/setup.rs:154-163`, `crates/cli/src/verbs/setup.rs:172-190`, `crates/cli/src/verbs/setup.rs:213-242`, `crates/cli/src/verbs/doctor.rs:30-65`.

---

## 10. Out-of-scope follow-ups

- Phase 2 — `8sync look` reborn as a `wallbash`-aware adapter (read HyDE palette, emit helix theme).
- Phase 2 — Real MCP server (currently stub).
- Phase 2 — Optional shell drop-in for both zsh and fish (detected at runtime).
