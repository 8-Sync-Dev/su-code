use anyhow::Result;
use clap::Args as ClapArgs;
use std::path::PathBuf;
use std::process::Command;

use crate::{assets, env_detect, pkg, ui, verbs::profile};

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES — quick start (community)
          8sync setup                          harness + curated y/N menu (community profiles)
          8sync setup --community              unattended: dev-stack + bluetooth
          8sync setup --profile dev-stack      just dev-stack (Docker + Node/Bun + Encore)
          8sync setup --no-profile             harness only (skip profile stage)
          8sync setup --profile terminal       kitty glass + helix + Nerd font (opt-in)
          8sync setup --dry-run                print the full plan, change nothing

        STAGE A — HARNESS (always run, idempotent)
          · pacman -S --needed github-cli       (required by `8sync ship`)
          · omp AI CLI                          (curl installer from omp.sh, if missing)
          · paru                                (AUR helper, if missing)
          · codegraph                           (semantic code index)
          · PATH bootstrap                      (~/.local/bin, ~/.cargo/bin, ~/.bun/bin,
                                                 ~/.encore/bin — zsh/bash + fish conf.d)
          · configs + skills under ~/.config/8sync/ and ~/.omp/skills/

        STAGE B — PROFILES (community-visible)
          dev-stack    Docker + Node/npm/bun/pnpm + Encore + TS LSP + build chain
          nvidia       Auto-detect GPU family → open-dkms / dkms (skipped if chwd active)
          warp         Cloudflare WARP VPN + DoH + MASQUE  (toggle via `8sync sec`)
          bluetooth    bluez + bluez-utils + service enable  (control via `8sync bt`)
          terminal     kitty (glass) + helix + JetBrains Nerd font (3-pane vibe loop)

        PROFILE MANAGEMENT
          8sync setup profile list             every profile (community + personal tag)
          8sync setup profile show <name>      resolved packages + services + post-install
          8sync setup profile apply <name>     (re-)apply one profile idempotently

        UNATTENDED MODE (auto-on with --community / --full / --profile)
          1. Preflight: print OS, display manager, sessions, GPU, tool presence
          2. Log every step to ~/.cache/8sync/setup-<unix_ts>.log
          3. On any step failure: log + track + CONTINUE (re-run to retry)
          4. Auto-yes (--noconfirm) for every pacman / AUR install

        SAFETY
          · Every install is transactional: failed pacman/AUR batch is rolled back.
          · Re-running setup is idempotent.
          · `--dry-run` is always safe.
    "}
)]
pub struct Args {
    /// Sub-command: `profile [list|show|apply <name>]`
    pub action: Option<String>,
    /// Arguments for the sub-command.
    pub rest: Vec<String>,

    /// Install EVERYTHING unattended (alexdev bundle = nvidia driver + all personal profiles).
    /// Equivalent to applying the `alexdev` bundle. Aliases: `--yall`, `--yes`, `-y`.
    /// Implies preflight + log + skip-on-error.
    #[arg(long = "full", alias = "yall", alias = "yes", short = 'y')]
    pub full: bool,

    /// Community bundle: dev-stack + bluetooth (unattended).
    /// Does NOT include `warp` — opt-in via `--profile warp`.
    #[arg(long)]
    pub community: bool,

    /// Skip Stage B entirely (harness only — no profile prompts).
    #[arg(long)]
    pub no_profile: bool,

    /// Apply a specific profile non-interactively (use after Stage A).
    #[arg(long)]
    pub profile: Option<String>,

    /// Auto-reboot after install completes (10s countdown — Ctrl-C cancels).
    /// Needed when a new kernel module landed (NVIDIA driver upgrade, etc.).
    /// Otherwise a logout is enough for new sessions to pick up the change.
    #[arg(long)]
    pub reboot: bool,

