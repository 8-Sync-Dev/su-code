// `8sync hz` — refresh-rate manager.
//
//   8sync hz                     report every output: current vs best available Hz
//   8sync hz max                 raise every output to its highest refresh
//   8sync hz 144                 set 144 Hz on every output that offers it
//   8sync hz max --output DP-4   one connector only
//   8sync hz max --dry-run       print the backend call, change nothing
//
// Two jobs, and the second is the one that actually matters on a fresh Fedora
// install:
//
//  1. **Set** the highest refresh the compositor offers. The RESOLUTION is never
//     changed — only the refresh field of the mode already in use — so a
//     "faster" screen can never come back smaller than the user left it.
//
//  2. **Explain** when the panel can do more than the driver offers. A 180 Hz
//     monitor stuck at 100 Hz is not a compositor setting: the kernel filters
//     out every mode that exceeds the negotiated DP link bandwidth, so the fast
//     mode is gone long before GNOME sees it. Reading the panel's own EDID
//     limits and comparing them against what the compositor advertises turns a
//     silent "100 Hz is your max" into a named cause and a fix.
use anyhow::{anyhow, bail, Result};
use clap::Args as ClapArgs;
use std::path::PathBuf;
use std::process::Command;

use crate::env_detect::{self, Family};
use crate::{platform, ui};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync hz                     report current vs highest available refresh
          8sync hz max                 raise every output to its highest refresh
          8sync hz 144                 set 144 Hz where the output offers it
          8sync hz max --output DP-4   only that connector
          8sync hz max --dry-run       show the call, change nothing

        NOTES
          Resolution is never changed — only the refresh rate of the mode in use.
          A panel that advertises more Hz than the driver exposes is diagnosed,
          not silently accepted: that gap is a driver/link problem, not a setting.
    "}
)]
pub struct Args {
    /// (empty) = report · `max` = highest available · a number = that refresh rate
    pub target: Option<String>,
    /// Limit to one connector (e.g. `DP-4`, `HDMI-A-1`, `eDP-1`)
    #[arg(long)]
    pub output: Option<String>,
    /// Print what would be applied, change nothing
    #[arg(long)]
    pub dry_run: bool,
}

// ─── model ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Mode {
    /// Backend-native mode handle (Mutter mode id, kscreen mode id, or `WxH@Hz`).
    pub id: String,
    pub w: u32,
    pub h: u32,
    pub hz: f64,
    pub current: bool,
}

#[derive(Debug, Clone)]
pub struct Output {
    /// Connector name as the kernel and the compositor both spell it (`DP-4`).
    pub name: String,
    /// Human label for the report; empty when the backend does not expose one.
    pub product: String,
    pub modes: Vec<Mode>,
}

impl Output {
    fn current(&self) -> Option<&Mode> {
        self.modes.iter().find(|m| m.current)
    }
}

/// What the panel itself claims it can do, straight out of its EDID. This is
/// independent of driver, cable and link state, which is exactly why it can be
/// compared against the compositor's mode list to prove a bottleneck exists.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Panel {
    /// `vmax` from the display-range-limits descriptor (0xFD).
    pub max_hz: Option<u32>,
    /// Max supported pixel clock in MHz from the same descriptor.
    pub max_pixclk_mhz: Option<u32>,
}

// ─── pure selection ──────────────────────────────────────────────────

/// Two refresh rates that differ by less than this are the same mode to a human
/// (`59.998874` is what a 60 Hz panel actually reports).
const HZ_EPS: f64 = 0.5;

/// Highest-refresh mode at the resolution currently in use.
///
/// Restricting to the current resolution is the whole safety story: the mode
/// list is sorted by neither field, and the fastest mode overall is routinely a
/// much smaller one (this box offers 2560x1440@144 while running 3440x1440@100).
/// Returning that would "fix" the refresh rate by shrinking the desktop.
pub fn best_at_current_res(modes: &[Mode]) -> Option<&Mode> {
    let cur = modes.iter().find(|m| m.current)?;
    modes
        .iter()
        .filter(|m| m.w == cur.w && m.h == cur.h)
        .max_by(|a, b| a.hz.total_cmp(&b.hz))
}

/// The mode matching `want` Hz at the current resolution, if the output has one.
pub fn at_rate(modes: &[Mode], want: f64) -> Option<&Mode> {
    let cur = modes.iter().find(|m| m.current)?;
    modes
        .iter()
        .filter(|m| m.w == cur.w && m.h == cur.h && (m.hz - want).abs() < HZ_EPS)
        .max_by(|a, b| a.hz.total_cmp(&b.hz))
}

