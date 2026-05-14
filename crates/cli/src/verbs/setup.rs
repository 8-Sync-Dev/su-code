use anyhow::Result;
use clap::Args as ClapArgs;

use crate::{assets, env_detect, pkg, ui};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync setup
          8sync setup --dry-run
          8sync setup --minimal
          8sync setup --no-warp --no-mobile --no-db
    "}
)]
pub struct Args {
    /// Print the plan without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Skip everything optional (mobile, db, warp)
    #[arg(long)]
    pub minimal: bool,

    /// Skip mobile dev (jdk, android-tools)
    #[arg(long)]
    pub no_mobile: bool,

    /// Skip local databases (postgresql, valkey)
    #[arg(long)]
    pub no_db: bool,

    /// Skip Cloudflare WARP
    #[arg(long)]
    pub no_warp: bool,

    /// Update outdated packages instead of skipping
    #[arg(long, short = 'u')]
    pub update: bool,
}

pub fn run(a: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    ui::header("8sync setup");

    if !env.is_cachyos_or_arch() {
        ui::warn(&format!(
            "Detected OS `{}` — su-code targets CachyOS/Arch. Some steps may fail.",
            env.os_id
        ));
    }

    let mobile = !a.minimal && !a.no_mobile;
    let db = !a.minimal && !a.no_db;
    let warp = !a.minimal && !a.no_warp;

    println!("Plan:");
    println!("  · core packages (kitty/helix/git/...)");
    if mobile { println!("  · mobile dev (jdk, android-tools)"); }
    if db     { println!("  · databases (postgresql, valkey)"); }
    if warp   { println!("  · cloudflare-warp + DoH MASQUE + auto-start"); }
    println!("  · forge (curl install)");
    println!("  · configs (kitty/helix/fish/im)");
    println!("  · wallpaper (bundle default)");
    println!("  · skills (karpathy, image-routing, 8sync-cli)");
    println!("  · systemd-user (8sync-mcp.service)");
    println!();

    if a.dry_run {
        ui::info("dry-run — exiting");
        return Ok(());
    }

    if !ui::prompt_yes_no("Continue?", true) {
        ui::info("aborted");
        return Ok(());
    }

    // 1. core pacman packages
    ui::step("Core packages");
    let mut core: Vec<&str> = vec![
        "kitty", "helix", "git", "github-cli", "lazygit",
        "nodejs", "npm", "pnpm", "bun", "docker", "docker-compose",
        "ripgrep", "fd", "fzf", "eza", "bat", "jq", "fastfetch", "btop",
        "zoxide", "protobuf", "unzip", "zip", "ufw", "fish",
        // python tooling (uv installs itself, but base python ok)
        "python", "python-pip",
        // image tooling for `8sync shot/pdf-img`
        "poppler", "imagemagick",
        // detached sessions (live across terminal close, replaces tmux)
        "abduco",
        // image-routing helpers
        "curl",
    ];
    if mobile { core.extend_from_slice(&["jdk-openjdk", "android-tools", "android-udev"]); }
    if db     { core.extend_from_slice(&["postgresql", "valkey"]); }
    pkg::pacman_ensure(&core, a.update)?;

    // 2. paru + AUR
    if warp {
        ui::step("paru + cloudflare-warp-bin");
        pkg::ensure_paru()?;
        pkg::paru_ensure(&["cloudflare-warp-bin"])?;
    }

    // 3. forge (curl installer if missing)
    install_forge(a.update)?;

    // 4. configs
    install_configs(&env)?;

    // 5. wallpaper bundle
    install_wallpaper(&env)?;

    // 6. skills
    install_skills(&env)?;

    // 7. services
    enable_services(&env, warp)?;

    // 8. user groups
    pkg::run_loud("sudo", &["usermod", "-aG", "docker", &whoami()])?;

    ui::header("Done — next steps");
    println!("  1. {} {} (remote control needs full restart, not reload)",
             "Close & reopen Kitty once".bold_str(), "—".bright_black_str());
    println!("  2. Reboot or re-login (for docker group)");
    println!("  3. {}", "forge login".bold_str());
    println!("  4. {}", "8sync doctor".bold_str());
    println!("  5. cd into a project and run {}", "8sync .".bold_str());
    Ok(())
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "alexdev".to_string())
}

fn install_forge(update: bool) -> Result<()> {
    ui::step("forge AI CLI");
    if which::which("forge").is_ok() && !update {
        let v = env_detect::cmd_version("forge", &["--version"]).unwrap_or_default();
        ui::skip("forge", &format!("present ({})", v));
        return Ok(());
    }
    // curl -fsSL https://forgecode.dev/cli | sh
    pkg::run_loud("sh", &["-c", "curl -fsSL https://forgecode.dev/cli | sh"])?;
    Ok(())
}

