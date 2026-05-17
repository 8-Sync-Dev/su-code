# Plan addendum v5 — Install-timestamp reconciliation (2026-05-18)

Reconciles v4 attribution with the user's stated memory of manual installs, using **`Install Date`** from `pacman -Qei` as ground truth. Resolves disagreements between v4 and user feedback.

---

## A. Method

`pacman -Qei | awk` extracted `Install Date | Name` for every **explicit** package, then sorted chronologically. Three distinct batches emerged:

| Batch | Window | Source | Count (approx.) |
|---|---|---|---|
| **B1** | `2026-05-17 06:25 – 06:30` | CachyOS installer (Hyprland edition base profile) | ~135 |
| **B2** | `2026-05-17 06:49 – 06:52` | HyDE installer (`Scripts/install.sh` etc.) | ~65 |
| **B3** | `2026-05-17 18:48 – 2026-05-18 05:56` | User manual installs (post-bootstrap) | **17** |

The install dates put the **truth beyond doubt** — no more guessing. The "user-added" bucket is **much smaller** than v4 claimed (≈17 vs ≈34).

---

## B. Authoritative user-added list (B3, 17 pkgs)

Sorted by install timestamp:

| Time | Package | Purpose |
|---|---|---|
| 17/05 18:48 | `bitwarden` | password manager |
| 17/05 18:55 | `fcitx5` | IM core |
| 17/05 18:55 | `fcitx5-configtool` | IM config UI |
| 17/05 18:55 | `fcitx5-gtk` | IM GTK integration |
| 17/05 18:55 | `fcitx5-qt` | IM Qt integration |
| 17/05 18:55 | `fcitx5-unikey` | **Vietnamese input** |
| 17/05 21:50 | `openrgb` | RGB control (RAM/mobo) |
| 17/05 22:14 | `coolercontrol` | cooler control GUI |
| 17/05 22:14 | `liquidctl` | liquid cooler CLI |
| 17/05 22:38 | `rustup` | Rust toolchain (for `lianli` build) |
| 17/05 22:50 | `webkit2gtk-4.1` | `lianli` GUI dep |
| 17/05 22:55 | `evdi-dkms` | DisplayLink driver |
| 17/05 22:58 | `cmake` | build dep |
| 17/05 22:58 | `nasm` | build dep |
| 17/05 23:18 | `yay-bin` | second AUR helper |
| 17/05 23:21 | `lianli-linux-git` | **Lian Li fan/RGB/LCD** (AUR) |
| **18/05 05:56** | **`github-cli`** | **gh — installed today, just before this audit** |