/// Every refresh rate available at the current resolution, for the error that
/// says "144 is not on offer, here is what is".
pub fn rates_at_current_res(modes: &[Mode]) -> Vec<String> {
    let Some(cur) = modes.iter().find(|m| m.current) else { return Vec::new() };
    let mut hz: Vec<f64> = modes
        .iter()
        .filter(|m| m.w == cur.w && m.h == cur.h)
        .map(|m| m.hz)
        .collect();
    hz.sort_by(|a, b| b.total_cmp(a));
    hz.dedup_by(|a, b| (*a - *b).abs() < HZ_EPS);
    hz.iter().map(|h| format!("{h:.0}")).collect()
}

// ─── EDID ────────────────────────────────────────────────────────────

/// Pull the display-range limits out of an EDID blob.
///
/// Descriptor 0xFD carries the panel's own vertical-rate ceiling and maximum
/// pixel clock. It lives in the four 18-byte descriptor slots of every 128-byte
/// block, so every block is scanned: this monitor repeats the base block, and
/// plenty of others put the useful copy in an extension.
///
/// EDID 1.4 added a 255 Hz offset flag (byte 4) because the rate fields are one
/// byte each — without it a 300 Hz panel reads as 45 Hz.
pub fn parse_edid_limits(edid: &[u8]) -> Panel {
    let mut p = Panel::default();
    for block in edid.chunks_exact(128) {
        for off in [54usize, 72, 90, 108] {
            let d = &block[off..off + 18];
            // A descriptor (not a detailed timing) has a zero pixel clock.
            if d[0] != 0 || d[1] != 0 || d[3] != 0xFD {
                continue;
            }
            let mut vmax = d[6] as u32;
            // bits 1:0 — 0b10: max +255, 0b11: min and max +255.
            if d[4] & 0x03 >= 0x02 {
                vmax += 255;
            }
            if vmax > 0 {
                p.max_hz = Some(p.max_hz.map_or(vmax, |cur: u32| cur.max(vmax)));
            }
            // Byte 9 is the max pixel clock in units of 10 MHz; 0 means "not stated".
            let px = d[9] as u32 * 10;
            if px > 0 {
                p.max_pixclk_mhz = Some(p.max_pixclk_mhz.map_or(px, |cur: u32| cur.max(px)));
            }
        }
    }
    p
}

/// `/sys/class/drm/<card>-<connector>` for a connector name. The card prefix
/// varies per machine and per boot, so it is matched, never assumed.
fn drm_dir(connector: &str) -> Option<PathBuf> {
    let suffix = format!("-{connector}");
    std::fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(&suffix)))
}

fn panel_limits(connector: &str) -> Option<Panel> {
    let edid = std::fs::read(drm_dir(connector)?.join("edid")).ok()?;
    (!edid.is_empty()).then(|| parse_edid_limits(&edid))
}