    /// Print the plan without making any changes.
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(a: Args) -> Result<()> {
    // Sub-command: `8sync setup profile <action>`
    if a.action.as_deref() == Some("profile") {
        return profile_sub(a.rest, a.full, a.dry_run);
    }

    ui::header("8sync setup");
    let env = env_detect::Env::detect()?;
    if !env.is_cachyos_or_arch() {
        ui::warn(&format!(
            "OS `{}` is not CachyOS/Arch — some steps may fail.",
            env.os_id
        ));
    }

    // ── YOLO mode setup: auto-on for any unattended path ─────────
    // Triggers when user requests an unambiguous install path:
    //   --full          → alexdev bundle
    //   --profile <n>   → just one profile
    // Strict mode (default `8sync setup` with no flags) keeps existing
    // behaviour: interactive prompts, errors bail, no log file.
    let yolo = a.full || a.profile.is_some() || a.community;
    let log_path = if yolo && !a.dry_run {
        init_yolo_log().ok()
    } else {
        None
    };
    if yolo {
        preflight(&env);
    }
    let mut failures: Vec<String> = Vec::new();

    // ── Stage A: Harness (always run) ────────────────────────────
    ui::step("Stage A — coding harness");
    if a.dry_run {
        ui::info("would install: github-cli");
        ui::info("would install omp (curl) if missing");
        ui::info("would install paru (AUR helper) if missing");
        ui::info("would install codegraph (curl) if missing");
        ui::info("would write: configs + skills");
        ui::info("would patch PATH in zsh/bash + ~/.config/fish/conf.d/8sync-path.fish");
        ui::info("would register codegraph as a global+local skill");
    } else {
        try_step("github-cli", yolo, &mut failures, || {
            pkg::pacman_install_safe(&["github-cli"], true)
        })?;
        try_step("omp",        yolo, &mut failures, install_omp)?;
        try_step("paru",       yolo, &mut failures, install_aur_helper)?;
        try_step("codegraph",  yolo, &mut failures, install_codegraph)?;
        try_step("path-bootstrap", yolo, &mut failures, || { ensure_path_in_shells(); Ok(()) })?;
        try_step("configs",    yolo, &mut failures, || install_configs(&env))?;
        try_step("skills",     yolo, &mut failures, || install_skills(&env))?;
        try_step("codegraph-skill", yolo, &mut failures, || register_codegraph_skill(&env))?;
    }

    // ── Stage B: Profiles (optional) ─────────────────────────────
    if a.no_profile {
        ui::info("--no-profile → skipping personal profiles");
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }

    let all = profile::load_all()?;

    // explicit --profile <name>
    if let Some(name) = a.profile.as_ref() {
        if name == "terminal" {
            ui::step("Stage B — terminal (kitty glass + helix + Nerd font)");
            try_step("terminal", yolo, &mut failures, || install_terminal(&env, a.dry_run))?;
            finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
            return Ok(());
        }
        ui::step(&format!("Stage B — applying profile `{}`", name));
        try_step(&format!("profile:{}", name), yolo, &mut failures, || {
            let resolved = profile::resolve(name, &all)?;
            profile::apply(&resolved, true, a.dry_run)?;
            if !a.dry_run {
                profile::mark_applied(name)?;
            }
            Ok(())
        })?;
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }

    // --full: apply alexdev bundle (nvidia + all personal profiles)
    if a.full {
        let bundle = if all.contains_key("alexdev") { "alexdev" } else { "" };
        if !bundle.is_empty() {
            ui::step(&format!("Stage B — --full: applying `{}` bundle", bundle));
            try_step(&format!("profile:{}", bundle), yolo, &mut failures, || {
                let resolved = profile::resolve(bundle, &all)?;
                profile::apply(&resolved, true, a.dry_run)?;
                if !a.dry_run {
                    profile::mark_applied(bundle)?;
                }
                Ok(())
            })?;
        } else {
            ui::warn("no `alexdev` bundle — applying every non-bundle profile individually");
            let mut names: Vec<&String> = all.keys().collect();
            names.sort();
            for n in &names {
                let p = match all.get(*n) { Some(p) => p, None => continue };
                if !p.extends.is_empty() { continue; } // skip bundles
                try_step(&format!("profile:{}", n), yolo, &mut failures, || {
                    let resolved = profile::resolve(n, &all)?;
                    profile::apply(&resolved, true, a.dry_run)?;
                    if !a.dry_run { profile::mark_applied(n)?; }
                    Ok(())
                })?;
            }
        }
        try_step("terminal", yolo, &mut failures, || install_terminal(&env, a.dry_run))?;
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }

    // --community: dev-stack + bluetooth (NOT warp)
    if a.community {
        let bundle = ["dev-stack", "bluetooth"];
        ui::step("Stage B — --community: dev-stack + bluetooth");
        for n in &bundle {
            if !all.contains_key(*n) {
                ui::warn(&format!("profile `{}` not found — skipping", n));
                continue;
            }
            try_step(&format!("profile:{}", n), yolo, &mut failures, || {
                let resolved = profile::resolve(n, &all)?;
                profile::apply(&resolved, true, a.dry_run)?;
                if !a.dry_run { profile::mark_applied(n)?; }
                Ok(())
            })?;
        }
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }

    // Interactive y/N per profile (skip bundle profiles)
    if !env_detect::has_tty() {
        ui::info("no TTY — skipping interactive profile prompt (use --full / --profile <name>)");
        finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
        return Ok(());
    }


    ui::step("Stage B — community profiles (y/N each)");
    let order = ["dev-stack", "nvidia", "bluetooth", "warp"];
    let mut names: Vec<&String> = all
        .iter()
        .filter(|(_, p)| p.extends.is_empty() && p.visibility == profile::Visibility::Community)
        .map(|(k, _)| k)
        .collect();
    names.sort_by_key(|n| {
        order
            .iter()
            .position(|o| o == &n.as_str())
            .unwrap_or(usize::MAX)
    });
    for name in &names {
        let p = match all.get(*name) {
            Some(p) => p,
            None => continue,
        };
        if !p.extends.is_empty() {
            continue;
        }
        let desc = if p.description.is_empty() {
            name.as_str()
        } else {
            p.description.as_str()
        };
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
    if ui::prompt_yes_no("Apply `terminal` — kitty glass + helix + Nerd font (3-pane vibe loop)", false) {
        if let Err(e) = install_terminal(&env, a.dry_run) {
            ui::err(&format!("terminal failed: {}", e));
        }
    }

    finish_summary(&failures, log_path.as_ref(), a.reboot, a.dry_run);
    Ok(())
}

fn finish_msg() {
    ui::header("Done — next steps");
    println!("  · 8sync doctor               — verify");
    println!("  · cd <project> && 8sync .    — seed agents/ + start omp --continue");
}

/// Print final summary: log path (if any) + list of failures (if any).
/// Always prints `finish_msg` next-steps at the end.
/// If `reboot=true` and no failures, triggers a 10s-cancellable reboot.
fn finish_summary(failures: &[String], log_path: Option<&PathBuf>, reboot: bool, dry_run: bool) {
    if let Some(p) = log_path {
        ui::info(&format!("full log: {}", p.display()));
    }
    if failures.is_empty() {
        ui::ok("all steps succeeded (no failures recorded)");
    } else {
        ui::warn(&format!(
            "{} step(s) failed but were skipped (unattended mode): {}",
            failures.len(),
            failures.join(", ")
        ));
        ui::info("re-run the same command to retry — every step is idempotent");
    }
    ui::close_log_file();
    finish_msg();

    if reboot && !dry_run {
        if !failures.is_empty() {
            ui::warn("--reboot requested but some steps failed — aborting reboot. Fix or re-run, then reboot manually.");
            return;
        }
        do_reboot_with_countdown(10);
    }
}

/// Print a `secs`-second countdown then `systemctl reboot`. Ctrl-C cancels.
fn do_reboot_with_countdown(secs: u32) {
    use std::io::Write;
    println!();
    ui::warn(&format!("rebooting in {}s — press Ctrl-C to cancel", secs));
    for i in (1..=secs).rev() {
        print!("\r  ⏱  {}s remaining... ", i);
        std::io::stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("\r  ⏱  rebooting now              ");
    let _ = Command::new("systemctl").arg("reboot").status();
}

// ─────────────────────────────────────────────────────────────────
// Stage A helpers
// ─────────────────────────────────────────────────────────────────

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
        &[
            "-c",
            "curl -fsSL https://raw.githubusercontent.com/colbymchenry/codegraph/main/install.sh | sh",
        ],
    )?;
    ensure_path_in_shells();
    Ok(())
}

/// Ensure user-local bin dirs are on PATH in zsh/bash, and drop a fish
/// `conf.d/` snippet that does the same via `fish_add_path`. Idempotent.
///
/// Paths covered (any that exist or will exist after setup):
///   ~/.local/bin   — codegraph, 8sync, encore (most installers)
///   ~/.cargo/bin   — rustup-installed binaries (cargo, rust-analyzer)
///   ~/.bun/bin     — bun runtime / `bun install -g` shims
///   ~/.encore/bin  — encore CLI
fn ensure_path_in_shells() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let dirs = [".local/bin", ".cargo/bin", ".bun/bin", ".encore/bin"];

