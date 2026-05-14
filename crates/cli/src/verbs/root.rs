use owo_colors::OwoColorize;

pub fn print_cheatsheet() {
    println!(
        "{}\n",
        "8sync — vibe coding harness for CachyOS + Kitty + Helix".bold().cyan()
    );

    println!("{}", "Vibe loop (daily):".bold().yellow());
    rows(&[
        (".",      "open/attach project session (kitty + forge in abduco)"),
        ("ai",     "ai prompt / resume session"),
        ("find",   "rg + fzf preview, pick → open in helix"),
        ("note",   "append idea to agents/NOTES.md"),
        ("run",    "dev | build | test | fmt | lint"),
        ("ship",   "git add + commit + push + PR"),
        ("end",    "AI capture knowledge → agents/*.md"),
    ]);

    println!("\n{}", "Session mgmt (subcommands of `.`):".bold().yellow());
    rows(&[
        (". ls",      "list live sessions"),
        (". to <n>",  "switch to another session"),
        (". new <n>", "create detached session"),
        (". rm <n>",  "kill + remove"),
        (". wipe",    "kill all of current project"),
    ]);

    println!("\n{}", "Look & feel:".bold().yellow());
    rows(&[
        ("bg",     "wallpaper: search / pick / set / opacity"),
        ("look",   "preset: neon | ice | mint | dark | dim"),
    ]);

    println!("\n{}", "Lifecycle:".bold().yellow());
    rows(&[
        ("setup",  "install everything (run once)"),
        ("up",     "update tools (idempotent)"),
        ("doctor", "health check"),
        ("flow",   "workflow help theo thứ tự dùng"),
    ]);

    println!("\n{}", "AI tooling:".bold().yellow());
    rows(&[
        ("skill",    "list / add / sync skills"),
        ("shot",     "render web/file to PNG"),
        ("diff-img", "render git diff to PNG"),
        ("pdf-img",  "render PDF pages to PNG"),
        ("mcp",      "MCP server for forge/cursor/opencode"),
    ]);

    println!("\nMọi verb có {} hoặc {}.", "-h".bold().green(), "--help".bold().green());
    println!("Lần đầu: {}", "8sync setup".bold().cyan());
    println!("Xem flow: {}", "8sync flow".bold().cyan());
}

fn rows(items: &[(&str, &str)]) {
    let w = items.iter().map(|(k, _)| k.len()).max().unwrap_or(8);
    for (k, v) in items {
        println!("  {:<width$}  {}", k.cyan().bold(), v, width = w);
    }
}
