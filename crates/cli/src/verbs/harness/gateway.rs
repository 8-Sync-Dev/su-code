//! `8sync harness gateway` — deploy & verify the omp model-gateway config
//! (`~/.omp/agent/models.yml`): the 9router provider, models, API key, and the
//! `thinking.mode = anthropic-budget-effort` fix that stops the gateway 400'ing
//! Claude (notably sonnet-5) on `thinking:{type:adaptive}`.
//!
//! This file is the ONLY place omp learns the provider URL, models, and how to
//! serialize thinking. Model ROLE routing (which model for code/plan/smol) is a
//! separate layer — that lives in `~/.config/8sync/models.toml` via
//! `8sync harness model`.
//!
//! Parsing is line-based (no serde_yaml dep): the template format is fixed and
//! simple, so a `key: value` scan is robust and keeps the binary lean.
use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

use crate::{assets, env_detect, ui};

/// Placeholder substituted at apply time. Must match the template asset.
const PLACEHOLDER: &str = "__NINE_ROUTER_KEY__";
/// Env var read for the key (keeps secrets out of shell history / git).
const ENV_KEY: &str = "NINE_ROUTER_KEY";

pub(crate) fn harness_gateway(env: &env_detect::Env, args: &[String]) -> Result<()> {
    let path = env.home.join(".omp/agent/models.yml");
    match args.first().map(String::as_str) {
        Some("apply") => apply(&path),
        Some("key") => match args.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()) {
            Some(k) => set_key(&path, k),
            None => {
                ui::warn("usage: 8sync harness gateway key <KEY>");
                ui::info("or set the $NINE_ROUTER_KEY env var and run `8sync harness gateway apply`");
                Ok(())
            }
        },
        Some("verify") => verify(&path),
        Some("status") | None => status(&path),
        Some(other) => {
            ui::warn(&format!("unknown gateway subcommand: {}", other));
            ui::info("try: 8sync harness gateway apply | key <KEY> | verify | status");
            Ok(())
        }
    }
}

/// Show the current gateway config: provider URL, masked key, model count, and
/// whether the thinking-fix (`anthropic-budget-effort`) is in place.
fn status(path: &Path) -> Result<()> {
    ui::header("8sync harness gateway — omp model-gateway config");
    println!("  file: {}", path.display());
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    if raw.is_empty() {
        println!();
        ui::warn("not deployed — omp has no 9router provider configured");
        ui::info("run: 8sync harness gateway apply");
        return Ok(());
    }
    let url = field(&raw, "baseUrl").unwrap_or("(missing)");
    let key = field(&raw, "apiKey").unwrap_or("(missing)");
    let n_models = raw.lines().filter(|l| l.trim_start().starts_with("- id:")).count();
    let fixed = raw.contains("anthropic-budget-effort");
    println!();
    println!("  provider : {}", url);
    println!("  apiKey   : {}", mask(key));
    println!("  models   : {}", n_models);
    println!("  thinking : {}",
        if fixed { "anthropic-budget-effort (OK — gateway-safe)".to_string() }
        else { "MISSING fix — sonnet-5 will 400. Run `gateway apply`.".to_string() });
    println!();
    ui::info("apply : 8sync harness gateway apply      (re-deploy from bundled template)");
    ui::info("key   : 8sync harness gateway key <KEY>  (rotate the API key in place)");
    ui::info("check : 8sync harness gateway verify     (ping sonnet-5 through the gateway)");
    ui::info("route : 8sync harness model              (which model for code/plan/smol)");
    Ok(())
}

/// Deploy `~/.omp/agent/models.yml` from the embedded template, substituting the
/// API key. Idempotent: identical content is a no-op; a differing file is backed
/// up to `models.yml.bak` before overwrite. Key resolution: `$NINE_ROUTER_KEY`
/// env var first, then the key already in the existing file (preserve on refresh).
fn apply(path: &Path) -> Result<()> {
    let tmpl = assets::read("configs/omp/gateway-models.yml")
        .ok_or_else(|| anyhow::anyhow!("embedded gateway template (configs/omp/gateway-models.yml) missing"))?;

    let preserved = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| field(&raw, "apiKey").filter(|k| !k.is_empty() && *k != PLACEHOLDER).map(str::to_string));
    let key = std::env::var(ENV_KEY)
        .ok()
        .filter(|s| !s.is_empty())
        .or(preserved)
        .ok_or_else(|| anyhow::anyhow!(
            "no API key: set $NINE_ROUTER_KEY or run `8sync harness gateway key <KEY>` first"
        ))?;
    if key == PLACEHOLDER {
        bail!("resolved key is still the placeholder — set $NINE_ROUTER_KEY");
    }

    let rendered = tmpl.replace(PLACEHOLDER, &key);
    // Preserve any local GGUF providers added by `8sync harness add-local-model`
    // across a gateway re-deploy (the sentinel block is re-attached under `providers:`).
    let rendered = super::local_model::insert_block(
        &rendered,
        &super::local_model::extract_block(&std::fs::read_to_string(path).unwrap_or_default())
            .unwrap_or_default(),
    );

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let old = std::fs::read_to_string(path).unwrap_or_default();
        if old == rendered {
            ui::ok(&format!("gateway config already up to date → {}", path.display()));
            return Ok(());
        }
        let bak = path.with_extension("yml.bak");
        std::fs::write(&bak, &old).ok();
        ui::info(&format!("backed up existing config → {}", bak.display()));
    }
    std::fs::write(path, &rendered)?;
    ui::ok(&format!("deployed omp gateway config → {}", path.display()));
    ui::info("verify: 8sync harness gateway verify");
    Ok(())
}