/// The kernel driver bound to the GPU this connector hangs off, plus whether
/// that GPU is an NVIDIA part (PCI vendor `0x10de`).
///
/// Walks up from the connector to the first ancestor with a bound driver: the
/// connector's own `device` link points at the DRM *card*, and only the card's
/// parent is the PCI function that carries the `driver` symlink. The hop count
/// is not fixed across drivers, so it is searched rather than hardcoded.
fn drm_driver(connector: &str) -> Option<(String, bool)> {
    let mut dir = std::fs::canonicalize(drm_dir(connector)?).ok()?;
    for _ in 0..8 {
        if let Ok(link) = std::fs::read_link(dir.join("driver")) {
            let driver = link.file_name()?.to_string_lossy().into_owned();
            let nvidia = std::fs::read_to_string(dir.join("vendor"))
                .is_ok_and(|v| v.trim().eq_ignore_ascii_case("0x10de"));
            return Some((driver, nvidia));
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

// ─── backends ────────────────────────────────────────────────────────

trait Backend {
    fn name(&self) -> &'static str;
    fn outputs(&self) -> Result<Vec<Output>>;
    fn apply(&self, output: &str, mode: &Mode, dry: bool) -> Result<()>;
    /// Does a change made here survive a logout?
    fn persists(&self) -> bool;
    /// Extra line printed after a successful apply on backends that need the
    /// user to write the change into a config file themselves.
    fn persist_hint(&self, _output: &str, _mode: &Mode) -> Option<String> {
        None
    }
}

fn capture(bin: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| anyhow!("{bin}: {e}"))?;
    if !out.status.success() {
        bail!("{bin} {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

// ── GNOME / Mutter ──
//
// `org.gnome.Mutter.DisplayConfig` is the only supported way to change a mode
// under GNOME Wayland, and it also drives GNOME on X11. Read and write both go
// through `busctl`: it is part of systemd (so it is present wherever GNOME is),
// and `--json=short` gives a parseable reply instead of GVariant prose.

const MUTTER_DEST: &str = "org.gnome.Mutter.DisplayConfig";
const MUTTER_PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const MUTTER_IFACE: &str = "org.gnome.Mutter.DisplayConfig";

/// One entry of Mutter's `logical_monitors`, kept whole so a mode change can be
/// replayed without disturbing position, scale, rotation or which head is
/// primary. `ApplyMonitorsConfig` replaces the ENTIRE layout, so anything not
/// echoed back is silently reset.
struct LogicalMonitor {
    x: i64,
    y: i64,
    scale: f64,
    transform: u64,
    primary: bool,
    /// (connector, current mode id)
    monitors: Vec<(String, String)>,
}

struct Mutter;

impl Mutter {
    fn available() -> bool {
        which::which("busctl").is_ok()
            && Command::new("busctl")
                .args(["--user", "--json=short", "call", MUTTER_DEST, MUTTER_PATH, MUTTER_IFACE, "GetCurrentState"])
                .output()
                .is_ok_and(|o| o.status.success())
    }

    fn state() -> Result<(u32, Vec<Output>, Vec<LogicalMonitor>)> {
        let raw = capture(
            "busctl",
            &["--user", "--json=short", "call", MUTTER_DEST, MUTTER_PATH, MUTTER_IFACE, "GetCurrentState"],
        )?;
        let v: serde_json::Value = serde_json::from_str(&raw)?;
        let data = v.get("data").and_then(|d| d.as_array()).ok_or_else(|| anyhow!("mutter: no reply body"))?;
        let serial = data.first().and_then(|s| s.as_u64()).ok_or_else(|| anyhow!("mutter: no serial"))? as u32;

        let mut outs = Vec::new();
        for m in data.get(1).and_then(|m| m.as_array()).into_iter().flatten() {
            let Some(m) = m.as_array() else { continue };
            let id = m.first().and_then(|i| i.as_array());
            let name = id.and_then(|i| i.first()).and_then(|s| s.as_str()).unwrap_or_default().to_string();
            if name.is_empty() {
                continue;
            }
            let product = id
                .and_then(|i| i.get(2))
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string();
            let mut modes = Vec::new();
            for md in m.get(1).and_then(|x| x.as_array()).into_iter().flatten() {
                let Some(md) = md.as_array() else { continue };
                let (Some(mid), Some(w), Some(h), Some(hz)) = (
                    md.first().and_then(|s| s.as_str()),
                    md.get(1).and_then(|s| s.as_u64()),
                    md.get(2).and_then(|s| s.as_u64()),
                    md.get(3).and_then(|s| s.as_f64()),
                ) else {
                    continue;
                };
                let current = md
                    .get(6)
                    .and_then(|p| p.get("is-current"))
                    .and_then(|p| p.get("data"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);
                modes.push(Mode { id: mid.to_string(), w: w as u32, h: h as u32, hz, current });
            }
            outs.push(Output { name, product, modes });
        }

        let mut logicals = Vec::new();
        for l in data.get(2).and_then(|l| l.as_array()).into_iter().flatten() {
            let Some(l) = l.as_array() else { continue };
            let monitors = l
                .get(5)
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_array()?.first()?.as_str().map(str::to_string))
                        .filter_map(|conn| {
                            let mode = outs
                                .iter()
                                .find(|o| o.name == conn)
                                .and_then(Output::current)
                                .map(|m| m.id.clone())?;
                            Some((conn, mode))
                        })
                        .collect()
                })
                .unwrap_or_default();
            logicals.push(LogicalMonitor {
                x: l.first().and_then(|v| v.as_i64()).unwrap_or(0),
                y: l.get(1).and_then(|v| v.as_i64()).unwrap_or(0),
                scale: l.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0),
                transform: l.get(3).and_then(|v| v.as_u64()).unwrap_or(0),
                primary: l.get(4).and_then(|v| v.as_bool()).unwrap_or(false),
                monitors,
            });
        }
        Ok((serial, outs, logicals))
    }

    /// Flatten the layout into busctl's positional encoding of
    /// `uua(iiduba(ssa{sv}))a{sv}`: every array is a count followed by its
    /// elements, every struct is its fields in order, and `a{sv}` we always
    /// send empty (`0`).
    fn apply_args(serial: u32, method: u32, logicals: &[LogicalMonitor]) -> Vec<String> {
        let mut a = vec![
            "uua(iiduba(ssa{sv}))a{sv}".to_string(),
            serial.to_string(),
            method.to_string(),
            logicals.len().to_string(),
        ];
        for l in logicals {
            a.push(l.x.to_string());
            a.push(l.y.to_string());
            // Shortest round-trip formatting: Mutter matches the scale against
            // its own supported list, and 1.3333333730697632 truncated to a few
            // decimals is not on it.
            a.push(l.scale.to_string());
            a.push(l.transform.to_string());
            a.push(l.primary.to_string());
            a.push(l.monitors.len().to_string());
            for (conn, mode) in &l.monitors {
                a.push(conn.clone());
                a.push(mode.clone());
                a.push("0".to_string()); // per-monitor properties
            }
        }
        a.push("0".to_string()); // top-level properties
        a
    }

    fn call_apply(args: &[String]) -> Result<()> {
        let mut argv = vec!["--user", "call", MUTTER_DEST, MUTTER_PATH, MUTTER_IFACE, "ApplyMonitorsConfig"];
        argv.extend(args.iter().map(String::as_str));
        capture("busctl", &argv).map(|_| ())
    }
}

/// Mutter's `ApplyMonitorsConfig` method argument: validate only, or persist.
const MUTTER_VERIFY: u32 = 0;
const MUTTER_PERSISTENT: u32 = 2;

impl Backend for Mutter {
    fn name(&self) -> &'static str {
        "gnome/mutter"
    }

    fn outputs(&self) -> Result<Vec<Output>> {
        Ok(Self::state()?.1)
    }

    fn apply(&self, output: &str, mode: &Mode, dry: bool) -> Result<()> {
        let (serial, _, mut logicals) = Self::state()?;
        let mut touched = false;
        for l in &mut logicals {
            for (conn, mid) in &mut l.monitors {
                if conn == output {
                    *mid = mode.id.clone();
                    touched = true;
                }
            }
        }
        if !touched {
            bail!("{output} is not part of the active layout");
        }
        let args = Mutter::apply_args(serial, MUTTER_PERSISTENT, &logicals);
        if dry {
            ui::info(&format!("busctl call … ApplyMonitorsConfig {}", args.join(" ")));
            return Ok(());
        }
        // VERIFY first: Mutter checks the whole layout (bandwidth, scale,
        // overlap) and refuses without touching the screen, so a rejected
        // config never costs the user a blank display.
        Mutter::call_apply(&Mutter::apply_args(serial, MUTTER_VERIFY, &logicals))
            .map_err(|e| anyhow!("mutter rejected the config: {e}"))?;
        Mutter::call_apply(&args)
    }

    fn persists(&self) -> bool {
        true
    }
}

// ── Hyprland ──

struct Hyprland;

impl Hyprland {
    fn available() -> bool {
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() && which::which("hyprctl").is_ok()
    }
}

impl Backend for Hyprland {
    fn name(&self) -> &'static str {
        "hyprland"
    }

    fn outputs(&self) -> Result<Vec<Output>> {
        let v: serde_json::Value = serde_json::from_str(&capture("hyprctl", &["-j", "monitors"])?)?;
        let mut outs = Vec::new();
        for m in v.as_array().into_iter().flatten() {
            let name = m.get("name").and_then(|s| s.as_str()).unwrap_or_default().to_string();
            if name.is_empty() {
                continue;
            }
            let (cw, ch, chz) = (
                m.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                m.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                m.get("refreshRate").and_then(|v| v.as_f64()).unwrap_or(0.0),
            );
            let mut modes: Vec<Mode> = m
                .get("availableModes")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|s| parse_wxh_at_hz(s.as_str()?))
                .collect();
            // `availableModes` does not mark the active one, and the active mode
            // is not guaranteed to appear in it verbatim, so it is injected.
            match modes
                .iter_mut()
                .find(|m| m.w == cw && m.h == ch && (m.hz - chz).abs() < HZ_EPS)
            {
                Some(m) => m.current = true,
                None => modes.push(Mode {
                    id: format!("{cw}x{ch}@{chz:.2}"),
                    w: cw,
                    h: ch,
                    hz: chz,
                    current: true,
                }),
            }
            outs.push(Output {
                name,
                product: m.get("description").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
                modes,
            });
        }
        Ok(outs)
    }

    fn apply(&self, output: &str, mode: &Mode, dry: bool) -> Result<()> {
        let v: serde_json::Value = serde_json::from_str(&capture("hyprctl", &["-j", "monitors"])?)?;
        let cur = v
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m.get("name").and_then(|s| s.as_str()) == Some(output))
            .ok_or_else(|| anyhow!("{output} is not an active monitor"))?;
        let spec = format!(
            "{output},{}x{}@{:.2},{}x{},{}",
            mode.w,
            mode.h,
            mode.hz,
            cur.get("x").and_then(|v| v.as_i64()).unwrap_or(0),
            cur.get("y").and_then(|v| v.as_i64()).unwrap_or(0),
            cur.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0),
        );
        if dry {
            ui::info(&format!("hyprctl keyword monitor {spec}"));
            return Ok(());
        }
        capture("hyprctl", &["keyword", "monitor", &spec]).map(|_| ())
    }

    fn persists(&self) -> bool {
        false
    }

    fn persist_hint(&self, output: &str, mode: &Mode) -> Option<String> {
        Some(format!(
            "make it stick: add `monitor={output},{}x{}@{:.2},auto,auto` to hyprland.conf",
            mode.w, mode.h, mode.hz
        ))
    }
}

