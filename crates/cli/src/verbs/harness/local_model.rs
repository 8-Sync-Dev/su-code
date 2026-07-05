//! `8sync harness add-local-model <path>` — load a GGUF model through a
//! Rust-native runtime (mistral.rs) and register it as an omp OpenAI provider,
//! so omp can route to on-device GGUF models exactly like a cloud model.
//!
//! `<path>` is auto-classified: an existing `*.gguf` FILE, a HuggingFace repo id
//! (`org/repo`), or a `*.gguf` download URL. This version is GGUF-only (the fast,
//! memory-safe path); other formats auto-detect later.
//!
//! Design (mirrors `gateway.rs`): mistral.rs is the ONLY thing that touches the
//! weights — pure Rust, memory-safe, exposes an OpenAI `/v1` endpoint. A per-model
//! systemd *user* service keeps it served; the endpoint is registered in
//! `~/.omp/agent/models.yml` under a managed sentinel block. A tiny TSV registry
//! (`~/.config/8sync/local-models.tsv`) is the source of truth we regenerate the
//! YAML block from, so `rm`/re-add and `gateway apply` never corrupt it.
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{env_detect, ui};

/// Sentinel markers wrapping the managed provider block inside models.yml.
/// Two-space indent so the block sits under the top-level `providers:` map.
pub(crate) const BLOCK_BEGIN: &str =
    "  # >>> 8sync:local-models (managed by `8sync harness add-local-model`) >>>";
pub(crate) const BLOCK_END: &str = "  # <<< 8sync:local-models <<<";

/// First port for a local model server; subsequent models take the next free one.
const PORT_BASE: u16 = 8770;

/// One registered local model. The TSV registry is the source of truth.
struct LocalModel {
    name: String,
    port: u16,
    /// The `-m` argument passed to mistral.rs (an absolute .gguf path or HF repo id).
    source: String,
}

/// A user-supplied model location, classified.
enum Source {
    File(PathBuf),
    Hf(String),
    Url(String),
}

pub(crate) fn harness_add_local_model(
    env: &env_detect::Env,
    args: &[String],
    port_override: Option<u16>,
) -> Result<()> {
    match args.first().map(String::as_str) {
        None | Some("") => {
            usage();
            Ok(())
        }
        Some("list") | Some("ls") => list(env),
        Some("rm") | Some("remove") => match args.get(1).filter(|s| !s.is_empty()) {
            Some(name) => remove(env, name),
            None => {
                ui::warn("usage: 8sync harness add-local-model rm <name>");
                Ok(())
            }
        },
        Some(path) => {
            let name = args.get(1).filter(|s| !s.is_empty()).cloned();
            add(env, path, name, port_override)
        }
    }
}

/// Add (or refresh) a local GGUF model: resolve source → ensure runtime → serve →
/// register with omp → verify the endpoint responds.
fn add(env: &env_detect::Env, path: &str, name: Option<String>, port_override: Option<u16>) -> Result<()> {
    ui::header("8sync harness add-local-model — load a GGUF model into omp (mistral.rs)");
    let src = classify(path)?;
    let name = slug(&name.unwrap_or_else(|| default_name(&src)));
    if name.is_empty() {
        bail!("could not derive a model name — pass one: add-local-model <path> <name>");
    }

    // 1. Rust GGUF runtime (installs the prebuilt binary if missing).
    let runner = ensure_mistralrs(env)?;

    // 2. Resolve the `-m` source (download a URL, validate GGUF magic for files).
    let source_arg = resolve_source(env, &src, &name)?;

    // 3. Port: reuse the model's existing port on refresh, else the next free one.
    let mut reg = load_registry(env);
    let port = port_override
        .or_else(|| reg.iter().find(|m| m.name == name).map(|m| m.port))
        .unwrap_or_else(|| free_port(&reg));

    // 4. Serve it via a systemd user service (survives across sessions with linger).
    write_service(env, &runner, &name, port, &source_arg)?;
    start_service(&name);

    // 5. Persist to the registry and regenerate the omp provider block.
    reg.retain(|m| m.name != name);
    reg.push(LocalModel { name: name.clone(), port, source: source_arg });
    save_registry(env, &reg)?;
    sync_models_yml(env, &reg)?;
    ui::ok(&format!("registered omp provider `local/{name}` → ~/.omp/agent/models.yml"));

    // 6. Verify the endpoint actually serves (model load can take a while).
    ui::step(&format!("waiting for the model to load on http://127.0.0.1:{port} …"));
    if wait_ready(port, 180) {
        ui::ok(&format!("model `local/{name}` is serving (mistral.rs · port {port})"));
    } else {
        ui::warn(&format!(
            "not ready yet — follow the load: journalctl --user -u 8sync-llm-{name} -f"
        ));
    }

    // 7. Usage.
    println!();
    ui::info(&format!("use : 8sync ai --model local/{name} \"…\""));
    ui::info(&format!("main: 8sync harness model default local/{name}   (route omp to it)"));
    ui::info(&format!("list: 8sync harness add-local-model list   ·   rm: … rm {name}"));
    ui::info("persist across logout: loginctl enable-linger $USER");
    Ok(())
}

