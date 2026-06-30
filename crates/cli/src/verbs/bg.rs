//! `8sync bg` — manage the kitty `background_image` (wallpaper) at runtime.
//!
//! View the current wallpaper inline (rendered via `kitten icat` — the kitty
//! graphics protocol, same mechanism omp uses), browse a local collection with
//! a live fzf image preview, and swap it live (rewrites the `background_image`
//! line in 8sync.conf + SIGUSR1-reloads kitty — no restart). The choice is
//! recorded in `~/.config/8sync/wallpaper` so `8sync setup` honors it.
//!
//! Kitty-only inline rendering (`kitten icat` needs a real kitty TTY). On other
//! terminals the path is still printed; images just won't appear inline.

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{env_detect::Env, ui};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync bg                  show the current wallpaper (rendered inline in kitty)
          8sync bg get              print the current wallpaper path (scriptable)
          8sync bg set ~/pic.jpg    set a wallpaper live (rewrites 8sync.conf + reloads kitty)
          8sync bg set              no arg → interactive fzf picker over the collection (live preview)
          8sync bg list             list the collection (fzf picker in kitty; plain paths otherwise)
          8sync bg add <url>        download a wallpaper into the collection
          8sync bg add ~/pic.jpg -s add AND set in one go

        WHERE WALLPAPERS LIVE
          active  → recorded in ~/.config/8sync/wallpaper (path), baked into 8sync.conf
          collection → ~/.config/8sync/wallpapers/  (populated via `8sync bg add`)

        NOTES
          Inline rendering uses `kitten icat` (kitty graphics protocol) — works inside
          kitty; on other terminals paths still print but images won't appear inline.
          `hydectl wallpaper` governs the Hyprland DESKTOP wallpaper; this governs
          kitty's in-terminal `background_image` — distinct surfaces.
    "}
)]
pub struct Args {
    /// sub-action: show | get | set [file] | list | add <url|file>. Empty = show.
    pub action: Vec<String>,

    /// With `add`: also set the added wallpaper as active.
    #[arg(long, short = 's')]
    pub set: bool,
}

pub fn run(a: Args) -> Result<()> {
    let env = Env::detect()?;
    let set_flag = a.set;
    let action = a.action.first().map(|s| s.as_str()).unwrap_or("show");
    match action {
        "show" => show(&env),
        "get" => get(&env),
        "set" => {
            let target = a.action.get(1).map(|s| s.as_str()).unwrap_or("");
            set(&env, target) // "" → interactive fzf picker
        }
        "list" => list(&env),
        "add" => {
            let src = a.action.get(1).map(|s| s.as_str()).unwrap_or("");
            if src.is_empty() {
                bail!("usage: 8sync bg add <url|file> [--set]");
            }
            let added = add(&env, src)?;
            if set_flag {
                set(&env, added.to_str().unwrap_or(""))?;
            } else {
                ui::ok(&format!("added → {}", added.display()));
            }
            Ok(())
        }
        other => {
            ui::warn(&format!("unknown action `{other}` — try: show | get | set | list | add"));
            Ok(())
        }
    }
}

// ─── paths ──────────────────────────────────────────────────────────────

fn conf_path(env: &Env) -> PathBuf {
    env.xdg_config.join("kitty/8sync.conf")
}

fn collection_dir(env: &Env) -> PathBuf {
    env.xdg_config.join("8sync/wallpapers")
}

fn record_path(env: &Env) -> PathBuf {
    env.xdg_config.join("8sync/wallpaper")
}

/// Parse the active wallpaper from the `background_image` line in 8sync.conf.
pub(crate) fn current_bg(env: &Env) -> Option<PathBuf> {
    let conf = std::fs::read_to_string(conf_path(env)).ok()?;
    conf.lines().find_map(|l| {
        let p = l.strip_prefix("background_image ")?.trim();
        if p.is_empty() {
            None
        } else {
            Some(PathBuf::from(p))
        }
    })
}

// ─── sub-actions ────────────────────────────────────────────────────────

fn show(env: &Env) -> Result<()> {
    match current_bg(env) {
        Some(p) => {
            ui::ok(&format!("current wallpaper: {}", p.display()));
            render_inline(&p);
        }
        None => ui::warn("no wallpaper set (background_image not found in 8sync.conf)"),
    }
    Ok(())
}

fn get(env: &Env) -> Result<()> {
    match current_bg(env) {
        Some(p) => println!("{}", p.display()),
        None => bail!("no wallpaper set"),
    }
    Ok(())
}

fn set(env: &Env, target: &str) -> Result<()> {
    // No arg → interactive fzf picker over the collection (+ active wallpaper).
    let picked: PathBuf = if target.is_empty() {
        match pick(env)? {
            Some(p) => p,
            None => {
                ui::info("no selection — nothing changed");
                return Ok(());
            }
        }
    } else {
        let p = expand_tilde(target);
        let p = if p.is_relative() {
            std::env::current_dir()?.join(p)
        } else {
            p
        };
        if !is_image(&p) {
            bail!("not a recognized image (PNG/JPEG/WEBP/GIF): {}", p.display());
        }
        p
    };

    rewrite_bg_line(env, &picked)?;
    if let Some(parent) = record_path(env).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(record_path(env), picked.to_string_lossy().as_bytes())?;
    ui::ok(&format!("wallpaper → {}", picked.display()));
    reload_kitty();
    Ok(())
}

