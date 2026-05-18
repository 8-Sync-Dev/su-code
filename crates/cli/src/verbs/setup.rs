use anyhow::Result;
use clap::Args as ClapArgs;

use crate::{assets, env_detect, pkg, ui, verbs::profile};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync setup                          install harness, then ask y/N for EACH personal profile
          8sync setup --yall                   install harness + ALL profiles (yes-to-all, no prompts)
          8sync setup --no-profile             install harness only (skip the profile stage)
          8sync setup --profile alexdev        install harness + apply the `alexdev` bundle non-interactively
          8sync setup --profile warp           install harness + apply just the WARP profile
          8sync setup --dry-run                print the full plan without changing anything

          8sync setup profile list             list every available profile (✓ = applied)
          8sync setup profile show alexdev     show resolved packages + services + post-install of a profile
          8sync setup profile apply warp       idempotently (re-)apply one profile

        STAGE A — HARNESS (always run, idempotent)
          · pacman -S --needed helix lazygit abduco github-cli   (4 pkgs, official repo)
          · forge AI CLI (curl installer, only if missing)
          · write configs: helix + kitty/8sync.session + 8sync/{global,skills}.toml
          · write skills:  ~/.forge/skills/{karpathy-guidelines, image-routing, 8sync-cli}/SKILL.md
                           + ~/.forge/skills/00-force-load.md  (auto-injected on every forge session)

        STAGE B — PROFILES (opt-in personal customization)
          vietnamese        fcitx5 + Unikey input method
          hardware-cooling  CoolerControl + OpenRGB + liquidctl
          hardware-lianli   lianli-linux-git from AUR (yay/paru auto-pulls build deps)
          displaylink      evdi-dkms (DisplayLink USB monitor driver)
          apps-personal     Bitwarden
          warp              Cloudflare WARP + DoH + MASQUE + malware DNS  (toggle daily via `8sync sec`)
          alexdev           BUNDLE — extends all of the above

        SAFETY
          · Every install is transactional: if pacman/AUR fails halfway, packages installed in
            that batch are rolled back automatically (pacman -Rns).
          · Re-running setup is idempotent — already-installed packages are skipped.
          · `--dry-run` is always safe to inspect what would change.
    "}
)]
pub struct Args {
    /// Sub-command: `profile [list|show|apply <name>]`
    pub action: Option<String>,
    /// Arguments for the sub-command.
    pub rest: Vec<String>,

    /// Yes-to-all: install every profile (or the `alexdev` bundle) with --noconfirm.
    #[arg(long = "yall", alias = "yes", short = 'y')]
    pub yall: bool,

    /// Skip Stage B entirely (harness only — no profile prompts).
    #[arg(long)]
    pub no_profile: bool,

    /// Apply a specific profile non-interactively (use after Stage A).
    #[arg(long)]
    pub profile: Option<String>,

