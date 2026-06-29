// Embed git commit at build time so the binary can compare against upstream.
fn main() {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None })
        .and_then(|b| String::from_utf8(b).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", commit);

    // Rebuild the Vite FE when dist is missing OR web/src changed, so edits can never
    // silently ship a stale bundle (build.rs previously built only when dist was
    // absent). rust-embed (assets.rs) then embeds the fresh web/dist.
    let web_dir = std::path::Path::new("../../web");
    let web_dist = web_dir.join("dist/index.html");
    if !web_dist.exists() || web_src_newer(web_dir, &web_dist) {
        build_web_fe(web_dir);
    }
    // Guarantee dist exists so rust-embed compiles; embed a styled, instructive
    // fallback when no JS toolchain (bun/pnpm/npm) was available to build the FE.
    if !web_dist.exists() {
        if let Some(p) = web_dist.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(&web_dist, FALLBACK_HTML);
        println!("cargo:warning=8sync: web FE not built (no bun/pnpm/npm found) — embedded fallback page; install bun then rebuild for the full dashboard");
    }
    println!("cargo:rerun-if-changed=../../web/dist/index.html");
    println!("cargo:rerun-if-changed=../../web/src");
    println!("cargo:rerun-if-changed=../../web/index.html");
    println!("cargo:rerun-if-changed=../../web/package.json");
    println!("cargo:rerun-if-changed=../../web/vite.config.ts");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads/main");
}

/// Try bun → pnpm → npm to install deps + build the Vite FE. The first toolchain
/// that produces web/dist/index.html wins; every failure is non-fatal so a plain
/// `cargo build` still succeeds on machines without a JS toolchain (fallback embeds).
fn build_web_fe(web_dir: &std::path::Path) {
    let dist = web_dir.join("dist/index.html");
    // (binary, install args, build args)
    let chains: &[(&str, &[&str], &[&str])] = &[
        ("bun", &["install"], &["run", "build"]),
        ("pnpm", &["install"], &["run", "build"]),
        ("npm", &["install", "--no-audit", "--no-fund"], &["run", "build"]),
    ];
    for (bin, install, build) in chains {
        if which_bin(bin).is_none() {
            continue;
        }
        println!("cargo:warning=8sync: building web FE with {bin} …");
        let installed = std::process::Command::new(bin)
            .args(*install)
            .current_dir(web_dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !installed {
            continue;
        }
        let built = std::process::Command::new(bin)
            .args(*build)
            .current_dir(web_dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if built && dist.exists() {
            println!("cargo:warning=8sync: web FE built with {bin} → web/dist");
            return;
        }
    }
}

/// Minimal PATH lookup (no `which` crate in the build script).
fn which_bin(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(bin);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

/// Newest mtime under a directory tree (recursive), UNIX_EPOCH if unreadable.
fn newest_in_dir(dir: &std::path::Path) -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let m = if p.is_dir() { newest_in_dir(&p) } else { file_mtime(&p) };
            if m > newest {
                newest = m;
            }
        }
    }
    newest
}

fn file_mtime(p: &std::path::Path) -> std::time::SystemTime {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

/// True when any FE source (web/src tree or key config) is newer than the built
/// dist — so a `cargo build` after editing web/src rebuilds the bundle.
fn web_src_newer(web_dir: &std::path::Path, dist: &std::path::Path) -> bool {
    let dist_m = file_mtime(dist);
    let mut newest = newest_in_dir(&web_dir.join("src"));
    for f in ["package.json", "vite.config.ts", "index.html", "tsconfig.json"] {
        let m = file_mtime(&web_dir.join(f));
        if m > newest {
            newest = m;
        }
    }
    newest > dist_m
}

const FALLBACK_HTML: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>8sync harness</title><style>
:root{color-scheme:dark}*{box-sizing:border-box}
body{margin:0;min-height:100vh;display:grid;place-items:center;font:15px/1.6 system-ui,-apple-system,sans-serif;
color:#e6e9ef;background:radial-gradient(1100px 760px at 72% -12%,#1b1f3a,#0b0d12)}
.card{max-width:540px;margin:24px;padding:34px 38px;border-radius:18px;background:rgba(20,23,31,.62);
backdrop-filter:blur(14px);-webkit-backdrop-filter:blur(14px);border:1px solid rgba(255,255,255,.08);
box-shadow:0 24px 70px rgba(0,0,0,.55)}
h1{margin:0 0 10px;font-size:20px;letter-spacing:.2px}p{color:#9aa3b2;margin:8px 0}
code{background:rgba(124,92,255,.14);padding:2px 8px;border-radius:7px;font:13px ui-monospace,Menlo,monospace;color:#b9c0ff}</style>
</head><body><div class="card"><h1>8sync harness · dashboard</h1>
<p>The web frontend was not compiled into this binary (no JS toolchain at build time).</p>
<p>Install <code>bun</code> and rebuild:</p>
<p><code>curl -fsSL https://bun.sh/install | bash</code></p>
<p><code>bun --cwd web install &amp;&amp; bun --cwd web run build</code></p>
<p><code>cargo build --release</code></p></div></body></html>"#;
