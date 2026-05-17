# Plan v6 — Profile system for `8sync setup`

- **Branch:** `chore/slim-down-for-hyde`
- **Date:** 2026-05-18
- **Supersedes:** v2 §3-5 (verb surface & setup footprint). v3/v4/v5 audit findings remain valid as evidence.
- **New feature:** opt-in personal profiles on top of the slim harness core.

---

## 1. Design

```
8sync setup [--dry-run] [--yes] [--profile <name>...] [--no-profile]
  Stage A. Core harness (always, idempotent):
    pacman -S --needed helix lazygit abduco github-cli
    forge curl install (if missing)
    write 8sync/global.toml, 8sync/skills.toml, kitty/8sync.session
    write helix/config.toml (if absent)
    write ~/.forge/skills/{karpathy,image-routing,8sync-cli}/SKILL.md + 00-force-load.md
  Stage B. Personal profiles:
    if --profile X     → apply X non-interactive
    elif --no-profile  → skip
    elif TTY          → interactive y/N per profile
    else              → skip (no-TTY = no-profile)
    save selection → ~/.config/8sync/profile.toml

8sync setup profile <action>
  list                      list available profiles (embedded + user)
  show [<name>]             show profile content (default: current)
  apply <name>...           idempotent apply
  add <name> [--pacman p,p] [--aur a,a]   add new profile (writes to ~/.config/8sync/profiles/)
  edit <name>               open in $EDITOR
  remove <name>             delete profile file (does NOT uninstall pkgs)
  uninstall <name>          pacman -Rns the profile's pkgs (with confirm)
```

---

## 2. Profile schema (TOML)

```toml
name = "vietnamese"
description = "Vietnamese input method (fcitx5 + Unikey)"

# Optional: compose other profiles
extends = ["..."]

[packages]
pacman = ["fcitx5", "fcitx5-configtool", "fcitx5-gtk", "fcitx5-qt", "fcitx5-unikey"]
aur    = []   # if non-empty, profile loader uses paru (fallback yay)

[configs]
# Map of "relative-to-XDG-config-home" → "file content"
"environment.d/im.conf" = """
GTK_IM_MODULE=fcitx
QT_IM_MODULE=fcitx
XMODIFIERS=@im=fcitx
"""

[post_install]
# Shell commands run once after package install (best-effort)
commands = []
```

---

## 3. Built-in profiles (`assets/profiles/`)

6 building blocks + 1 bundle, all committed in the repo (no secrets, just package names):

| File | Packages / Actions |
|---|---|
| `vietnamese.toml` | pacman: `fcitx5, fcitx5-configtool, fcitx5-gtk, fcitx5-qt, fcitx5-unikey`<br>configs: `environment.d/im.conf` |
| `hardware-lianli.toml` | aur: `lianli-linux-git` (yay/paru auto-pulls all deps: cmake, nasm, rustup, webkit2gtk-4.1, hidapi, libusb, gtk3, librsvg, ffmpeg)<br>requires: `aur_helper = true` |
| `hardware-cooling.toml` | pacman: `openrgb, coolercontrol, liquidctl` |
| `displaylink.toml` | pacman: `evdi-dkms` |
| `apps-personal.toml` | pacman: `bitwarden` |
| `warp.toml` | aur: `cloudflare-warp-bin`<br>requires: `aur_helper = true`<br>post_install: enable `warp-svc.service`, register, mode=doh, MASQUE tunnel, malware DNS filter, connect |
| `alexdev.toml` (bundle) | `extends = ["vietnamese", "hardware-lianli", "hardware-cooling", "displaylink", "apps-personal", "warp"]` |

User-added profiles live in `~/.config/8sync/profiles/*.toml` and override built-ins on name conflict.

---

## 4. CLI behavior matrix

| Invocation | Stage A | Stage B |
|---|---|---|
| `8sync setup` | run (no "Continue?" prompt — gõ là cài) | TTY: prompt y/N per profile<br>no-TTY: skip |
| `8sync setup --yes` | run | apply ALL profiles, no prompts |
| `8sync setup --no-profile` | run | skip |
| `8sync setup --profile alexdev` | run | apply `alexdev` non-interactive |
| `8sync setup --profile vietnamese --profile displaylink` | run | apply both, non-interactive |
| `8sync setup --dry-run` | print plan | print profile list as inert text |
| `8sync setup profile apply alexdev` | skip | apply only |

---

## 5. File-by-file changes (vs v2 §6)

| File | Action |
|---|---|
| `crates/cli/src/main.rs` | edit — remove `Cmd::Bg/Look/Mcp`; `Cmd::Setup` already exists, sub-action `profile` handled inside `verbs::setup`. |
| `crates/cli/src/verbs/mod.rs` | edit — drop `bg, look, mcp`; add `profile`. |
| `crates/cli/src/verbs/bg.rs`, `look.rs`, `mcp.rs` | delete. |
| `crates/cli/src/verbs/setup.rs` | rewrite — Stage A logic + dispatch to Stage B / `profile` sub-action. |
| `crates/cli/src/verbs/profile.rs` | **new** — profile loader, applier, sub-actions (`list/show/apply/add/edit/remove/uninstall`). |
| `crates/cli/src/pkg.rs` | keep `ensure_paru()` + `paru_ensure()` (needed for AUR profiles); add `aur_helper_detect()` returning `"paru"` → `"yay"`. |
| `crates/cli/src/verbs/doctor.rs` | edit — read `~/.config/8sync/profile.toml`, verify each applied profile's packages. Honor `$EDITOR`. Hard-check `gh`. |
| `crates/cli/src/verbs/find.rs` | edit — `$EDITOR → hx → helix → code → vim` fallback. |
| `crates/cli/src/verbs/here.rs` | edit — default 1-window + abduco; never auto-edit kitty.conf. |
| `crates/cli/src/verbs/flow.rs`, `root.rs` | edit — drop refs to `bg`/`look`/`mcp`; add `setup profile` hint. |
| `crates/cli/src/env_detect.rs` | edit — add `is_hyde()`, `has_tty()`, `aur_helper()`. |
| `assets/profiles/*.toml` | **new** — 6 files per §3. |
| Removed assets | `presets/*.conf`, `wallpapers/wallpapers.toml`, `configs/kitty.conf`, `configs/fish-config.fish`, `configs/environment-im.conf`, `configs/helix-glass_black.toml`, `configs/8sync-mcp.service`. |
| `AGENTS.md` | edit — verb count 12 + `setup profile`; describe profile system. |
| `README.md` | edit — install section: `8sync setup` for harness, `8sync setup --profile <bundle>` for personal. |

