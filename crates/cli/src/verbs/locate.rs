//! `8sync locate <image> "<prompt>"` — fast open-vocabulary visual grounding via
//! NVIDIA LocateAnything-3B, run through mudler/locate-anything.cpp (an MIT ggml
//! port with prebuilt GGUFs — no Python at inference time). Turns an image + a
//! text description into labeled boxes, so the agent gets exact pixel coordinates
//! (GUI element grounding → click points, OCR/text localization, object detection)
//! instead of eyeballing a screenshot. Complements `8sync shot` and zai-vision.
//!
//! Setup (`--setup`) clones + cmake-builds the CLI and downloads the q8_0 GGUF.
//! The model is under NVIDIA's non-commercial / research license.
use anyhow::{anyhow, bail, Context, Result};
use clap::Args as ClapArgs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ui;

const MODEL_URL: &str =
    "https://huggingface.co/mudler/locate-anything.cpp-gguf/resolve/main/locate-anything-q8_0.gguf";

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync locate --setup                       one-time: build locate-anything.cpp + fetch the GGUF (~6.3 GB)
      8sync locate ui.png \"the Sign in button\"    grounding → box + click-center coordinates
      8sync locate street.jpg \"person</c>car\"     multi-category (separate with </c>)
      8sync locate ui.png \"submit button\" --annotated boxed.png   also draw the boxes

    PIPELINE
      8sync shot http://localhost:3000 -o /tmp/ui.png && 8sync locate /tmp/ui.png \"the search field\"

    NOTE  Model: NVIDIA LocateAnything-3B — research / non-commercial use only.
"})]
pub struct Args {
    /// Image file to analyze (png/jpg). Optional when using --setup alone.
    pub image: Option<String>,
    /// Open-vocabulary description of what to find (e.g. "the login button").
    pub prompt: Option<String>,
    /// One-time install: clone + build locate-anything.cpp and download the GGUF.
    #[arg(long)]
    pub setup: bool,
    /// Also write an annotated PNG with the boxes drawn.
    #[arg(long)]
    pub annotated: Option<String>,
    /// Decode mode: hybrid (default, Parallel Box Decoding) | slow | fast.
    #[arg(long, default_value = "hybrid")]
    pub mode: String,
}

pub fn run(a: Args) -> Result<()> {
    let base = dirs::home_dir()
        .context("no HOME")?
        .join(".cache/8sync/locate-anything");

    if a.setup {
        setup(&base)?;
        if a.image.is_none() {
            return Ok(());
        }
    }

    let (image, prompt) = match (a.image.as_deref(), a.prompt.as_deref()) {
        (Some(i), Some(p)) => (i, p),
        (Some(_), None) => {
            ui::warn("need a prompt: 8sync locate <image> \"<what to find>\"");
            return Ok(());
        }
        _ => {
            usage();
            return Ok(());
        }
    };

    let cli = find_cli(&base)
        .ok_or_else(|| anyhow!("locate-anything-cli not found — run: 8sync locate --setup"))?;
    let model = find_model(&base)
        .ok_or_else(|| anyhow!("no LocateAnything GGUF — run: 8sync locate --setup"))?;
    if !Path::new(image).exists() {
        bail!("image not found: {image}");
    }

    let out_json = std::env::temp_dir().join("8sync-locate.json");
    ui::info(&format!(
        "detecting `{prompt}` in {image} (LocateAnything-3B · ggml, no Python) …"
    ));
    let mut cmd = Command::new(&cli);
    cmd.arg("detect")
        .arg("--model")
        .arg(&model)
        .arg("--input")
        .arg(image)
        .arg("--prompt")
        .arg(prompt)
        .arg("--mode")
        .arg(&a.mode)
        .arg("--output")
        .arg(&out_json);
    if let Some(ann) = &a.annotated {
        cmd.arg("--annotated").arg(ann);
    }
    let st = cmd.status().context("run locate-anything-cli")?;
    if !st.success() {
        bail!("locate-anything-cli exited non-zero");
    }

    print_detections(&std::fs::read_to_string(&out_json).unwrap_or_default());
    if let Some(ann) = &a.annotated {
        ui::ok(&format!("annotated image → {ann}"));
    }
    Ok(())
}

/// Pretty-print the CLI's `{"detections":[{"label","box":[x1,y1,x2,y2]}]}` output,
/// adding a click-center per box (handy for driving the browser tool).
fn print_detections(raw: &str) {
    let v: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => {
            if !raw.trim().is_empty() {
                println!("{raw}");
            } else {
                ui::warn("no output from locate-anything-cli");
            }
            return;
        }
    };
    match v.get("detections").and_then(|d| d.as_array()) {
        Some(arr) if !arr.is_empty() => {
            ui::ok(&format!("{} detection(s):", arr.len()));
            for d in arr {
                let label = d.get("label").and_then(|l| l.as_str()).unwrap_or("?");
                let nums: Vec<f64> = d
                    .get("box")
                    .and_then(|b| b.as_array())
                    .map(|b| b.iter().filter_map(|x| x.as_f64()).collect())
                    .unwrap_or_default();
                if nums.len() == 4 {
                    let (cx, cy) = ((nums[0] + nums[2]) / 2.0, (nums[1] + nums[3]) / 2.0);
                    println!(
                        "  {:<24} box [{:.0}, {:.0}, {:.0}, {:.0}]  click≈({:.0}, {:.0})",
                        label, nums[0], nums[1], nums[2], nums[3], cx, cy
                    );
                } else {
                    println!(
                        "  {label}  {}",
                        d.get("box").map(|b| b.to_string()).unwrap_or_default()
                    );
                }
            }
        }
        _ => ui::warn("no detections for that prompt"),
    }
}

