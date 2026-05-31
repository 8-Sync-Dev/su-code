// `8sync bt` — Bluetooth control + troubleshooting (bluez).
//
//   8sync bt              status: rfkill, service, controller power, paired count
//   8sync bt on           rfkill unblock + enable/start service + power on
//   8sync bt off          power off controller + stop service
//   8sync bt fix          the "nothing works" sequence: unblock rfkill, reload
//                         btusb, restart service, ensure AutoEnable, power on
//   8sync bt restart      restart bluetooth.service + power on
//   8sync bt status       same as no-arg

use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync bt                   status (rfkill / service / power / paired)
          8sync bt on                unblock + enable service + power on
          8sync bt off               power off + stop service
          8sync bt fix               full troubleshoot when bluetooth is dead
          8sync bt restart           restart service + power on
    "}
)]
pub struct Args {
    /// Action: (empty=status) | on | off | fix | restart | status
    pub action: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    if !present() {
        ui::warn("bluez not installed — `8sync setup --profile bluetooth` (bluez + bluez-utils)");
        return Ok(());
    }
    match a.action.as_deref() {
        None | Some("status") => { status(); Ok(()) }
        Some("on")      => { bt_on(); Ok(()) }
        Some("off")     => { bt_off(); Ok(()) }
        Some("fix")     => { bt_fix(); Ok(()) }
        Some("restart") => { bt_restart(); Ok(()) }
        Some(other) => {
            ui::warn(&format!("unknown action `{}` — try `8sync bt -h`", other));
            Ok(())
        }
    }
}

pub fn status() {
    ui::header("8sync bt");
    status_quiet();
}

/// Compact status (also used by `8sync doctor`).
pub fn status_quiet() {
    if !present() {
        ui::skip("bluetooth", "bluez not installed");
        return;
    }
    match rfkill_blocked() {
        Some(true)  => ui::warn("bluetooth: rfkill BLOCKED (run `8sync bt fix`)"),
        Some(false) => ui::ok("bluetooth: rfkill unblocked"),
        None        => {} // rfkill absent — skip the line
    }
    if svc_active() { ui::ok("bluetooth.service: active"); }
    else            { ui::warn("bluetooth.service: inactive (run `8sync bt on`)"); }
    match powered() {
        Some(true)  => ui::ok("controller: powered on"),
        Some(false) => ui::info("controller: powered off (run `8sync bt on`)"),
        None        => ui::info("controller: none detected"),
    }
    let n = paired_count();
    if n > 0 { ui::info(&format!("paired devices: {}", n)); }
}

// ─── detection ──────────────────────────────────────────────────

fn present() -> bool { which::which("bluetoothctl").is_ok() }
fn rfkill_present() -> bool { which::which("rfkill").is_ok() }

fn svc_active() -> bool {
    Command::new("systemctl").args(["is-active", "bluetooth.service"]).output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false)
}

/// `Some(true)` = soft/hard blocked, `Some(false)` = unblocked, `None` = no rfkill.
fn rfkill_blocked() -> Option<bool> {
    if !rfkill_present() { return None; }
    let out = Command::new("rfkill").args(["list", "bluetooth"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    if s.trim().is_empty() { return None; }
    Some(s.lines().any(|l| {
        let l = l.trim();
        (l.starts_with("Soft blocked:") || l.starts_with("Hard blocked:")) && l.ends_with("yes")
    }))
}

fn show() -> String {
    Command::new("bluetoothctl").arg("show").output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

/// `Some(true/false)` = controller present & powered/off, `None` = no controller.
fn powered() -> Option<bool> {
    let s = show();
    if s.trim().is_empty() || s.contains("No default controller") { return None; }
    Some(s.lines().any(|l| l.trim() == "Powered: yes"))
}

fn paired_count() -> usize {
    Command::new("bluetoothctl").args(["devices", "Paired"]).output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().filter(|l| l.starts_with("Device ")).count())
        .unwrap_or(0)
}

// ─── actions ────────────────────────────────────────────────────

fn rfkill_unblock() {
    if !rfkill_present() { return; }
    if rfkill_blocked() == Some(true) {
        let _ = Command::new("rfkill").args(["unblock", "bluetooth"]).status();
        ui::ok("rfkill: unblocked bluetooth");
    }
}

fn power_on() {
    let _ = Command::new("bluetoothctl").args(["power", "on"]).status();
}

fn bt_on() {
    ui::header("8sync bt on");
    rfkill_unblock();
    if svc_active() {
        ui::skip("bluetooth.service", "already active");
    } else {
        let _ = Command::new("sudo").args(["systemctl", "enable", "--now", "bluetooth.service"]).status();
        ui::ok("bluetooth.service: enabled + started");
    }
    power_on();
    ui::ok("controller: power on");
    status_quiet();
}

fn bt_off() {
    ui::header("8sync bt off");
    let _ = Command::new("bluetoothctl").args(["power", "off"]).status();
    ui::ok("controller: power off");
    let _ = Command::new("sudo").args(["systemctl", "stop", "bluetooth.service"]).status();
    ui::ok("bluetooth.service: stopped (still enabled — auto-starts next boot)");
}

fn bt_restart() {
    ui::header("8sync bt restart");
    let _ = Command::new("sudo").args(["systemctl", "restart", "bluetooth.service"]).status();
    ui::ok("bluetooth.service: restarted");
    power_on();
    status_quiet();
}

fn bt_fix() {
    ui::header("8sync bt fix");
    // 1. Unblock radio.
    rfkill_unblock();
    // 2. Reload the USB Bluetooth module (covers a wedged adapter after suspend).
    let _ = Command::new("sudo").args(["modprobe", "-r", "btusb"]).status();
    let _ = Command::new("sudo").args(["modprobe", "btusb"]).status();
    ui::ok("btusb: reloaded");
    // 3. Ensure the controller auto-powers on every boot (bluez [Policy] AutoEnable).
    ensure_autoenable();
    // 4. Restart the service so the new config + module take effect.
    let _ = Command::new("sudo").args(["systemctl", "restart", "bluetooth.service"]).status();
    ui::ok("bluetooth.service: restarted");
    // 5. Power the controller on for this session.
    power_on();
    ui::step("status after fix:");
    status_quiet();
}

/// Idempotently set `[Policy] AutoEnable=true` in /etc/bluetooth/main.conf so the
/// controller powers on at boot. Mirrors the `bluetooth` profile's post_install.
fn ensure_autoenable() {
    let script = r#"CONF=/etc/bluetooth/main.conf; \
[ -f "$CONF" ] || exit 0; \
if grep -qE "^[[:space:]]*#?[[:space:]]*AutoEnable[[:space:]]*=" "$CONF"; then \
  sed -i -E "s/^[[:space:]]*#?[[:space:]]*AutoEnable[[:space:]]*=.*/AutoEnable=true/" "$CONF"; \
elif grep -q "^\[Policy\]" "$CONF"; then \
  sed -i "/^\[Policy\]/a AutoEnable=true" "$CONF"; \
else \
  printf "\n[Policy]\nAutoEnable=true\n" >> "$CONF"; \
fi"#;
    let st = Command::new("sudo").args(["bash", "-c", script]).status();
    match st {
        Ok(s) if s.success() => ui::ok("AutoEnable=true ensured (/etc/bluetooth/main.conf)"),
        _ => ui::warn("could not update /etc/bluetooth/main.conf (sudo?)"),
    }
}
