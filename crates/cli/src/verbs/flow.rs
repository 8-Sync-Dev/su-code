use anyhow::Result;
use owo_colors::OwoColorize;

pub fn run() -> Result<()> {
    println!("{}\n", "8sync flow — commands in the order you'll actually use them".bold().cyan());

    section("1. FIRST-TIME INSTALL (new machine)", &[
        ("git clone https://github.com/8-Sync-Dev/su-code", "clone the source"),
        ("cd su-code && bash scripts/bootstrap.sh", "installs rustup if missing, builds 8sync, drops binary into ~/.local/bin"),
        ("8sync setup", "harness (helix/lazygit/abduco/gh) then prompts y/N per personal profile"),
        ("# or  8sync setup --yall", "install everything without prompts (good for re-imaging your own machine)"),
        ("# or  8sync setup --profile alexdev", "apply one specific bundle non-interactively"),
        ("forge login", "paste your AI provider API key into forge"),
        ("gh auth login", "log into GitHub (required by `8sync ship`)"),
        ("8sync doctor", "verify everything is in place"),
    ]);

    section("2. VIBE LOOP — open a project, code with AI, ship a PR", &[
        ("cd ~/code/my-app", ""),
        ("8sync .", "attach or create the project session (kitty 3-pane + forge in abduco)"),
        ("8sync ai \"explain this codebase\"", "AI reads AGENTS.md + agents/* automatically for memory"),
        ("8sync ai \"add login form with email + password\"", "vibe code — forge edits files directly"),
        ("8sync run dev", "start the dev server inside the session"),
        ("8sync shot http://localhost:3000/login", "screenshot the UI so forge can review it visually (cheap on tokens)"),
        ("8sync ai \"fix the z-index on the header\"", "iterate"),
        ("8sync find \"useAuth\"", "search the codebase (rg + fzf preview), pick a match to open at file:line"),
        ("8sync note --tag idea \"switch to zustand for global state\"", "save a thought without breaking flow"),
        ("8sync ship \"feat: login form\"", "git add -A + commit + push + open a GitHub PR"),
        ("8sync end", "have forge summarize the session into agents/{DECISIONS,KNOWLEDGE,...}.md"),
    ]);

    section("3. RESUME later (next day, after reboot)", &[
        ("cd ~/code/my-app", ""),
        ("8sync .", "forge re-reads AGENTS.md + agents/* and picks up where you left off"),
    ]);

    section("4. PARALLEL SESSIONS (work on two things at once)", &[
        ("8sync . ls",                "list all live sessions"),
        ("8sync . to other-project",  "switch to another project's session"),
        ("8sync . new hotfix forge",  "spawn a detached forge session named `hotfix`"),
        ("8sync . rm hotfix",         "kill and remove session `hotfix`"),
        ("8sync . wipe",              "kill all sessions belonging to the current project"),
    ]);

    section("5. LOOK & FEEL (handled by HyDE, not 8sync)", &[
        ("hydectl wallpaper next",      "change wallpaper (HyDE built-in)"),
        ("hydectl wallpaper select",    "pick wallpaper from gallery"),
        ("hydectl theme set <name>",    "switch theme (propagates to kitty/gtk/qt via wallbash)"),
        ("hydectl theme next",          "rotate to next theme"),
    ]);

    section("6. SECURITY (VPN + firewall)", &[
        ("8sync sec",                   "show current status of WARP and ufw"),
        ("8sync sec on",                "enable WARP VPN + ufw firewall (going to a cafe)"),
        ("8sync sec off",               "disable both (back home)"),
        ("8sync sec toggle",            "flip both based on their current state"),
        ("8sync sec warp on",           "control WARP only"),
        ("8sync sec ufw status",        "show ufw status only"),
    ]);

    section("7. MAINTENANCE", &[
        ("8sync up",                       "self-update the 8sync binary and forge (no `pacman -Syu`)"),
        ("8sync doctor",                   "full health check"),
        ("8sync skill",                    "list installed skills + auto-inject status"),
        ("8sync skill sync",               "refresh ~/.forge/skills/00-force-load.md"),
        ("8sync setup profile list",       "show all profiles and which are applied"),
        ("8sync setup profile show warp",  "show resolved content of a profile"),
        ("8sync setup profile apply warp", "(re-)apply a profile idempotently"),
    ]);

    println!("Every verb supports {} and {} for detailed help.", "-h".bold().green(), "--help".bold().green());
    println!("Show this page anytime: {} or {}.", "8sync flow".bold().cyan(), "8sync".bold().cyan());
    Ok(())
}

fn section(title: &str, rows: &[(&str, &str)]) {
    println!("{}", title.bold().yellow());
    let w = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(20).min(45);
    for (cmd, desc) in rows {
        if desc.is_empty() {
            println!("  {}", cmd.cyan());
        } else {
            println!("  {:<w$}  {}", cmd.cyan(), desc.dimmed(), w = w);
        }
    }
    println!();
}
