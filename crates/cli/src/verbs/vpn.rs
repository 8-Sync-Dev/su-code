// `8sync vpn` — SoftEther VPN Client + VPN Gate (University of Tsukuba academic
// public relays). Install the right SoftEther build, list relays, connect
// through one as a full tunnel, and cleanly restore.
//
//   8sync vpn                 status (service · account · egress IP/country)
//   8sync vpn install         softethervpn (AUR) + dhcpcd + enable client service
//   8sync vpn list [CC]       fetch VPN Gate relays (opt. 2-letter country), by score
//   8sync vpn on [CC|host|ip] connect best (or matching) relay + full-tunnel route
//   8sync vpn off             disconnect + restore routes/DNS
//   8sync vpn status          same as no-arg
//
// VPN Gate is an academic experiment: relays are volunteer-run and LOG traffic.
// Treat it as a study/learning tunnel, never for anything sensitive.
use std::process::Command;

use anyhow::{anyhow, Result};
use clap::Args as ClapArgs;

use crate::{env_detect, pkg, ui};

const API: &str = "https://www.vpngate.net/api/iphone/";
const HUB: &str = "VPNGATE";
const VUSER: &str = "vpn";
const VPASS: &str = "vpn";
const NIC: &str = "se"; // SoftEther virtual NIC -> OS device `vpn_se`
const ACCOUNT: &str = "vpngate";
const DEFAULT_PORT: u16 = 443;
const SERVICE: &str = "softethervpn-client.service";

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync vpn                 status: service · account · egress IP/country
      8sync vpn install         SoftEther engine (AUR) + Windows GUI via Wine + dhcpcd + enable service
      8sync vpn install --no-gui headless: engine + dhcpcd only, no Wine desktop GUI
      8sync vpn gui             open the Windows VPN Client Manager (Wine) — the region-switch plugin
      8sync vpn list            top VPN Gate relays by score
      8sync vpn list JP         relays in a country (2-letter code)
      8sync vpn on              connect the best relay + route all traffic through it
      8sync vpn on JP           connect the best relay in a country
      8sync vpn on 219.100.37.94 connect a specific relay IP (or host substring)
      8sync vpn off             disconnect + restore routes/DNS

    VPN Gate = academic public relays (U. Tsukuba). Volunteer servers that LOG
    traffic — a learning/experiment tunnel, not for anything sensitive.
"})]
pub struct Args {
    /// install | gui | list | on | off | status | help  (default: status)
    pub sub: Option<String>,
    /// on/list: 2-letter country (JP); on: also a relay host substring or IP
    pub arg: Option<String>,
    /// on: SoftEther SSL-VPN port (default 443)
    #[arg(long)]
    pub port: Option<u16>,
    /// install/on: don't prompt (auto-yes)
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// install: skip the Wine desktop GUI (headless: CLI-only engine)
    #[arg(long)]
    pub no_gui: bool,
}

pub fn run(a: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    match a.sub.as_deref() {
        None | Some("status") | Some("st") => {
            status();
            Ok(())
        }
        Some("install") => install(&env, a.yes, !a.no_gui),
        Some("gui") | Some("manager") | Some("ui") => gui(),
        Some("list") | Some("ls") => list(a.arg.as_deref()),
        Some("on") | Some("connect") | Some("up") => {
            on(&env, a.arg.as_deref(), a.port.unwrap_or(DEFAULT_PORT))
        }
        Some("off") | Some("disconnect") | Some("down") => off(&env),
        Some("help") | Some("-h") | Some("--help") => {
            print_help();
            Ok(())
        }
        Some(other) => {
            ui::warn(&format!("unknown subcommand: {other}"));
            ui::info("try: 8sync vpn [install | gui | list [CC] | on [CC|ip] | off | status]");
            Ok(())
        }
    }
}

// ─── install ────────────────────────────────────────────────────────────