    // ── zsh / bash ────────────────────────────────────────────────
    let marker = "# 8sync: PATH bootstrap (user-local bins for codegraph/bun/encore/cargo)";
    let mut posix_block = String::from("\n");
    posix_block.push_str(marker);
    posix_block.push('\n');
    for d in &dirs {
        let p = home.join(d);
        posix_block.push_str(&format!(
            "case \":$PATH:\" in *\":{lb}:\"*) ;; *) export PATH=\"{lb}:$PATH\" ;; esac\n",
            lb = p.display(),
        ));
    }
    for rc in [home.join(".zshrc"), home.join(".bashrc")] {
        if !rc.exists() {
            continue;
        }
        let existing = std::fs::read_to_string(&rc).unwrap_or_default();
        if existing.contains(marker) {
            continue;
        }
        if let Err(e) = std::fs::OpenOptions::new()
            .append(true)
            .open(&rc)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(posix_block.as_bytes())
            })
        {
            ui::warn(&format!("could not patch {}: {}", rc.display(), e));
            continue;
        }
        ui::ok(&format!("patched {} (PATH bootstrap)", rc.display()));
    }

    // ── fish (conf.d snippet — sourced on every interactive session) ─
    let fish_dir = home.join(".config/fish/conf.d");
    if let Err(e) = std::fs::create_dir_all(&fish_dir) {
        ui::warn(&format!("could not create {}: {}", fish_dir.display(), e));
        return;
    }
    let fish_file = fish_dir.join("8sync-path.fish");
    let mut fish_body = String::new();
    fish_body.push_str("# 8sync: PATH bootstrap — regenerated by `8sync setup`. Do not edit.\n");
    fish_body.push_str("if status is-interactive\n");
    fish_body.push_str("    fish_add_path --path \\\n");
    let entries: Vec<String> = dirs
        .iter()
        .map(|d| format!("        $HOME/{}", d))
        .collect();
    fish_body.push_str(&entries.join(" \\\n"));
    fish_body.push('\n');
    fish_body.push_str("end\n");
    let existing = std::fs::read_to_string(&fish_file).unwrap_or_default();
    if existing == fish_body {
        ui::skip(&fish_file.display().to_string(), "unchanged");
        return;
    }
    if let Err(e) = std::fs::write(&fish_file, &fish_body) {
        ui::warn(&format!("could not write {}: {}", fish_file.display(), e));
        return;
    }
    ui::ok(&format!("wrote {} (fish PATH bootstrap)", fish_file.display()));
}