// ── KDE / kscreen ──

struct Kscreen;

impl Kscreen {
    fn available() -> bool {
        which::which("kscreen-doctor").is_ok()
    }
}

impl Backend for Kscreen {
    fn name(&self) -> &'static str {
        "kde/kscreen"
    }

    fn outputs(&self) -> Result<Vec<Output>> {
        let v: serde_json::Value = serde_json::from_str(&capture("kscreen-doctor", &["-j"])?)?;
        let mut outs = Vec::new();
        for o in v.get("outputs").and_then(|o| o.as_array()).into_iter().flatten() {
            if !o.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false) {
                continue;
            }
            let name = o.get("name").and_then(|s| s.as_str()).unwrap_or_default().to_string();
            if name.is_empty() {
                continue;
            }
            let cur_id = o.get("currentModeId").and_then(|s| s.as_str()).unwrap_or_default();
            let modes = o
                .get("modes")
                .and_then(|m| m.as_array())
                .into_iter()
                .flatten()
                .filter_map(|m| {
                    let id = m.get("id")?.as_str()?.to_string();
                    let size = m.get("size")?;
                    Some(Mode {
                        current: id == cur_id,
                        w: size.get("width")?.as_u64()? as u32,
                        h: size.get("height")?.as_u64()? as u32,
                        hz: m.get("refreshRate")?.as_f64()?,
                        id,
                    })
                })
                .collect();
            outs.push(Output {
                name,
                product: o.get("model").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
                modes,
            });
        }
        Ok(outs)
    }

    fn apply(&self, output: &str, mode: &Mode, dry: bool) -> Result<()> {
        let spec = format!("output.{output}.mode.{}", mode.id);
        if dry {
            ui::info(&format!("kscreen-doctor {spec}"));
            return Ok(());
        }
        capture("kscreen-doctor", &[&spec]).map(|_| ())
    }

    fn persists(&self) -> bool {
        true
    }
}

