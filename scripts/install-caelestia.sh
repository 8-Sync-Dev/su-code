#!/usr/bin/env bash
# =============================================================================
# Caelestia Shell — one-shot installer for CachyOS / Arch
# =============================================================================
# Standalone. Does NOT need 8sync. Self-contained: this script + curl is enough.
#
# Usage — local file:
#     chmod +x install-caelestia.sh
#     ./install-caelestia.sh                           # interactive defaults (minimal)
#     ./install-caelestia.sh --with-apps --with-nvidia # full kit + NVIDIA driver
#     ./install-caelestia.sh --noconfirm --reboot      # unattended, reboot at end
#
# Usage — one-liner over curl (no clone):
#     bash <(curl -fsSL https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/scripts/install-caelestia.sh)
#     bash <(curl -fsSL https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/scripts/install-caelestia.sh) --noconfirm --reboot
#
# Reference: https://github.com/caelestia-dots/caelestia
# =============================================================================

set -e
trap 'echo -e "\n\033[1;31m[error]\033[0m line $LINENO failed — see log: $LOG"; exit 1' ERR

# ─── Flags ───────────────────────────────────────────────────────────────
NOCONFIRM=0     # --noconfirm  → no prompts to install.fish + pacman -Sy noconfirm
REBOOT=0        # --reboot     → systemctl reboot at the end (after 10s countdown)
WITH_APPS=0     # --with-apps  → also install firefox/libreoffice/codium/etc.
WITH_NVIDIA=0   # --with-nvidia → auto-detect + install NVIDIA driver
MINIMAL=1       # default — minimal install (just Caelestia stack)

while [[ $# -gt 0 ]]; do
  case "$1" in
    --noconfirm)   NOCONFIRM=1 ;;
    --reboot)      REBOOT=1 ;;
    --with-apps)   WITH_APPS=1; MINIMAL=0 ;;
    --with-nvidia) WITH_NVIDIA=1 ;;
    --full)        WITH_APPS=1; WITH_NVIDIA=1; NOCONFIRM=1; REBOOT=1; MINIMAL=0 ;;
    -h|--help)
      sed -n '2,17p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *) echo "unknown flag: $1 (try --help)"; exit 2 ;;
  esac
  shift
done

# ─── Colours ─────────────────────────────────────────────────────────────
G='\033[1;32m'; B='\033[1;34m'; Y='\033[1;33m'; R='\033[1;31m'; D='\033[0m'
step() { echo -e "\n${B}==>${D} ${G}$*${D}"; }
warn() { echo -e "${Y}[warn]${D} $*"; }
info() { echo -e "${B}·${D} $*"; }
ok()   { echo -e "${G}✓${D} $*"; }

# ─── Log file (always — for debug regardless of mode) ────────────────────
LOG_DIR="$HOME/.cache/8sync"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/caelestia-install-$(date +%s).log"
exec > >(tee -a "$LOG") 2>&1
ok "logging to $LOG"

# ─── Sanity ──────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]] && { echo -e "${R}don't run as root — script invokes sudo on its own${D}"; exit 1; }
command -v pacman >/dev/null 2>&1 || { echo -e "${R}pacman not found — this script is CachyOS / Arch only${D}"; exit 1; }
command -v curl   >/dev/null 2>&1 || { sudo pacman -S --needed --noconfirm curl; }
command -v git    >/dev/null 2>&1 || { sudo pacman -S --needed --noconfirm git; }

# ─── Banner ──────────────────────────────────────────────────────────────
cat <<'EOF'

╔══════════════════════════════════════════════════════════════╗
║          Caelestia Shell — CachyOS one-shot installer        ║
║          Hyprland + Quickshell + caelestia-shell             ║
╚══════════════════════════════════════════════════════════════╝
EOF

echo
info "mode:          $([[ $MINIMAL == 1 ]] && echo minimal || echo with-apps)"
info "with NVIDIA:   $([[ $WITH_NVIDIA == 1 ]] && echo yes || echo no)"
info "auto-yes:      $([[ $NOCONFIRM == 1 ]] && echo yes || echo no)"
info "reboot at end: $([[ $REBOOT == 1 ]] && echo yes || echo no)"
info "log file:      $LOG"
echo

