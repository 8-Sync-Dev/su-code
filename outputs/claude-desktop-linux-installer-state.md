# STATE — Claude Desktop Linux installer  (DONE)

## Goal (met)
One-command cross-distro installer for the latest official Claude Desktop (Linux beta).
Built, tested, installed on this CachyOS host, GUI launch verified. Deliverable: `claude-cowork-linux/`.

## Deliverables
- claude-cowork-linux/claude-desktop-linux.sh  (installer, v1.0.0)
- claude-cowork-linux/tests/smoke.sh           (post-install launch test)
- claude-cowork-linux/README.md, CHANGELOG.md

## What it does
- Debian/Ubuntu -> official apt repo (auto-updates). Arch/other -> download latest .deb,
  verify SHA-256, extract (ar+bsdtar), install to prefix.
- Rootless ~/.local by default (no sudo); --system for /usr.
- Latest version resolved LIVE from apt Packages index (sort -V), pinned fallback 1.18286.2.
- Launcher auto-picks Chromium sandbox: setuid chrome-sandbox -> userns -> --no-sandbox fallback.
- .desktop Exec rewritten to absolute launcher path; hicolor icons; caches refreshed.
- Subcmds: install|update|uninstall|status|--print-latest|--help|--version.

## Verified (CachyOS x86_64, KDE/Wayland)
- Installed Claude Desktop 1.18286.2 rootless; all shared libs resolve; 15s GUI launch survived.
- status/update idempotent; uninstall clean (no residue; unrelated `claude` CLI untouched);
  full uninstall->reinstall->relaunch lifecycle passed.

## Key facts (learned)
- chrome-sandbox ships setuid-root (4755) as fallback; primary = namespace sandbox (needs userns=1, present here).
  Rootless install loses setuid bit -> userns covers it, no --no-sandbox needed.
- .deb layout: usr/lib/claude-desktop/* (Electron), usr/bin/claude-desktop symlink,
  usr/share/applications/claude-desktop.desktop, usr/share/icons/hicolor/{16..256}.
- bundled libffmpeg.so etc. load via $ORIGIN rpath -> "not found" in ldd only when files not colocated.

## Bugs fixed during build
- download_deb ran in $()-subshell -> LATEST_VERSION lost (VERSION file "unknown"). Now sets global DEB_PATH, called directly.
- `trap ... RETURN` referencing local $stage fired on a later return -> unbound under set -u. Removed; explicit rm -rf "$stage".

## Status: all 6 engine tasks done. Independent reviewer subagent running. Not committed (no git repo here; push not requested).

## Review hardening applied (2026-07-08)
Independent reviewer found 4 real issues, all fixed + re-verified:
- HIGH: `--system` deb path aborted for non-root -> now `exec sudo -E bash "$0" "$ORIG_ARGS"`.
- MED: status/update missed apt installs -> installed_version() falls back to `dpkg-query -W`.
- MED: local `--deb` wrote VERSION="unknown" -> control_version() parses control Version (payload lowercase `version`=42.5.1 is ELECTRON ver, NOT package ver).
- LOW: `--method`/`--deb` missing-value + invalid `--method` now die with clear message.
Re-verified: syntax, --print-latest, arg validation, local-deb version=1.18286.2, normal reinstall, smoke (15s GUI), status/update. All engine gates green. --system sudo re-exec is correct-by-inspection (needs interactive password; not runtime-tested here).