/// Classify a user path into a GGUF file, an HF repo id, or a GGUF URL.
fn classify(path: &str) -> Result<Source> {
    let low = path.to_lowercase();
    if path.starts_with("http://") || path.starts_with("https://") {
        if low.ends_with(".gguf") {
            return Ok(Source::Url(path.to_string()));
        }
        bail!("URL must point directly at a .gguf file (GGUF-only this version): {path}");
    }
    let p = Path::new(path);
    if p.exists() {
        if low.ends_with(".gguf") {
            return Ok(Source::File(std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())));
        }
        bail!("file exists but is not a .gguf (GGUF-only this version): {path}");
    }
    // HF repo id: `org/repo`, no whitespace, not an fs path.
    if path.contains('/')
        && !path.contains(char::is_whitespace)
        && !path.starts_with('/')
        && !path.starts_with('.')
        && !path.starts_with('~')
    {
        return Ok(Source::Hf(path.to_string()));
    }
    bail!("`{path}` is not a .gguf file, a HuggingFace repo id (org/repo), or a .gguf URL");
}

/// A reasonable default model name from the source.
fn default_name(src: &Source) -> String {
    match src {
        Source::File(p) => p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        Source::Url(u) => {
            let last = u.rsplit('/').next().unwrap_or("model");
            last.strip_suffix(".gguf").unwrap_or(last).to_string()
        }
        Source::Hf(r) => r.rsplit('/').next().unwrap_or(r).to_string(),
    }
}

/// Resolve the classified source into the concrete `-m` argument for mistral.rs.
fn resolve_source(env: &env_detect::Env, src: &Source, name: &str) -> Result<String> {
    match src {
        Source::Hf(r) => Ok(r.clone()),
        Source::File(p) => {
            verify_gguf(p)?;
            Ok(p.display().to_string())
        }
        Source::Url(u) => {
            let dir = env.home.join(".cache/8sync/models");
            std::fs::create_dir_all(&dir)?;
            let dest = dir.join(format!("{name}.gguf"));
            if !dest.exists() {
                ui::step(&format!("downloading GGUF → {}", dest.display()));
                let st = Command::new("curl")
                    .args(["-fL", "--progress-bar", "-o"])
                    .arg(&dest)
                    .arg(u)
                    .status()
                    .context("curl download")?;
                if !st.success() {
                    bail!("download failed: {u}");
                }
            }
            verify_gguf(&dest)?;
            Ok(dest.display().to_string())
        }
    }
}

/// A GGUF file starts with the ASCII magic `GGUF`.
fn verify_gguf(p: &Path) -> Result<()> {
    use std::io::Read;
    let mut f = std::fs::File::open(p).with_context(|| format!("open {}", p.display()))?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)
        .with_context(|| format!("read {}", p.display()))?;
    if &magic != b"GGUF" {
        bail!("{} is not a valid GGUF file (bad magic)", p.display());
    }
    Ok(())
}

/// Locate mistral.rs (`mistralrs`), installing the prebuilt binary if absent.
/// The official installer picks a per-GPU CUDA or CPU build — no Rust or CUDA
/// toolkit needed, just the NVIDIA driver for the GPU path.
fn ensure_mistralrs(env: &env_detect::Env) -> Result<PathBuf> {
    for b in ["mistralrs", "mistralrs-server"] {
        if let Ok(p) = which::which(b) {
            ui::ok(&format!("mistral.rs runtime: {}", p.display()));
            return Ok(p);
        }
    }
    let local = env.home.join(".local/bin/mistralrs");
    if local.exists() {
        ui::ok(&format!("mistral.rs runtime: {}", local.display()));
        return Ok(local);
    }

    ui::step("mistral.rs (Rust GGUF runtime) not found — installing the prebuilt binary …");
    ui::info("via EricLBuehler/mistral.rs install.sh (prebuilt CUDA/CPU — no toolchain needed)");
    let st = Command::new("sh")
        .arg("-c")
        .arg("curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/EricLBuehler/mistral.rs/master/install.sh | sh")
        .status()
        .context("run mistral.rs installer")?;
    if !st.success() {
        bail!("mistral.rs install failed — install manually: cargo install mistralrs-cli");
    }
    for b in ["mistralrs", "mistralrs-server"] {
        if let Ok(p) = which::which(b) {
            return Ok(p);
        }
    }
    if local.exists() {
        return Ok(local);
    }
    bail!("mistral.rs installed but not on PATH — add ~/.local/bin to PATH and retry");
}

