use anyhow::{bail, Result};
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

          8sync setup --caelestia              auto-detect: HyDE present → additive overlay, else fresh full stack
          8sync setup --caelestia=fresh        force fresh CachyOS path (Hyprland + Quickshell + nvidia auto-detect)
          8sync setup --caelestia=hyde         force HyDE-additive path (caelestia-shell + userprefs.conf overlay)
          8sync setup --caelestia=rollback     remove HyDE overlay block, restore waybar

          8sync setup --end4                   end-4/dots-hyprland medium tier (Hyprland + Quickshell, no fish/fonts/misc)
          8sync setup --end4=minimal           bare Hyprland keybinds only, skip Quickshell + fish + fonts + misc
          8sync setup --end4=medium            Hyprland + Quickshell (default tier when bare --end4 is given)
          8sync setup --end4=full              everything upstream installs (incl. fish/fonts/plasma-browser-integration)
          8sync setup --end4=rollback          run upstream `./setup uninstall -f`

          8sync setup profile list             list every available profile (✓ = applied)
          8sync setup profile show alexdev     show resolved packages + services + post-install of a profile
          8sync setup profile apply warp       idempotently (re-)apply one profile

        STAGE A — HARNESS (always run, idempotent)
          · pacman -S --needed github-cli       (gh — required by `8sync ship`)
          · omp AI CLI                          (curl installer from omp.sh, only if missing)
          · write configs: 8sync/{global,skills}.toml
          · write skills:  ~/.omp/skills/{karpathy-guidelines, image-routing, 8sync-cli}/SKILL.md
                           + ~/.omp/skills/00-force-load.md  (auto-injected on every omp session)

        STAGE B — PROFILES (opt-in personal customization)
          vietnamese        fcitx5 + Unikey input method
          hardware-cooling  CoolerControl + OpenRGB + liquidctl
          hardware-lianli   lianli-linux-git from AUR
          displaylink       evdi-dkms (DisplayLink USB monitor driver)
          apps-personal     Bitwarden
          warp              Cloudflare WARP + DoH + MASQUE  (toggle daily via `8sync sec`)
          nvidia            auto-detect driver (Blackwell→Turing: open-dkms; Maxwell/Pascal: dkms)
          caelestia         fresh Hyprland + Quickshell + caelestia-shell  (extends nvidia)
          caelestia-hyde    additive overlay for existing HyDE installs
          alexdev           BUNDLE — caelestia + all personal profiles

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

    /// Apply the Caelestia profile (auto|fresh|hyde|rollback). Plain `--caelestia` = auto-detect.
    #[arg(
        long,
        value_name = "MODE",
        num_args = 0..=1,
        default_missing_value = "auto",
    )]
    pub caelestia: Option<String>,

    /// Nuke every desktop-shell customisation 8sync ever applied (end-4 +
    /// caelestia overlays, bridge keybinds, palette cycler, masked services,
    /// cloned dotfiles) and restore HyDE-only state. Combine with --dry-run.
    #[arg(long)]
    pub reset_shells: bool,

    /// With --reset-shells: also pacman -Rns the shell packages
    /// (caelestia-shell, quickshell, aubio). Off by default — packages stay.
    #[arg(long)]
    pub purge_packages: bool,

    /// Apply end-4/dots-hyprland (minimal|medium|full|rollback). Auto-yes — no prompts.
    #[arg(
        long = "end4",
        value_name = "TIER",
        num_args = 0..=1,
        default_missing_value = "medium",
    )]
    pub end4: Option<String>,

    /// Print the plan without making any changes.
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(a: Args) -> Result<()> {
    // Sub-command: `8sync setup profile <action>`
    // Short-circuit: full nuke of every shell customisation we ever applied.
    if a.reset_shells {
        return reset_shells(a.purge_packages, a.dry_run);
    }

    if a.action.as_deref() == Some("profile") {
        return profile_sub(a.rest, a.yall, a.dry_run);
    }

    // Special: `--caelestia=rollback` — no Stage A, just undo the HyDE overlay.
    if a.caelestia.as_deref() == Some("rollback") {
        return rollback_caelestia_hyde(a.dry_run);
    }

    // Special: `--end4=rollback` — no Stage A, just uninstall upstream.
    if a.end4.as_deref() == Some("rollback") {
        return rollback_end4(a.dry_run);
    }
    if a.end4.as_deref() == Some("rollback-overlay") {
        return rollback_end4_overlay(a.dry_run);
    }

    ui::header("8sync setup");
    let env = env_detect::Env::detect()?;
    if !env.is_cachyos_or_arch() {
        ui::warn(&format!(
            "OS `{}` is not CachyOS/Arch — some steps may fail.", env.os_id
        ));
    }
    if env_detect::is_hyde() {
        ui::ok("HyDE detected — `--caelestia` (auto) will use the additive overlay path");
    }

    // ── Stage A: Harness (always run) ────────────────────────────
    ui::step("Stage A — coding harness");
    if a.dry_run {
        ui::info("would install: github-cli");
        ui::info("would install omp (curl) if missing");
        ui::info("would install codegraph (curl) if missing");
        ui::info("would write: configs + skills");
        ui::info("would register codegraph as a global+local skill");
    } else {
        let core = ["github-cli"];
        pkg::pacman_install_safe(&core, true)?;
        install_omp()?;
        install_codegraph()?;
        install_aur_helper()?;
        install_configs(&env)?;
        install_skills(&env)?;
        register_codegraph_skill(&env)?;
    }

    // ── Caelestia shortcut (resolves to a profile name, applied yall-style) ──
    if let Some(mode) = a.caelestia.as_deref() {
        let chosen = match mode {
            "auto"  => if env_detect::is_hyde() { "caelestia-hyde" } else { "caelestia" },
            "hyde"  => "caelestia-hyde",
            "fresh" => "caelestia",
            other   => bail!("--caelestia accepts: auto|fresh|hyde|rollback (got `{}`)", other),
        };
        ui::step(&format!("Stage B — --caelestia={} → profile `{}`", mode, chosen));
        let all = profile::load_all()?;
        let resolved = profile::resolve(chosen, &all)?;
        profile::apply(&resolved, true, a.dry_run)?;
        if !a.dry_run { profile::mark_applied(chosen)?; }
        finish_msg();
        return Ok(());
    }

    // ── end-4/dots-hyprland shortcut ─────────────────────────────
    if let Some(tier) = a.end4.as_deref() {
        if tier == "overlay" {
            apply_end4_overlay(a.dry_run)?;
        } else {
            run_end4(tier, a.dry_run)?;
        }
        finish_msg();
        return Ok(());
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
    println!("  · cd <project> && 8sync .    — seed agents/ + start omp --continue");
}

fn install_omp() -> Result<()> {
    ui::step("omp AI CLI");
    if which::which("omp").is_ok() {
        let v = env_detect::cmd_version("omp", &["--version"]).unwrap_or_default();
        ui::skip("omp", &format!("present ({})", v));
        return Ok(());
    }
    pkg::run_loud("sh", &["-c", "curl -fsSL https://omp.sh/install | sh"])?;
    Ok(())
}

fn install_aur_helper() -> Result<()> {
    ui::step("AUR helper (paru)");
    if let Some(h) = env_detect::aur_helper() {
        ui::skip(h, "present");
        return Ok(());
    }
    // Bootstrap paru from source — no AUR helper exists yet, so we can't
    // pacman/AUR-install it directly. Standard makepkg flow.
    pkg::pacman_install_safe(&["git", "base-devel"], true)?;
    let cmd = "cd /tmp && rm -rf paru-bootstrap && \
        git clone https://aur.archlinux.org/paru.git paru-bootstrap && \
        cd paru-bootstrap && makepkg -si --noconfirm && \
        cd .. && rm -rf paru-bootstrap";
    pkg::run_loud("sh", &["-c", cmd])?;
    ui::ok("paru installed");
    Ok(())
}
fn install_codegraph() -> Result<()> {
    ui::step("codegraph (semantic code index for omp / claude / cursor)");
    if which::which("codegraph").is_ok() {
        let v = env_detect::cmd_version("codegraph", &["--version"]).unwrap_or_default();
        ui::skip("codegraph", &format!("present ({})", v));
        return Ok(());
    }
    pkg::run_loud(
        "sh",
        &["-c", "curl -fsSL https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.sh | sh"],
    )?;
    ensure_local_bin_on_path();
    Ok(())
}

/// Ensure `~/.local/bin` is on PATH in ~/.zshrc and ~/.bashrc (idempotent).
/// codegraph's installer drops its binary there; without PATH the AI cannot
/// invoke it from new shells.
fn ensure_local_bin_on_path() {
    let Some(home) = dirs::home_dir() else { return; };
    let local_bin = home.join(".local/bin");
    let marker = "# 8sync: ensure ~/.local/bin on PATH (for codegraph + 8sync)";
    let snippet = format!(
        "\n{marker}\ncase \":$PATH:\" in *\":{lb}:\"*) ;; *) export PATH=\"{lb}:$PATH\" ;; esac\n",
        lb = local_bin.display(),
    );
    for rc in [home.join(".zshrc"), home.join(".bashrc")] {
        if !rc.exists() { continue; }
        let existing = std::fs::read_to_string(&rc).unwrap_or_default();
        if existing.contains(marker) { continue; }
        if let Err(e) = std::fs::OpenOptions::new().append(true).open(&rc)
            .and_then(|mut f| { use std::io::Write; f.write_all(snippet.as_bytes()) })
        {
            ui::warn(&format!("could not patch {}: {}", rc.display(), e));
            continue;
        }
        ui::ok(&format!("patched {} (added ~/.local/bin to PATH)", rc.display()));
    }
}

/// Register codegraph as a skill (global ~/.omp/skills/codegraph/ + project local).
/// Synthesizes SKILL.md with proper YAML frontmatter from upstream README, so
/// AI auto-discovery (Agent Skills open standard) works.
fn register_codegraph_skill(env: &env_detect::Env) -> Result<()> {
    ui::step("Register codegraph skill (force-load)");
    let skills_toml = env.xdg_config.join("8sync/skills.toml");
    // Reuse the same code path as `8sync skill add gh:colbymchenry/codegraph`
    // so we get the README-as-skill synthesis (proper SKILL.md frontmatter)
    // and skills.toml registration in one shot.
    if let Err(e) = crate::verbs::skill::add_spec(
        env,
        &skills_toml,
        "gh:colbymchenry/codegraph",
    ) {
        ui::warn(&format!("could not auto-register codegraph: {} (skill will still work but missing frontmatter)", e));
    }
    Ok(())
}

fn install_configs(env: &env_detect::Env) -> Result<()> {
    ui::step("Configs (8sync/{global,skills}.toml)");
    let pairs = [
        ("configs/global.toml", env.xdg_config.join("8sync/global.toml")),
        ("configs/skills.toml", env.xdg_config.join("8sync/skills.toml")),
    ];
    for (asset, target) in &pairs {
        let changed = assets::install(asset, target, false)?;
        if changed { ui::ok(&format!("wrote {}", target.display())); }
        else       { ui::skip(&target.display().to_string(), "unchanged"); }
    }
    Ok(())
}

fn install_skills(env: &env_detect::Env) -> Result<()> {
    ui::step("Skills (~/.omp/skills/)");
    let skills_dir = env.home.join(".omp/skills");
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

// ─── `--caelestia=rollback` ────────────────────────────────────

fn rollback_caelestia_hyde(dry_run: bool) -> Result<()> {
    ui::header("8sync setup --caelestia=rollback");
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let conf = home.join(".config/hypr/userprefs.conf");

    if !conf.exists() {
        ui::info(&format!("{} not found — nothing to roll back", conf.display()));
        return Ok(());
    }

    let sed_cmd = format!(
        "sed -i '/# === CAELESTIA-SHELL-OVERRIDE ===/,/# === END-CAELESTIA-OVERRIDE ===/d' {}",
        shell_quote(conf.to_str().unwrap_or(""))
    );
    let cmds = [
        sed_cmd.as_str(),
        "pkill -f 'qs -c caelestia' || true",
        // Unmask + restart HyDE waybar service (symmetric with apply).
        "systemctl --user unmask waybar.service 2>/dev/null; systemctl --user start hyde-Hyprland-bar.service 2>/dev/null || (command -v waybar >/dev/null && (setsid waybar >/dev/null 2>&1 &) || true)",
        "pgrep -x Hyprland >/dev/null && hyprctl reload >/dev/null || true",
    ];
    for c in &cmds {
        if dry_run {
            ui::info(&format!("would run: {}", c));
        } else {
            ui::info(&format!("$ {}", c));
            let _ = std::process::Command::new("sh").arg("-c").arg(c).status();
        }
    }
    ui::ok("caelestia-hyde overlay removed");
    Ok(())
}

fn shell_quote(s: &str) -> String {
    let mut out = String::from("'");
    for c in s.chars() {
        if c == '\'' { out.push_str(r"'\''"); } else { out.push(c); }
    }
    out.push('\'');
    out
}

// ─── `--end4=<tier>` ────────────────────────────────────────────

const END4_REPO: &str = "https://github.com/end-4/dots-hyprland.git";
const END4_DIR_REL: &str = ".local/share/dots-hyprland";

fn end4_flags(tier: &str) -> Result<&'static [&'static str]> {
    // All tiers pass: -f (force, no confirm) -s (skip pacman -Syu) --skip-allgreeting
    // --ignore-outdate. Backup is LEFT ENABLED — end-4 will move overwritten
    // files into `~/.config/<x>.bak.<ts>/` (or rename to `.old`) so a botched
    // install never destroys the user's existing keybinds silently.
    // minimal+medium also pass --skip-hyprland-entry so HyDE's hyprland.conf
    // (or any custom entry) stays the active one. Use `--end4=overlay` to
    // additionally launch end-4's Quickshell shell on top of an HyDE session.
    Ok(match tier {
        // Bare Hyprland config, no widget shell — preserve user's existing entry
        // config (HyDE's hyprland.conf etc. stays untouched).
        "minimal" => &["install", "-f", "-s", "--skip-allgreeting", "--ignore-outdate", "--core", "--skip-quickshell", "--skip-hyprland-entry"],
        // Hyprland + Quickshell widget shell — also preserve entry. Caveat: user
        // must opt in to end-4's entry manually (rename hyprland.lua back) if
        // they want the full end-4 keybinds.
        "medium"  => &["install", "-f", "-s", "--skip-allgreeting", "--ignore-outdate", "--core", "--skip-hyprland-entry"],
        // Everything upstream installs — DOES overwrite the entry config (this
        // is the "I'm starting fresh, use end-4 as my main DE" mode).
        "full"    => &["install", "-f", "-s", "--skip-allgreeting", "--ignore-outdate"],
        other     => bail!("--end4 accepts: minimal|medium|full|rollback (got `{}`)", other),
    })
}

fn run_end4(tier: &str, dry_run: bool) -> Result<()> {
    let args = end4_flags(tier)?;
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let dir = home.join(END4_DIR_REL);

    ui::step(&format!("end-4/dots-hyprland → tier `{}`", tier));
    // Warn when end-4 will collide with HyDE. On `full` tier the entry config IS
    // overwritten and HyDE keybinds will be inactive — bail loudly unless user
    // re-runs with a non-full tier. minimal+medium pass --skip-hyprland-entry
    // so they're safe to co-exist.
    if env_detect::is_hyde() {
        if tier == "full" {
            ui::err("HyDE detected + --end4=full → end-4 WILL overwrite hyprland.conf entry.");
            ui::err("HyDE keybinds will stop working. Recover with `mv ~/.config/hypr/hyprland.conf.old ~/.config/hypr/hyprland.conf`.");
            ui::err("If you really want this, run: 8sync setup --end4=rollback first, then re-run --end4=full.");
            bail!("refusing --end4=full on a HyDE system without explicit rollback");
        }
        ui::warn("HyDE detected — using --skip-hyprland-entry so HyDE keybinds stay active.");
        ui::warn("end-4's `hyprland.lua` entry will NOT be installed. To switch to end-4 keybinds later:");
        ui::warn("  mv ~/.config/hypr/hyprland.conf ~/.config/hypr/hyprland.conf.hyde");
        ui::warn("  cd ~/.local/share/dots-hyprland && ./setup install-files");
    }
    if dry_run {
        if dir.exists() {
            ui::info(&format!("would: git -C {} pull --ff-only", dir.display()));
        } else {
            ui::info(&format!("would: git clone {} {}", END4_REPO, dir.display()));
        }
        ui::info(&format!("would: cd {} && ./setup {}", dir.display(), args.join(" ")));
        return Ok(());
    }

    if !which::which("git").is_ok() {
        bail!("git missing — install it first (`sudo pacman -S git`)");
    }

    if dir.exists() {
        ui::info(&format!("$ git -C {} pull --ff-only", dir.display()));
        let _ = std::process::Command::new("git")
            .args(["-C", dir.to_str().unwrap_or(""), "pull", "--ff-only"])
            .status();
    } else {
        if let Some(parent) = dir.parent() { std::fs::create_dir_all(parent)?; }
        pkg::run_loud("git", &["clone", END4_REPO, dir.to_str().unwrap_or("")])?;
    }

    ui::info(&format!("$ cd {} && ./setup {}", dir.display(), args.join(" ")));
    let status = std::process::Command::new("./setup")
        .args(args.iter().copied())
        .current_dir(&dir)
        .status()?;
    if !status.success() {
        bail!("end-4 setup exited with {}", status);
    }
    ui::ok(&format!("end-4/dots-hyprland `{}` applied", tier));
    Ok(())
}

fn rollback_end4(dry_run: bool) -> Result<()> {
    ui::header("8sync setup --end4=rollback");
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let dir = home.join(END4_DIR_REL);

    if !dir.exists() {
        ui::info(&format!("{} not found — nothing to roll back", dir.display()));
        return Ok(());
    }
    let cmd = format!("cd {} && ./setup uninstall -f", shell_quote(dir.to_str().unwrap_or("")));
    if dry_run {
        ui::info(&format!("would run: {}", cmd));
    } else {
        ui::info(&format!("$ {}", cmd));
        let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).status();
    }
    ui::ok("end-4 uninstall invoked");
    Ok(())
}