fn install(env: &env_detect::Env, yes: bool, gui: bool) -> Result<()> {
    ui::header("8sync vpn — install SoftEther client + VPN Gate");
    if !env.is_cachyos_or_arch() {
        ui::warn(&format!("auto-install is Arch-only (detected: {})", env.os_id));
        ui::info("install SoftEther VPN Client + a DHCP client (dhcpcd) manually, then `8sync vpn on`");
        return Ok(());
    }
    let Some(helper) = env_detect::aur_helper() else {
        return Err(anyhow!("no AUR helper (paru/yay) on PATH — run `8sync setup` first"));
    };
    // softethervpn = the maintained RTM 4.44 build (vpnclient + vpncmd + client
    // service) — the native Linux ENGINE that does the actual tunnel. The -git
    // package is the unstable 5.x dev edition — not what we want.
    pkg::aur_install_safe(helper, &["softethervpn"], yes)?;
    // The SoftEther virtual NIC is a tap device that needs DHCP; this box may
    // have no DHCP client (NetworkManager only), so ensure dhcpcd (official repo).
    pkg::pacman_install_safe(&["dhcpcd"], yes)?;
    // SoftEther has NO native Linux GUI. The desktop app = the Windows VPN
    // Client Manager (vpncmgr.exe) run under Wine — this is what carries the
    // Windows-style VPN Gate region-switcher plugin. It remote-controls the
    // native Linux client above. Pulls `wine` (heavy) as a dependency.
    if gui {
        ui::step("desktop GUI: softethervpn-client-manager (Windows vpncmgr via Wine)");
        if let Err(e) = pkg::aur_install_safe(helper, &["softethervpn-client-manager"], yes) {
            ui::warn(&format!("GUI install failed ({e}) — engine still works via `8sync vpn on`"));
        }
    } else {
        ui::skip("desktop GUI", "--no-gui (headless: CLI-only)");
    }

    ui::step(&format!("enable + start {SERVICE}"));
    let ok = priv_cmd("systemctl")
        .args(["enable", "--now", SERVICE])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        ui::ok("SoftEther VPN client running");
    } else {
        ui::warn(&format!("could not start {SERVICE} — try: sudo systemctl enable --now {SERVICE}"));
    }
    ui::info("GUI (Windows-like, region plugin): `8sync vpn gui`  ·  CLI region-switch + routing: `8sync vpn on JP`");
    Ok(())
}

// ─── gui (Windows VPN Client Manager under Wine) ──────────────────────────

/// Launch the desktop GUI — the Windows VPN Client Manager (vpncmgr.exe) via
/// Wine, packaged as `softethervpn-client-manager`. This is where the
/// Windows-style VPN Gate region-switcher plugin lives. It drives the native
/// Linux client service; the reliable full-tunnel ROUTING on Linux is still
/// `8sync vpn on` (the Linux client can't rewrite the routing table itself).
fn gui() -> Result<()> {
    ui::header("8sync vpn — desktop GUI (VPN Client Manager)");
    let bin = if which::which("vpncmgr").is_ok() {
        "vpncmgr"
    } else {
        ui::warn("desktop GUI not installed (softethervpn-client-manager / vpncmgr)");
        ui::info("install it: `8sync vpn install`  (pulls the Windows manager + Wine)");
        return Ok(());
    };
    // Make sure the engine it controls is up.
    let _ = priv_cmd("systemctl").args(["start", SERVICE]).status();
    ui::info("launching Windows VPN Client Manager under Wine — connect to `localhost`, then use the VPN Gate plugin to pick a region.");
    ui::warn("GUI drives the connection; if traffic isn't routed through it, run `8sync vpn on` for the Linux routing.");
    // Detach so the terminal returns.
    match Command::new(bin).spawn() {
        Ok(_) => ui::ok("VPN Client Manager launched"),
        Err(e) => ui::err(&format!("could not launch {bin}: {e}")),
    }
    Ok(())
}

// ─── relays (VPN Gate CSV API) ────────────────────────────────────────────

struct Relay {
    host: String,
    ip: String,
    score: u64,
    ping: String,
    speed: u64, // bps
    cc: String,
    country: String,
}

fn fetch_relays() -> Result<Vec<Relay>> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "25", API])
        .output()
        .map_err(|e| anyhow!("curl not found: {e}"))?;
    if !out.status.success() {
        return Err(anyhow!("VPN Gate API unreachable"));
    }
    let body = String::from_utf8_lossy(&out.stdout);
    // CSV columns: HostName,IP,Score,Ping,Speed,CountryLong,CountryShort,...
    let mut v = Vec::new();
    for line in body.lines() {
        if line.starts_with('*') || line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() < 15 {
            continue;
        }
        let ip = f[1].trim();
        if ip.is_empty() {
            continue;
        }
        v.push(Relay {
            host: f[0].trim().to_string(),
            ip: ip.to_string(),
            score: f[2].trim().parse().unwrap_or(0),
            ping: f[3].trim().to_string(),
            speed: f[4].trim().parse().unwrap_or(0),
            country: f[5].trim().to_string(),
            cc: f[6].trim().to_string(),
        });
    }
    if v.is_empty() {
        return Err(anyhow!("no relays parsed from VPN Gate API"));
    }
    Ok(v)
}