if [[ $NOCONFIRM -ne 1 ]]; then
  read -r -p "Continue? (y/N) " -n 1 reply
  echo
  [[ ! "$reply" =~ ^[Yy]$ ]] && exit 0
fi

PAC_FLAGS=(--needed --noconfirm)

# ─── Phase 1: base + display manager ─────────────────────────────────────
step "[1/6] System update"
sudo pacman -Syu --noconfirm

step "[2/6] Display manager + Bluetooth"
sudo pacman -S "${PAC_FLAGS[@]}" sddm qt6-svg bluez bluez-utils networkmanager
sudo systemctl enable sddm.service bluetooth.service NetworkManager.service
info "sddm/bluetooth/NetworkManager enabled (start at next boot)"

# ─── Phase 2: build tools + Caelestia minimum deps ───────────────────────
# Direct deps for `caelestia/install.fish` and the upstream PKGBUILD's runtime:
#  - fish      → install.fish is a fish script
#  - sassc     → builds the Caelestia GTK theme during install
#  - hyprpicker, wl-clipboard, cliphist, inotify-tools, trash-cli → caelestia-cli
#  - foot, fastfetch, starship, btop, jq, eza → default Caelestia user apps
#  - adw-gtk-theme, papirus-icon-theme, ttf-jetbrains-mono-nerd → theme/font
#  - hyprland, xdg-desktop-portal-hyprland/gtk, polkit-gnome → Wayland session
#  - pipewire/pulse/alsa, wireplumber → audio
step "[3/6] Hyprland + Caelestia minimum runtime"
sudo pacman -S "${PAC_FLAGS[@]}" \
    hyprland xdg-desktop-portal-hyprland xdg-desktop-portal-gtk polkit-gnome \
    pipewire pipewire-pulse pipewire-alsa wireplumber \
    fish sassc \
    foot fastfetch starship btop jq eza \
    hyprpicker wl-clipboard cliphist inotify-tools trash-cli \
    adw-gtk-theme papirus-icon-theme ttf-jetbrains-mono-nerd \
    git curl wget perl gcc make cmake

# ─── Phase 3 (optional): user applications ───────────────────────────────
if [[ $WITH_APPS -eq 1 ]]; then
  step "[4/6] User applications (--with-apps)"
  sudo pacman -S "${PAC_FLAGS[@]}" \
      firefox \
      nautilus \
      libreoffice-fresh \
      gwenview spectacle kate ark okular \
      flatpak \
      vlc
else
  info "[4/6] skipping user apps (rerun with --with-apps to include firefox/libreoffice/etc.)"
fi

# ─── Phase 4 (optional): NVIDIA driver auto-detect ───────────────────────
if [[ $WITH_NVIDIA -eq 1 ]]; then
  step "[5/6] NVIDIA driver (auto-detect GPU family)"
  GPU=$(lspci -nn | grep -iE 'vga|3d|2d' | grep -i nvidia || true)
  if [[ -z "$GPU" ]]; then
    warn "no NVIDIA GPU detected — skipping driver step"
  else
    if   echo "$GPU" | grep -qiE "RTX 50|GB20[0-9]|Blackwell"; then DRV=nvidia-open-dkms
    elif echo "$GPU" | grep -qiE "RTX 40|AD10[0-9]|Ada";       then DRV=nvidia-open-dkms
    elif echo "$GPU" | grep -qiE "RTX 30|GA10[0-9]|Ampere";    then DRV=nvidia-open-dkms
    elif echo "$GPU" | grep -qiE "RTX 20|GTX 16|TU10[0-9]|Turing"; then DRV=nvidia-open-dkms
    else DRV=nvidia-dkms
    fi
    info "detected: $GPU"
    info "installing: $DRV + linux-cachyos-headers"
    sudo pacman -S "${PAC_FLAGS[@]}" linux-cachyos-headers "$DRV" \
        nvidia-utils lib32-nvidia-utils egl-wayland libva-nvidia-driver \
        vulkan-icd-loader lib32-vulkan-icd-loader
    # Enable DRM modesetting (Wayland requires it).
    echo "options nvidia-drm modeset=1 fbdev=1" | sudo tee /etc/modprobe.d/nvidia.conf >/dev/null
    if ! grep -q 'nvidia_drm' /etc/mkinitcpio.conf; then
      sudo sed -i 's/^MODULES=(\(.*\))/MODULES=(\1 nvidia nvidia_modeset nvidia_uvm nvidia_drm)/' /etc/mkinitcpio.conf
      sudo mkinitcpio -P
      info "patched /etc/mkinitcpio.conf + regenerated initramfs"
    fi
    warn "REBOOT required after NVIDIA driver install (handled at end if --reboot is set)"
  fi
