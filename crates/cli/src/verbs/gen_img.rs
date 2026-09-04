//! `8sync gen-img <prompt>` — generate image via AI (gpt-image-2 / OpenAI Images API).
//! Downloads from image URL or decodes base64 PNG directly to file.

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use std::path::PathBuf;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync gen-img \"a high tech neon robotic 8 logo\" -o ./logo.png
      8sync gen-img \"architecture diagram of modern microservices\" --size 1024x1024
      8sync gen-img \"a serene Vietnamese lotus pond at dawn\" -o ./pond.png --model gpt-image-2

    USE CASE
      Generate an AI image from a text prompt via `gpt-image-2` (OpenAI / api.apikey.fun).
      Saves directly to a PNG file. VLM models (GLM-5.3-Flash, Opus) can immediately
      inspect the result with `read <path>`.

    REQUIREMENTS
      · An OpenAI-compatible API key set in `OPENAI_API_KEY`, in `~/.omp/agent/models.yml`,
        or passed via `--key`.
"})]
pub struct Args {
    /// Text prompt describing the image to generate.
    pub prompt: String,

    /// Output PNG path.
    #[arg(short, long, default_value = "./generated-image.png")]
    pub output: String,

    /// Model name (default: gpt-image-2).
    #[arg(short, long, default_value = "gpt-image-2")]
    pub model: String,

    /// Image dimensions (e.g. 1024x1024, 512x512, 1536x1024).
    #[arg(short, long, default_value = "1024x1024")]
    pub size: String,

    /// Optional API key override (defaults to OPENAI_API_KEY or models.yml).
    #[arg(short, long)]
    pub key: Option<String>,

    /// Optional API endpoint override (defaults to https://api.apikey.fun/v1/images/generations).
    #[arg(long)]
    pub endpoint: Option<String>,
}

/// Simple RFC 4648 base64 decoder with 0 extra dependencies.
pub(crate) fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for &b in s.as_bytes() {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\r' | b'\n' | b' ' => continue,
            _ => return None,
        };
        buf = (buf << 6) | (v as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// Resolve API key from arguments, environment, or ~/.omp/agent/models.yml.
fn resolve_key(explicit: Option<&str>) -> Option<String> {
    if let Some(k) = explicit {
        if !k.trim().is_empty() {
            return Some(k.trim().to_string());
        }
    }
    if let Ok(k) = std::env::var("OPENAI_API_KEY") {
        if !k.trim().is_empty() {
            return Some(k.trim().to_string());
        }
    }
    // Scan ~/.omp/agent/models.yml
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".omp/agent/models.yml");
        if let Ok(raw) = std::fs::read_to_string(&path) {
            // Find apiKey under codex or openai block
            let mut in_provider = false;
            for line in raw.lines() {
                let trimmed = line.trim();
                if trimmed == "codex:" || trimmed == "openai:" {
                    in_provider = true;
                    continue;
                }
                if in_provider && (line.starts_with("  ") || line.starts_with("    ")) {
                    if let Some(rest) = trimmed.strip_prefix("apiKey:") {
                        let k = rest.trim().trim_matches('"').trim_matches('\'');
                        if !k.is_empty() {
                            return Some(k.to_string());
                        }
                    }
                } else if !line.starts_with("  ") && !line.starts_with("    ") && !line.is_empty() {
                    in_provider = false;
                }
            }
        }
    }
    None
}

/// Resolve API endpoint.
fn resolve_endpoint(explicit: Option<&str>) -> String {
    if let Some(ep) = explicit {
        if !ep.trim().is_empty() {
            return ep.trim().to_string();
        }
    }
    if let Ok(base) = std::env::var("OPENAI_BASE_URL") {
        let b = base.trim().trim_end_matches('/');
        if b.ends_with("/images/generations") {
            return b.to_string();
        }
        return format!("{b}/images/generations");
    }
    "https://api.apikey.fun/v1/images/generations".to_string()
}

pub fn run(a: Args) -> Result<()> {
    let key = resolve_key(a.key.as_deref())
        .context("no OpenAI/Codex API key found — set OPENAI_API_KEY, pass --key, or configure ~/.omp/agent/models.yml")?;

    let endpoint = resolve_endpoint(a.endpoint.as_deref());
    let out_path = PathBuf::from(&a.output);

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create parent dir: {}", parent.display()))?;
        }
    }

    ui::info(&format!(
        "generating image with {} → {} (size: {})",
        a.model,
        out_path.display(),
        a.size
    ));
    ui::info(&format!("prompt: \"{}\"", a.prompt));

    let payload = serde_json::json!({
        "model": a.model,
        "prompt": a.prompt,
        "size": a.size,
        "n": 1
    });
    let payload_str = payload.to_string();

    let output = Command::new("curl")
        .args([
            "-s",
            "-X", "POST",
            &endpoint,
            "-H", &format!("Authorization: Bearer {}", key),
            "-H", "x-openai-actor-authorization: apikey.fun",
            "-H", "Content-Type: application/json",
            "-d", &payload_str,
        ])
        .output()
        .context("failed to execute curl")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("curl failed with exit code {:?}: {}", output.status.code(), stderr);
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let resp: serde_json::Value = serde_json::from_str(&stdout_str)
        .with_context(|| format!("failed to parse API response JSON: {}", stdout_str.chars().take(200).collect::<String>()))?;

    if let Some(err) = resp.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
        bail!("Image API error: {err}");
    }

    let data_arr = resp.get("data")
        .and_then(|d| d.as_array())
        .context("response missing 'data' array")?;

    if data_arr.is_empty() {
        bail!("response 'data' array is empty");
    }

    let first_item = &data_arr[0];

    if let Some(b64) = first_item.get("b64_json").and_then(|b| b.as_str()) {
        let bytes = decode_base64(b64).context("failed to decode base64 image data")?;
        std::fs::write(&out_path, &bytes)
            .with_context(|| format!("failed to write image file: {}", out_path.display()))?;
        ui::ok(&format!(
            "wrote {} ({} KB) from base64 payload",
            out_path.display(),
            bytes.len() / 1024
        ));
    } else if let Some(url) = first_item.get("url").and_then(|u| u.as_str()) {
        ui::info(&format!("downloading image from: {}", url));
        let dl_out = Command::new("curl")
            .args(["-s", "-o", a.output.as_str(), url])
            .output()
            .context("failed to download image from URL")?;
        if !dl_out.status.success() {
            bail!("failed to download image from URL: {}", url);
        }
        let size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
        ui::ok(&format!(
            "downloaded {} ({} KB)",
            out_path.display(),
            size / 1024
        ));
    } else {
        bail!("neither 'b64_json' nor 'url' found in response data: {}", first_item);
    }

    ui::info(&format!(
        "Tip: VLM agents can immediately view this image using `read path: \"{}\"`",
        out_path.display()
    ));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_base64() {
        let raw = b"hello world 8sync image generation";
        let encoded = "aGVsbG8gd29ybGQgOHN5bmMgaW1hZ2UgZ2VuZXJhdGlvbg==";
        let decoded = decode_base64(encoded).unwrap();
        assert_eq!(decoded, raw);
    }

    #[test]
    fn test_resolve_endpoint() {
        assert_eq!(
            resolve_endpoint(Some("https://custom.api/v1/images")),
            "https://custom.api/v1/images"
        );
        assert_eq!(
            resolve_endpoint(None),
            "https://api.apikey.fun/v1/images/generations"
        );
    }
}