fn register_codegraph_skill(env: &env_detect::Env) -> Result<()> {
    ui::step("Register codegraph skill (bundled)");
    // SKILL.md tree is shipped from embedded assets via `install_skills`
    // (no upstream README synthesis). Here we just append a registry entry to
    // skills.toml so `8sync skill list` shows codegraph as an always-on skill.
    let toml_path = env.xdg_config.join("8sync/skills.toml");
    if let Some(parent) = toml_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = std::fs::read_to_string(&toml_path).unwrap_or_default();
    if existing.contains("[codegraph]") {
        ui::skip(&toml_path.display().to_string(), "codegraph already registered");
        return Ok(());
    }
    let mut s = existing;
    if !s.ends_with('\n') && !s.is_empty() {
        s.push('\n');
    }
    s.push_str("\n[codegraph]\nsrc  = \"builtin:codegraph\"\nwhen = \"always\"\n");
    std::fs::write(&toml_path, s)?;
    ui::ok(&format!("registered 'codegraph' → {}", toml_path.display()));
    Ok(())
}

fn install_configs(env: &env_detect::Env) -> Result<()> {
    ui::step("Configs (8sync/{global,skills}.toml)");
    let pairs = [
        ("configs/global.toml", env.xdg_config.join("8sync/global.toml")),
        ("configs/skills.toml", env.xdg_config.join("8sync/skills.toml")),
        ("configs/models.toml", env.xdg_config.join("8sync/models.toml")),
    ];
    for (asset, target) in &pairs {
        let changed = assets::install(asset, target, false)?;
        if changed {
            ui::ok(&format!("wrote {}", target.display()));
        } else {
            ui::skip(&target.display().to_string(), "unchanged");
        }
    }
    Ok(())
}

/// Opt-in terminal/editor nicety (Stage B, NOT the AI core): kitty (terminal),
/// helix (`hx`), and a Nerd font for the glass theme. Docker lives in `dev-stack`.
fn install_terminal_pkgs() -> Result<()> {
    pkg::pacman_install_safe(&["kitty", "helix", "ttf-jetbrains-mono-nerd"], true)
}