    /// Print the plan without making any changes.
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(a: Args) -> Result<()> {
    // Sub-command: `8sync setup profile <action>`
    if a.action.as_deref() == Some("profile") {
        return profile_sub(a.rest, a.yall, a.dry_run);
    }

    ui::header("8sync setup");
    let env = env_detect::Env::detect()?;
    if !env.is_cachyos_or_arch() {
        ui::warn(&format!(
            "OS `{}` is not CachyOS/Arch — some steps may fail.", env.os_id
        ));
    }
    if env_detect::is_hyde() {
        ui::ok("HyDE detected — will skip kitty/theme/wallpaper (HyDE manages them)");
    }

    // ── Stage A: Harness (always run) ────────────────────────────
    ui::step("Stage A — coding harness");
    if a.dry_run {
        ui::info("would install: helix lazygit abduco github-cli");
        ui::info("would install forge (curl) if missing");
        ui::info("would write: configs + skills");
    } else {
        let core = ["helix", "lazygit", "abduco", "github-cli"];
        pkg::pacman_install_safe(&core, true)?;
        install_forge()?;
        install_configs(&env)?;
        install_skills(&env)?;
    }

    // ── Stage B: Profiles (optional) ─────────────────────────────
    if a.no_profile {
        ui::info("--no-profile → skipping personal profiles");
        finish_msg();
        return Ok(());
    }

    let all = profile::load_all()?;

    // explicit --profile <name>
    if let Some(name) = a.profile.as_ref() {
        ui::step(&format!("Stage B — applying profile `{}`", name));
        let resolved = profile::resolve(name, &all)?;
        profile::apply(&resolved, true, a.dry_run)?;
        if !a.dry_run { profile::mark_applied(name)?; }
        finish_msg();
        return Ok(());
    }

    // --yall: apply all non-bundle profiles
    if a.yall {
        ui::step("Stage B — --yall: applying ALL profiles");
        // Apply alexdev bundle if present (covers everything), else apply each
        let name = if all.contains_key("alexdev") { "alexdev" } else {
            ui::warn("no `alexdev` bundle — applying every profile individually");
            for (n, _) in &all {
                let res = profile::resolve(n, &all)?;
                let _ = profile::apply(&res, true, a.dry_run);
                if !a.dry_run { let _ = profile::mark_applied(n); }
            }
            finish_msg();
            return Ok(());
        };
        let resolved = profile::resolve(name, &all)?;
        profile::apply(&resolved, true, a.dry_run)?;
        if !a.dry_run { profile::mark_applied(name)?; }
        finish_msg();
        return Ok(());
    }

    // Interactive y/N per profile (skip bundle profiles)
    if !env_detect::has_tty() {
        ui::info("no TTY — skipping interactive profile prompt (use --yall or --profile)");
        finish_msg();
        return Ok(());
    }

    ui::step("Stage B — personal profiles (y/N each)");
    let mut names: Vec<&String> = all.keys().collect();
    names.sort();
    for name in &names {
        let p = match all.get(*name) { Some(p) => p, None => continue };
        // Skip bundles (they `extend` others) — apply via --profile flag instead
        if !p.extends.is_empty() { continue; }
        let desc = if p.description.is_empty() { name.as_str() } else { p.description.as_str() };
        let q = format!("Apply `{}` — {}", name, desc);
        if ui::prompt_yes_no(&q, false) {
            let resolved = profile::resolve(name, &all)?;
            if let Err(e) = profile::apply(&resolved, false, a.dry_run) {
                ui::err(&format!("profile {} failed: {}", name, e));
            } else if !a.dry_run {
                let _ = profile::mark_applied(name);
            }
        }
    }

    finish_msg();
    Ok(())
}

fn finish_msg() {
    ui::header("Done — next steps");
    println!("  · 8sync doctor               — verify");
    println!("  · forge login                — connect AI provider");
    println!("  · cd <project> && 8sync .    — start a session");
}

fn install_forge() -> Result<()> {
    ui::step("forge AI CLI");
    if which::which("forge").is_ok() {
        let v = env_detect::cmd_version("forge", &["--version"]).unwrap_or_default();
        ui::skip("forge", &format!("present ({})", v));
        return Ok(());
    }
    pkg::run_loud("sh", &["-c", "curl -fsSL https://forgecode.dev/cli | sh"])?;
    Ok(())
}

fn install_configs(env: &env_detect::Env) -> Result<()> {
    ui::step("Configs (helix + 8sync session/global)");
    // Only safe, non-HyDE-conflicting configs:
    //   • helix config + theme  (only written if user doesn't have one)
    //   • kitty/8sync.session   (separate file, NOT kitty.conf)
    //   • 8sync/global.toml + skills.toml
    let pairs = [
        ("configs/helix-config.toml",      env.xdg_config.join("helix/config.toml")),
        ("configs/helix-glass_black.toml", env.xdg_config.join("helix/themes/glass_black.toml")),
        ("configs/kitty.session",          env.xdg_config.join("kitty/8sync.session")),
        ("configs/global.toml",            env.xdg_config.join("8sync/global.toml")),
        ("configs/skills.toml",            env.xdg_config.join("8sync/skills.toml")),
    ];
    for (asset, target) in &pairs {
        let changed = assets::install(asset, target, false)?;
        if changed { ui::ok(&format!("wrote {}", target.display())); }
        else       { ui::skip(&target.display().to_string(), "unchanged"); }
    }
    Ok(())
}

fn install_skills(env: &env_detect::Env) -> Result<()> {
    ui::step("Skills (~/.forge/skills/)");
    let skills_dir = env.home.join(".forge/skills");
    std::fs::create_dir_all(&skills_dir)?;
    let trio = [
        ("skills/karpathy/SKILL.md",      "karpathy-guidelines/SKILL.md"),
        ("skills/image-routing/SKILL.md", "image-routing/SKILL.md"),
        ("skills/8sync-cli/SKILL.md",     "8sync-cli/SKILL.md"),
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

// ─── `8sync setup profile <sub>` ────────────────────────────────

fn profile_sub(rest: Vec<String>, yall: bool, dry_run: bool) -> Result<()> {
    let action = rest.first().map(|s| s.as_str()).unwrap_or("list");
    let all = profile::load_all()?;
    let state = profile::load_state();

    match action {
        "list" => {
            ui::header("Profiles");
            let mut names: Vec<&String> = all.keys().collect();
            names.sort();
            for n in names {
                let p = &all[n];
                let marker = if state.applied.iter().any(|x| x == n) { "✓" } else { " " };
                let kind = if !p.extends.is_empty() { "(bundle)" } else { "" };
                println!("  {} {:20} {} {}", marker, n, kind, p.description);
            }
            Ok(())
        }
        "show" => {
            let name = rest.get(1).ok_or_else(|| anyhow::anyhow!("usage: 8sync setup profile show <name>"))?;
            let r = profile::resolve(name, &all)?;
            println!("name         = {}", r.name);
            println!("description  = {}", r.description);
            println!("needs AUR    = {}", r.requires.aur_helper);
            println!("pacman       = {:?}", r.packages.pacman);
            println!("aur          = {:?}", r.packages.aur);
            println!("sys services = {:?}", r.services.system_enable);
            println!("user services= {:?}", r.services.user_enable);
            println!("commands     = {:?}", r.post_install.commands);
            if !r.post_install.hint.is_empty() {
                println!("\nhints:\n{}", r.post_install.hint);
            }
            Ok(())
        }
        "apply" => {
            let name = rest.get(1).ok_or_else(|| anyhow::anyhow!("usage: 8sync setup profile apply <name>"))?;
            let resolved = profile::resolve(name, &all)?;
            profile::apply(&resolved, yall, dry_run)?;
            if !dry_run { profile::mark_applied(name)?; }
            Ok(())
        }
        other => {
            ui::warn(&format!("unknown sub-action `{}` — try list | show | apply", other));
            Ok(())
        }
    }
}
