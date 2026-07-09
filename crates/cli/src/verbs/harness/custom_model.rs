//! `8sync harness add-model <provider/model> --url <baseUrl> [flags]` — register a
//! REMOTE model that omp's fetched catalog lacks (or lists with null metadata —
//! e.g. a brand-new `xai/grok-4.5` that shows `context: -`, `max-out: -`) as a
//! full custom provider in `~/.omp/agent/models.yml`, so it appears in `/model`
//! and routes exactly like a built-in model.
//!
//! This is the cloud sibling of `add-local-model` (GGUF via mistral.rs). No
//! process is spawned — the endpoint already exists upstream; we only teach omp
//! about it. omp REQUIRES `baseUrl` when defining custom models (its validator
//! rejects a metadata-only merge with "baseUrl is required"), so `--url` is
//! mandatory. The model selector omp exposes is `<providerKey>/<modelId>`.
//!
//! A TSV registry (`~/.config/8sync/custom-models.tsv`) is the source of truth we
//! regenerate a managed sentinel block from — so it coexists with the
//! local-models block and the 9router gateway providers, and survives
//! `gateway apply` / `add-local-model` re-syncs (each strips only its own block).
use anyhow::{bail, Result};
use std::path::PathBuf;
use std::process::Command;

use crate::{env_detect, ui};

/// Sentinel markers wrapping the managed custom-model block inside models.yml.
/// Two-space indent so the block sits under the top-level `providers:` map.
pub(crate) const BLOCK_BEGIN: &str =
    "  # >>> 8sync:custom-models (managed by `8sync harness add-model`) >>>";
pub(crate) const BLOCK_END: &str = "  # <<< 8sync:custom-models <<<";

/// Optional per-model attributes parsed from CLI flags (`--url`, `--key`, …).
pub(crate) struct Flags {
    pub url: Option<String>,
    pub key: Option<String>,
    pub api: Option<String>,
    pub ctx: Option<u64>,
    pub max: Option<u64>,
    pub vision: bool,
    pub think: Option<String>,
}

/// One registered remote model. The TSV registry is the source of truth.
#[derive(Clone)]
struct CustomModel {
    /// `<provider>/<model-id>` — exactly the selector omp shows in `/model`.
    selector: String,
    base_url: String,
    api_key: String,
    /// omp API dialect: `openai-completions` | `anthropic-messages`.
    api: String,
    ctx: u64,
    max: u64,
    vision: bool,
    /// Comma-separated thinking efforts (empty = non-reasoning model).
    think: String,
}

pub(crate) fn harness_add_model(env: &env_detect::Env, args: &[String], flags: Flags) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("list") => list(env),
        Some("rm") | Some("remove") => match args.get(1) {
            Some(sel) => remove(env, sel),
            None => {
                ui::warn("usage: 8sync harness add-model rm <provider/model>");
                Ok(())
            }
        },
        Some(sel) if !sel.is_empty() => add(env, sel, flags),
        _ => {
            usage();
            Ok(())
        }
    }
}

/// Add (or replace) a remote model, then regenerate the managed block and verify
/// omp still loads the config (a bad `--think`/`--api` combo can be rejected).
fn add(env: &env_detect::Env, selector: &str, flags: Flags) -> Result<()> {
    ui::header("8sync harness add-model — register a remote model with omp");

    let (provider, model_id) = split_selector(selector)?;
    let selector = format!("{provider}/{model_id}");

    let base_url = flags
        .url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "--url <baseUrl> is required (omp rejects custom models without it)\n  \
                 e.g. 8sync harness add-model {selector} --url https://api.x.ai/v1 --key $XAI_API_KEY"
            )
        })?
        .to_string();

    // Key: explicit --key, else the conventional `<PROVIDER>_API_KEY` env var,
    // else a visible placeholder the user edits in-place (block still deploys).
    let env_key = format!("{}_API_KEY", provider.to_uppercase().replace('-', "_"));
    let api_key = flags
        .key
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var(&env_key).ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| {
            ui::warn(&format!(
                "no API key (pass --key, or set ${env_key}) — wrote a placeholder; edit models.yml"
            ));
            "REPLACE_WITH_API_KEY".to_string()
        });

    let api = normalize_api(flags.api.as_deref());
    let think = flags
        .think
        .map(|s| s.split(',').map(|e| e.trim()).filter(|e| !e.is_empty()).collect::<Vec<_>>().join(","))
        .unwrap_or_default();

    let m = CustomModel {
        selector: selector.clone(),
        base_url,
        api_key,
        api,
        ctx: flags.ctx.unwrap_or(256_000),
        max: flags.max.unwrap_or(32_000),
        vision: flags.vision,
        think,
    };

    // Upsert into the registry (same selector replaces), then sync the YAML.
    let mut reg = load_registry(env);
    reg.retain(|r| r.selector != m.selector);
    reg.push(m.clone());
    save_registry(env, &reg)?;
    sync_models_yml(env, &reg)?;

    ui::ok(&format!("registered {} → ~/.omp/agent/models.yml", selector));
    println!(
        "   {} · api {} · ctx {} · max {}{}{}",
        m.base_url,
        m.api,
        m.ctx,
        m.max,
        if m.vision { " · vision" } else { "" },
        if m.think.is_empty() { String::new() } else { format!(" · think[{}]", m.think) },
    );
    validate_loads(&selector);
    println!();
    ui::info(&format!("use:  8sync ai --model {selector} \"…\"   ·   or pick it in omp's /model"));
    ui::info(&format!("main: 8sync harness model default {selector}   (route omp to it)"));
    ui::info(&format!("list: 8sync harness add-model list   ·   rm: … rm {selector}"));
    Ok(())
}

