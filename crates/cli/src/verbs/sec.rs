// `8sync sec` — unified VPN (Cloudflare WARP) + firewall (ufw) toggle.
//
//   8sync sec               status both
//   8sync sec on            connect WARP + enable ufw
//   8sync sec off           disconnect WARP + disable ufw
//   8sync sec toggle        flip both based on current state
//   8sync sec warp on|off|status
//   8sync sec ufw  on|off|status
//   8sync sec status        same as no-arg

use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync sec                  show status of WARP + ufw
          8sync sec on               connect WARP + enable ufw
          8sync sec off              disconnect WARP + disable ufw
          8sync sec toggle           flip both
          8sync sec warp on          only WARP on
          8sync sec ufw off          only ufw off
    "}
)]
pub struct Args {
    /// Action: (empty=status) | on | off | toggle | status | warp <sub> | ufw <sub>
    pub action: Option<String>,
    /// Sub-action for `warp` / `ufw` (on|off|status)
    pub sub: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    match a.action.as_deref() {
        None | Some("status") => { status(); Ok(()) }
        Some("on")     => { warp_on(); ufw_on(); Ok(()) }
        Some("off")    => { warp_off(); ufw_off(); Ok(()) }
        Some("toggle") => { toggle(); Ok(()) }
        Some("warp") => match a.sub.as_deref() {
            Some("on")  => { warp_on(); Ok(()) }
            Some("off") => { warp_off(); Ok(()) }
            None | Some("status") => { print_warp(); Ok(()) }
            Some(x) => { ui::warn(&format!("unknown sub: warp {}", x)); Ok(()) }
        },
        Some("ufw") => match a.sub.as_deref() {
            Some("on")  => { ufw_on(); Ok(()) }
            Some("off") => { ufw_off(); Ok(()) }
            None | Some("status") => { print_ufw(); Ok(()) }
            Some(x) => { ui::warn(&format!("unknown sub: ufw {}", x)); Ok(()) }
        },
        Some(other) => {
            ui::warn(&format!("unknown action `{}` — try `8sync sec -h`", other));
            Ok(())
        }
    }
}

pub fn status() {
    ui::header("8sync sec");
    print_ufw();
    print_warp();
}

/// Compact one-liner status (used by doctor).
pub fn status_quiet() {
    print_ufw();
    print_warp();
}

// ─── ufw ────────────────────────────────────────────────────────

fn ufw_present() -> bool { which::which("ufw").is_ok() }

fn ufw_active() -> bool {
    Command::new("sudo").args(["ufw", "status"]).output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("Status: active"))
        .unwrap_or(false)
}

fn print_ufw() {
    if !ufw_present() {
        ui::skip("ufw", "not installed");
        return;
    }
    if ufw_active() { ui::ok("ufw: active"); }
    else            { ui::info("ufw: inactive"); }
}

fn ufw_on() {
    if !ufw_present() { ui::warn("ufw missing"); return; }
    if ufw_active()   { ui::skip("ufw", "already active"); return; }
    let _ = Command::new("sudo").args(["systemctl", "enable", "--now", "ufw.service"]).status();
    let _ = Command::new("sudo").args(["ufw", "--force", "enable"]).status();
    ui::ok("ufw: enabled");
}

fn ufw_off() {
    if !ufw_present() { return; }
    if !ufw_active()  { ui::skip("ufw", "already inactive"); return; }
    let _ = Command::new("sudo").args(["ufw", "disable"]).status();
    ui::ok("ufw: disabled");
}

// ─── warp ───────────────────────────────────────────────────────

fn warp_present() -> bool { which::which("warp-cli").is_ok() }

fn warp_status_str() -> String {
    Command::new("warp-cli").arg("status").output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn warp_connected() -> bool {
    warp_status_str().contains("Connected")
}

fn print_warp() {
    if !warp_present() {
        ui::skip("warp", "not installed (8sync setup --profile warp)");
        return;
    }
    let s = warp_status_str();
    let first = s.lines().find(|l| l.contains("Status update")).unwrap_or_else(|| s.lines().next().unwrap_or(""));
    if warp_connected() { ui::ok(&format!("warp: {}", first.trim())); }
    else                { ui::info(&format!("warp: {}", first.trim())); }
}

fn warp_on() {
    if !warp_present() { ui::warn("warp-cli missing"); return; }
    if warp_connected() { ui::skip("warp", "already connected"); return; }
    let _ = Command::new("warp-cli").args(["--accept-tos", "connect"]).status();
    ui::ok("warp: connected");
}

fn warp_off() {
    if !warp_present() { return; }
    if !warp_connected() { ui::skip("warp", "already disconnected"); return; }
    let _ = Command::new("warp-cli").args(["--accept-tos", "disconnect"]).status();
    ui::ok("warp: disconnected");
}

fn toggle() {
    ui::header("8sync sec toggle");
    if ufw_active() { ufw_off(); } else { ufw_on(); }
    if warp_connected() { warp_off(); } else { warp_on(); }
}