---

## 6. Verb count

```
Lifecycle (5):  setup [profile <subcmd>]  up  doctor  flow  help
Vibe loop (5):  .  ai  end  ship  note
Workflow (2):   find  run
Forge (1):      skill
Image (3):      shot  diff-img  pdf-img
                                                 = 16 commands, 12 top-level verbs
```

(Down from 20; `bg`, `look`, `mcp` removed; `setup profile` is a sub-command, not a new top verb.)

---

## 7. Implementation steps

Each step ends with `cargo check`. Single feature branch, atomic commits.

1. Commit this plan v6.
2. Drop verb code & assets (`bg`, `look`, `mcp`, `presets/`, `wallpapers/`, conflicting configs).
3. Add `env_detect::is_hyde() / has_tty() / aur_helper()`.
4. Create `assets/profiles/{vietnamese,hardware-lianli,hardware-cooling,displaylink,apps-personal,alexdev}.toml`.
5. Implement `verbs/profile.rs`: schema + loader (merging embedded + user) + applier + sub-actions.
6. Rewrite `verbs/setup.rs`: Stage A + Stage B + arg routing.
7. Trim `verbs/doctor.rs`: profile-aware checks; hard-check `gh`.
8. Tweak `verbs/find.rs`, `verbs/here.rs` per §5.
9. Update `verbs/flow.rs`, `verbs/root.rs`, `main.rs` HELP_AFTER.
10. Docs: `AGENTS.md`, `README.md`.
11. Build + smoke test (§8).
12. Open PR.

---

## 8. Verification

```bash
cargo build --release
B=./target/release/8sync

$B --version
$B help
$B flow                                       # no bg/look refs
$B doctor                                     # HyDE=YES, gh=OK (error if missing), profile=<none yet>
$B setup --dry-run                            # plan: 3-4 pkgs, no extras
$B setup --no-profile --yes                   # core only
$B setup profile list                         # 6 profiles
$B setup profile show alexdev                 # bundle definition
$B setup profile apply vietnamese --dry-run   # 5 pkgs preview
$B setup --profile alexdev --dry-run          # full bundle preview
$B . -h
$B find --no-open --type rs "fn run"
```

Manual:
- `~/.config/kitty/kitty.conf` untouched (still `include hyde.conf`).
- `~/.config/8sync/profile.toml` written after a real `setup` with profiles.
- `~/.config/8sync/profiles/` is reachable (user-override dir, may be empty).
- Binary size < 1.1 MB stripped (profiles add ~5 KB of embedded TOML).

---

## 9. Risk

| Risk | Mitigation |
|---|---|
| Stage B prompts pollute non-interactive scripts | `has_tty()` defaults to no-profile when stdin is not a TTY. |
| AUR install fails on profile with `aur=[...]` | Profile applier checks `aur_helper()` before pacman step; fails fast with clear error if neither `paru` nor `yay` is present. Both are on host. |
| User edits profile.toml incorrectly | Schema validation on load; bad files reported by name, others applied. |
| Profile rename / removal breaks `apply` history | `profile.toml` records the applied snapshot, not a reference, so renames are safe. |
| Embedded profiles get out of date | `8sync up` can re-pull and merge new built-in profiles (phase 2). |

---

## 10. Decision log (confirmed)

- TTY-less default → `--no-profile`.
- AUR helper preference → `paru` > `yay`.
- `alexdev.toml` lives in `assets/profiles/` (in repo, public — only package names, no secrets).
- Sub-command lives under `setup` (`8sync setup profile ...`), not its own top-level verb.

---

## 11. Final install footprint (per current host state)

```
$ 8sync setup --profile alexdev --yes
[harness] pacman -S --needed helix lazygit abduco github-cli   → installs: helix lazygit abduco  (gh ✓)
[harness] forge: ✓ present — skip
[harness] configs: 4 files
[harness] skills: 4 files
[profile] vietnamese:       pacman → all 5 pkgs ✓ present — skip
                            configs: 1 file (environment.d/im.conf)
[profile] hardware-cooling: pacman → openrgb ✓, coolercontrol ✓, liquidctl ✓ — skip
[profile] displaylink:      pacman → evdi-dkms ✓ — skip
[profile] apps-personal:    pacman → bitwarden ✓ — skip
[profile] hardware-lianli:  aur (paru) → lianli-linux-git ✓ — skip
                            (yay/paru tự kéo deps lúc build)
[profile] warp:             aur (paru) → cloudflare-warp-bin ✓ — skip
                            post: warp-svc enabled, mode=doh, MASQUE, malware DNS, connected
Done. profile.toml saved → ~/.config/8sync/profile.toml
```

On a fresh machine the same command performs the full install in ~5 minutes. On this host it's a no-op verifier.