// ─── `--end4=overlay` (HyDE-side-by-side: end-4 Quickshell over HyDE keybinds) ─

/// Inject an idempotent block into `~/.config/hypr/userprefs.conf` that kills
/// waybar and launches end-4's Quickshell config (`qs -c ii`). Mirrors the
/// `caelestia-hyde` overlay pattern. Triggers a live reload + spawn when
/// Hyprland is already running.
fn apply_end4_overlay(dry_run: bool) -> Result<()> {
    ui::header("8sync setup --end4=overlay");
    if which::which("qs").is_err() {
        ui::warn("`qs` (Quickshell) not on PATH — install end-4 deps first: `8sync setup --end4=medium`");
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let conf = home.join(".config/hypr/userprefs.conf");
    let bridge = home.join(".config/hypr/8sync-end4-bridge.conf");
    let glass = home.join(".config/hypr/8sync-end4-glass.conf");
    let palette_init = home.join(".config/hypr/8sync-palette.conf");
    let cycler_dir = home.join(".local/share/8sync");
    let cycler = cycler_dir.join("8sync-palette-cycle.sh");

    // 1. Write embedded assets (bridge keybinds + glass theme + palette cycler).
    let assets_to_write: &[(&str, &std::path::Path, bool)] = &[
        ("configs/end4-bridge-keybinds.conf", &bridge, false),
        ("configs/end4-glass-theme.conf",     &glass,  false),
        ("configs/8sync-palette-cycle.sh",    &cycler, true), // chmod +x
    ];
    if !dry_run {
        let _ = std::fs::create_dir_all(&cycler_dir);
        for (asset, target, executable) in assets_to_write {
            let Some(body) = crate::assets::read(asset) else {
                ui::warn(&format!("asset missing: {}", asset));
                continue;
            };
            if let Some(parent) = target.parent() { let _ = std::fs::create_dir_all(parent); }
            if let Err(e) = std::fs::write(target, &body) {
                ui::warn(&format!("could not write {}: {}", target.display(), e));
                continue;
            }
            if *executable {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755));
            }
            ui::ok(&format!("wrote {}", target.display()));
        }
        // Seed an initial palette file so the sourced `source = 8sync-palette.conf`
        // line in glass theme has something to read. Idempotent (script reads
        // existing index from ~/.config/hypr/.8sync-palette-index).
        let _ = std::process::Command::new("sh").arg("-c")
            .arg(format!("{} show", shell_quote(cycler.to_str().unwrap_or(""))))
            .status();
        let _ = palette_init; // initialized by the script above
    } else {
        for (_, target, _) in assets_to_write {
            ui::info(&format!("would write: {}", target.display()));
        }
    }

    let cmds = [
        format!(
            "[ -f {0} ] && cp {0} {0}.bak.$(date +%s) || (mkdir -p $(dirname {0}) && touch {0})",
            shell_quote(conf.to_str().unwrap_or(""))
        ),
        format!(
            "sed -i '/# === END4-SHELL-OVERLAY ===/,/# === END-END4-OVERLAY ===/d' {}",
            shell_quote(conf.to_str().unwrap_or(""))
        ),
        format!(
            "printf '\\n# === END4-SHELL-OVERLAY ===\\n# Managed by `8sync setup --end4=overlay`. Re-run to refresh, or `8sync setup --end4=rollback-overlay` to remove.\\nsource = {glass}\\nsource = {bridge}\\nexec-once = qs -c ii\\n# === END-END4-OVERLAY ===\\n' >> {conf}",
            glass = shell_quote(glass.to_str().unwrap_or("")),
            bridge = shell_quote(bridge.to_str().unwrap_or("")),
            conf = shell_quote(conf.to_str().unwrap_or(""))
        ),
        "pgrep -x Hyprland >/dev/null && hyprctl reload >/dev/null || true".to_string(),
        // Stop HyDE's transient waybar service (otherwise systemd respawns it
        // after a plain pkill). `mask waybar.service` survives reboot so HyDE
        // doesn't respawn it on next session. Falls back to pkill for
        // non-HyDE setups. See docs/known-issues.md#hyde-waybar-respawn.
        "systemctl --user stop hyde-Hyprland-bar.service 2>/dev/null; systemctl --user mask waybar.service 2>/dev/null; pkill -x waybar || true".to_string(),
        "command -v qs >/dev/null && (setsid qs -c ii >/dev/null 2>&1 &) || true".to_string(),
    ];
    for c in &cmds {
        if dry_run {
            ui::info(&format!("would run: {}", c));
        } else {
            ui::info(&format!("$ {}", c));
            let _ = std::process::Command::new("sh").arg("-c").arg(c).status();
        }
    }
    ui::ok("end-4 overlay applied — Quickshell `ii` running, bridge keybinds sourced");
    ui::info("press Super+Shift+/ for the merged bridge keybind cheatsheet (Super+/ still shows HyDE's)");
    ui::info("AI: Super+O → settings cog → set Gemini/OpenAI/Mistral API key (or export GOOGLE_AI_API_KEY / OPENAI_API_KEY / MISTRAL_API_KEY)");
    Ok(())
}