/// First free port at/above PORT_BASE not already claimed by another local model.
fn free_port(reg: &[LocalModel]) -> u16 {
    let used: BTreeSet<u16> = reg.iter().map(|m| m.port).collect();
    let mut p = PORT_BASE;
    loop {
        if !used.contains(&p) && std::net::TcpListener::bind(("127.0.0.1", p)).is_ok() {
            return p;
        }
        match p.checked_add(1) {
            Some(n) => p = n,
            None => return PORT_BASE,
        }
    }
}

fn service_name(name: &str) -> String {
    format!("8sync-llm-{name}.service")
}

/// Write the per-model systemd user unit that serves the GGUF via mistral.rs.
fn write_service(
    env: &env_detect::Env,
    runner: &Path,
    name: &str,
    port: u16,
    source: &str,
) -> Result<()> {
    let dir = env.home.join(".config/systemd/user");
    std::fs::create_dir_all(&dir)?;
    let unit = format!(
        "[Unit]\n\
         Description=8sync local GGUF model `{name}` served by mistral.rs (Rust)\n\
         After=network-online.target\n\
         \n\
         [Service]\n\
         ExecStart={runner} serve --port {port} -m {source}\n\
         Restart=on-failure\n\
         RestartSec=3\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        runner = runner.display(),
    );
    std::fs::write(dir.join(service_name(name)), unit)?;
    Ok(())
}

fn start_service(name: &str) {
    run_user_systemctl(&["daemon-reload"]);
    run_user_systemctl(&["enable", "--now", &service_name(name)]);
}

/// Run `systemctl --user <args>`, warning (not failing) on error so a missing
/// user D-Bus session degrades gracefully — the provider is still registered.
fn run_user_systemctl(args: &[&str]) {
    let mut full = vec!["--user"];
    full.extend_from_slice(args);
    match Command::new("systemctl").args(&full).status() {
        Ok(s) if s.success() => {}
        _ => ui::warn(&format!("systemctl --user {} did not succeed", args.join(" "))),
    }
}

fn registry_path(env: &env_detect::Env) -> PathBuf {
    env.xdg_config.join("8sync/local-models.tsv")
}

fn load_registry(env: &env_detect::Env) -> Vec<LocalModel> {
    let raw = std::fs::read_to_string(registry_path(env)).unwrap_or_default();
    raw.lines()
        .filter_map(|l| {
            let mut it = l.splitn(3, '\t');
            let name = it.next()?.trim();
            if name.is_empty() {
                return None;
            }
            let port: u16 = it.next()?.trim().parse().ok()?;
            let source = it.next().unwrap_or("").trim().to_string();
            Some(LocalModel { name: name.to_string(), port, source })
        })
        .collect()
}

fn save_registry(env: &env_detect::Env, reg: &[LocalModel]) -> Result<()> {
    let p = registry_path(env);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body: String = reg
        .iter()
        .map(|m| format!("{}\t{}\t{}\n", m.name, m.port, m.source))
        .collect();
    std::fs::write(p, body)?;
    Ok(())
}

/// Regenerate the managed local-models block inside `~/.omp/agent/models.yml`
/// from the registry. Preserves everything outside the sentinel block (e.g. the
/// 9router gateway providers).
pub(crate) fn sync_models_yml(env: &env_detect::Env, reg: &[LocalModel]) -> Result<()> {
    let path = env.home.join(".omp/agent/models.yml");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let merged = insert_block(&strip_block(&existing), &render_block(reg));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, merged)?;
    Ok(())
}

/// Remove the managed sentinel block (if present) from a models.yml body.
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