fn list(env: &env_detect::Env) -> Result<()> {
    ui::header("8sync harness add-model — registered remote models");
    let reg = load_registry(env);
    if reg.is_empty() {
        ui::info("none — add one: 8sync harness add-model <provider/model> --url <baseUrl> [--key K]");
        return Ok(());
    }
    for m in &reg {
        println!(
            "  {:<28} {}  (api {} · ctx {} · max {}{}{})",
            m.selector,
            m.base_url,
            m.api,
            m.ctx,
            m.max,
            if m.vision { " · vision" } else { "" },
            if m.think.is_empty() { String::new() } else { format!(" · think[{}]", m.think) },
        );
    }
    println!();
    ui::info("use: 8sync ai --model <provider/model> \"…\"   ·   rm: 8sync harness add-model rm <provider/model>");
    Ok(())
}

fn remove(env: &env_detect::Env, selector: &str) -> Result<()> {
    let mut reg = load_registry(env);
    let before = reg.len();
    reg.retain(|m| m.selector != selector);
    if reg.len() == before {
        ui::warn(&format!("no custom model `{selector}` (see: add-model list)"));
        return Ok(());
    }
    save_registry(env, &reg)?;
    sync_models_yml(env, &reg)?;
    ui::ok(&format!("removed {selector} (provider block regenerated)"));
    Ok(())
}

/// Split `provider/model-id` (model id may itself contain `/`). Both parts required.
fn split_selector(sel: &str) -> Result<(String, String)> {
    let (prov, model) = sel
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("selector must be `<provider>/<model-id>`, e.g. xai/grok-4.5"))?;
    if prov.trim().is_empty() || model.trim().is_empty() {
        bail!("selector must be `<provider>/<model-id>`, e.g. xai/grok-4.5");
    }
    Ok((prov.trim().to_string(), model.trim().to_string()))
}

/// Map a friendly `--api` value to the omp dialect string. Default: OpenAI.
fn normalize_api(api: Option<&str>) -> String {
    match api.map(str::trim).unwrap_or("openai") {
        "anthropic" | "anthropic-messages" | "claude" => "anthropic-messages",
        "openai" | "openai-completions" | "" => "openai-completions",
        other => other, // pass through any exact dialect omp supports
    }
    .to_string()
}

fn registry_path(env: &env_detect::Env) -> PathBuf {
    env.xdg_config.join(crate::brand::NS).join("custom-models.tsv")
}

fn load_registry(env: &env_detect::Env) -> Vec<CustomModel> {
    let raw = std::fs::read_to_string(registry_path(env)).unwrap_or_default();
    raw.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 8 || f[0].trim().is_empty() {
                return None;
            }
            Some(CustomModel {
                selector: f[0].trim().to_string(),
                base_url: f[1].trim().to_string(),
                api_key: f[2].trim().to_string(),
                api: f[3].trim().to_string(),
                ctx: f[4].trim().parse().unwrap_or(256_000),
                max: f[5].trim().parse().unwrap_or(32_000),
                vision: f[6].trim() == "1",
                think: f[7].trim().to_string(),
            })
        })
        .collect()
}

fn save_registry(env: &env_detect::Env, reg: &[CustomModel]) -> Result<()> {
    let p = registry_path(env);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body: String = reg
        .iter()
        .map(|m| {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                m.selector,
                m.base_url,
                m.api_key,
                m.api,
                m.ctx,
                m.max,
                if m.vision { "1" } else { "0" },
                m.think,
            )
        })
        .collect();
    std::fs::write(p, body)?;
    Ok(())
}

/// Regenerate the managed custom-models block inside `~/.omp/agent/models.yml`
/// from the registry. Strips only our own sentinel block, so the local-models
/// block and the 9router gateway providers are preserved.
fn sync_models_yml(env: &env_detect::Env, reg: &[CustomModel]) -> Result<()> {
    let path = env.home.join(".omp/agent/models.yml");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let merged = super::local_model::insert_block(&strip_block(&existing), &render_block(reg));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, merged)?;
    Ok(())
}

