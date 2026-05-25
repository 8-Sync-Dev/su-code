# 🛟 Stable baseline rollback

The current stable baseline is **v0.7.0+** with Caelestia as the only
supported desktop integration (Hyprland-based, runs as a parallel
SDDM session). To return any machine to a clean state:

```sh
# If Caelestia was applied via `8sync setup --caelestia`:
8sync setup --caelestia=rollback           # restore ~/.config/hypr from backup
8sync setup --caelestia=rollback --purge   # ...also pacman -Rns the Caelestia pkgs
```

Fresh install (no 8sync yet):

```sh
curl -fsSL -o ~/.local/bin/8sync \
  https://github.com/8-Sync-Dev/su-code/releases/latest/download/8sync-linux-x86_64 \
  && chmod +x ~/.local/bin/8sync \
  && 8sync setup
```

The rollback is idempotent: safe to re-run; safe on a machine that
never had Caelestia installed in the first place.

---

# Removed integrations (v0.7.0)

The following experimental integrations existed in v0.5.x → v0.6.x and
were **removed in v0.7.0** because they violated the "ưu tiên hiệu năng,
gọn nhẹ, không thừa" rule:

- `--end4` / `--end4=minimal|medium|full|rollback` (end-4/dots-hyprland)
- `--end4=overlay` / `--end4=rollback-overlay` (Quickshell-over-HyDE bridge)
- `--caelestia=hyde` (HyDE-additive overlay variant of Caelestia)
- `--reset-shells [--purge-packages]` (full desktop nuke)

Reason: the overlay paths tried to coexist two shells in the same Hyprland
session, which inevitably caused keybind conflicts and `.config/hypr` config
fights. The new model installs **one** Hyprland config (Caelestia's) and
lets multi-DE coexistence happen via SDDM session entries instead. Cleaner,
no overlay merge logic, no rollback needed unless the user explicitly asks.

If you need the old behaviour, pin to v0.6.11:

```sh
8sync up --to v0.6.11
```
