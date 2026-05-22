# Plan addendum v4 — Attribution of installed packages (2026-05-18)

Supplements v2 + v3 with a clean **3-way diff** between HyDE upstream pkg lists, the live `pacman -Qe`, and (by elimination) the user's manual installs.

Goal: when `8sync` (or its doctor / setup) reasons about the environment, it must not confuse **HyDE-provided** vs **CachyOS-base** vs **user-added** packages. Only the user-added bucket is at risk of disappearing if the user reinstalls; the other two are guaranteed by the platform.

---

## A. Method

```
A = HyDE upstream (pkg_core.lst ∪ pkg_extra.lst, comments/`|`-suffix stripped)
H = installed-explicit on host (pacman -Qeq)

A ∩ H  → HyDE-provided (and kept by user)
A − H  → HyDE-listed but user opted out
H − A  → CachyOS base + user manual installs (further split below)
```

Counts: |A| = 84 (70 core + 19 extra), |H| = 234, |A ∩ H| = 65, |A − H| = 21, |H − A| = 169.

The `H − A` bucket is split into **C1 (CachyOS base / system bootstrap)** and **C2 (user-added)** by pattern + name knowledge.

---

## B. Bucket A ∩ H — HyDE-provided (65 pkgs, KEEP as-is, do not re-install)

`ark, awww, blueman, bluez, bluez-utils, brightnessctl, cliphist, code, dolphin, duf, dunst, fastfetch, ffmpegthumbs, firefox, fzf, grim, gst-plugin-pipewire, hypridle, hyprland, hyprlock, hyprpicker, hyprpolkitagent, hyprsunset, jq, kde-cli-tools, kitty, kvantum, kvantum-qt5, networkmanager, network-manager-applet, noto-fonts-emoji, nwg-displays, nwg-look, pacman-contrib, pamixer, pavucontrol, pipewire-alsa, pipewire-jack, pipewire-pulse, playerctl, qt5ct, qt5-graphicaleffects, qt5-imageformats, qt5-quickcontrols, qt5-quickcontrols2, qt5-wayland, qt6ct, qt6-wayland, rofi, satty, sddm, slurp, starship, udiskie, unzip, uwsm, vim, waybar, wireplumber, wl-clip-persist, wlogout, xdg-desktop-portal-hyprland, xdg-user-dirs`

These are the **expected baseline**. `8sync setup` must not list any of these. Doctor reports them as "OK / HyDE".

## C. Bucket A − H — HyDE-listed but user opted out (21 pkgs)

`bat, cava, ddcui, eza, gamemode, hyprquery, imagemagick, kimageformats, libnotify, mangohud, parallel, pipewire, pipewire-audio, python-requests, spicetify-cli, spotify, steam, swayosd-git, wf-recorder, wttrbar, xdg-desktop-portal-gtk`

Implications:

- `imagemagick`, `libnotify`, `parallel`, `python-requests`, `pipewire`, `pipewire-audio` are still **present as dependencies** (e.g. `magick`/`convert` on PATH) — so 8sync image-routing keeps working — but they are **not explicit**. If user later runs `pacman -Rns` on a parent, they could vanish. → 8sync `doctor` should test the **binary** (`magick`, `convert`, `pdftoppm`), not the package.
- The user **chose `hyprquery-git`** (AUR) instead of HyDE's listed `hyprquery`. → If 8sync ever uses hyprquery, it should locate the binary on PATH, not assume a package name.
- The user **opted out of `bat`/`eza`** which HyDE lists as optional zsh/fish extras. → 8sync slim setup should not silently install them; offer as `--with-shell-extras`.

## D. Bucket H − A — CachyOS base + user-added (169 pkgs)

### D1. CachyOS base / system bootstrap (~135 pkgs — NOT installed by user, NOT installed by 8sync)

System spine that comes with a CachyOS install (kernel, fs tools, network, fonts, build base, snapper, etc.):

