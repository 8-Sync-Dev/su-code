# Plan v7 — Add `8sync sec` verb (warp + ufw unified toggle)

- **Branch:** `chore/slim-down-for-hyde`
- **Date:** 2026-05-18
- **Supersedes:** v6 §3 (warp profile post_install) — `connect` step moved out of profile into `sec` verb.
- **Reason:** install-time vs runtime separation. Profile `warp` only does **one-shot configuration**; daily on/off lives in `8sync sec`.

---

## 1. New verb `sec`

```
8sync sec [<action>] [<target>]

Actions:           Targets:
  status (default)   (omitted) = both ufw + warp
  on                 ufw
  off                warp
  toggle

Examples:
  8sync sec                  # status of both
  8sync sec on               # turn on both
  8sync sec off              # turn off both
  8sync sec warp on          # only warp
  8sync sec ufw  off         # only ufw
  8sync sec status warp      # detailed warp status
```

### Action semantics

| Action | ufw target | warp target |
|---|---|---|
| `status` | `sudo ufw status` (parse "active"/"inactive") | `warp-cli status` |
| `on` | `sudo systemctl enable --now ufw.service`<br>`sudo ufw --force enable` | `warp-cli --accept-tos connect` |
| `off` | `sudo ufw disable` (no service stop — keep autostart for next boot, just disable rules) | `warp-cli --accept-tos disconnect` |
| `toggle` | flip based on current status | flip based on current status |

Missing tool handling:
- If `ufw` binary missing → skip ufw silently with a hint.
- If `warp-cli` missing → skip warp silently with a hint (point to `8sync setup --profile warp`).

---

## 2. Updated `warp` profile (config-only)

```toml
# assets/profiles/warp.toml
name = "warp"
description = "Cloudflare WARP — install + one-time config (no auto-connect)"

[requires]
aur_helper = true

[packages]
aur = ["cloudflare-warp-bin"]

[post_install]
# One-shot config; daily on/off via `8sync sec`
commands = [
  "sudo systemctl enable warp-svc.service",
  "sudo systemctl start warp-svc.service",
  "warp-cli --accept-tos registration new || true",
  "warp-cli --accept-tos mode doh",
  "warp-cli --accept-tos tunnel protocol set MASQUE",
  "warp-cli --accept-tos dns families malware",
  # NO `warp-cli connect` here — user uses `8sync sec on` to connect
]
```

→ After this profile, daemon is running but tunnel is *configured but not connected*. User runs `8sync sec on` to actually start the VPN.

---

## 3. ufw handling

`ufw` is **not** a profile (it's part of CachyOS base, no install needed). The verb `8sync sec` handles its lifecycle:

- First-time enable: `8sync sec ufw on` runs both `systemctl enable --now ufw.service` and `sudo ufw --force enable` (adds default deny-incoming/allow-outgoing rules).
- Daily on/off: same command toggles.
- Doctor reports current state via `sudo ufw status` (or `systemctl is-active ufw.service` if no sudo).

---

## 4. File-by-file additions (vs v6 §5)

| File | Action |
|---|---|
| `crates/cli/src/verbs/sec.rs` | **new** — implements `status / on / off / toggle` × `(both / ufw / warp)`. |
| `crates/cli/src/verbs/mod.rs` | add `pub mod sec;`. |
| `crates/cli/src/main.rs` | add `Cmd::Sec(verbs::sec::Args)` + match arm; update `HELP_AFTER`. |
| `crates/cli/src/verbs/flow.rs` | add `sec` to workflow listing. |
| `crates/cli/src/verbs/root.rs` | add `sec` to cheatsheet. |
| `crates/cli/src/verbs/doctor.rs` | shell out to `8sync sec status` (or call `sec::status_quiet()` directly) to include both states in the doctor report. |
| `assets/profiles/warp.toml` | edit — drop `warp-cli connect` from post_install. |

---

## 5. CLI behavior

```
$ 8sync sec
  ufw:   inactive
  warp:  Disconnected (configured: mode=doh, MASQUE, dns=malware)

$ 8sync sec on
  ufw:   enable... OK (incoming=deny, outgoing=allow)
  warp:  connect... OK

$ 8sync sec status
  ufw:   active (Status: active)
  warp:  Connected (Default endpoint)

$ 8sync sec ufw off
  ufw:   disable... OK
  warp:  (unchanged) Connected

$ 8sync sec toggle
  ufw:   was inactive → enable... OK
  warp:  was Connected → disconnect... OK
```

---

## 6. Verb count

```
Lifecycle (5):  setup [profile <sub>]  up  doctor  flow  help
Vibe loop (5):  .  ai  end  ship  note
Workflow (2):   find  run
Forge (1):      skill
Image (3):      shot  diff-img  pdf-img
Security (1):   sec [on|off|status|toggle ufw|warp]
                                                       = 13 top-level
```

Sub-command groups: `setup profile <sub>` + `sec <sub>` = 2 nested.

---

## 7. Implementation steps (delta over v6 §7)

After v6 steps 1-9, add:

10. Create `verbs/sec.rs` with `Args { action: Option<Action>, target: Option<Target> }`, `run()`, and a `status_quiet() -> SecStatus` helper for `doctor`.
11. Wire into `main.rs`, `mod.rs`, `flow.rs`, `root.rs`.
12. Edit `assets/profiles/warp.toml` to drop `connect`.
13. Update `doctor.rs` to call `sec::status_quiet()`.
14. Build + smoke test (§8).
15. Open PR.

---

## 8. Verification (additions)

```bash
$B sec                                      # status both
$B sec on                                   # both on
$B sec status warp                          # warp detail
$B sec ufw off                              # ufw only off
$B sec toggle                               # flip both
$B doctor                                   # now shows ufw + warp status from sec helper
```

---

## 9. Risk / edge cases

| Risk | Mitigation |
|---|---|
| `sudo` prompt mid-command | All ufw subcalls use `sudo -n` first; if fail, print a clear "run again with sudo cached" message. |
| WARP service stopped → `warp-cli connect` fails | Pre-check `systemctl is-active warp-svc.service`; if down, try `sudo systemctl start` then retry. |
| User runs `sec on` without ever installing warp profile | Detect `which warp-cli`; if absent, print: "warp not installed — run `8sync setup --profile warp`". |
| User has alternate firewall (firewalld/iptables-nft) | Detect via `pacman -Qi firewalld 2>&1` — if active, print warning before touching ufw. |

---

## 10. Decision summary (locked)

- **`8sync setup`** = install + 1-time config only (idempotent).
- **`8sync sec`** = daily on/off control for the security pair (ufw + warp).
- ufw is **not** a profile — it's a CachyOS base tool managed at runtime via `sec`.
- warp profile only registers and configures; user explicitly turns it on with `sec on`.
