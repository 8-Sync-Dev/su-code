use owo_colors::OwoColorize;

pub fn print_cheatsheet() {
    println!(
        "{}",
        "8sync — vibe coding harness for CachyOS + Kitty + Helix + Forge".bold().cyan()
    );
    println!("{}\n", "Run any verb with `-h` for detailed help and examples.".dimmed());

    println!("{}", "Vibe loop (daily):".bold().yellow());
    rows(&[
        (".",      "open/attach project session (kitty 3-pane + forge in abduco)"),
        ("ai",     "AI session — one-shot prompt or resume last forge chat"),
        ("find",   "search code (rg) or files (fd), preview in fzf, open at file:line"),
        ("note",   "append a timestamped line to <repo>/agents/NOTES.md"),
        ("run",    "project runner: dev | build | test | fmt | lint"),
        ("ship",   "git add -A + commit + push + `gh pr create` in one shot"),
        ("end",    "capture session knowledge into agents/{STATE,KNOWLEDGE,...}.md"),
    ]);

    println!("\n{}", "Session management (subcommands of `.`):".bold().yellow());
    rows(&[
        (". ls",         "list live sessions (abduco-backed) for current project"),
        (". to <name>",  "switch / attach a different named session"),
        (". new <name>", "create a detached session (e.g. for a parallel task)"),
        (". rm <name>",  "kill a session and remove its abduco socket"),
        (". wipe",       "kill every session belonging to the current project"),
        (". kick",       "detach any current attach (frees the socket)"),
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
        ("setup",          "install harness, then ask y/N for each personal profile"),
        ("setup --yall",   "install harness + ALL profiles, no prompts (yes-to-all)"),
        ("setup --profile <name>",
                           "install harness + apply one profile non-interactively"),
        ("setup --dry-run","print the plan without changing anything"),
        ("setup profile",  "manage profiles: list | show <name> | apply <name>"),
        ("up",             "update 8sync binary + forge (system pkgs not touched)"),
        ("doctor",         "health check: tools, configs, VPN/firewall, profiles"),
        ("flow",           "workflow help in chronological order"),
    ]);

    println!("\n{}", "AI tooling (for forge):".bold().yellow());
    rows(&[
        ("skill",    "list / add / sync forge skills (karpathy, image-routing, 8sync-cli)"),
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
