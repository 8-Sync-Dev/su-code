// `8sync lcd` — drive Lian Li fan/AIO screens from the shell.
//
//   8sync lcd                       daemon status + every screen it found
//   8sync lcd photo.png             show it on every screen
//   8sync lcd loop.gif --fps 24     animation
//   8sync lcd '#ff0055'             solid colour
//   8sync lcd off                   blank them
//   8sync lcd bright 60             brightness, 0-100
//   8sync lcd gui                   open the GUI with the WebKit/Wayland workaround
//
// The `lian-li-linux` daemon already owns the reverse-engineered USB protocol
// and runs as a user service; its GUI is a Tauri app that needs a working
// WebKit GPU path, which is exactly what is missing on a machine still running
// the nouveau driver ("Error 71 (Protocol error) dispatching to Wayland
// display"). Everything the screens need, though, is a line of JSON on
// `$XDG_RUNTIME_DIR/lianli-daemon.sock` — so this verb talks to the daemon
// directly and skips the GUI entirely.
//
// Protocol (upstream `lianli_shared::ipc`): newline-delimited JSON,
// `{"method": "...", "params": {...}}` in, `{"status": "ok"|"error", ...}` back.
use anyhow::{anyhow, bail, Context, Result};
use clap::Args as ClapArgs;
use std::path::Path;
use std::process::Command;

use crate::{platform, ui};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync lcd                       daemon status + every screen found
          8sync lcd wallpaper.png         show it on every screen
          8sync lcd clip.mp4 --fps 30     video / GIF (looped by the daemon)
          8sync lcd '#ff0055'             solid colour
          8sync lcd off                   blank every screen
          8sync lcd bright 60             brightness 0-100
          8sync lcd photo.png --device 2  only screen #2 (index, id, or a unique part of it)
          8sync lcd gui                   open the GUI with the WebKit/Wayland workaround

        NOTES
          Needs the `lian-li-linux` daemon (Arch: `lianli-linux-git`, Fedora COPR:
          `sgtaziz/lian-li-linux`) — `8sync setup --profile hardware-lianli`.
          The setting is written to the daemon's config, so it survives a reboot.
    "}
)]
pub struct Args {
    /// (empty) = status · a file · `#RRGGBB` · `off` · `bright <0-100>` · `gui`
    pub target: Option<String>,
    /// Value for `bright`
    pub value: Option<String>,
    /// One screen: 1-based index, full id, or a unique part of it
    #[arg(long)]
    pub device: Option<String>,
    /// Frames per second for video/GIF (default: the daemon's, 30)
    #[arg(long)]
    pub fps: Option<f32>,
    /// Rotate the image, in degrees
    #[arg(long)]
    pub orientation: Option<f32>,
    /// Brightness 0-100 to apply along with the media
    #[arg(long)]
    pub bright: Option<u8>,
    /// Print the request instead of sending it
    #[arg(long)]
    pub dry_run: bool,
}

// ─── IPC ─────────────────────────────────────────────────────────────

fn socket_path() -> String {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    format!("{dir}/lianli-daemon.sock")
}

/// One request, one response. The daemon reads newline-delimited JSON and
/// answers per line, so a fresh connection per call keeps this free of any
/// stream state to get wrong.
#[cfg(unix)]
fn request(method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let path = socket_path();
    let mut sock = UnixStream::connect(&path).with_context(|| {
        format!("no Lian Li daemon on {path} — `systemctl --user status lianli-daemon`")
    })?;
    sock.set_read_timeout(Some(std::time::Duration::from_secs(10))).ok();
    let mut line = match params {
        Some(p) => serde_json::json!({ "method": method, "params": p }),
        None => serde_json::json!({ "method": method }),
    }
    .to_string();
    line.push('\n');
    sock.write_all(line.as_bytes())?;
    sock.flush()?;

    let mut reply = String::new();
    BufReader::new(&sock).read_line(&mut reply)?;
    let v: serde_json::Value = serde_json::from_str(reply.trim())
        .with_context(|| format!("daemon sent something that is not JSON: {reply:?}"))?;
    match v.get("status").and_then(|s| s.as_str()) {
        Some("ok") => Ok(v.get("data").cloned().unwrap_or(serde_json::Value::Null)),
        _ => bail!(
            "daemon refused {method}: {}",
            v.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error")
        ),
    }
}

#[cfg(not(unix))]
fn request(_method: &str, _params: Option<serde_json::Value>) -> Result<serde_json::Value> {
    bail!("the Lian Li daemon speaks over a Unix socket — Linux only")
}

#[derive(Debug, Clone)]
struct Screen {
    id: String,
    name: String,
    w: u32,
    h: u32,
}

