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

    // Build the Vite FE if dist is missing, so rust-embed (in assets.rs) can include it.
    let web_dist = std::path::Path::new("../../web/dist/index.html");
    if !web_dist.exists() {
        let r = std::process::Command::new("pnpm")
            .args(["--dir", "../../web", "run", "build"])
            .status();
        match r {
            Ok(s) if s.success() => println!("cargo:warning=8sync web FE built → web/dist"),
            _ => println!("cargo:warning=8sync web FE build skipped — run `pnpm --dir web build`"),
        }
    }
    // Always ensure the dist folder exists so rust-embed compiles; stub if build failed.
    if !web_dist.exists() {
        if let Some(p) = web_dist.parent() { let _ = std::fs::create_dir_all(p); }
        let _ = std::fs::write(
            web_dist,
            "<!doctype html><meta charset=utf-8><title>8sync harness web</title>\
             <p>8sync harness web FE not built. Run <code>pnpm --dir web build</code> then rebuild.</p>",
        );
        println!("cargo:warning=wrote stub web/dist/index.html (FE build unavailable)");
    }
    println!("cargo:rerun-if-changed=../../web/dist/index.html");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads/main");
}