/// Remove the managed custom-models sentinel block (if present) from a body.
pub(crate) fn strip_block(s: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for line in s.lines() {
        if line.trim_end() == BLOCK_BEGIN {
            skip = true;
            continue;
        }
        if line.trim_end() == BLOCK_END {
            skip = false;
            continue;
        }
        if !skip {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Extract the managed block (BEGIN..=END inclusive) so `gateway apply` can
/// re-attach it after rewriting models.yml from the template.
pub(crate) fn extract_block(s: &str) -> Option<String> {
    let mut out = String::new();
    let mut in_block = false;
    for line in s.lines() {
        if line.trim_end() == BLOCK_BEGIN {
            in_block = true;
        }
        if in_block {
            out.push_str(line);
            out.push('\n');
        }
        if line.trim_end() == BLOCK_END {
            return Some(out);
        }
    }
    None
}

/// Render the managed block for the registry (empty string if no models). Models
/// sharing a provider are grouped under one provider key (YAML forbids dup keys);
/// the provider-level baseUrl/apiKey/api come from that group's first entry.
fn render_block(reg: &[CustomModel]) -> String {
    if reg.is_empty() {
        return String::new();
    }
    // Group by provider prefix, preserving first-seen order.
    let mut groups: Vec<(String, Vec<&CustomModel>)> = Vec::new();
    for m in reg {
        let prov = m.selector.split('/').next().unwrap_or("").to_string();
        match groups.iter_mut().find(|(p, _)| *p == prov) {
            Some(g) => g.1.push(m),
            None => groups.push((prov, vec![m])),
        }
    }

    let mut b = String::new();
    b.push_str(BLOCK_BEGIN);
    b.push('\n');
    for (prov, models) in &groups {
        let head = models[0];
        b.push_str(&format!("  {prov}:\n"));
        b.push_str(&format!("    baseUrl: {}\n", head.base_url));
        b.push_str(&format!("    apiKey: {}\n", head.api_key));
        b.push_str(&format!("    api: {}\n", head.api));
        b.push_str("    models:\n");
        for m in models {
            let id = m.selector.splitn(2, '/').nth(1).unwrap_or(&m.selector);
            let input = if m.vision { "[text, image]" } else { "[text]" };
            b.push_str(&format!("      - id: {id}\n"));
            b.push_str(&format!("        name: {id} (custom)\n"));
            b.push_str(&format!("        reasoning: {}\n", !m.think.is_empty()));
            b.push_str(&format!("        input: {input}\n"));
            b.push_str(&format!("        contextWindow: {}\n", m.ctx));
            b.push_str(&format!("        maxTokens: {}\n", m.max));
            b.push_str("        cost: {input: 0, output: 0, cacheRead: 0, cacheWrite: 0}\n");
            if !m.think.is_empty() {
                let efforts: Vec<&str> = m.think.split(',').map(str::trim).filter(|e| !e.is_empty()).collect();
                let default = efforts.last().copied().unwrap_or("high");
                b.push_str("        thinking:\n");
                b.push_str("          mode: anthropic-budget-effort\n");
                b.push_str(&format!("          efforts: [{}]\n", efforts.join(", ")));
                b.push_str(&format!("          defaultLevel: {default}\n"));
            }
        }
    }
    b.push_str(BLOCK_END);
    b.push('\n');
    b
}

/// After writing, confirm omp still loads the config — a bad `--think`/`--api`
/// combo makes omp reject the WHOLE file (all custom models vanish). Best-effort:
/// if `omp` is absent we skip; a load error is a loud warning, not a hard fail.
fn validate_loads(selector: &str) {
    let Ok(out) = Command::new("omp").args(["models", "--json"]).output() else {
        return; // omp not on PATH — nothing to validate against
    };
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if combined.contains("Failed to load config") || combined.contains("Validate(models)") {
        ui::warn("omp REJECTED models.yml — the custom block has an invalid field:");
        for l in combined.lines().filter(|l| l.contains("error") || l.contains("required")) {
            println!("    {}", l.trim());
        }
        ui::info(&format!("fix or drop it: 8sync harness add-model rm {selector}  (try without --think)"));
    } else if combined.contains(selector) {
        ui::ok("omp loaded the config — model is live in the catalog");
    }
}

fn usage() {
    ui::header("8sync harness add-model — register a remote model omp's catalog lacks");
    println!("  add : 8sync harness add-model <provider/model> --url <baseUrl> [flags]");
    println!("  list: 8sync harness add-model list");
    println!("  rm  : 8sync harness add-model rm <provider/model>");
    println!();
    println!("  flags: --url <baseUrl>   (required)   --key <apiKey | $<PROVIDER>_API_KEY>");
    println!("         --api openai|anthropic (default openai)   --ctx <N>   --max <N>");
    println!("         --vision (accept images)   --think \"minimal,low,medium,high\" (reasoning)");
    println!();
    ui::info("e.g. 8sync harness add-model xai/grok-4.5 --url https://api.x.ai/v1 --key $XAI_API_KEY --ctx 256000 --max 32000");
    ui::info("Writes a custom provider into ~/.omp/agent/models.yml; selector = <provider>/<model>.");
    ui::info("For a local GGUF instead, use `8sync harness add-local-model`.");
}
