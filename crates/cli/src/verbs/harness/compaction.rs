//! `8sync harness compaction [pct]` — view or set omp's auto-compaction threshold
//! (`compaction.thresholdPercent` in `~/.omp/agent/config.yml`). No value prints
//! the current setting; `N` (1..=95) sets it. This is the user-facing knob for the
//! "auto-clean context at 50%" behavior the harness wires by default.
use anyhow::{anyhow, Result};
use std::path::Path;

use crate::ui;

pub(crate) fn harness_compaction(home: &Path, value: Option<&str>) -> Result<()> {
    ui::header("8sync harness compaction");
    let cfg = home.join(".omp/agent/config.yml");

    let Some(raw) = value else {
        match current_threshold(&cfg) {
            Some(p) => ui::ok(&format!(
                "compaction.thresholdPercent = {p} — omp auto-compacts at {p}% of the context window"
            )),
            None => ui::info(
                "compaction.thresholdPercent not set — omp uses its default; set it with `8sync harness compaction 50`",
            ),
        }
        return Ok(());
    };

    let pct: u64 = raw
        .trim()
        .trim_end_matches('%')
        .parse()
        .map_err(|_| anyhow!("invalid percent `{raw}` — use a number 1..95"))?;
    if !(1..=95).contains(&pct) {
        return Err(anyhow!("percent {pct} out of range — use 1..95"));
    }
    set_threshold(&cfg, pct)?;
    ui::ok(&format!("compaction.thresholdPercent = {pct} → {}", cfg.display()));
    ui::info("takes effect on the next omp session");
    Ok(())
}

/// Current `thresholdPercent:` value from config.yml, if present.
fn current_threshold(cfg: &Path) -> Option<u64> {
    let s = std::fs::read_to_string(cfg).ok()?;
    for line in s.lines() {
        if let Some(rest) = line.trim().strip_prefix("thresholdPercent:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

/// Set `compaction.thresholdPercent` in config.yml. Updates the line in place if
/// present (indentation preserved), inserts under an existing `compaction:` block,
/// or appends a fresh block. omp normalizes config.yml, so a line-level edit is safe.
fn set_threshold(cfg: &Path, pct: u64) -> Result<()> {
    if let Some(p) = cfg.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut s = std::fs::read_to_string(cfg).unwrap_or_default();

    if s.lines().any(|l| l.trim().starts_with("thresholdPercent:")) {
        s = s
            .lines()
            .map(|l| {
                if l.trim().starts_with("thresholdPercent:") {
                    let indent: String = l.chars().take_while(|c| c.is_whitespace()).collect();
                    format!("{indent}thresholdPercent: {pct}")
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    } else if s.lines().any(|l| l.starts_with("compaction:")) {
        s = s
            .lines()
            .map(|l| {
                if l.starts_with("compaction:") {
                    format!("{l}\n  thresholdPercent: {pct}")
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    } else {
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&format!("\ncompaction:\n  thresholdPercent: {pct}\n  idleEnabled: true\n"));
    }
    if !s.ends_with('\n') {
        s.push('\n');
    }
    std::fs::write(cfg, s)?;
    Ok(())
}