/// Deploy the kitty glass theme (transparency + wallpaper + splits) without
/// clobbering the user's kitty.conf, plus a transparent helix config if absent.
fn install_terminal_config(env: &env_detect::Env) -> Result<()> {
    ui::step("Terminal (kitty glass + wallpaper + helix)");
    let kitty_dir = env.xdg_config.join("kitty");
    std::fs::create_dir_all(&kitty_dir)?;

    // Wallpaper → ~/.config/8sync/wallpaper.png (bundled asset preferred, else URL).
    let wp = env.xdg_config.join("8sync/wallpaper.png");
    let wp_ready = deploy_wallpaper(env, &wp);

    // Glass conf → ~/.config/kitty/8sync.conf (absolute wallpaper path baked in).
    let conf_path = kitty_dir.join("8sync.conf");
    std::fs::write(&conf_path, render_kitty_conf(wp_ready.then_some(wp.as_path())))?;
    ui::ok(&format!("wrote {}", conf_path.display()));

    // Make the user's kitty.conf include ours (managed line, idempotent, no clobber).
    ensure_kitty_include(&kitty_dir)?;

    // Helix: transparent config if the user has none yet (never overwrite).
    let hx_cfg = env.xdg_config.join("helix/config.toml");
    if !hx_cfg.exists() && assets::read("configs/helix/config.toml").is_some() {
        assets::install("configs/helix/config.toml", &hx_cfg, false)?;
        ui::ok(&format!("wrote {}", hx_cfg.display()));
    } else {
        ui::skip("helix config", "exists or no asset — left as-is");
    }
    Ok(())
}

/// Opt-in terminal stack (packages + glass config). Used by the Stage B menu,
/// `--profile terminal`, and `--full` — never in the default AI-core Stage A.
fn install_terminal(env: &env_detect::Env, dry_run: bool) -> Result<()> {
    if dry_run {
        ui::info("would install: kitty + helix + JetBrains Nerd font");
        ui::info("would deploy kitty glass config + wallpaper + helix config (if absent)");
        return Ok(());
    }
    install_terminal_pkgs()?;
    install_terminal_config(env)
}