`accountsservice, alsa-firmware, alsa-plugins, alsa-utils, amd-ucode, awesome-terminal-fonts, base, base-devel, bash-completion, bind, bluez-hid2hci, bluez-libs, bluez-obex, btrfs-assistant, btrfs-progs, cachyos-hooks, cachyos-keyring, cachyos-mirrorlist, cachyos-plymouth-bootanimation, cachyos-plymouth-theme, cachyos-rate-mirrors, cachyos-settings, cachyos-snapper-support, cachyos-v3-mirrorlist, cachyos-v4-mirrorlist, cantarell-fonts, chaotic-keyring, chaotic-mirrorlist, chwd, cpupower, cryptsetup, device-mapper, diffutils, dmidecode, dmraid, dnsmasq, dosfstools, e2fsprogs, efibootmgr, efitools, egl-wayland, ethtool, exfatprogs, f2fs-tools, ffmpegthumbnailer, fsarchiver, gst-libav, gst-plugin-va, gst-plugins-bad, gst-plugins-ugly, hdparm, hwdetect, hwinfo, inetutils, iwd, jfsutils, less, lib32-vulkan-icd-loader, lib32-vulkan-radeon, libdvdcss, libgsf, libopenraw, linux-firmware, logrotate, lsb-release, lsscsi, lvm2, man-db, man-pages, mdadm, mesa-utils, mkinitcpio, modemmanager, mtools, nano, nano-syntax-highlighting, netctl, networkmanager-openvpn, nfs-utils, nilfs-utils, noto-fonts, noto-fonts-cjk, nss-mdns, openssh, os-prober, pacman-contrib, paru, perl, pkgfile, plocate, plymouth, poppler-glib, power-profiles-daemon, pv, python, python-defusedxml, python-packaging, rebuild-detector, reflector, ripgrep, rsync, s-nail, sg3_utils, smartmontools, snapper, sof-firmware, sudo, sysfsutils, texinfo, ttf-bitstream-vera, ttf-dejavu, ttf-liberation, ttf-meslo-nerd, ttf-opensans, ufw, ufw-extras, unrar, upower, usb_modeswitch, usbutils, vulkan-icd-loader, vulkan-radeon, webkit2gtk-4.1, wget, which, wpa_supplicant, xf86-video-amdgpu, xfsprogs, xl2tpd, yay-bin, zsh`

→ 8sync can **rely on these existing** on any CachyOS install (e.g. `ripgrep`, `fd` (via `findutils`?), `git`, `jq`-via-HyDE, `unzip`, `python`, `wget`, `rsync`, `openssh` ...). Doctor lists them as informational only.

### D2. USER-added (≈34 pkgs — explicitly chosen by user)

Grouped by purpose:

