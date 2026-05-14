use anyhow::Result;
use std::path::PathBuf;

use crate::ui;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_root(&cwd).unwrap_or(cwd);
    let gsd = root.join(".gsd");
    if !gsd.exists() {
        ui::warn("no .gsd/ here — open with `8sync .` first");
        return Ok(());
    }
    // Append a timestamped end marker; real "capture" requires forge cooperation.
    let ts = chrono_now();
    for f in ["STATE.md", "KNOWLEDGE.md", "DECISIONS.md", "PREFERENCES.md"] {
        let p = gsd.join(f);
        let mut s = std::fs::read_to_string(&p).unwrap_or_default();
        s.push_str(&format!("\n<!-- session-end {} -->\n", ts));
        std::fs::write(&p, s)?;
    }
    ui::ok(&format!("session-end marker appended to {}", gsd.display()));
    ui::info("Tip: ask forge to run `8sync mcp capture` to write structured entries (phase 2).");
    Ok(())
}

fn find_root(start: &std::path::Path) -> Option<PathBuf> {
    let mut p = start.to_path_buf();
    loop {
        if p.join(".git").exists() { return Some(p); }
        if !p.pop() { return None; }
    }
}

fn chrono_now() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch-{}", t)
}
