use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use std::path::PathBuf;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync shot http://localhost:3000/dashboard
      8sync shot http://127.0.0.1:8731/codegraph -o /tmp/cg.png
      8sync shot http://localhost:5173/login -o /tmp/login.png
      8sync shot ./docs/index.html
      8sync shot <url> --width 1400 --height 2200 --wait 4000

    USE CASE
      Render a URL or local HTML file to PNG so a VISION-capable model (Opus-class)
      can read it as ONE image instead of parsing the DOM/text. Anthropic bills
      ceil(w/28)*ceil(h/28) vision tokens (28x28 patch, pay-per-pixel; Opus 4.7+
      dropped the cheap-resize cap). Image WINS for STRUCTURE — a graph/dashboard/
      diagram carries spatial relations that cost far more as text (a 12k-edge graph
      ~25x). It LOSES for prose/code — plain text is cheaper AND lossless. See the
      image-routing skill for the decision table (and the DeepSeek-OCR caveat: the
      10x/90% figure needs a dedicated encoder, not a screenshot).

    REQUIREMENTS
      · a headless Chromium: system `chromium` (pacman -S chromium), else omp's
        bundled copy (~/.omp/puppeteer/chrome/<ver>/chrome-linux64/chrome).
"})]
pub struct Args {
    /// URL (http/https/file) or local HTML file to render.
    pub target: String,
    /// Output PNG path.
    #[arg(short, long, default_value = "/tmp/8sync-shot.png")]
    pub output: String,
    /// Viewport width in px.
    #[arg(long, default_value_t = 1280)]
    pub width: u32,
    /// Viewport height in px (taller = more content in one image).
    #[arg(long, default_value_t = 1600)]
    pub height: u32,
    /// Milliseconds to let JS/SPA render before capturing.
    #[arg(long, default_value_t = 3000)]
    pub wait: u32,
}

/// Locate a headless-capable Chromium: system install first, then omp's bundled
/// copy under `~/.omp/puppeteer/chrome/<version>/chrome-linux64/chrome`.
fn find_chromium() -> Option<PathBuf> {
    for bin in [
        "chromium",
        "chromium-browser",
        "google-chrome-stable",
        "google-chrome",
        "chrome",
    ] {
        if let Ok(p) = which::which(bin) {
            return Some(p);
        }
    }
    let base = dirs::home_dir()?.join(".omp/puppeteer/chrome");
    for entry in std::fs::read_dir(&base).ok()?.flatten() {
        let cand = entry.path().join("chrome-linux64/chrome");
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

pub fn run(a: Args) -> Result<()> {
    let Some(chrome) = find_chromium() else {
        ui::err(
            "no Chromium found — install `chromium` (pacman -S chromium), \
             or run `8sync setup` so omp's bundled copy is present",
        );
        return Ok(());
    };

    // http/https/file pass through; a bare local path becomes a file:// URL.
    let target = if a.target.starts_with("http://")
        || a.target.starts_with("https://")
        || a.target.starts_with("file://")
    {
        a.target.clone()
    } else {
        let abs = std::fs::canonicalize(&a.target)
            .with_context(|| format!("local target not found: {}", a.target))?;
        format!("file://{}", abs.display())
    };

    ui::info(&format!(
        "shot {} → {} ({}×{})",
        a.target, a.output, a.width, a.height
    ));
    let status = Command::new(&chrome)
        .args([
            "--headless".into(),
            "--no-sandbox".into(),
            "--disable-gpu".into(),
            "--hide-scrollbars".into(),
            format!("--screenshot={}", a.output),
            format!("--window-size={},{}", a.width, a.height),
            format!("--virtual-time-budget={}", a.wait),
            target,
        ])
        .status()
        .context("failed to launch Chromium")?;
    if !status.success() {
        ui::warn("Chromium exited non-zero — screenshot may be incomplete");
    }

    match std::fs::metadata(&a.output) {
        Ok(m) if m.len() > 0 => {
            // Anthropic vision cost = ceil(w/28)*ceil(h/28) tokens (28x28 patch,
            // pay-per-pixel; Opus 4.7+ dropped the ~1.15 MP cheap-resize cap).
            let est = a.width.div_ceil(28) as u64 * a.height.div_ceil(28) as u64;
            ui::ok(&format!(
                "wrote {} ({} KB, ~{} vision tokens)",
                a.output,
                m.len() / 1024,
                est
            ));
        }
        _ => ui::err("screenshot not produced — does the target render headless?"),
    }
    Ok(())
}
