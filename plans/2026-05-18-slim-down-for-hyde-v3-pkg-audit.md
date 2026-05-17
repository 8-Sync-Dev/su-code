# Plan addendum v3 — Package-manager audit (2026-05-18)

Supplements `plans/2026-05-18-slim-down-for-hyde-v2.md` with findings from a full `pacman` / `paru` / `yay` / `flatpak` audit on the live host.

---

## A. Host package-manager surface

| Manager | Status | Notes |
|---|---|---|
| `pacman` | installed | 8 repos enabled: `cachyos-znver4`, `cachyos-core-znver4`, **`cachyos-extra-znver4`**, `cachyos`, `core`, `extra`, `multilib`, **`chaotic-aur`**. |
| `paru` 2.1.0 | installed | AUR helper (HyDE prefers it). |
| `yay-bin` 12.5.7 | installed | Second AUR helper, also present. |
| `pkgfile` | installed | File-to-pkg lookup. |
| `flatpak` 1.16.6 | installed | 5 apps present (Bitwarden, Clapper, OBS, …); irrelevant for 8sync. |
| `snap`, `pamac`, `pikaur`, `trizen`, `aurman`, `nix`, `pipx`, `pip` | absent | — |
| `rustup` 1.29.0 + `cargo` 1.95 + `rustc` 1.95 | installed (explicit) | `bootstrap.sh` rustup-init step is **redundant** on HyDE hosts. |
| `uv` 0.11.14 | installed (explicit) | System uv; HyDE also has its own venv at `~/.local/share/hyde/python_env`. |
| `cmake`, `nasm`, `perl`, `python` 3.14.4, `texinfo`, `base-devel` | installed | Full build toolchain ready. |

## B. Inventory

- `pacman -Q` = **1170** packages.
- `pacman -Qe` = **234** explicit (user/HyDE-chosen). Full list captured in audit.
- `pacman -Qm` = **1 AUR-only package** (`lianli-linux-git`). Everything else comes from official repos or chaotic-aur.

→ **`paru` is essentially idle.** The only reason `setup.rs:100-104` calls `pkg::ensure_paru()` + `paru_ensure(&["cloudflare-warp-bin"])` is the WARP block, which v2 already drops. Therefore the **entire AUR branch can be removed** from `setup.rs` and `pkg.rs` for the slim profile.

## C. Availability of slim-setup targets

All three required + the optional shell extras are in **`cachyos-extra-znver4`** (official repo, prebuilt, znver4-optimised). Zero AUR / chaotic / paru needed:

```
cachyos-extra-znver4/helix     25.07.1-1.1
cachyos-extra-znver4/lazygit   0.61.1-1.1
cachyos-extra-znver4/abduco    0.6-8.1
cachyos-extra-znver4/eza       0.23.4-3.1   (optional)
cachyos-extra-znver4/bat       0.26.1-2.1   (optional)
cachyos-extra-znver4/zoxide    0.9.9-2.1    (optional)
```

`forge` remains **not** packaged anywhere (pacman/chaotic/AUR) → keep `curl https://forgecode.dev/cli | sh` (writes to `~/.local/bin/forge` per current host: `~/.local/bin/forge`, 42 MB). Already installed on this host.

## D. Already-present pieces relevant to 8sync (explicit pkgs)

From `pacman -Qe` excerpt:

- Terminals: `kitty`, `alacritty 0.17.0`. → 8sync should not assume kitty-only; alacritty has no `kitty @` equivalent but works for single-pane fallback.
- Editors: `code 1.119`, `vim 9.2`, `nano 9.0`. (`helix` missing → install candidate.)
- Shells: `zsh 5.9` (default), `starship 1.25.1`. (No fish.)
- Search / nav: `fzf 0.72`, `ripgrep 15.1`, `plocate`, `pkgfile`.
- Sysinfo: `btop`, `fastfetch`, `glances`, `duf`.
- Build: `cmake`, `nasm`, `perl`, `python`, `uv`, `rustup`, `base-devel`.
- Image / PDF: `poppler-glib`, `imagemagick` (via deps; `magick`/`convert`/`pdftoppm`/`pdftocairo`/`identify` all on PATH).
- Net: `wget`, `rsync`, `openssh`, `bind`, `networkmanager`, `ufw 0.36.2`.
- DE bits: full Hyprland stack, `dunst`, `rofi`, `waybar`, `wlogout`, `hyprlock`, `nwg-displays`, `nwg-look`, `qt5/qt6ct`, `flatpak`.
- Fonts: `noto-fonts`, `noto-fonts-cjk`, `noto-fonts-emoji`, `ttf-meslo-nerd`, `ttf-bitstream-vera`, `ttf-dejavu`, `ttf-liberation`, `ttf-opensans`, `awesome-terminal-fonts`. → 8sync should NOT assume `CaskaydiaCove Nerd Font` even though `hyde.conf` references it. (Out of scope to fix.)

## E. Updates to plan v2

Adopt these clarifications when executing:

1. **`bootstrap.sh`** — detect `rustup` already in PATH → skip the rustup-init step (Idempotency: just print "rustup present, skipping"). Currently it always runs.
2. **`setup.rs`** — completely **remove** the AUR branch (`ensure_paru()` + `paru_ensure`). Single `pacman -S --needed` for `helix lazygit abduco` only. Note in dry-run output: "Using cachyos-extra-znver4 (official repo, no AUR needed)".
3. **`pkg.rs`** — keep `paru_ensure` / `ensure_paru` definitions (don't delete) since they may be reused, but mark unused in slim setup.
4. **`doctor.rs`** — add an informational line: `pacman -Qm` count (so user sees AUR footprint). Add `flatpak` detection only as info.
5. **`env_detect::is_hyde()`** — also check `pacman -Qq hyprland` to confirm Hyprland-based DE, in addition to the `~/.config/hyde/wallbash` presence test.
6. **README install section** — replace `bootstrap.sh` recommendation with a one-liner for HyDE hosts: `cargo install --path crates/cli` (since cargo+rustup are already there). Keep `bootstrap.sh` for non-HyDE / non-Cachy users.

## F. Re-affirmed drops (no change in direction, just confirmed)

- No need to install `nodejs/npm/pnpm/bun` → not in explicit list, not desired.
- No need to install `docker/docker-compose` → not present, not desired.
- No need to install `fish` → zsh is the explicit shell; starship already configured.
- No need to install `cloudflare-warp-bin` → out of scope.
- No need to touch `paru`/`yay` → already installed, slim setup avoids AUR entirely.

## G. New verb-count summary (post-cuts, confirmed)

12 verbs (or 9 if image-routing trio is also dropped per user feedback). Slim setup installs exactly **3** repo packages + 1 curl tool (`forge`).

---

## H. Re-confirmed execution gate

Plan v2 + this v3 addendum are aligned. Awaiting user answers to the 3 gating questions in v2 §11:

1. Keep image-routing trio (`shot`/`diff-img`/`pdf-img`)? Default keep — verified all deps (`magick`, `pdftoppm`, `convert`) are on PATH.
2. Add phase-2 thin wrappers (`8sync bg → hydectl wallpaper`, `8sync look → hydectl theme set`)? Default no in this PR.
3. `8sync .` soft-mode 1-pane + abduco as default (since kitty `allow_remote_control` is off on HyDE)? Default yes.