fn rollback_end4_overlay(dry_run: bool) -> Result<()> {
    ui::header("8sync setup --end4=rollback-overlay");
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let conf = home.join(".config/hypr/userprefs.conf");
    if !conf.exists() {
        ui::info(&format!("{} not found — nothing to roll back", conf.display()));
        return Ok(());
    }
    let cmds = [
        format!(
            "sed -i '/# === END4-SHELL-OVERLAY ===/,/# === END-END4-OVERLAY ===/d' {}",
            shell_quote(conf.to_str().unwrap_or(""))
        ),
        "pkill -f 'qs -c ii' || true".to_string(),
        "rm -f $HOME/.config/hypr/8sync-end4-bridge.conf $HOME/.config/hypr/8sync-end4-glass.conf $HOME/.config/hypr/8sync-palette.conf $HOME/.config/hypr/.8sync-palette-index".to_string(),
        "rm -f $HOME/.local/share/8sync/8sync-palette-cycle.sh".to_string(),
        // Prefer restarting HyDE's transient service if it exists; fall back to
        // a plain waybar spawn (non-HyDE setups). Unmask waybar.service so HyDE
        // can spawn it again on next reboot.
        "systemctl --user unmask waybar.service 2>/dev/null; systemctl --user start hyde-Hyprland-bar.service 2>/dev/null || (command -v waybar >/dev/null && (setsid waybar >/dev/null 2>&1 &) || true)".to_string(),
        "pgrep -x Hyprland >/dev/null && hyprctl reload >/dev/null || true".to_string(),
    ];
    for c in &cmds {
        if dry_run {
            ui::info(&format!("would run: {}", c));
        } else {
            ui::info(&format!("$ {}", c));
            let _ = std::process::Command::new("sh").arg("-c").arg(c).status();
        }
    }
    ui::ok("end-4 overlay removed — waybar restored");
    Ok(())
}

