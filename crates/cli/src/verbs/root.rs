use owo_colors::OwoColorize;

pub fn print_cheatsheet() {
    println!(
        "{}\n",
        "8sync — vibe coding harness for CachyOS + Kitty + Helix".bold().cyan()
    );
    println!("{}", "Daily:".bold());
    rows(&[
        ("setup",  "install everything (run once)"),
        ("up",     "update tools (idempotent)"),
        ("doctor", "health check"),
        (".",      "open project session (kitty + forge)"),
        ("ai",     "ai prompt / resume session"),
        ("ship",   "commit + push + PR"),
        ("run",    "dev | build | test | fmt | lint"),
        ("bg",     "wallpaper: search / pick / set / opacity"),
        ("look",   "style preset: neon | ice | mint | dark | dim"),
        ("end",    "capture knowledge, close session"),
    ]);
    println!("\n{}", "Skill & context:".bold());
    rows(&[
        ("skill",    "list / add / sync skills"),
        ("shot",     "render web/file to PNG"),
        ("diff-img", "render git diff to PNG"),
        ("pdf-img",  "render PDF pages to PNG"),
        ("mcp",      "run MCP server for forge/cursor/opencode"),
    ]);
    println!("\nEvery verb supports {}.", "-h".bold().green());
    println!("Try: {}", "8sync setup".bold().cyan());
}

fn rows(items: &[(&str, &str)]) {
    let w = items.iter().map(|(k, _)| k.len()).max().unwrap_or(8);
    for (k, v) in items {
        println!("  {:<width$}  {}", k.cyan().bold(), v, width = w);
    }
}