Implicit additions (deps pulled in by user's explicit installs, marked `dep` in `pacman -Qi`):
- `pkgconf`, `hidapi`, `libusb`, `gtk3`, `librsvg`, `ffmpeg` → pulled in by `lianli-linux-git` / `webkit2gtk-4.1`.

User-mentioned but **not present**: `bun`, `gamemode`, `lib32-gamemode`, `ddcutil`, `ddcui` (all `MISSING` per `pacman -Qi`).

User-thought-missing but **actually present**: **`openrgb` ✓**, **`github-cli` ✓** (user memory had them in the "chưa cài" list — but both are installed; `github-cli` was added today at 05:56).

---

## C. CachyOS Hyprland-profile bootstrap (B1, ~135 pkgs — NOT user choice)

These came with the CachyOS Hyprland-edition installer, even though `pacman -Qe` marks them as "Explicitly installed" (the installer flags everything in its profile as explicit). User did **not** opt in or out of these individually.

Notable items in B1 that I previously over-attributed to the user in v4 §D2:

- `alacritty` (06:29:59) — comes with the CachyOS profile.
- `btop`, `glances` — CachyOS sysmon defaults.
- `micro`, `meld` — CachyOS editor defaults.
- `nvidia-utils`, `nvidia-settings`, `opencl-nvidia`, `libva-nvidia-driver`, `lib32-nvidia-utils`, `lib32-opencl-nvidia` — CachyOS NVIDIA profile (auto-detected GPU).
- `linux-cachyos`, `linux-cachyos-headers`, `linux-cachyos-lts*`, `linux-cachyos-*-nvidia-open` — CachyOS kernel set.
- `limine`, `limine-mkinitcpio-hook`, `limine-snapper-sync` — CachyOS default bootloader.
- `vlc-plugins-all`, `flatpak`, `paru`, `webkit2gtk-4.1` (initial copy)... wait — `webkit2gtk-4.1` shows 22:50 = B3 (user installed it for lianli, not CachyOS).

So **the only NVIDIA / Limine / kernel choices were made by the CachyOS installer profile, not the user**. Good to know for 8sync's documentation: a "stock HyDE+CachyOS Hyprland" host already has all of these.

---

## D. HyDE batch (B2)

Around `2026-05-17 06:49`–`06:52`, HyDE's `install.sh` pulled in its 65 packages plus a handful of CachyOS choices (e.g. `uv` 06:49:57, `flatpak` 06:52:17, `hyprquery-git` 06:49:52). Anything in `A ∩ H` from v4 sits here.

→ Treat `uv` and `flatpak` as **HyDE-batch present** (not user, not pure CachyOS).

---

## E. Implications for `8sync`

### E1. `gh` on a "stock" install
On a freshly bootstrapped CachyOS-Hyprland + HyDE box, **`gh` is absent**. The user installed it today. → 8sync `doctor` must treat `gh` missing as a hard error (no `ship` without it), and `setup` should explicitly suggest `sudo pacman -S github-cli` (or even auto-install it since it's in `extra` and is critical — but per user "chỉ cần gh, forge, agents harness", that means *user expects gh to be there as a prerequisite*).

Decision (proposed):
- `doctor`: gh missing → **error**.
- `setup`: include `github-cli` in the install list **if missing**, alongside `helix lazygit abduco`. → 4 packages total. User's "gh + forge + agents harness" minimum is enforced.

### E2. `rustup` — user-added, but only for `lianli`
Not strictly an 8sync prerequisite (we ship a pre-built `8sync` binary or build via `cargo install --path crates/cli`). `bootstrap.sh` must still detect `rustup` and skip if present (per v3).

### E3. `webkit2gtk-4.1` — irrelevant to 8sync
Stays. Just confirms the lianli toolchain.

### E4. Hardware extras (`openrgb`, `coolercontrol`, `liquidctl`, `evdi-dkms`, `lianli-linux-git`, `fcitx5*`)
Totally orthogonal to 8sync. Doctor should not even mention these.

### E5. Trash mentioned by user
`~/configs_dev/lian-li-linux` (2GB manual-build folder). Out of 8sync scope, but worth a one-line tip in `doctor` future enhancements: "you have an `lianli-linux-git` package installed, you can `rm -rf ~/configs_dev/lian-li-linux` if you built it manually".

---

## F. Final 8sync install footprint (locked)

Slim `8sync setup` will run exactly:

```
pacman -S --needed helix lazygit abduco github-cli   # 4 pkgs, all in cachyos-extra-znver4
sh -c "curl -fsSL https://forgecode.dev/cli | sh"     # only if `forge` not on PATH
# write 4 config files: 8sync/global.toml, 8sync/skills.toml, kitty/8sync.session,
#                       helix/config.toml (if absent)
# install 3 forge skills + 00-force-load.md
# (NO services, NO wallpaper, NO kitty.conf, NO fish, NO IM, NO docker, NO WARP)
```

`github-cli` is added because the audit proved it is **not** present on a stock HyDE host (only on this dev box because user pacman'd it today). Conditional install (`--needed`) keeps it idempotent.

Adjustments to plan v2 §5.1: pkg count `3 → 4`.

---

## G. Re-affirmed answers to gating questions

Given the data, here are recommended defaults; speak up if any need flipping.

1. **Image-routing trio (`shot`/`diff-img`/`pdf-img`)** → **keep**. `magick`, `convert`, `pdftoppm`, `pdftocairo`, `identify` all on PATH (deps via HyDE/CachyOS).
2. **Thin wrappers for `bg`/`look`** → **drop in this PR**; possible phase-2.
3. **`8sync .` 1-window + abduco by default** → **yes**. Reinforced by alacritty (no `kitty @` equivalent) + kitty remote control not enabled.
4. **`gh` missing handling** → **error** in doctor, and **install in setup** (`pacman -S --needed github-cli`).

---

## H. Branch state after this addendum

```
plans/2026-05-18-slim-down-for-hyde-v1.md                (initial)
plans/2026-05-18-slim-down-for-hyde-v2.md                (HyDE deeper audit, delegate strategy)
plans/2026-05-18-slim-down-for-hyde-v3-pkg-audit.md      (pacman/paru/yay/flatpak)
plans/2026-05-18-slim-down-for-hyde-v4-attribution.md    (A/A∩H/A−H/H−A buckets)
plans/2026-05-18-slim-down-for-hyde-v5-timestamp.md      (this file — install-date reconciliation)
```

v5 is the **final input** for execution. Plan v2 + v5 corrections supersede v3/v4 details where they conflict (specifically: v2 §5.1 footprint is now `helix lazygit abduco github-cli`, not `helix lazygit abduco`).