/// Rotate just the API key in an already-deployed config (every `apiKey:` line),
/// preserving comments and layout. Creates the file from the template if absent.
fn set_key(path: &Path, key: &str) -> Result<()> {
    if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        let new: String = raw
            .lines()
            .map(|line| {
                let t = line.trim_start();
                if t.starts_with("apiKey:") {
                    let indent = &line[..line.len() - t.len()];
                    format!("{}apiKey: {}", indent, key)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(path, new)?;
        ui::ok(&format!("API key updated → {}", path.display()));
        Ok(())
    } else {
        // No config yet: create it from the template with this key.
        std::env::set_var(ENV_KEY, key);
        apply(path)
    }
}

/// Ping the gateway through the first `cc/` (Claude) model with a 1-token
/// request — the exact path that 400'd before the thinking fix. 200 = healthy.
fn verify(path: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(path)
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("no gateway config — run `8sync harness gateway apply` first"))?;
    let url = field(&raw, "baseUrl").ok_or_else(|| anyhow::anyhow!("baseUrl not found in {}", path.display()))?;
    let key = field(&raw, "apiKey").ok_or_else(|| anyhow::anyhow!("apiKey not found in {}", path.display()))?;
    let model = verify_model(&raw).ok_or_else(|| anyhow::anyhow!("no cc/ (Claude) model found to test"))?;

    ui::header(&format!("8sync harness gateway verify — pinging {}", model));
    let endpoint = format!("{}/messages", url.trim_end_matches('/'));
    let body = format!(
        "{{\"model\":\"{}\",\"max_tokens\":16,\"messages\":[{{\"role\":\"user\",\"content\":\"hi\"}}]}}",
        model
    );
    // curl appends the HTTP code on its own line via -w.
    let out = Command::new("curl")
        .args([
            "-sS", "-m", "30", "-w", "\n%{http_code}",
            "-H", &format!("Authorization: Bearer {}", key),
            "-H", "anthropic-version: 2023-06-01",
            "-H", "content-type: application/json",
            "-d", &body,
            &endpoint,
        ])
        .output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (body_txt, code) = match text.rfind('\n') {
        Some(i) => (text[..i].to_string(), text[i + 1..].trim().to_string()),
        None => (text.to_string(), String::from("?")),
    };
    match code.as_str() {
        "200" => {
            ui::ok(&format!("gateway healthy — {} → HTTP 200", model));
            Ok(())
        }
        c => {
            ui::warn(&format!("gateway returned HTTP {}", c));
            // Show a compact slice of the error so the cause is visible.
            let snip: String = body_txt.chars().take(240).collect();
            println!("  {}", snip);
            if body_txt.contains("budget_tokens") {
                ui::info("cause: thinking serialization — run `8sync harness gateway apply` to restore the fix");
            }
            bail!("gateway verify failed (HTTP {})", c);
        }
    }
}

/// First `key: value` field in the YAML whose stripped key matches `needle`.
fn field<'a>(raw: &'a str, needle: &str) -> Option<&'a str> {
    for line in raw.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix(&format!("{}:", needle)) {
            if rest.is_empty() || rest.starts_with(' ') {
                return Some(rest.trim().trim_matches('"'));
            }
        }
    }
    None
}

/// The model to verify with: prefer `cc/claude-sonnet-5` (the exact path that
/// 400'd before the thinking fix), else the first `cc/` (Claude) model found.
fn verify_model(raw: &str) -> Option<String> {
    let mut fallback = None;
    for line in raw.lines() {
        let t = line.trim_start();
        let id = t.strip_prefix("- id:").or_else(|| t.strip_prefix("id:"));
        if let Some(v) = id {
            let id = v.trim().trim_matches('"');
            if id.starts_with("cc/") {
                if id.contains("sonnet-5") {
                    return Some(id.to_string());
                }
                if fallback.is_none() {
                    fallback = Some(id.to_string());
                }
            }
        }
    }
    fallback
}

/// Mask a secret for display: `sk-2d4e55340498...6efc`. Short values stay intact.
fn mask(k: &str) -> String {
    if k.len() <= 10 {
        k.to_string()
    } else {
        format!("{}...{}", &k[..6], &k[k.len() - 4..])
    }
}