/// Render the managed block for the current registry (empty string if no models).
fn render_block(reg: &[LocalModel]) -> String {
    if reg.is_empty() {
        return String::new();
    }
    let mut b = String::new();
    b.push_str(BLOCK_BEGIN);
    b.push('\n');
    for m in reg {
        b.push_str(&format!(
            "  local-{name}:\n\
             \x20   baseUrl: http://127.0.0.1:{port}/v1\n\
             \x20   apiKey: sk-local\n\
             \x20   api: openai-completions\n\
             \x20   models:\n\
             \x20     - id: local/{name}\n\
             \x20       name: {name} (local GGUF · mistral.rs)\n\
             \x20       input: [text]\n\
             \x20       contextWindow: 32768\n\
             \x20       maxTokens: 8192\n\
             \x20       cost: {{input: 0, output: 0, cacheRead: 0, cacheWrite: 0}}\n",
            name = m.name,
            port = m.port,
        ));
    }
    b.push_str(BLOCK_END);
    b.push('\n');
    b
}

/// Insert the managed block right after the top-level `providers:` line,
/// creating that key (and a header) if the file has none.
pub(crate) fn insert_block(base: &str, block: &str) -> String {
    let base = ensure_providers(base);
    if block.is_empty() {
        return base;
    }
    let mut out = String::new();
    let mut inserted = false;
    for line in base.lines() {
        out.push_str(line);
        out.push('\n');
        if !inserted && line.trim_end() == "providers:" {
            out.push_str(block);
            inserted = true;
        }
    }
    if !inserted {
        out.push_str("providers:\n");
        out.push_str(block);
    }
    out
}

/// Guarantee a top-level `providers:` line exists.
fn ensure_providers(base: &str) -> String {
    if base.lines().any(|l| l.trim_end() == "providers:") {
        return base.to_string();
    }
    if base.trim().is_empty() {
        return "# omp model providers — managed by 8sync (`harness gateway` / `add-local-model`)\nproviders:\n".to_string();
    }
    let mut s = base.to_string();
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s.push_str("providers:\n");
    s
}

/// Poll the OpenAI `/v1/models` endpoint until it returns 200 or the timeout.
fn wait_ready(port: u16, secs: u64) -> bool {
    let url = format!("http://127.0.0.1:{port}/v1/models");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        if let Ok(o) = Command::new("curl")
            .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "--max-time", "3", &url])
            .output()
        {
            if String::from_utf8_lossy(&o.stdout).trim() == "200" {
                return true;
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    false
}

fn list(env: &env_detect::Env) -> Result<()> {
    ui::header("8sync local GGUF models (mistral.rs → omp)");
    let reg = load_registry(env);
    if reg.is_empty() {
        ui::info("none — add one: 8sync harness add-local-model <path.gguf | org/repo | url>");
        return Ok(());
    }
    for m in &reg {
        let active = Command::new("systemctl")
            .args(["--user", "is-active", &service_name(&m.name)])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
            .unwrap_or(false);
        println!(
            "  local/{:<22} port {}  [{}]  {}",
            m.name,
            m.port,
            if active { "serving" } else { "stopped" },
            m.source
        );
    }
    println!();
    ui::info("use: 8sync ai --model local/<name> \"…\"   ·   rm: 8sync harness add-local-model rm <name>");
    Ok(())
}

fn remove(env: &env_detect::Env, name: &str) -> Result<()> {
    let name = slug(name);
    let mut reg = load_registry(env);
    if !reg.iter().any(|m| m.name == name) {
        ui::warn(&format!("no local model named `{name}` (see: add-local-model list)"));
        return Ok(());
    }
    run_user_systemctl(&["disable", "--now", &service_name(&name)]);
    let unit = env.home.join(".config/systemd/user").join(service_name(&name));
    std::fs::remove_file(&unit).ok();
    run_user_systemctl(&["daemon-reload"]);
    reg.retain(|m| m.name != name);
    save_registry(env, &reg)?;
    sync_models_yml(env, &reg)?;
    ui::ok(&format!(
        "removed local model `local/{name}` (service + provider). The GGUF file is left in place."
    ));
    Ok(())
}

/// Lowercase, hyphenate a display name into a safe model/provider id.
fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn usage() {
    ui::header("8sync harness add-local-model — load a GGUF model into omp (mistral.rs)");
    println!("  add : 8sync harness add-local-model <path.gguf | org/repo | https://….gguf> [name]");
    println!("  list: 8sync harness add-local-model list");
    println!("  rm  : 8sync harness add-local-model rm <name>");
    println!();
    ui::info("Loads GGUF via mistral.rs (Rust, memory-safe), serves an OpenAI endpoint,");
    ui::info("and registers it in ~/.omp/agent/models.yml as `local/<name>`.");
    ui::info("Then: 8sync ai --model local/<name> \"…\"  ·  main: 8sync harness model default local/<name>");
}