fn screens() -> Result<Vec<Screen>> {
    let devices = request("ListDevices", None)?;
    Ok(devices
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d.get("has_lcd").and_then(|b| b.as_bool()).unwrap_or(false))
        .filter_map(|d| {
            Some(Screen {
                id: d.get("device_id")?.as_str()?.to_string(),
                name: d.get("name").and_then(|s| s.as_str()).unwrap_or("LCD").to_string(),
                w: d.get("screen_width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                h: d.get("screen_height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            })
        })
        .collect())
}

// ─── media ───────────────────────────────────────────────────────────

/// What the daemon should draw. Mirrors upstream's `MediaType` for the four
/// kinds that need no extra descriptor — sensor gauges and templates carry a
/// whole editor's worth of state and stay in the GUI.
#[derive(Debug, PartialEq)]
pub enum Media {
    File { kind: &'static str, path: String },
    Color([u8; 3]),
}

/// `#ff0055`, `ff0055`, `#f05` — all the same colour.
pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    let full = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => h.to_string(),
        _ => return None,
    };
    if !full.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&full[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// The daemon distinguishes still images, GIFs and video, and picks a different
/// encoder for each — so the extension has to be classified, not guessed at.
pub fn media_kind(path: &str) -> Option<&'static str> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "bmp" | "webp" | "tiff" => Some("image"),
        "gif" => Some("gif"),
        "mp4" | "mkv" | "webm" | "mov" | "avi" => Some("video"),
        _ => None,
    }
}

/// Resolve the positional argument into something to display.
///
/// A path is canonicalised here rather than passed through: the daemon is a
/// separate process with its own working directory, so a relative path would
/// resolve somewhere else entirely (or nowhere).
fn parse_media(arg: &str) -> Result<Media> {
    if arg.eq_ignore_ascii_case("off") {
        return Ok(Media::Color([0, 0, 0]));
    }
    if let Some(rgb) = parse_hex(arg) {
        return Ok(Media::Color(rgb));
    }
    let p = Path::new(arg);
    if !p.exists() {
        bail!("`{arg}` is neither a file, `off`, nor a #RRGGBB colour");
    }
    let abs = std::fs::canonicalize(p)?.to_string_lossy().into_owned();
    let kind = media_kind(&abs)
        .ok_or_else(|| anyhow!("`{arg}`: unsupported type — use png/jpg/bmp/webp, gif, or mp4/mkv/webm/mov"))?;
    Ok(Media::File { kind, path: abs })
}

/// Build the `LcdConfig` the daemon persists for one screen.
///
/// `serial` is the field the daemon matches against a live device's id, and the
/// enclosing `SetLcdMedia.device_id` must be `serial:<id>` because that is how
/// upstream's `LcdConfig::device_id()` renders itself — send anything else and
/// the entry is appended as a duplicate instead of replacing the old one.
fn lcd_config(id: &str, media: &Media, a: &Args) -> serde_json::Value {
    let mut cfg = serde_json::json!({
        "serial": id,
        "orientation": a.orientation.unwrap_or(0.0),
    });
    match media {
        Media::File { kind, path } => {
            cfg["type"] = serde_json::json!(kind);
            cfg["path"] = serde_json::json!(path);
            if let Some(fps) = a.fps {
                cfg["fps"] = serde_json::json!(fps);
            }
        }
        Media::Color(rgb) => {
            cfg["type"] = serde_json::json!("color");
            cfg["rgb"] = serde_json::json!(rgb);
        }
    }
    if let Some(b) = a.bright {
        cfg["brightness"] = serde_json::json!(b.min(100));
    }
    cfg
}

// ─── targeting ───────────────────────────────────────────────────────

/// Narrow to one screen by 1-based index, exact id, or a unique substring of it.
///
/// The ids are 16 hex characters (`hid:634aa893881e03a6`); nobody is typing
/// those, and eight identical fans make the index the only thing a human can
/// tell apart — so both work, and an ambiguous substring is an error rather
/// than a coin flip.
fn select<'a>(all: &'a [Screen], want: &str) -> Result<Vec<&'a Screen>> {
    if let Ok(n) = want.parse::<usize>() {
        return all
            .get(n.wrapping_sub(1))
            .map(|s| vec![s])
            .ok_or_else(|| anyhow!("no screen #{n} — there are {}", all.len()));
    }
    let hits: Vec<&Screen> = all.iter().filter(|s| s.id.contains(want)).collect();
    match hits.len() {
        1 => Ok(hits),
        0 => bail!("no screen matches `{want}`"),
        n => bail!("`{want}` matches {n} screens — use the index or a longer id"),
    }
}