fn list(cc: Option<&str>) -> Result<()> {
    ui::header("8sync vpn — VPN Gate relays");
    let mut relays = fetch_relays()?;
    if let Some(c) = cc {
        let c = c.to_uppercase();
        relays.retain(|r| r.cc.eq_ignore_ascii_case(&c));
        if relays.is_empty() {
            ui::warn(&format!("no relays in country {c}"));
            return Ok(());
        }
    }
    relays.sort_by(|a, b| b.score.cmp(&a.score));
    relays.truncate(20);
    println!(
        "  {:<3} {:<3} {:<22} {:<16} {:>5} {:>7} {:>11}",
        "#", "CC", "host", "ip", "ping", "Mbps", "score"
    );
    for (i, r) in relays.iter().enumerate() {
        println!(
            "  {:<3} {:<3} {:<22} {:<16} {:>5} {:>7.1} {:>11}",
            i + 1,
            r.cc,
            trunc(&r.host, 22),
            r.ip,
            r.ping,
            r.speed as f64 / 1e6,
            r.score
        );
    }
    ui::info("connect best: `8sync vpn on`  ·  by country: `8sync vpn on JP`");
    Ok(())
}

fn pick(sel: Option<&str>) -> Result<Relay> {
    let mut relays = fetch_relays()?;
    relays.sort_by(|a, b| b.score.cmp(&a.score));
    match sel {
        None => relays.into_iter().next().ok_or_else(|| anyhow!("no relays available")),
        Some(s) => {
            // Explicit IP.
            if s.contains('.') && s.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return Ok(relays.into_iter().find(|r| r.ip == s).unwrap_or(Relay {
                    host: s.to_string(),
                    ip: s.to_string(),
                    score: 0,
                    ping: "?".into(),
                    speed: 0,
                    cc: "??".into(),
                    country: "custom".into(),
                }));
            }
            // 2-letter country code -> best in that country.
            if s.len() == 2 {
                let c = s.to_uppercase();
                return relays
                    .into_iter()
                    .find(|r| r.cc.eq_ignore_ascii_case(&c))
                    .ok_or_else(|| anyhow!("no relay in country {c} (try `8sync vpn list {c}`)"));
            }
            // Host substring.
            relays
                .into_iter()
                .find(|r| r.host.contains(s))
                .ok_or_else(|| anyhow!("no relay matching '{s}'"))
        }
    }
}

// ─── connect / disconnect ─────────────────────────────────────────────────

