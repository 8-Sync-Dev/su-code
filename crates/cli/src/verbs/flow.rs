use anyhow::Result;
use owo_colors::OwoColorize;

pub fn run() -> Result<()> {
    println!("{}\n", "8sync flow — commands in the order you'll actually use them".bold().cyan());

    section("1. FIRST-TIME INSTALL (new machine)", &[
        ("git clone https://github.com/8-Sync-Dev/su-code", "clone the source"),
        ("cd su-code && bash scripts/bootstrap.sh", "installs rustup if missing, builds 8sync, drops binary into ~/.local/bin"),
        ("8sync setup", "harness (gh + omp + skills) then prompts y/N per personal profile"),
        ("# or  8sync setup --yall", "install harness + alexdev bundle without prompts"),
        ("# or  8sync setup --profile alexdev", "apply one specific bundle non-interactively"),
        ("# or  8sync setup --caelestia", "auto-detect: HyDE → additive overlay; else fresh Hyprland+Caelestia (+nvidia)"),
        ("gh auth login", "log into GitHub (required by `8sync ship`)"),
        ("8sync doctor", "verify everything is in place"),
    ]);

    section("2. VIBE LOOP — open a project, code with AI, ship a PR", &[
        ("cd ~/code/my-app", ""),
        ("8sync .", "seed agents/* memory + run `omp --continue` (omp manages its own session)"),
        ("8sync ai \"explain this codebase\"", "AI reads AGENTS.md + agents/* automatically for memory"),
        ("8sync ai \"add login form with email + password\"", "vibe code — omp edits files directly"),
        ("8sync run dev", "start the dev server"),
        ("8sync shot http://localhost:3000/login", "screenshot the UI so omp can review it visually (cheap on tokens)"),
        ("8sync ai \"fix the z-index on the header\"", "iterate"),
        ("8sync find \"useAuth\"", "search the codebase (rg + fzf preview), pick a match to open at file:line"),
        ("8sync note --tag idea \"switch to zustand for global state\"", "save a thought without breaking flow"),
        ("8sync ship \"feat: login form\"", "git add -A + commit + push + open a GitHub PR"),
    ]);

    section("3. RESUME later (next day, after reboot)", &[
        ("cd ~/code/my-app", ""),
        ("8sync .", "omp re-reads AGENTS.md + agents/* and picks up where you left off"),
    ]);

    section("4. CAELESTIA DESKTOP (optional)", &[
        ("8sync setup --caelestia",          "auto-detect: HyDE present → additive; else fresh full stack"),
        ("8sync setup --caelestia=fresh",    "force fresh CachyOS path (Hyprland + Quickshell + nvidia auto-detect)"),
        ("8sync setup --caelestia=hyde",     "force HyDE additive overlay (caelestia-shell + userprefs override)"),
        ("8sync setup --caelestia=rollback", "remove HyDE overlay, restart waybar"),
    ]);

    section("4b. END-4/DOTS-HYPRLAND (optional)", &[
        ("8sync setup --end4",          "default medium tier (Hyprland + Quickshell, skip fish/fonts/misc)"),
        ("8sync setup --end4=minimal",  "bare Hyprland keybinds, no widget shell"),
        ("8sync setup --end4=full",     "everything upstream installs"),
        ("8sync setup --end4=rollback", "run upstream `./setup uninstall -f`"),
    ]);

    section("5. SECURITY (VPN + firewall)", &[
        ("8sync sec",                   "show current status of WARP and ufw"),
        ("8sync sec on",                "enable WARP VPN + ufw firewall (going to a cafe)"),
        ("8sync sec off",               "disable both (back home)"),
        ("8sync sec toggle",            "flip both based on their current state"),
        ("8sync sec warp on",           "control WARP only"),
        ("8sync sec ufw status",        "show ufw status only"),
    ]);

    section("6. MAINTENANCE", &[
        ("8sync up",                       "self-update the 8sync binary and omp (no `pacman -Syu`)"),
        ("8sync doctor",                   "full health check"),
        ("8sync skill",                    "list installed skills + project-local skills"),
        ("8sync skill add <url>",          "clone a skill repo into ~/.omp/skills/ and project agents/skills/"),
        ("8sync skill sync",               "refresh ~/.omp/skills/00-force-load.md"),
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
