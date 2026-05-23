# Known issues & resolutions

Living log of HyDE / Caelestia / end-4 conflicts we've hit and the fix that
landed. **Read this before adding a new desktop-shell integration to `setup.rs`.**

| ID | First hit | Fixed in | Subsystem | Severity |
|----|-----------|----------|-----------|----------|
| [KI-001](#ki-001) | v0.5.1 | v0.6.1 | `--end4` keybind destruction | 🔴 critical |
| [KI-002](#ki-002) | v0.6.0 | v0.6.1 | HyDE waybar respawn (stop) | 🟡 medium |
| [KI-003](#ki-003) | v0.6.0 | v0.6.2 | HyDE waybar respawn (reboot) | 🟡 medium |

---

## KI-001 — `--end4` overwrote `hyprland.conf`, broke HyDE keybinds

**Symptom**
After `8sync setup --end4` on a HyDE system, all HyDE keybinds stop working.
`~/.config/hypr/hyprland.conf` was renamed to `hyprland.conf.old`; a new
`hyprland.lua` entry from end-4 took over. End-4 keybinds (different
mod-mappings) became active; HyDE's `keybindings.conf` was no longer sourced.

**Root cause**
We passed `--skip-backup` to upstream `./setup install`. End-4's installer
honoured it and overwrote the entry config without making a backup. HyDE's
own anti-overwrite marker (`$HYDE_HYPRLAND=set`) saved the file content by
renaming it to `.old`, but the active entry was now end-4's.

**Fix** — `crates/cli/src/verbs/setup.rs::end4_flags`
- Drop `--skip-backup` from all three tiers — backup is cheap insurance.
- `minimal` + `medium` pass `--skip-hyprland-entry` → user's existing entry
  config stays the active one. End-4 files are installed but inactive
  until the user opts in.
- `--end4=full` is **refused on HyDE systems** (bail with explicit
  recovery hint pointing at `mv hyprland.conf.old hyprland.conf`).
- New mode `--end4=overlay`: mirrors `--caelestia=hyde`. Launches end-4's
  Quickshell shell (`qs -c ii`) on top of HyDE keybinds without touching
  the entry config.

**Recovery for affected users**
```sh
cd ~/.config/hypr
TS=$(date +%s)
mkdir -p ~/.config/hypr.end4-stash.$TS
for f in hyprland.lua hypridle.conf hyprlock.conf animations.conf hyprland custom; do
  [ -e "$f" ] && mv "$f" ~/.config/hypr.end4-stash.$TS/
done
for f in hyprland.conf.old hypridle.conf.old hyprlock.conf.old; do
  new="${f%.old}"
  [ -f "$f" ] && [ ! -e "$new" ] && mv "$f" "$new"
done
hyprctl reload
```

---

## KI-002 — `pkill waybar` ineffective on HyDE (instant respawn)

**Symptom**
After `--end4=overlay` or `--caelestia=hyde`, end-4 Quickshell launches
successfully but waybar comes back within seconds — two bars overlap.

**Root cause**
HyDE runs waybar as a **transient systemd user service**
`hyde-Hyprland-bar.service`, not as a plain `exec-once` from
`hyprland.conf`. When `pkill -x waybar` kills the process, systemd's
`Restart=` directive (or the service's transient nature) spawns it again
immediately. The shell sentinel block in `userprefs.conf` runs
`exec-once = pkill -x waybar` once at Hyprland startup; that fires before
systemd is fully up, so by the time the user sees the desktop, waybar is
already back.

**Fix** — `crates/cli/src/verbs/setup.rs::apply_end4_overlay`
```sh
systemctl --user stop hyde-Hyprland-bar.service 2>/dev/null
pkill -x waybar || true
```
The `2>/dev/null` swallows "Unit not loaded" on non-HyDE systems so the
overlay still works on bare Hyprland.

---

## KI-003 — HyDE waybar respawns on reboot even after KI-002 fix

**Symptom**
After applying `--end4=overlay` and rebooting, waybar starts again on
next login. `8sync setup --end4=overlay` has to be re-run every session.

**Root cause**
KI-002 used `systemctl --user stop` — that only kills the running unit
for the current session. HyDE's autostart re-creates the transient unit
on next Hyprland session.

**Fix** — same file
```sh
systemctl --user mask waybar.service 2>/dev/null
```
`mask` survives reboot. Rollback (`--end4=rollback-overlay` /
`--caelestia=rollback`) does the symmetric `unmask` so the user can
restore HyDE's bar without manual intervention.

---

## Rules for future shell-integration work

When adding a new desktop-shell profile / overlay mode, **before merging**
verify each of these against a HyDE-base machine:

- [ ] `~/.config/hypr/hyprland.conf` is unchanged (or backed up with a
      restorable `.old` / `.bak.<ts>` sibling) after install.
- [ ] HyDE keybinds in `~/.config/hypr/keybindings.conf` still fire
      (test `$mainMod+T` for terminal, `$mainMod+/` for keybind hint).
- [ ] No two bars / shells fight for the same screen real estate. Use
      `pgrep -fa waybar` and `pgrep -fa "qs -c"` to verify exactly one
      shell is running.
- [ ] Conflicting systemd user services are **masked**, not just stopped
      — otherwise they come back on reboot.
- [ ] Rollback flag exists and is symmetric (every `mask` has an
      `unmask`, every sentinel-block insert has a sed-delete).
- [ ] Dry-run prints every command that will run, with no destructive
      side effects.

## Index — what 8sync touches on HyDE installs

| Path | When | Reversible by |
|------|------|---------------|
| `~/.config/hypr/userprefs.conf` | overlay modes inject sentinel block | rollback (sed -d) |
| `~/.config/hypr/hyprland.conf` | `--end4=full` only (REFUSED on HyDE) | manual `mv .old` |
| `systemctl --user waybar.service` | overlay modes mask it | rollback unmasks |
| `~/.config/hypr/*.bak.<ts>` | created by upstream on overwrite | manual restore |
| `~/.local/share/dots-hyprland/` | cloned by `--end4` | `--end4=rollback` |
| `~/.local/share/caelestia/` | cloned by `--caelestia=fresh` | manual `rm -rf` |