fn install_configs(env: &env_detect::Env) -> Result<()> {
    ui::step("Configs (kitty/helix/fish/im/8sync)");
    let pairs = [
        ("configs/kitty.conf",            env.xdg_config.join("kitty/kitty.conf")),
        ("configs/kitty.session",         env.xdg_config.join("kitty/8sync.session")),
        ("configs/helix-config.toml",     env.xdg_config.join("helix/config.toml")),
        ("configs/helix-glass_black.toml",env.xdg_config.join("helix/themes/glass_black.toml")),
        ("configs/fish-config.fish",      env.xdg_config.join("fish/conf.d/8sync.fish")),
        ("configs/environment-im.conf",   env.xdg_config.join("environment.d/im.conf")),
        ("configs/global.toml",           env.xdg_config.join("8sync/global.toml")),
        ("configs/skills.toml",           env.xdg_config.join("8sync/skills.toml")),
    ];
    for (asset, target) in &pairs {
        let changed = assets::install(asset, target, false)?;
        if changed { ui::ok(&format!("wrote {}", target.display())); }
        else       { ui::skip(&target.display().to_string(), "unchanged"); }
    }
    Ok(())
}

fn install_wallpaper(env: &env_detect::Env) -> Result<()> {
    ui::step("Wallpaper bundle");
    let wp_dir = env.xdg_data.join("8sync/wallpapers");
    std::fs::create_dir_all(&wp_dir)?;
    let default_path = wp_dir.join("default.jpg");
    if default_path.exists() {
        ui::skip("default.jpg", "already exists");
        return Ok(());
    }
    // wallpapers.toml has a list of URLs to choose from
    let list = assets::read("wallpapers/wallpapers.toml").unwrap_or_default();
    let url = list
        .lines()
        .find_map(|l| l.strip_prefix("default = ").map(|s| s.trim().trim_matches('"').to_string()))
        .unwrap_or_else(|| "https://images.unsplash.com/photo-1506318137071-a8e063b4bec0?w=3840".to_string());
    pkg::run_loud("curl", &["-fL", "-o", default_path.to_str().unwrap(), &url])?;
    ui::ok(&format!("downloaded → {}", default_path.display()));
    Ok(())
}

fn install_skills(env: &env_detect::Env) -> Result<()> {
    ui::step("Skills (~/.forge/skills/)");
    let skills_dir = env.home.join(".forge/skills");
    std::fs::create_dir_all(&skills_dir)?;
    let trio = [
        ("skills/karpathy/SKILL.md",            "karpathy-guidelines/SKILL.md"),
        ("skills/image-routing/SKILL.md",       "image-routing/SKILL.md"),
        ("skills/8sync-cli/SKILL.md",           "8sync-cli/SKILL.md"),
    ];
    for (src, rel) in &trio {
        let target = skills_dir.join(rel);
        let changed = assets::install(src, &target, false)?;
        if changed { ui::ok(&format!("wrote {}", target.display())); }
        else       { ui::skip(&target.display().to_string(), "unchanged"); }
    }
    let master = skills_dir.join("00-force-load.md");
    assets::install("skills/00-force-load.md", &master, true)?;
    ui::ok(&format!("wrote {}", master.display()));
    Ok(())
}

fn enable_services(env: &env_detect::Env, warp: bool) -> Result<()> {
    ui::step("Services (warp / ufw / docker / mcp)");

    // systemd-user MCP service
    let svc_target = env.xdg_config.join("systemd/user/8sync-mcp.service");
    assets::install("configs/8sync-mcp.service", &svc_target, true)?;
    let _ = pkg::run_quiet("systemctl", &["--user", "daemon-reload"]);
    let _ = pkg::run_quiet("systemctl", &["--user", "enable", "--now", "8sync-mcp.service"]);
    ui::ok("8sync-mcp user service enabled");

    // ufw
    let _ = pkg::run_loud("sudo", &["systemctl", "enable", "--now", "ufw.service"]);
    let _ = pkg::run_loud("sudo", &["ufw", "--force", "enable"]);

    // docker
    let _ = pkg::run_loud("sudo", &["systemctl", "enable", "--now", "docker.service"]);

    if warp {
        // warp-svc
        let _ = pkg::run_loud("sudo", &["systemctl", "enable", "--now", "warp-svc.service"]);
        // 8sync wraps the rest: registration + mode + protocol + dns + connect
        let _ = pkg::run_loud("warp-cli", &["--accept-tos", "registration", "new"]);
        let _ = pkg::run_loud("warp-cli", &["--accept-tos", "mode", "doh"]);
        let _ = pkg::run_loud("warp-cli", &["--accept-tos", "tunnel", "protocol", "set", "MASQUE"]);
        let _ = pkg::run_loud("warp-cli", &["--accept-tos", "dns", "families", "malware"]);
        let _ = pkg::run_loud("warp-cli", &["--accept-tos", "connect"]);
        ui::ok("WARP: DoH + MASQUE + malware filter, auto-start on boot");
    }
    Ok(())
}

trait BoldStr {
    fn bold_str(&self) -> String;
    fn bright_black_str(&self) -> String;
}
impl BoldStr for &str {
    fn bold_str(&self) -> String {
        use owo_colors::OwoColorize;
        self.bold().to_string()
    }
    fn bright_black_str(&self) -> String {
        use owo_colors::OwoColorize;
        self.bright_black().to_string()
    }
}
