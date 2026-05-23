use owo_colors::OwoColorize;

pub fn print_cheatsheet() {
    println!(
        "{}",
        "8sync — vibe coding harness for CachyOS + omp".bold().cyan()
    );
    println!("{}\n", "Run any verb with `-h` for detailed help and examples.".dimmed());

    println!("{}", "Vibe loop (daily):".bold().yellow());
    rows(&[
        (".",      "seed agents/* context, then exec `omp --continue` in the project root"),
        ("ai",     "AI session — one-shot prompt or resume last omp chat"),
        ("find",   "search code (rg) or files (fd), preview in fzf, open at file:line"),
        ("note",   "append a timestamped line to <repo>/agents/NOTES.md"),
        ("run",    "project runner: dev | build | test | fmt | lint"),
        ("ship",   "git add -A + commit + push + `gh pr create` in one shot"),
    ]);

    println!("\n{}", "Security (VPN + firewall):".bold().yellow());
    rows(&[
        ("sec",         "show WARP and ufw status"),
        ("sec on",      "enable both WARP VPN and ufw firewall"),
        ("sec off",     "disable both"),
        ("sec toggle",  "flip both based on their current state"),
        ("sec warp on", "control WARP only (also: off | status)"),
        ("sec ufw on",  "control ufw only (also: off | status)"),
    ]);

    println!("\n{}", "Lifecycle:".bold().yellow());
    rows(&[
        ("setup",                   "install harness (gh + omp), then ask y/N per personal profile"),
        ("setup --yall",            "install harness + alexdev bundle, no prompts"),
        ("setup --profile <name>",  "install harness + apply one profile non-interactively"),
        ("setup --caelestia",       "auto-detect: HyDE → additive overlay, else fresh Hyprland+Caelestia (+nvidia)"),
        ("setup --caelestia=fresh", "force fresh CachyOS path"),
        ("setup --caelestia=hyde",  "force HyDE-additive overlay"),
        ("setup --caelestia=rollback", "remove HyDE overlay block"),
        ("setup --end4=<tier>",     "end-4/dots-hyprland (minimal|medium|full|rollback) — auto-yes"),
        ("setup --dry-run",         "print the plan without changing anything"),
        ("setup profile",           "manage profiles: list | show <name> | apply <name>"),
        ("up",                      "update 8sync binary + omp (system pkgs not touched)"),
        ("doctor",                  "health check: tools, configs, VPN/firewall, profiles"),
        ("flow",                    "workflow help in chronological order"),
    ]);

    println!("\n{}", "AI tooling (for omp):".bold().yellow());
    rows(&[
        ("skill",    "list / add / sync omp skills (karpathy, image-routing, 8sync-cli)"),
        ("shot",     "render a URL or HTML file to PNG (cheap visual context for AI)"),
        ("diff-img", "render `git diff` to PNG"),
        ("pdf-img",  "render PDF pages to PNG"),
    ]);

    println!("\n{}", "Tips:".bold().yellow());
    println!("  · Every verb has {} and {} flags.", "-h".bold().green(), "--help".bold().green());
    println!("  · First time?         run {}", "8sync setup".bold().cyan());
    println!("  · Want workflow tour? run {}", "8sync flow".bold().cyan());
    println!("  · Show this page:     run {} or {}", "8sync".bold().cyan(), "8sync help".bold().cyan());
}

fn rows(items: &[(&str, &str)]) {
    let w = items.iter().map(|(k, _)| k.len()).max().unwrap_or(8);
    for (k, v) in items {
        println!("  {:<width$}  {}", k.cyan().bold(), v, width = w);
    }
}