// ─── run ─────────────────────────────────────────────────────────────

pub fn run(a: Args) -> Result<()> {
    if !platform::require_linux("lcd", "the Lian Li daemon is a Linux user service") {
        return Ok(());
    }
    match a.target.as_deref() {
        None | Some("status") | Some("ls") => status(),
        Some("gui") => gui(),
        Some("bright") => {
            let v = a
                .value
                .as_deref()
                .ok_or_else(|| anyhow!("`8sync lcd bright <0-100>` needs a value"))?;
            let pct: u8 = v.parse().map_err(|_| anyhow!("`{v}` is not a 0-100 brightness"))?;
            brightness(&a, pct.min(100))
        }
        Some(arg) => show(&a, arg),
    }
}

fn status() -> Result<()> {
    ui::header("8sync lcd");
    match request("Ping", None) {
        Ok(_) => ui::ok(&format!("lianli-daemon responding on {}", socket_path())),
        Err(e) => {
            ui::err(&format!("{e}"));
            ui::info("install: 8sync setup --profile hardware-lianli");
            ui::info("start:   systemctl --user enable --now lianli-daemon.service");
            return Ok(());
        }
    }
    let found = screens()?;
    if found.is_empty() {
        ui::warn("no LCD-capable device found");
        ui::info("a wireless fan's screen only answers while its controller is also cabled over USB");
        return Ok(());
    }
    for (i, s) in found.iter().enumerate() {
        ui::info(&format!("#{}  {}  {}x{}  {}", i + 1, s.name, s.w, s.h, s.id));
    }
    ui::info(&format!("{} screen(s) — `8sync lcd <file|#RRGGBB>` to draw on them", found.len()));
    Ok(())
}

fn targets<'a>(all: &'a [Screen], a: &Args) -> Result<Vec<&'a Screen>> {
    match &a.device {
        Some(want) => select(all, want),
        None => Ok(all.iter().collect()),
    }
}

fn show(a: &Args, arg: &str) -> Result<()> {
    let media = parse_media(arg)?;
    let all = screens()?;
    if all.is_empty() {
        ui::warn("no LCD-capable device found — `8sync lcd` for details");
        return Ok(());
    }
    ui::header("8sync lcd");
    for s in targets(&all, a)? {
        let cfg = lcd_config(&s.id, &media, a);
        let params = serde_json::json!({ "device_id": format!("serial:{}", s.id), "config": cfg });
        if a.dry_run {
            ui::info(&format!("SetLcdMedia {params}"));
            continue;
        }
        request("SetLcdMedia", Some(params))?;
        let what = match &media {
            Media::File { kind, path } => format!("{kind} {path}"),
            Media::Color([0, 0, 0]) => "off (black)".to_string(),
            Media::Color([r, g, b]) => format!("colour #{r:02x}{g:02x}{b:02x}"),
        };
        ui::ok(&format!("{} ← {what}", s.name));
    }
    Ok(())
}

/// Brightness twice, on purpose: `SetLcdBrightness` is the live one the panel
/// reacts to immediately, while the value stored in the screen's media config
/// is what survives a daemon restart. Sending only one of the two gives a
/// setting that either does not apply or does not stick.
fn brightness(a: &Args, pct: u8) -> Result<()> {
    let all = screens()?;
    let picked = targets(&all, a)?;
    if picked.is_empty() {
        ui::warn("no LCD-capable device found");
        return Ok(());
    }
    let stored = request("GetConfig", None).unwrap_or(serde_json::Value::Null);
    ui::header("8sync lcd");
    for s in picked {
        if a.dry_run {
            ui::info(&format!("SetLcdBrightness {} → {pct}", s.id));
            continue;
        }
        request(
            "SetLcdBrightness",
            Some(serde_json::json!({ "device_id": s.id, "brightness": pct })),
        )?;
        // Persist only where the screen already has a media config; inventing
        // one here would blank whatever it is currently showing.
        if let Some(mut cfg) = stored
            .get("lcds")
            .and_then(|l| l.as_array())
            .and_then(|l| l.iter().find(|c| c.get("serial").and_then(|s| s.as_str()) == Some(&s.id)))
            .cloned()
        {
            cfg["brightness"] = serde_json::json!(pct);
            request(
                "SetLcdMedia",
                Some(serde_json::json!({ "device_id": format!("serial:{}", s.id), "config": cfg })),
            )?;
        }
        ui::ok(&format!("{}: brightness {pct}", s.name));
    }
    Ok(())
}