fn on(env: &env_detect::Env, sel: Option<&str>, port: u16) -> Result<()> {
    ui::header("8sync vpn — connect VPN Gate");
    if which::which("vpncmd").is_err() {
        ui::warn("SoftEther VPN client not installed");
        ui::info("run: `8sync vpn install`");
        return Ok(());
    }
    let _ = priv_cmd("systemctl").args(["start", SERVICE]).status();

    let before = egress();
    let r = pick(sel)?;
    ui::step(&format!(
        "relay: {} {} ({})  score {}  ~{:.0} Mbps",
        r.cc,
        r.host,
        r.ip,
        r.score,
        r.speed as f64 / 1e6
    ));

    let (gw, dev) = default_uplink()
        .ok_or_else(|| anyhow!("no default route found — is this box online?"))?;

    // Virtual NIC (idempotent: NicCreate errors if it already exists).
    let _ = vpncmd(&["NicCreate", NIC]);
    let tap = detect_tap().unwrap_or_else(|| format!("vpn_{NIC}"));

    // (Re)create the VPN Gate account cleanly.
    let _ = vpncmd(&["AccountDisconnect", ACCOUNT]);
    let _ = vpncmd(&["AccountDelete", ACCOUNT]);
    let server = format!("/SERVER:{}:{}", r.ip, port);
    vpncmd(&[
        "AccountCreate",
        ACCOUNT,
        &server,
        &format!("/HUB:{HUB}"),
        &format!("/USERNAME:{VUSER}"),
        &format!("/NICNAME:{NIC}"),
    ])?;
    let _ = vpncmd(&["AccountPasswordSet", ACCOUNT, &format!("/PASSWORD:{VPASS}"), "/TYPE:standard"]);
    vpncmd(&["AccountConnect", ACCOUNT])?;

    if !wait_connected(12) {
        let _ = vpncmd(&["AccountDisconnect", ACCOUNT]);
        return Err(anyhow!(
            "VPN Gate handshake failed (relay busy/unreachable) — try another: `8sync vpn list`"
        ));
    }
    ui::ok("SoftEther session established");

    // Pin the relay's own route to the physical uplink so tunnel packets don't
    // loop through the tunnel (the classic SoftEther full-tunnel footgun).
    let cidr = format!("{}/32", r.ip);
    let _ = priv_cmd("ip").args(["route", "replace", &cidr, "via", &gw, "dev", &dev]).status();

    // DHCP the tap device.
    ui::step(&format!("DHCP {tap}"));
    let _ = priv_cmd("dhcpcd").args(["-q", "-4", "-t", "20", &tap]).status();
    let Some(addr) = iface_addr(&tap) else {
        rollback(&r.ip, &tap, None);
        return Err(anyhow!("{tap} got no DHCP lease from the relay — rolled back; try another relay"));
    };
    let tapgw = tap_gateway(&tap).or_else(|| subnet_dot1(&addr)).ok_or_else(|| {
        rollback(&r.ip, &tap, None);
        anyhow!("could not determine tunnel gateway — rolled back")
    })?;

    // Route everything through the tunnel (low metric wins over physical).
    let _ = priv_cmd("ip").args(["route", "del", "default", "dev", &tap]).status();
    let set_default = priv_cmd("ip")
        .args(["route", "add", "default", "via", &tapgw, "dev", &tap, "metric", "5"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !set_default {
        rollback(&r.ip, &tap, None);
        return Err(anyhow!("failed to set tunnel default route — rolled back"));
    }

    // Point DNS at a public resolver so lookups don't leak to the (now
    // unreachable) LAN router. Back up the original for restore.
    let resolv_bak = swap_dns(env);

    // Verify egress actually changed; roll back on failure.
    let after = egress();
    let changed = matches!((&before, &after), (_, Some((ip, _))) if Some(ip) != before.as_ref().map(|(i, _)| i));
    if after.is_none() || !changed {
        rollback(&r.ip, &tap, resolv_bak.as_deref());
        return Err(anyhow!("egress IP did not change through the tunnel — rolled back"));
    }

    save_state(env, &r.ip, &tap, &tapgw, resolv_bak.as_deref());
    let (eip, eloc) = after.unwrap();
    ui::ok(&format!("all traffic → VPN Gate {} {}  ·  egress {} ({})", r.cc, r.country, eip, eloc));
    ui::warn("VPN Gate relays are volunteer + LOGGED (academic use only) — nothing sensitive.");
    ui::info("disconnect + restore: `8sync vpn off`  ·  status: `8sync vpn`");
    Ok(())
}

fn off(env: &env_detect::Env) -> Result<()> {
    ui::header("8sync vpn — disconnect + restore");
    let st = load_state(env);
    // Nothing set up (no saved state, no tap, not connected) -> don't touch
    // routes or prompt for sudo.
    let connected = which::which("vpncmd").is_ok()
        && vpncmd(&["AccountList"]).map(|s| s.contains("Connected")).unwrap_or(false);
    if st.is_none() && detect_tap().is_none() && !connected {
        ui::info("nothing to disconnect — no active VPN Gate tunnel");
        return Ok(());
    }
    let tap = st.as_ref().map(|s| s.tap.clone()).unwrap_or_else(|| format!("vpn_{NIC}"));
    let server_ip = st.as_ref().map(|s| s.server_ip.clone());
    rollback(server_ip.as_deref().unwrap_or(""), &tap, st.as_ref().and_then(|s| s.resolv_bak.as_deref()));
    let _ = std::fs::remove_file(state_path(env));
    ui::ok("disconnected — routes/DNS restored to the physical uplink");
    Ok(())
}

/// Undo everything `on` set up. Safe to call partially (best-effort).
fn rollback(server_ip: &str, tap: &str, resolv_bak: Option<&str>) {
    let _ = vpncmd(&["AccountDisconnect", ACCOUNT]);
    let _ = priv_cmd("ip").args(["route", "del", "default", "dev", tap]).status();
    let _ = priv_cmd("dhcpcd").args(["-k", tap]).status();
    if !server_ip.is_empty() {
        let _ = priv_cmd("ip").args(["route", "del", &format!("{server_ip}/32")]).status();
    }
    if let Some(bak) = resolv_bak {
        let _ = priv_cmd("cp").args(["-a", "--remove-destination", bak, "/etc/resolv.conf"]).status();
        let _ = std::fs::remove_file(bak);
    }
}

// ─── status ───────────────────────────────────────────────────────────────

fn status() {
    ui::header("8sync vpn — status");
    if which::which("vpncmd").is_err() {
        ui::warn("SoftEther VPN client not installed");
        ui::info("install: `8sync vpn install`");
        return;
    }
    let active = Command::new("systemctl")
        .args(["is-active", "--quiet", SERVICE])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    println!("  {} {SERVICE}", if active { "v" } else { "·" });

    let connected = vpncmd(&["AccountList"]).map(|s| s.contains("Connected")).unwrap_or(false);
    println!("  {} VPN Gate account: {}", if connected { "v" } else { "·" }, if connected { "Connected" } else { "not connected" });

    match egress() {
        Some((ip, loc)) => println!("  · egress: {ip} ({loc})"),
        None => println!("  · egress: unknown"),
    }
    ui::info("connect: `8sync vpn on`  ·  disconnect: `8sync vpn off`  ·  relays: `8sync vpn list`");
}

// ─── vpncmd + network helpers ──────────────────────────────────────────────

/// Run a VPN Client management command non-interactively against the local
/// client service: `vpncmd localhost /CLIENT /CMD <args…>`.
fn vpncmd(args: &[&str]) -> Result<String> {
    let out = Command::new("vpncmd")
        .arg("localhost")
        .arg("/CLIENT")
        .arg("/CMD")
        .args(args)
        .output()
        .map_err(|e| anyhow!("vpncmd not found — run `8sync vpn install`: {e}"))?;
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        let tail = s.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
        return Err(anyhow!("vpncmd {}: {tail}", args.join(" ")));
    }
    Ok(s)
}

fn wait_connected(secs: u64) -> bool {
    for _ in 0..secs {
        if let Ok(s) = vpncmd(&["AccountList"]) {
            if s.contains("Connected") {
                return true;
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    false
}

/// Build a Command that runs as root (prefix `sudo` unless already root).
fn priv_cmd(bin: &str) -> Command {
    if is_root() {
        Command::new(bin)
    } else {
        let mut c = Command::new("sudo");
        c.arg(bin);
        c
    }
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

/// Lowest-metric default route -> (gateway, dev).
fn default_uplink() -> Option<(String, String)> {
    let out = Command::new("ip").args(["-4", "route", "show", "default"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let mut best: Option<(u64, String, String)> = None;
    for line in s.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        let gw = toks.iter().position(|t| *t == "via").and_then(|i| toks.get(i + 1)).map(|s| s.to_string());
        let dev = toks.iter().position(|t| *t == "dev").and_then(|i| toks.get(i + 1)).map(|s| s.to_string());
        let metric = toks
            .iter()
            .position(|t| *t == "metric")
            .and_then(|i| toks.get(i + 1))
            .and_then(|m| m.parse::<u64>().ok())
            .unwrap_or(0);
        // Skip our own tunnel default if present.
        if dev.as_deref().map(|d| d.starts_with("vpn_")).unwrap_or(false) {
            continue;
        }
        if let (Some(gw), Some(dev)) = (gw, dev) {
            if best.as_ref().map(|(m, _, _)| metric < *m).unwrap_or(true) {
                best = Some((metric, gw, dev));
            }
        }
    }
    best.map(|(_, gw, dev)| (gw, dev))
}

/// First `vpn_*` interface (the SoftEther tap).
fn detect_tap() -> Option<String> {
    let out = Command::new("ip").args(["-o", "link", "show"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        // "3: vpn_se: <...>"
        if let Some(name) = line.split(':').nth(1).map(|s| s.trim()) {
            let name = name.split('@').next().unwrap_or(name);
            if name.starts_with("vpn_") {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// IPv4 addr in CIDR form for an interface, e.g. "192.168.30.6/24".
fn iface_addr(dev: &str) -> Option<String> {
    let out = Command::new("ip").args(["-4", "-o", "addr", "show", "dev", dev]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.split_whitespace()
        .skip_while(|t| *t != "inet")
        .nth(1)
        .map(|c| c.to_string())
}

/// Default gateway installed on the tap (if dhcpcd added one).
fn tap_gateway(dev: &str) -> Option<String> {
    let out = Command::new("ip").args(["-4", "route", "show", "default", "dev", dev]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let toks: Vec<&str> = s.split_whitespace().collect();
    toks.iter().position(|t| *t == "via").and_then(|i| toks.get(i + 1)).map(|s| s.to_string())
}

/// Fallback SecureNAT gateway: the `.1` of the tap's /24 (VPN Gate convention).
fn subnet_dot1(cidr: &str) -> Option<String> {
    let ip = cidr.split('/').next()?;
    let mut octs: Vec<&str> = ip.split('.').collect();
    if octs.len() != 4 {
        return None;
    }
    octs[3] = "1";
    Some(octs.join("."))
}

/// Public egress (ip, country) via Cloudflare's IP-addressed trace so it works
/// even when DNS is mid-swap. Returns None if unreachable.
fn egress() -> Option<(String, String)> {
    let out = Command::new("curl")
        .args(["-fsS", "--max-time", "8", "https://1.1.1.1/cdn-cgi/trace"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let ip = s.lines().find_map(|l| l.strip_prefix("ip="))?.trim().to_string();
    let loc = s.lines().find_map(|l| l.strip_prefix("loc=")).unwrap_or("").trim().to_string();
    Some((ip, loc))
}

/// Back up /etc/resolv.conf and point DNS at 1.1.1.1. Returns the backup path.
fn swap_dns(env: &env_detect::Env) -> Option<String> {
    let bak = state_dir(env).join("resolv.conf.bak");
    std::fs::create_dir_all(state_dir(env)).ok()?;
    let bak_s = bak.to_string_lossy().to_string();
    // Preserve mode + symlink-ness so restore is exact.
    let saved = priv_cmd("cp")
        .args(["-a", "--remove-destination", "/etc/resolv.conf", &bak_s])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !saved {
        return None;
    }
    let wrote = priv_cmd("sh")
        .args(["-c", "rm -f /etc/resolv.conf && printf 'nameserver 1.1.1.1\\nnameserver 8.8.8.8\\n' > /etc/resolv.conf"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if wrote {
        Some(bak_s)
    } else {
        None
    }
}

// ─── tiny state (for a clean `off`) ─────────────────────────────────────────

struct State {
    server_ip: String,
    tap: String,
    resolv_bak: Option<String>,
}

fn state_dir(env: &env_detect::Env) -> std::path::PathBuf {
    env.home.join(".cache/8sync")
}
fn state_path(env: &env_detect::Env) -> std::path::PathBuf {
    state_dir(env).join("vpn.state")
}

fn save_state(env: &env_detect::Env, server_ip: &str, tap: &str, tapgw: &str, resolv_bak: Option<&str>) {
    let _ = std::fs::create_dir_all(state_dir(env));
    let body = format!(
        "server_ip={server_ip}\ntap={tap}\ntapgw={tapgw}\nresolv_bak={}\n",
        resolv_bak.unwrap_or("")
    );
    let _ = std::fs::write(state_path(env), body);
}

fn load_state(env: &env_detect::Env) -> Option<State> {
    let s = std::fs::read_to_string(state_path(env)).ok()?;
    let get = |k: &str| {
        s.lines()
            .find_map(|l| l.strip_prefix(&format!("{k}=")))
            .map(|v| v.trim().to_string())
    };
    Some(State {
        server_ip: get("server_ip").unwrap_or_default(),
        tap: get("tap").unwrap_or_else(|| format!("vpn_{NIC}")),
        resolv_bak: get("resolv_bak").filter(|v| !v.is_empty()),
    })
}

fn trunc(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n.saturating_sub(1)])
    }
}

fn print_help() {
    ui::header("8sync vpn — SoftEther client + VPN Gate");
    println!("{}", crate::brand::render("  8sync vpn                 status: service · account · egress IP/country"));
    println!("{}", crate::brand::render("  8sync vpn install         SoftEther engine (softethervpn) + Windows GUI via Wine + dhcpcd + service"));
    println!("{}", crate::brand::render("  8sync vpn gui             open the Windows VPN Client Manager (Wine) — Windows-style region-switch plugin"));
    println!("{}", crate::brand::render("  8sync vpn list [CC]       VPN Gate relays by score (optional 2-letter country)"));
    println!("{}", crate::brand::render("  8sync vpn on [CC|ip]      connect best (or matching) relay + full-tunnel route (the reliable Linux path)"));
    println!("{}", crate::brand::render("  8sync vpn off             disconnect + restore routes/DNS"));
    println!();
    println!("{}", crate::brand::render("  VPN Gate = academic public relays (U. Tsukuba). Volunteer + LOGGED — learning tunnel only."));
}