// ── X11 / xrandr ──

struct Xrandr;

impl Xrandr {
    fn available() -> bool {
        which::which("xrandr").is_ok() && std::env::var_os("DISPLAY").is_some()
    }
}

/// Parse `xrandr` output into connectors and modes.
///
/// The format is positional, not tabular: a connector line is flush-left, its
/// modes are indented as `  3440x1440   100.00*+  59.98`, and each rate may
/// carry `*` (current) and/or `+` (preferred). Kept pure so the parser is
/// testable without an X server.
pub fn parse_xrandr(text: &str) -> Vec<Output> {
    let mut outs: Vec<Output> = Vec::new();
    for line in text.lines() {
        if !line.starts_with(char::is_whitespace) {
            let mut f = line.split_whitespace();
            let (Some(name), Some(state)) = (f.next(), f.next()) else { continue };
            if state == "connected" {
                outs.push(Output { name: name.to_string(), product: String::new(), modes: Vec::new() });
            }
            continue;
        }
        let Some(out) = outs.last_mut() else { continue };
        let mut f = line.split_whitespace();
        let Some(res) = f.next() else { continue };
        let Some((w, h)) = res.split_once('x') else { continue };
        let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) else { continue };
        for tok in f {
            let current = tok.contains('*');
            let hz: String = tok.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            let Ok(hz) = hz.parse::<f64>() else { continue };
            out.modes.push(Mode { id: format!("{w}x{h}"), w, h, hz, current });
        }
    }
    outs
}

impl Backend for Xrandr {
    fn name(&self) -> &'static str {
        "x11/xrandr"
    }

    fn outputs(&self) -> Result<Vec<Output>> {
        Ok(parse_xrandr(&capture("xrandr", &["--query"])?))
    }

    fn apply(&self, output: &str, mode: &Mode, dry: bool) -> Result<()> {
        let res = format!("{}x{}", mode.w, mode.h);
        let rate = format!("{:.2}", mode.hz);
        let args = ["--output", output, "--mode", &res, "--rate", &rate];
        if dry {
            ui::info(&format!("xrandr {}", args.join(" ")));
            return Ok(());
        }
        capture("xrandr", &args).map(|_| ())
    }

    fn persists(&self) -> bool {
        false
    }

    fn persist_hint(&self, output: &str, mode: &Mode) -> Option<String> {
        Some(format!(
            "X11 forgets this at logout — put `xrandr --output {output} --mode {}x{} --rate {:.2}` in your session autostart",
            mode.w, mode.h, mode.hz
        ))
    }
}