/// Launch the upstream GUI with the WebKit workaround already applied.
///
/// `lianli-gui` is Tauri/WebKitGTK, and its DMA-BUF renderer dies on this class
/// of setup with `Error 71 (Protocol error) dispatching to Wayland display` —
/// the window never appears. Disabling that one renderer path costs nothing
/// visible and is the difference between an app that starts and one that does
/// not, so the verb always sets it rather than making the user find out.
fn gui() -> Result<()> {
    if which::which("lianli-gui").is_err() {
        ui::err("lianli-gui is not installed");
        ui::info("8sync setup --profile hardware-lianli");
        return Ok(());
    }
    ui::info("launching lianli-gui with WEBKIT_DISABLE_DMABUF_RENDERER=1 (Wayland/WebKit fix)");
    let status = Command::new("lianli-gui")
        .env("WEBKIT_DISABLE_DMABUF_RENDERER", "1")
        .status()?;
    if !status.success() {
        ui::warn(&format!("lianli-gui exited with {status}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scr(id: &str) -> Screen {
        Screen { id: id.into(), name: "TLV2".into(), w: 400, h: 400 }
    }

    #[test]
    fn hex_colours() {
        assert_eq!(parse_hex("#ff0055"), Some([0xff, 0x00, 0x55]));
        assert_eq!(parse_hex("ff0055"), Some([0xff, 0x00, 0x55]));
        assert_eq!(parse_hex("#f05"), Some([0xff, 0x00, 0x55]));
        assert_eq!(parse_hex("#gggggg"), None);
        assert_eq!(parse_hex("photo.png"), None);
    }

    /// The daemon encodes each kind differently, so a `.gif` must never be sent
    /// as a still and a `.mp4` must never be sent as a GIF.
    #[test]
    fn media_kinds_are_distinguished() {
        assert_eq!(media_kind("a.PNG"), Some("image"));
        assert_eq!(media_kind("a.gif"), Some("gif"));
        assert_eq!(media_kind("/x/y/clip.mp4"), Some("video"));
        assert_eq!(media_kind("notes.txt"), None);
        assert_eq!(media_kind("noext"), None);
    }

    /// Eight identical fans: the index is the only handle a human has, and an
    /// ambiguous substring must fail loudly rather than pick one.
    #[test]
    fn screen_selection() {
        let all = vec![scr("hid:aaa1"), scr("hid:aab2"), scr("hid:ccc3")];
        assert_eq!(select(&all, "2").unwrap()[0].id, "hid:aab2");
        assert_eq!(select(&all, "ccc3").unwrap()[0].id, "hid:ccc3");
        assert!(select(&all, "aa").is_err(), "ambiguous substring must not guess");
        assert!(select(&all, "9").is_err());
        assert!(select(&all, "0").is_err(), "the index is 1-based");
        assert!(select(&all, "nope").is_err());
    }

    /// `device_id` must be `serial:<id>` — upstream matches the request against
    /// `LcdConfig::device_id()`, and anything else appends a second entry for
    /// the same screen instead of replacing the first.
    #[test]
    fn colour_config_shape() {
        let a = Args {
            target: None,
            value: None,
            device: None,
            fps: None,
            orientation: None,
            bright: Some(80),
            dry_run: false,
        };
        let cfg = lcd_config("hid:abc", &Media::Color([1, 2, 3]), &a);
        assert_eq!(cfg["serial"], "hid:abc");
        assert_eq!(cfg["type"], "color");
        assert_eq!(cfg["rgb"], serde_json::json!([1, 2, 3]));
        assert_eq!(cfg["brightness"], 80);
        assert_eq!(cfg["orientation"], 0.0);
        assert!(cfg.get("path").is_none(), "a colour has no file");
    }

    #[test]
    fn video_config_carries_fps_not_rgb() {
        let a = Args {
            target: None,
            value: None,
            device: None,
            fps: Some(24.0),
            orientation: Some(90.0),
            bright: None,
            dry_run: false,
        };
        let cfg = lcd_config("hid:abc", &Media::File { kind: "video", path: "/tmp/a.mp4".into() }, &a);
        assert_eq!(cfg["type"], "video");
        assert_eq!(cfg["path"], "/tmp/a.mp4");
        assert_eq!(cfg["fps"], 24.0);
        assert_eq!(cfg["orientation"], 90.0);
        assert!(cfg.get("rgb").is_none());
        assert!(cfg.get("brightness").is_none(), "unset brightness must not be invented");
    }

    #[test]
    fn off_is_a_black_frame() {
        assert_eq!(parse_media("off").unwrap(), Media::Color([0, 0, 0]));
        assert_eq!(parse_media("OFF").unwrap(), Media::Color([0, 0, 0]));
        assert!(parse_media("/definitely/not/here.png").is_err());
    }
}