/// Clone + build locate-anything.cpp and download the q8_0 GGUF model.
fn setup(base: &Path) -> Result<()> {
    ui::header("8sync locate — set up LocateAnything-3B (ggml port)");
    ui::warn("Model license: NVIDIA LocateAnything-3B is research / non-commercial use only.");
    let src = base.join("src");
    let models = base.join("models");
    std::fs::create_dir_all(&models)?;

    // 1. Clone (shallow, with the ggml submodule).
    if !src.join(".git").exists() {
        ui::step("cloning mudler/locate-anything.cpp …");
        sh("git", &[
            "clone",
            "--recursive",
            "--depth",
            "1",
            "https://github.com/mudler/locate-anything.cpp",
            &src.to_string_lossy(),
        ])?;
    } else {
        ui::skip("locate-anything.cpp", "already cloned");
    }

    // 2. Build the CLI (CUDA if the toolkit is present, else CPU — always builds).
    let cuda = which::which("nvcc").is_ok();
    ui::step(&format!(
        "building locate-anything-cli (cmake · {}) …",
        if cuda { "CUDA" } else { "CPU" }
    ));
    let mut cfg = vec![
        "-B".to_string(),
        "build".to_string(),
        "-DLA_BUILD_CLI=ON".to_string(),
    ];
    if cuda {
        cfg.push("-DLA_GGML_CUDA=ON".to_string());
    }
    run_in(&src, "cmake", &cfg.iter().map(String::as_str).collect::<Vec<_>>())?;
    run_in(&src, "cmake", &["--build", "build", "-j"])?;
    let cli = walk_find(&src.join("build"), "locate-anything-cli")
        .ok_or_else(|| anyhow!("build finished but locate-anything-cli not found under {}/build", src.display()))?;
    ui::ok(&format!("built {}", cli.display()));

    // 3. Download the recommended q8_0 GGUF (box-identical, ~6.3 GB).
    let gguf = models.join("locate-anything-q8_0.gguf");
    if gguf.exists() {
        ui::skip("model", "already downloaded");
    } else {
        ui::step("downloading locate-anything-q8_0.gguf (~6.3 GB) from HuggingFace …");
        sh("curl", &["-fL", "--progress-bar", "-o", &gguf.to_string_lossy(), MODEL_URL])?;
    }
    ui::ok("LocateAnything ready — 8sync locate <image> \"<what to find>\"");
    Ok(())
}

/// Find `locate-anything-cli`: PATH, then the built copy under the cache.
fn find_cli(base: &Path) -> Option<PathBuf> {
    if let Ok(p) = which::which("locate-anything-cli") {
        return Some(p);
    }
    walk_find(&base.join("src/build"), "locate-anything-cli")
}

/// First `*.gguf` under the cache models dir.
fn find_model(base: &Path) -> Option<PathBuf> {
    let dir = base.join("models");
    std::fs::read_dir(&dir).ok()?.flatten().find_map(|e| {
        let p = e.path();
        (p.extension().and_then(|x| x.to_str()) == Some("gguf")).then_some(p)
    })
}

/// Recursively search `dir` for a file named `name` (bounded — build trees are small).
fn walk_find(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            subdirs.push(p);
        } else if p.file_name().and_then(|f| f.to_str()) == Some(name) {
            return Some(p);
        }
    }
    subdirs.iter().find_map(|d| walk_find(d, name))
}

fn sh(bin: &str, args: &[&str]) -> Result<()> { let st = Command::new(bin)
    .args(args)
    .status()
    .with_context(|| format!("run {bin}"))?;
if !st.success() {
    bail!("{bin} {} failed", args.join(" "));
}
Ok(()) }

fn run_in(dir: &Path, bin: &str, args: &[&str]) -> Result<()> {
    let st = Command::new(bin)
        .current_dir(dir)
        .args(args)
        .status()
        .with_context(|| format!("run {bin} in {}", dir.display()))?;
    if !st.success() {
        bail!("{bin} {} failed", args.join(" "));
    }
    Ok(())
}

fn usage() {
    ui::header("8sync locate — visual grounding (LocateAnything-3B)");
    println!("  setup : 8sync locate --setup");
    println!("  find  : 8sync locate <image> \"<what to find>\" [--annotated out.png] [--mode hybrid|slow|fast]");
    println!();
    ui::info("Returns labeled boxes + click-center coords from an image (GUI grounding, OCR, detection).");
    ui::info("Pipe with shot: 8sync shot <url> -o /tmp/ui.png && 8sync locate /tmp/ui.png \"the button\"");
}