else
  info "[5/6] skipping NVIDIA driver (rerun with --with-nvidia if you have an NVIDIA GPU)"
fi

# ─── Phase 5: AUR helper (paru) + Caelestia dots ─────────────────────────
step "[6/6] Caelestia dots (clone + install.fish)"

# Bootstrap paru if missing — caelestia-meta and a few deps live in AUR.
if ! command -v paru >/dev/null 2>&1 && ! command -v yay >/dev/null 2>&1; then
  info "no AUR helper found — bootstrapping paru from source"
  sudo pacman -S "${PAC_FLAGS[@]}" base-devel
  rm -rf /tmp/paru-bootstrap
  git clone https://aur.archlinux.org/paru.git /tmp/paru-bootstrap
  ( cd /tmp/paru-bootstrap && makepkg -si --noconfirm )
  rm -rf /tmp/paru-bootstrap
  ok "paru installed"
fi

AUR_HELPER=paru
command -v paru >/dev/null 2>&1 || AUR_HELPER=yay

CAELESTIA_DIR="$HOME/.local/share/caelestia"
if [[ -d "$CAELESTIA_DIR/.git" ]]; then
  info "Caelestia dots already cloned — updating"
  ( cd "$CAELESTIA_DIR" && git pull --ff-only )
else
  info "cloning Caelestia dots → $CAELESTIA_DIR"
  rm -rf "$CAELESTIA_DIR"
  git clone https://github.com/caelestia-dots/caelestia.git "$CAELESTIA_DIR"
fi

# IMPORTANT: install.fish symlinks configs FROM this directory. Do NOT move it.
warn "DO NOT delete or move $CAELESTIA_DIR after install — Hyprland will fail to start."

FISH_FLAGS=(--aur-helper="$AUR_HELPER")
[[ $NOCONFIRM -eq 1 ]] && FISH_FLAGS+=(--noconfirm)

info "running: fish ./install.fish ${FISH_FLAGS[*]}"
( cd "$CAELESTIA_DIR" && fish ./install.fish "${FISH_FLAGS[@]}" )

# ─── Done ────────────────────────────────────────────────────────────────
cat <<'EOF'

╔══════════════════════════════════════════════════════════════╗
║                  ✅  Caelestia installed                     ║
╚══════════════════════════════════════════════════════════════╝
EOF

echo
ok "next steps:"
echo "   1. reboot                          (sudo reboot)"
echo "   2. at SDDM login screen, pick 'Hyprland'"
echo "   3. Caelestia shell loads automatically"
echo
echo "shortcuts:"
echo "   Super         — app launcher"
echo "   Super + T     — terminal (foot)"
echo "   Super + W     — browser"
echo "   Super + I     — Caelestia control panel"
echo "   Ctrl+Alt+Del  — session menu"
echo
info "full log: $LOG"

if [[ $REBOOT -eq 1 ]]; then
  warn "rebooting in 10s — Ctrl-C to cancel"
  for i in 10 9 8 7 6 5 4 3 2 1; do printf "\r  ⏱  %2ds remaining " "$i"; sleep 1; done
  echo
  sudo systemctl reboot
elif [[ $NOCONFIRM -ne 1 ]]; then
  read -r -p "Reboot now? (y/N) " -n 1 reply
  echo
  [[ "$reply" =~ ^[Yy]$ ]] && sudo systemctl reboot
fi