/// Put a wallpaper at `target`. Bundled `assets/wallpapers/default.png` wins; else
/// download `[ui].wallpaper_url` from global.toml with curl. True if present after.
fn deploy_wallpaper(env: &env_detect::Env, target: &std::path::Path) -> bool {
    if target.exists() {
        return true;
    }
    if let Some(p) = target.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if let Some(bytes) = assets::read_bytes("wallpapers/default.png") {
        if std::fs::write(target, bytes).is_ok() {
            ui::ok(&format!("wallpaper → {}", target.display()));
            return true;
        }
    }
    if let Some(url) = wallpaper_url(env) {
        let ok = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(target)
            .arg(&url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok && target.exists() {
            ui::ok(&format!("wallpaper ↓ {}", target.display()));
            return true;
        }
        ui::skip("wallpaper", "no bundled image and download failed");
    }
    false
}

/// `[ui].wallpaper_url` from the deployed global.toml, else the embedded default.
fn wallpaper_url(env: &env_detect::Env) -> Option<String> {
    let s = std::fs::read_to_string(env.xdg_config.join("8sync/global.toml"))
        .ok()
        .or_else(|| assets::read("configs/global.toml"))?;
    let v: toml::Value = s.parse().ok()?;
    v.get("ui")?.get("wallpaper_url")?.as_str().map(str::to_string)
}

/// The glass kitty theme body: transparency + blur (Hyprland/KDE/picom), optional
/// wallpaper, violet accent, and 3-pane split keymaps.
fn render_kitty_conf(wallpaper: Option<&std::path::Path>) -> String {
    let bg_image = match wallpaper {
        Some(p) => format!(
            "background_image {}\nbackground_image_layout cscaled\nbackground_image_linear yes\nbackground_tint 0.92\nbackground_tint_gaps 0.0\n",
            p.display()
        ),
        None => String::new(),
    };
    let header = indoc::indoc! {"
        # 8sync — glass dark terminal (managed by `8sync setup`; included from kitty.conf)
        # Transparency + blur (honored by Hyprland/KDE/picom compositors).
        background_opacity 0.82
        dynamic_background_opacity yes
        background_blur 28
        background #0b0d12
    "};
    let rest = indoc::indoc! {"
        # Font (JetBrains Mono Nerd Font installed by setup)
        font_family JetBrainsMono Nerd Font
        bold_font auto
        italic_font auto
        font_size 12.0
        # Window + layouts
        enabled_layouts splits:split_axis=horizontal,stack,tall,grid
        window_padding_width 8
        hide_window_decorations yes
        confirm_os_window_close 0
        # Tabs
        tab_bar_edge top
        tab_bar_style powerline
        tab_powerline_style slanted
        # Colors (glass black + violet accent)
        foreground #e6e9ef
        selection_background #2a2f3a
        selection_foreground #e6e9ef
        cursor #7c5cff
        cursor_text_color #0b0d12
        url_color #8ab4ff
        active_border_color #7c5cff
        inactive_border_color #262b36
        active_tab_background #14171f
        active_tab_foreground #e6e9ef
        inactive_tab_background #0b0d12
        inactive_tab_foreground #9aa3b2
        # 3-pane splits (gsd-style)
        map ctrl+shift+enter launch --location=hsplit --cwd=current
        map ctrl+shift+minus launch --location=vsplit --cwd=current
        map ctrl+shift+] next_window
        map ctrl+shift+[ previous_window
    "};
    format!("{header}{bg_image}{rest}")
}

/// Ensure `~/.config/kitty/kitty.conf` includes our managed 8sync.conf. Creates
/// the file if missing; appends the include once (idempotent, never clobbers).
fn ensure_kitty_include(kitty_dir: &std::path::Path) -> Result<()> {
    let main = kitty_dir.join("kitty.conf");
    let mut body = std::fs::read_to_string(&main).unwrap_or_default();
    if body.contains("include 8sync.conf") {
        ui::skip("kitty.conf", "already includes 8sync.conf");
        return Ok(());
    }
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    body.push_str("\n# 8sync glass theme (managed by `8sync setup`)\ninclude 8sync.conf\n");
    std::fs::write(&main, body)?;
    ui::ok("kitty.conf now includes 8sync.conf");
    Ok(())
}

fn install_skills(env: &env_detect::Env) -> Result<()> {
    ui::step("Skills (~/.omp/skills/)");
    let skills_dir = env.home.join(".omp/skills");
    std::fs::create_dir_all(&skills_dir)?;
    // Bundled skills: deploy entire tree (SKILL.md + scripts/ + references/).
    let bundled: [(&str, &str); 4] = [
        ("skills/karpathy",      "karpathy-guidelines"),
        ("skills/image-routing", "image-routing"),
        ("skills/8sync-cli",     "8sync-cli"),
        ("skills/codegraph",     "codegraph"),
    ];
    for (prefix, name) in &bundled {
        let target = skills_dir.join(name);
        std::fs::create_dir_all(&target)?;
        let (written, _unchanged) = assets::install_tree(prefix, &target)?;
        if written > 0 {
            ui::ok(&format!("synced {} ({} file(s)) → {}", name, written, target.display()));
        } else {
            ui::skip(&target.display().to_string(), "unchanged");
        }
        // Ensure 3-folder layout even if the bundled tree didn't ship a
        // `scripts/` or `references/` subdir (karpathy/image-routing/8sync-cli).
        for sub in ["scripts", "references"] {
            let _ = std::fs::create_dir_all(target.join(sub));
        }
    }
    let master = skills_dir.join("00-force-load.md");
    assets::install("skills/00-force-load.md", &master, true)?;
    ui::ok(&format!("wrote {}", master.display()));
    Ok(())
}

// ─── systemd helper (used by preflight) ─────────────────────────

fn systemctl_is_enabled(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["is-enabled", &format!("{}.service", unit)])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ─── `8sync setup profile <sub>` ────────────────────────────────

fn profile_sub(rest: Vec<String>, yes_to_all: bool, dry_run: bool) -> Result<()> {
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
                let marker = if state.applied.iter().any(|x| x == n) {
                    "✓"
                } else {
                    " "
                };
                let kind = if !p.extends.is_empty() { "(bundle)" } else { "" };
                let vis = match p.visibility {
                    profile::Visibility::Community => "community",
                    profile::Visibility::Personal => "personal ",
                };
                println!("  {} {:20} [{}] {} {}", marker, n, vis, kind, p.description);
            }
            Ok(())
        }
        "show" => {
            let name = rest
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("usage: 8sync setup profile show <name>"))?;
            let r = profile::resolve(name, &all)?;
            println!("name         = {}", r.name);
            println!("description  = {}", r.description);
            println!("visibility   = {:?}", r.visibility);
            println!("needs AUR    = {}", r.requires.aur_helper);
            println!("pacman       = {:?}", r.packages.pacman);
            println!("aur          = {:?}", r.packages.aur);
            println!("aur (yay)    = {:?}", r.packages.aur_yay);
            println!("sys services = {:?}", r.services.system_enable);
            println!("user services= {:?}", r.services.user_enable);
            println!("commands     = {:?}", r.post_install.commands);
            if !r.post_install.hint.is_empty() {
                println!("\nhints:\n{}", r.post_install.hint);
            }
            Ok(())
        }
        "apply" => {
            let name = rest
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("usage: 8sync setup profile apply <name>"))?;
            let resolved = profile::resolve(name, &all)?;
            profile::apply(&resolved, yes_to_all, dry_run)?;
            if !dry_run {
                profile::mark_applied(name)?;
            }
            Ok(())
        }
        other => {
            ui::warn(&format!(
                "unknown sub-action `{}` — try list | show | apply",
                other
            ));
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// YOLO mode helpers (auto-on for --full / --community / --profile)
// ─────────────────────────────────────────────────────────────────

/// Open `~/.cache/8sync/setup-<unix_ts>.log` and wire `ui::*` to tee into it.
/// Idempotent across runs (timestamped filename).
fn init_yolo_log() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME"))?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = home.join(format!(".cache/8sync/setup-{}.log", ts));
    let final_path = ui::set_log_file(path)?;
    ui::ok(&format!("logging to {}", final_path.display()));
    Ok(final_path)
}