/// `3440x1440@100.00Hz` → a mode. Hyprland's `availableModes` format.
fn parse_wxh_at_hz(s: &str) -> Option<Mode> {
    let (res, rate) = s.split_once('@')?;
    let (w, h) = res.split_once('x')?;
    let hz: String = rate.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    Some(Mode {
        id: s.to_string(),
        w: w.trim().parse().ok()?,
        h: h.trim().parse().ok()?,
        hz: hz.parse().ok()?,
        current: false,
    })
}

/// Pick the backend that actually owns mode-setting in this session.
///
/// Order matters: a Hyprland or KDE session may still have a stray `DISPLAY`
/// set, and on GNOME the Mutter bus is authoritative for both Wayland and X11 —
/// so the compositor-specific probes run before the generic X11 fallback.
fn select_backend() -> Option<Box<dyn Backend>> {
    if Hyprland::available() {
        return Some(Box::new(Hyprland));
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    if desktop.contains("kde") && Kscreen::available() {
        return Some(Box::new(Kscreen));
    }
    if Mutter::available() {
        return Some(Box::new(Mutter));
    }
    if Kscreen::available() {
        return Some(Box::new(Kscreen));
    }
    if Xrandr::available() {
        return Some(Box::new(Xrandr));
    }
    None
}

// ─── report ──────────────────────────────────────────────────────────

fn describe(o: &Output) -> String {
    let label = if o.product.is_empty() { String::new() } else { format!("  {}", o.product) };
    match (o.current(), best_at_current_res(&o.modes)) {
        (Some(cur), Some(best)) => format!(
            "{}{label}  {}x{} @ {:.2} Hz  (best available {:.2} Hz)",
            o.name, cur.w, cur.h, cur.hz, best.hz
        ),
        (Some(cur), None) => format!("{}{label}  {}x{} @ {:.2} Hz", o.name, cur.w, cur.h, cur.hz),
        _ => format!("{}{label}  (no active mode)", o.name),
    }
}

/// Print the panel-vs-driver gap, if there is one.
///
/// Returns true when a bottleneck was reported, so callers can avoid claiming
/// "already at max" for a screen that is demonstrably capable of more.
fn report_bottleneck(o: &Output) -> bool {
    let Some(best) = best_at_current_res(&o.modes) else { return false };
    let Some(panel) = panel_limits(&o.name) else { return false };
    let Some(panel_hz) = panel.max_hz else { return false };
    if (panel_hz as f64) <= best.hz + 1.0 {
        return false;
    }
    ui::warn(&format!(
        "{}: the panel reports up to {} Hz, the driver only offers {:.0} Hz",
        o.name, panel_hz, best.hz
    ));
    match drm_driver(&o.name) {
        Some((drv, true)) if drv == "nouveau" => {
            ui::info("  nouveau on an NVIDIA GPU: no DSC and no high-bitrate link, so the kernel");
            ui::info("  filters out every mode above the DisplayPort HBR2 ceiling before GNOME sees it.");
            ui::info(&format!("  fix: {}", nvidia_fix()));
        }
        Some((drv, _)) => {
            ui::info(&format!("  driver in use: {drv}"));
            ui::info("  the fast mode needs more link bandwidth than the current cable/port negotiated —");
            ui::info("  use a DP 1.4 (HBR3) cable and set the monitor's OSD DisplayPort version to 1.4 / DSC on.");
        }
        None => {}
    }
    if let Some(px) = panel.max_pixclk_mhz {
        ui::info(&format!("  panel EDID: max {panel_hz} Hz, max pixel clock {px} MHz"));
    }
    true
}

/// The driver-install command for this distro. The nvidia profile already knows
/// both families (RPM Fusion `akmod-nvidia` on Fedora, `nvidia-open-dkms` on
/// Arch), so the advice points at it rather than duplicating package names.
fn nvidia_fix() -> String {
    let extra = match env_detect::distro_family() {
        Family::Fedora => " (RPM Fusion akmod-nvidia)",
        Family::Arch => " (nvidia-open-dkms)",
        Family::Other => "",
    };
    format!("8sync setup --profile nvidia{extra}, then reboot")
}

// ─── run ─────────────────────────────────────────────────────────────

enum Target {
    Status,
    Max,
    Rate(f64),
}

fn parse_target(s: Option<&str>) -> Result<Target> {
    match s {
        None | Some("status") => Ok(Target::Status),
        Some("max") => Ok(Target::Max),
        Some(other) => other
            .trim_end_matches("hz")
            .trim_end_matches("Hz")
            .parse::<f64>()
            .map(Target::Rate)
            .map_err(|_| anyhow!("`{other}` is neither `max`, `status`, nor a refresh rate — see `8sync hz -h`")),
    }
}

pub fn run(a: Args) -> Result<()> {
    if !platform::require_linux("hz", "display modes come from Wayland/X11 and DRM") {
        return Ok(());
    }
    let target = parse_target(a.target.as_deref())?;
    let Some(backend) = select_backend() else {
        ui::err("no supported display backend found");
        ui::info("needs one of: GNOME (busctl + Mutter), Hyprland (hyprctl), KDE (kscreen-doctor), X11 (xrandr)");
        return Ok(());
    };
    let all = backend.outputs()?;
    let outs: Vec<&Output> = match &a.output {
        Some(want) => all.iter().filter(|o| o.name.eq_ignore_ascii_case(want)).collect(),
        None => all.iter().collect(),
    };
    if outs.is_empty() {
        match &a.output {
            Some(want) => ui::warn(&format!(
                "no active output named `{want}` — found: {}",
                all.iter().map(|o| o.name.as_str()).collect::<Vec<_>>().join(", ")
            )),
            None => ui::warn("no active output"),
        }
        return Ok(());
    }

    ui::header(&format!("8sync hz  ({})", backend.name()));
    for o in outs {
        match target {
            Target::Status => {
                ui::info(&describe(o));
                report_bottleneck(o);
            }
            Target::Max => {
                let Some(best) = best_at_current_res(&o.modes) else {
                    ui::skip(&o.name, "no mode list");
                    continue;
                };
                set_mode(&*backend, o, best, a.dry_run)?;
            }
            Target::Rate(want) => match at_rate(&o.modes, want) {
                Some(m) => set_mode(&*backend, o, m, a.dry_run)?,
                None => ui::warn(&format!(
                    "{}: no {want:.0} Hz at {}  — available: {} Hz",
                    o.name,
                    o.current().map_or("this resolution".into(), |c| format!("{}x{}", c.w, c.h)),
                    rates_at_current_res(&o.modes).join(", ")
                )),
            },
        }
    }
    Ok(())
}

fn set_mode(backend: &dyn Backend, o: &Output, mode: &Mode, dry: bool) -> Result<()> {
    let already = o.current().is_some_and(|c| (c.hz - mode.hz).abs() < HZ_EPS);
    if already && !dry {
        ui::skip(&o.name, &format!("already at {:.2} Hz", mode.hz));
        report_bottleneck(o);
        return Ok(());
    }
    backend.apply(&o.name, mode, dry)?;
    if dry {
        return Ok(());
    }
    ui::ok(&format!("{}: {}x{} @ {:.2} Hz", o.name, mode.w, mode.h, mode.hz));
    if !backend.persists() {
        if let Some(hint) = backend.persist_hint(&o.name, mode) {
            ui::info(&format!("  {hint}"));
        }
    }
    Ok(())
}

/// One line per output for `8sync doctor`, silent when nothing is wrong.
pub fn status_quiet() {
    let Some(backend) = select_backend() else { return };
    let Ok(outs) = backend.outputs() else { return };
    for o in &outs {
        let Some(cur) = o.current() else { continue };
        let best = best_at_current_res(&o.modes).map_or(cur.hz, |m| m.hz);
        let capped = panel_limits(&o.name)
            .and_then(|p| p.max_hz)
            .is_some_and(|hz| (hz as f64) > best + 1.0);
        let line = format!("display {}: {}x{} @ {:.0} Hz", o.name, cur.w, cur.h, cur.hz);
        if capped || best > cur.hz + HZ_EPS {
            ui::warn(&format!("{line} — `8sync hz` for why"));
        } else {
            ui::ok(&line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(w: u32, h: u32, hz: f64, current: bool) -> Mode {
        Mode { id: format!("{w}x{h}@{hz}"), w, h, hz, current }
    }

    /// The fastest mode overall is routinely a SMALLER one. Raising the refresh
    /// rate must never shrink the desktop.
    #[test]
    fn max_never_changes_resolution() {
        let modes = vec![
            m(3440, 1440, 100.0, true),
            m(3440, 1440, 60.0, false),
            m(2560, 1440, 144.0, false),
        ];
        let best = best_at_current_res(&modes).unwrap();
        assert_eq!((best.w, best.h), (3440, 1440));
        assert_eq!(best.hz, 100.0);
    }

    #[test]
    fn no_current_mode_means_no_pick() {
        let modes = vec![m(1920, 1080, 60.0, false)];
        assert!(best_at_current_res(&modes).is_none());
        assert!(at_rate(&modes, 60.0).is_none());
    }

    /// Panels report 59.998874, not 60. An exact compare would never match.
    #[test]
    fn requested_rate_tolerates_the_panels_own_rounding() {
        let modes = vec![m(3440, 1440, 100.0, true), m(3440, 1440, 59.998874, false)];
        assert_eq!(at_rate(&modes, 60.0).unwrap().hz, 59.998874);
        assert!(at_rate(&modes, 144.0).is_none());
        assert_eq!(rates_at_current_res(&modes), vec!["100", "60"]);
    }

    #[test]
    fn target_parsing() {
        assert!(matches!(parse_target(None).unwrap(), Target::Status));
        assert!(matches!(parse_target(Some("max")).unwrap(), Target::Max));
        assert!(matches!(parse_target(Some("144")).unwrap(), Target::Rate(r) if r == 144.0));
        assert!(matches!(parse_target(Some("144Hz")).unwrap(), Target::Rate(r) if r == 144.0));
        assert!(parse_target(Some("fast")).is_err());
    }

    /// Real EDID range descriptor from the MSI MAG 342CQR E2 that motivated this
    /// verb: 48–180 Hz, 970 MHz max pixel clock. The panel says 180; the nouveau
    /// driver only ever offered 100.
    #[test]
    fn edid_range_limits_are_read() {
        let mut edid = [0u8; 128];
        edid[54..72].copy_from_slice(&[
            0x00, 0x00, 0x00, 0xFD, 0x00, 48, 180, 255, 255, 97, 0x00, 0x0A, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20,
        ]);
        let p = parse_edid_limits(&edid);
        assert_eq!(p.max_hz, Some(180));
        assert_eq!(p.max_pixclk_mhz, Some(970));
    }

    /// EDID 1.4 stores the vertical rate in ONE byte, so anything above 255 Hz
    /// sets the offset flag. Ignoring it turns a 300 Hz panel into a 45 Hz one.
    #[test]
    fn edid_255hz_offset_flag_is_applied() {
        let mut edid = [0u8; 128];
        edid[54..72].copy_from_slice(&[
            0x00, 0x00, 0x00, 0xFD, 0x02, 48, 45, 255, 255, 100, 0x00, 0x0A, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20,
        ]);
        assert_eq!(parse_edid_limits(&edid).max_hz, Some(300));
    }

    #[test]
    fn edid_without_a_range_descriptor_says_nothing() {
        assert_eq!(parse_edid_limits(&[0u8; 128]), Panel::default());
        assert_eq!(parse_edid_limits(&[]), Panel::default());
    }

    /// `ApplyMonitorsConfig` replaces the WHOLE layout. Position, scale,
    /// rotation and the primary flag must survive a refresh-rate change, and the
    /// busctl encoding is count-prefixed at every array.
    #[test]
    fn mutter_apply_preserves_the_layout() {
        let logicals = vec![
            LogicalMonitor {
                x: 0,
                y: 0,
                scale: 1.3333333730697632,
                transform: 1,
                primary: true,
                monitors: vec![("DP-4".into(), "3440x1440@180.000".into())],
            },
            LogicalMonitor {
                x: 3440,
                y: 0,
                scale: 1.0,
                transform: 0,
                primary: false,
                monitors: vec![("HDMI-1".into(), "1920x1080@60.000".into())],
            },
        ];
        assert_eq!(
            Mutter::apply_args(7, MUTTER_PERSISTENT, &logicals),
            [
                "uua(iiduba(ssa{sv}))a{sv}",
                "7",
                "2",
                "2",
                "0", "0", "1.3333333730697632", "1", "true", "1", "DP-4", "3440x1440@180.000", "0",
                "3440", "0", "1", "0", "false", "1", "HDMI-1", "1920x1080@60.000", "0",
                "0",
            ]
        );
    }

    #[test]
    fn hyprland_mode_strings_parse() {
        let m = parse_wxh_at_hz("3440x1440@99.98Hz").unwrap();
        assert_eq!((m.w, m.h), (3440, 1440));
        assert_eq!(m.hz, 99.98);
        assert!(parse_wxh_at_hz("3440x1440").is_none());
    }

    /// xrandr marks the active rate with `*` and the preferred one with `+`; the
    /// two can appear on the same token.
    #[test]
    fn xrandr_output_parses() {
        let text = "\
Screen 0: minimum 320 x 200, current 3440 x 1440, maximum 16384 x 16384
DP-4 connected primary 3440x1440+0+0 (normal left inverted right x axis y axis) 800mm x 335mm
   3440x1440    100.00*+  59.98    50.00
   2560x1440    144.00
HDMI-1 disconnected (normal left inverted right x axis y axis)
";
        let outs = parse_xrandr(text);
        assert_eq!(outs.len(), 1, "a disconnected connector is not an output");
        assert_eq!(outs[0].name, "DP-4");
        assert_eq!(outs[0].modes.len(), 4);
        let best = best_at_current_res(&outs[0].modes).unwrap();
        assert_eq!((best.w, best.h, best.hz), (3440, 1440, 100.0));
    }
}
