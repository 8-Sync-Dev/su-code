//! `8sync harness model` — view / edit the adaptive-model config
//! (`~/.config/8sync/models.toml`), the single source of truth for which model
//! `8sync ai`/`8sync .` and the `/auto` engine steer omp toward per task class.
//! omp resolves names fuzzily and falls back to an authenticated model when the
//! configured one isn't logged in (`retry.modelFallback`).
use anyhow::Result;

use crate::{assets, env_detect, ui};

const ROLE_KEYS: &[&str] = &["default", "plan", "smol", "slow"];

pub(crate) fn harness_model(env: &env_detect::Env, args: &[String]) -> Result<()> {
    let path = env.xdg_config.join("8sync/models.toml");
    // Seed the user file from the embedded default on first touch.
    if !path.exists() {
        if let (Some(def), Some(parent)) = (assets::read("configs/models.toml"), path.parent()) {
            std::fs::create_dir_all(parent)?;
            std::fs::write(&path, def)?;
        }
    }

    // Set mode: `harness model <key> <value>`.
    if args.len() >= 2 {
        let key = args[0].trim().to_string();
        let val = args[1..].join(" ").trim().to_string();
        let section = if ROLE_KEYS.contains(&key.as_str()) { "roles" } else { "tasks" };
        set_model_toml(&path, section, &key, &val)?;
        ui::ok(&format!("set [{}].{} = \"{}\" → {}", section, key, val, path.display()));
        ui::info("re-run `8sync harness model` to view; takes effect on the next `8sync ai`/`8sync .`/`/auto`.");
        return Ok(());
    }

    // View mode.
    ui::header("8sync harness model — adaptive model routing");
    let cfg = crate::models::ModelConfig::load();
    let shown = |s: &str| if s.is_empty() { "(omp default)".to_string() } else { s.to_string() };
    println!("  config: {}", path.display());
    println!();
    println!("  [roles]  (omp's own routing: --plan/--smol/--slow + main)");
    println!("   default = {}", shown(&cfg.roles.default));
    println!("   plan    = {}", shown(&cfg.roles.plan));
    println!("   smol    = {}", shown(&cfg.roles.smol));
    println!("   slow    = {}", shown(&cfg.roles.slow));
    println!();
    println!("  [tasks]  (per-prompt class → model)");
    if cfg.tasks.is_empty() {
        println!("   (none — falls back to roles.default)");
    } else {
        for (k, v) in &cfg.tasks {
            println!("   {:<8} = {}", k, shown(v));
        }
    }
    println!();
    ui::info("set: 8sync harness model <default|plan|smol|slow | plan|review|debug|code|trivial> <model>");
    ui::info("e.g. 8sync harness model review opus · 8sync harness model default codex");
    ui::info("names resolve fuzzily in omp; unconfigured/unauthed → omp falls back to an authed model.");
    Ok(())
}

/// Set `[section].key = value` in a `models.toml` document, creating the table
/// and section as needed, then write it back. Shared by the CLI set mode and
/// the `harness web` `POST /api/models` handler so both edit the toml identically.
pub(crate) fn set_model_toml(
    path: &std::path::Path,
    section: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut doc: toml::Value =
        toml::from_str(&raw).unwrap_or_else(|_| toml::Value::Table(Default::default()));
    let tbl = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("models.toml is not a table"))?;
    let sect = tbl
        .entry(section.to_string())
        .or_insert_with(|| toml::Value::Table(Default::default()));
    sect.as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[{}] is not a table", section))?
        .insert(key.to_string(), toml::Value::String(value.to_string()));
    std::fs::write(path, toml::to_string(&doc)?)?;
    Ok(())
}