/// Quick read-only probe of system state. Prints what's already installed so
/// the install plan below is predictable. No side effects.
fn preflight(env: &env_detect::Env) {
    ui::step("Preflight — detecting current system state");

    // OS + DM
    ui::info(&format!("OS: {} ({})", env.os_id, if env.is_cachyos_or_arch() { "supported" } else { "best-effort" }));
    let dm = ["display-manager", "sddm", "plasmalogin", "gdm", "lightdm", "greetd"]
        .iter()
        .find(|d| systemctl_is_enabled(d));
    match dm {
        Some(d) => ui::info(&format!("display manager: {}.service enabled", d)),
        None => ui::info("display manager: none enabled (fresh install path)"),
    }

    // Wayland / X sessions
    let sessions = enumerate_sessions();
    if sessions.is_empty() {
        ui::info("desktop sessions: none registered");
    } else {
        ui::info(&format!("desktop sessions: {}", sessions.join(", ")));
    }

    // Core tools
    for (label, bin) in [
        ("omp", "omp"),
        ("paru", "paru"),
        ("yay", "yay"),
        ("codegraph", "codegraph"),
        ("gh", "gh"),
        ("encore", "encore"),
    ] {
        let present = which::which(bin).is_ok();
        if present {
            let v = env_detect::cmd_version(bin, &["--version"]).unwrap_or_default();
            ui::skip(label, if v.is_empty() { "present" } else { &v });
        } else {
            ui::info(&format!("{}: missing — will be installed", label));
        }
    }

    // GPU
    if let Ok(out) = std::process::Command::new("sh")
        .arg("-c")
        .arg("lspci -nn 2>/dev/null | grep -iE 'vga|3d' | head -3")
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            for line in s.lines() {
                ui::info(&format!("gpu: {}", line.trim()));
            }
        }
    }
}

fn enumerate_sessions() -> Vec<String> {
    let mut out = Vec::new();
    for dir in ["/usr/share/wayland-sessions", "/usr/share/xsessions"] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if let Some(stripped) = n.strip_suffix(".desktop") {
                        out.push(stripped.to_string());
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// In YOLO mode: log the error and continue. In strict mode: propagate.
/// `failures` tracks step labels that errored, surfaced in the summary.
fn try_step<F>(label: &str, yolo: bool, failures: &mut Vec<String>, f: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    match f() {
        Ok(()) => Ok(()),
        Err(e) if yolo => {
            ui::err(&format!("[{}] failed: {} — continuing (unattended mode)", label, e));
            failures.push(label.to_string());
            Ok(())
        }
        Err(e) => Err(e),
    }
}