| Group | Packages |
|---|---|
| **Hardware (NVIDIA + cooling/RGB)** | `nvidia-settings`, `nvidia-utils`, `opencl-nvidia`, `lib32-nvidia-utils`, `lib32-opencl-nvidia`, `libva-nvidia-driver`, `linux-cachyos-nvidia-open`, `linux-cachyos-lts-nvidia-open`, `coolercontrol`, `openrgb`, `liquidctl`, `evdi-dkms` |
| **Bootloader (replaces grub)** | `limine`, `limine-mkinitcpio-hook`, `limine-snapper-sync` |
| **Kernel choice** | `linux-cachyos`, `linux-cachyos-headers`, `linux-cachyos-lts`, `linux-cachyos-lts-headers` |
| **Terminals / editors** | `alacritty`, `micro`, `meld` |
| **Sysmon / tools** | `btop`, `glances` |
| **IM (Vietnamese)** | `fcitx5`, `fcitx5-configtool`, `fcitx5-gtk`, `fcitx5-qt`, `fcitx5-unikey` |
| **Apps** | `bitwarden`, `vlc-plugins-all`, `flatpak` |
| **AUR (1)** | `lianli-linux-git` (Lian Li control) |
| **HyDE replacement** | `hyprquery-git` (user picked `-git` over HyDE's `hyprquery`) |
| **Dev toolchain (critical for 8sync!)** | **`rustup`**, **`uv`**, **`cmake`**, **`nasm`**, **`github-cli`**, `git` (technically CachyOS), `shelly` |
| **Misc** | `pv` (CachyOS base), `cantarell-fonts` (CachyOS) |

### D3. Critical observations for 8sync

1. **`github-cli` (`gh`) is USER-added.** Neither HyDE nor CachyOS base provides it. `8sync ship` depends on `gh` for the `gh pr create` step. → `doctor` must check `gh` and treat its absence as a **blocker**, not a hint.

2. **`rustup` / `cargo` / `uv` are USER-added.** `bootstrap.sh` rustup-init is therefore not always redundant; it's only redundant **on hosts where the user already installed Rust**. The right behaviour: detect, then skip with a one-line note.

3. **`cmake`, `nasm`** — user-added build helpers. 8sync's Cargo build doesn't strictly need them (no cmake-sys / nasm crates in `Cargo.toml` as far as we know), so no impact.

4. **`flatpak` is USER-added** — irrelevant to 8sync, but `doctor` can mention it as informational.

5. **`alacritty` exists alongside `kitty`.** 8sync's `here.rs` hard-codes kitty. If we ever want to support alacritty, we'd lose `kitty @` remote-control entirely (no equivalent) and **must** use abduco. → reinforces v2's recommendation to default to "1 window + abduco" soft-mode.

6. **NVIDIA + DisplayLink + custom kernels + Limine bootloader** = this host is far from a "stock HyDE box". The slim-down plan is even more justified: do not assume system state, only assume the HyDE A-bucket.

---

## E. Plan refinements

Add to v2 §6 (file-by-file changes):

- **`doctor.rs`** — replace "package present?" checks with **binary present?** checks for HyDE-opt-out bucket (`magick`, `convert`, `pdftoppm`). Add a "critical user-adds" section that checks `gh`, `rustup` (only if 8sync ever needs to self-rebuild), `forge`, `helix`/`hx`. Treat `gh` missing as **error**, not warning.
- **`env_detect.rs`** — `is_hyde()` should test `pacman -Qq hyprland` **and** existence of `~/.config/hyde/wallbash` (both). Don't trust either alone; user might have nuked HyDE configs while keeping the packages.
- **`bootstrap.sh`** — guard with `command -v rustup >/dev/null && { echo "rustup present, skipping"; } || { run rustup-init; }`.
- **`setup.rs`** dry-run output — categorise packages it intends to install: "(missing on HyDE box)" vs "(missing in CachyOS base — should not happen)". The slim list is only `helix lazygit abduco` so this is mostly cosmetic.
- **`README.md`** — add a "Prerequisites" subsection clarifying:
  1. CachyOS (or Arch) + HyDE installed (gives you A-bucket).
  2. User must have `gh` installed (`pacman -S github-cli`) and authenticated (`gh auth login`).
  3. User must have `rustup` + a stable Rust toolchain if building from source (`pacman -S rustup && rustup default stable`).
  4. (Optional) `uv` for any HyDE-tracked Python venv work.

---

## F. Quick reference table — "do I need to install this for 8sync?"

| Tool | Bucket | 8sync action |
|---|---|---|
| `kitty` | A | use as-is, never overwrite config |
| `fzf`, `jq`, `unzip`, `vim`, `code`, `git` | A / CachyOS | use as-is |
| `starship`, `duf`, `dunst`, `rofi`, `waybar` | A | informational only |
| `ripgrep`, `python`, `wget`, `rsync`, `openssh` | CachyOS base | assume present |
| `magick`, `convert`, `pdftoppm`, `pdfinfo` | dep of HyDE-A | **check by binary, not pkg** |
| `gh` | **user** | **doctor must hard-check** |
| `forge` | curl-installed | install if missing |
| `rustup`, `cargo`, `uv` | user | bootstrap should detect & skip |
| `hyprquery` / `hyprquery-git` | varies | locate by binary if used |
| `helix`, `lazygit`, `abduco` | not present | slim setup installs these |
| `eza`, `bat`, `zoxide` | HyDE-extra (opt-out on this host) | optional, behind a flag |
| `fish`, `nodejs`, `docker`, `cloudflare-warp-bin` | absent + unwanted | drop entirely |

---

## G. Open questions still gating execution (no change from v2/v3)

1. Keep image-routing trio? — Default keep, all binaries present.
2. Add thin wrappers for `bg`/`look`? — Default no.
3. `8sync .` default to single-window + abduco? — Default yes (reinforced by alacritty presence + kitty remote-control off).

Plus one new question from this addendum:

4. **Treat `gh` absent as error or warning in doctor?** — Default: **error** (since `ship` cannot work without it). Setup should not auto-install `gh` (it's user territory), just instruct.