fn list(env: &Env) -> Result<()> {
    let files = collect(env);
    if files.is_empty() {
        ui::warn(&format!(
            "collection empty: {}. Add some via `8sync bg add <url>`.",
            collection_dir(env).display()
        ));
        return Ok(());
    }

    if crate::env_detect::has_tty() {
        match pick_from(&files)? {
            Some(p) => set(env, p.to_str().unwrap_or(""))?,
            None => ui::info("no selection"),
        }
    } else {
        ui::step(&format!("wallpaper collection ({})", collection_dir(env).display()));
        for (i, f) in files.iter().enumerate() {
            println!("  [{i}] {}", f.display());
        }
        println!("\n  set one: 8sync bg set <path>");
    }
    Ok(())
}

fn add(env: &Env, src: &str) -> Result<PathBuf> {
    let dir = collection_dir(env);
    std::fs::create_dir_all(&dir)?;
    let dest = if src.starts_with("http://") || src.starts_with("https://") {
        let name = src.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or("wallpaper");
        let dest = dir.join(clean_name(name));
        let ok = Command::new("curl")
            .args(["-fsSL", "--retry", "2", "-A", "Mozilla/5.0", "-o"])
            .arg(&dest)
            .arg(src)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            bail!("download failed: {src}");
        }
        dest
    } else {
        let p = expand_tilde(src);
        let p = if p.is_relative() { std::env::current_dir()?.join(&p) } else { p };
        if !p.exists() {
            bail!("file not found: {}", p.display());
        }
        if !is_image(&p) {
            bail!("not a recognized image (PNG/JPEG/WEBP/GIF): {}", p.display());
        }
        let dest = dir.join(p.file_name().unwrap_or_default());
        std::fs::copy(&p, &dest)?;
        dest
    };
    if !is_image(&dest) {
        let _ = std::fs::remove_file(&dest);
        bail!("downloaded file is not a valid image (purged): {}", dest.display());
    }
    Ok(dest)
}

// ─── helpers ────────────────────────────────────────────────────────────

/// Collection images, sorted, with the active wallpaper first (even if it lives
/// outside the collection dir).
fn collect(env: &Env) -> Vec<PathBuf> {
    let dir = collection_dir(env);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| is_image(p))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    if let Some(cur) = current_bg(env) {
        if is_image(&cur) && !files.iter().any(|f| f == &cur) {
            files.insert(0, cur);
        }
    }
    files
}

/// Rewrite the `background_image <path>` line in 8sync.conf (inserts a block if
/// the file has no wallpaper line). Leaves every other directive untouched.
fn rewrite_bg_line(env: &Env, new: &Path) -> Result<()> {
    let path = conf_path(env);
    let body = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "8sync.conf not found at {} — run `8sync setup --profile terminal` first",
            path.display()
        )
    })?;
    let line = format!("background_image {}", new.display());
    let mut out = String::new();
    let mut replaced = false;
    for l in body.lines() {
        if l.starts_with("background_image ") {
            out.push_str(&line);
            replaced = true;
        } else {
            out.push_str(l);
        }
        out.push('\n');
    }
    if !replaced {
        out.push_str(&line);
        out.push_str("\nbackground_image_layout cscaled\n");
    }
    std::fs::write(&path, out)?;
    Ok(())
}

/// Render an image inline via `kitten icat` (kitty graphics protocol). Inherits
/// stdout so the escape sequences reach the terminal. Graceful no-op if kitten
/// isn't installed or the terminal isn't kitty.
fn render_inline(file: &Path) {
    let ok = Command::new("kitten")
        .args(["icat", file.to_str().unwrap_or_default()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        ui::info("(inline preview needs kitty + `kitten`; path printed above)");
    }
}

/// Interactive fzf picker. `--preview 'kitten icat {}'` renders each candidate
/// live in the preview pane. Returns None on Esc / no selection.
fn pick(env: &Env) -> Result<Option<PathBuf>> {
    pick_from(&collect(env))
}

fn pick_from(files: &[PathBuf]) -> Result<Option<PathBuf>> {
    if files.is_empty() {
        return Ok(None);
    }
    let list = files
        .iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut child = Command::new("fzf")
        .args([
            "--preview",
            "kitten icat '{}'",
            "--preview-window",
            "right:55%:wrap",
            "--prompt",
            "wallpaper> ",
            "--header",
            "Enter=set  Esc=cancel  (preview renders via kitten icat)",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("fzf not found — install it, or use `8sync bg set <path>`")?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(list.as_bytes());
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Ok(None);
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(s)))
    }
}

fn reload_kitty() {
    let ok = Command::new("pkill")
        .args(["-SIGUSR1", "-x", "kitty"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        ui::info("kitty reloaded (SIGUSR1) — wallpaper applied live");
    } else {
        ui::info("kitty not running — wallpaper applies on next start");
    }
}

/// Recognize common raster formats by magic bytes (PNG/JPEG/WEBP/GIF). Guards
/// the 0-byte / HTML-error downloads that make kitty fail to render.
fn is_image(p: &Path) -> bool {
    let mut buf = [0u8; 12];
    let n = match std::fs::File::open(p).and_then(|mut f| f.read(&mut buf)) {
        Ok(n) => n,
        Err(_) => return false,
    };
    let b = &buf[..n];
    b.starts_with(&[0x89, b'P', b'N', b'G'])                             // PNG
        || b.starts_with(&[0xFF, 0xD8, 0xFF])                            // JPEG
        || (b.starts_with(b"RIFF") && b.len() >= 12 && &b[8..12] == b"WEBP") // WEBP
        || b.starts_with(b"GIF8")                                        // GIF
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

fn clean_name(s: &str) -> String {
    let s = s.split('?').next().unwrap_or(s);
    s.chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect()
}