// ─── `--reset-shells` ───────────────────────────────────────────
//
// Idempotent "undo everything we ever did to the desktop shell". Safe to
// run multiple times; safe to run on a machine that never had end-4 /
// caelestia installed (every step `|| true`s).

fn reset_shells(purge_packages: bool, dry_run: bool) -> Result<()> {
    ui::header("8sync setup --reset-shells");
    if dry_run { ui::info("DRY RUN — no changes will be made"); }

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let userprefs = home.join(".config/hypr/userprefs.conf");
    let userprefs_q = shell_quote(userprefs.to_str().unwrap_or(""));

    // Step-by-step plan. Each cmd is independent, errors swallowed via `|| true`.
    let mut cmds: Vec<String> = Vec::new();

    // 1. Strip sentinel blocks from userprefs.conf (both end-4 and caelestia).
    if userprefs.exists() {
        cmds.push(format!(
            "cp {0} {0}.reset.$(date +%s).bak 2>/dev/null || true",
            userprefs_q
        ));
        cmds.push(format!(
            "sed -i '/# === END4-SHELL-OVERLAY ===/,/# === END-END4-OVERLAY ===/d' {}",
            userprefs_q
        ));
        cmds.push(format!(
            "sed -i '/# === CAELESTIA-SHELL-OVERRIDE ===/,/# === END-CAELESTIA-OVERRIDE ===/d' {}",
            userprefs_q
        ));
    }

    // 2. Kill every Quickshell instance.
    cmds.push("pkill -f 'qs -c ii' || true".into());
    cmds.push("pkill -f 'qs -c caelestia' || true".into());
    cmds.push("pkill -x quickshell || true".into());

    // 3. Remove the bridge / glass / palette assets + cycler script.
    cmds.push("rm -f \
        $HOME/.config/hypr/8sync-end4-bridge.conf \
        $HOME/.config/hypr/8sync-end4-glass.conf \
        $HOME/.config/hypr/8sync-palette.conf \
        $HOME/.config/hypr/.8sync-palette-index".into());
    cmds.push("rm -f $HOME/.local/share/8sync/8sync-palette-cycle.sh".into());

    // 4. Remove cloned upstream dotfiles.
    cmds.push("rm -rf $HOME/.local/share/dots-hyprland".into());
    cmds.push("rm -rf $HOME/.local/share/caelestia".into());

    // 5. Remove Caelestia config symlinks (its installer drops these via
    //    `ln -s` to ~/.local/share/caelestia). Only unlink symlinks, never
    //    delete real dirs — guards user data on HyDE-only setups.
    for d in ["hypr", "foot", "fish", "fastfetch", "uwsm", "btop",
              "spicetify", "starship.toml"] {
        cmds.push(format!(
            "[ -L $HOME/.config/{0} ] && rm -f $HOME/.config/{0} || true", d
        ));
    }

    // 6. Restore HyDE's waybar — unmask + restart its transient service.
    cmds.push("systemctl --user unmask waybar.service 2>/dev/null || true".into());
    // HyDE's `hyde-Hyprland-bar.service` is transient (created by systemd-run
    // at session start) — `systemctl start` can't recreate it. Plain waybar
    // spawn is the reliable fallback that works on both HyDE and bare Hyprland.
    cmds.push("pkill -x waybar 2>/dev/null; sleep 0.3; (setsid waybar >/dev/null 2>&1 &) 2>/dev/null || true".into());

    // 7. Reload Hyprland so all of the above takes effect immediately.
    cmds.push("pgrep -x Hyprland >/dev/null && hyprctl reload >/dev/null || true".into());
    cmds.push("pgrep -x Hyprland >/dev/null && hyprctl dismissnotify -1 >/dev/null || true".into());

    // 8. Optional package purge.
    if purge_packages {
        cmds.push("sudo pacman -Rns --noconfirm caelestia-shell quickshell aubio 2>/dev/null || true".into());
    }

    // Run.
    for c in &cmds {
        if dry_run {
            ui::info(&format!("would run: {}", c));
        } else {
            ui::info(&format!("$ {}", c));
            let _ = std::process::Command::new("sh").arg("-c").arg(c).status();
        }
    }

    // 9. Report any backups still on disk so user can restore manually.
    if !dry_run {
        let candidates = [
            ".config.bak.caelestia.",
            ".config/hypr/userprefs.conf.bak.",
            ".config/hypr/userprefs.conf.reset.",
            ".config/hypr/hyprland.conf.old",
            ".config/hypr/hypridle.conf.old",
            ".config/hypr/hyprlock.conf.old",
            ".config/hypr.end4-stash.",
        ];
        let found: Vec<String> = candidates.iter()
            .flat_map(|stem| {
                let pattern = home.join(stem.trim_start_matches('/'));
                glob_like(&pattern.display().to_string())
            })
            .collect();
        if !found.is_empty() {
            ui::info("backups still on disk (delete or restore manually):");
            for b in &found {
                println!("    {}", b);
            }
        }
    }

    ui::ok("desktop shells reset — HyDE-only state restored");
    if !purge_packages {
        ui::info("packages NOT removed (rerun with --purge-packages to also \
            `pacman -Rns caelestia-shell quickshell aubio`)");
    }
    Ok(())
}

/// Lightweight glob — returns paths matching `pattern` where `*` is wildcard.
/// Avoids pulling in the `glob` crate for this one use site.
fn glob_like(pattern: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (dir, prefix) = match pattern.rfind('/') {
        Some(i) => (&pattern[..i], &pattern[i + 1..]),
        None => (".", pattern),
    };
    let Ok(entries) = std::fs::read_dir(dir) else { return out; };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if prefix.ends_with('.') {
            if name.starts_with(prefix) {
                out.push(e.path().display().to_string());
            }
        } else if name == prefix {
            out.push(e.path().display().to_string());
        }
    }
    out
}
