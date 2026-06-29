// 8sync — vibe coding harness for CachyOS + omp
// Org: 8-Sync-Dev

mod ui;
mod env_detect;
mod pkg;
mod assets;
mod models;
mod verbs;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "8sync",
    bin_name = "8sync",
    version,
    about = "vibe coding harness for CachyOS + omp",
    long_about = None,
    disable_help_subcommand = true,
    after_help = HELP_AFTER,
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

const HELP_AFTER: &str = "\
QUICK START
  8sync                             show this overview (any time)
  8sync flow                        same as above but ordered by workflow
  8sync setup                       install harness, then ask y/N per profile
  8sync setup --community           install harness + dev-stack + bluetooth, no prompts
  8sync .                           seed agents/* context and run `omp --continue`
  8sync ai \"add dark mode toggle\"   one-shot AI prompt (or resume with `8sync ai`)
  8sync find \"useAuth\"              rg + fzf preview, open at file:line
  8sync ship \"feat: dark mode\"      commit + push + open a GitHub PR
  8sync sec on                      enable WARP VPN + ufw firewall
  8sync bt fix                      troubleshoot bluetooth (unblock + restart + power on)

Every verb supports -h / --help for detailed help with examples:
  8sync setup -h    8sync ai -h    8sync bt -h    8sync find -h
";

#[derive(Subcommand)]
enum Cmd {
    /// Install harness (gh + omp + configs + skills) then prompt per personal profile
    Setup(verbs::setup::Args),

    /// Full update: 8sync + omp + system pkgs (pacman/AUR) + rustup + flatpak. See `8sync up -h`.
    #[command(alias = "update")]
    Up(verbs::up::Args),

    /// Health-check; report what's installed and what's missing
    Doctor,

    /// Seed agents/* context for the current project and exec `omp --continue`
    #[command(name = ".", alias = "here")]
    Here(verbs::here::Args),

    /// AI session / one-shot prompt (omp)
    Ai(verbs::ai::Args),

    /// Commit + push + PR (smart shortcut)
    Ship(verbs::ship::Args),

    /// Run project command per recipe (dev/build/test/fmt/lint)
    Run(verbs::run::Args),

    /// Security toggle: WARP VPN + ufw firewall (on/off/status/toggle)
    Sec(verbs::sec::Args),

    /// Bluetooth control + troubleshoot (status/on/off/fix/restart)
    Bt(verbs::bt::Args),

    /// Reclaim disk/RAM, tidy caches, report CPU/GPU (--deep/--ram/--gpu/--watch/--timer)
    Clean(verbs::clean::Args),

    /// Manage skill library (list/add/sync)
    Skill(verbs::skill::Args),

    /// Stand up / refresh the agent harness (init = deploy skills+codegraph+AGENTS.md+memory; up = refresh)
    Harness(verbs::harness::Args),

    /// Render web route / file to PNG (for AI image-routing)
    Shot(verbs::shot::Args),

    /// Render git diff to PNG
    #[command(name = "diff-img")]
    DiffImg(verbs::diff_img::Args),

    /// Render PDF pages to PNG
    #[command(name = "pdf-img")]
    PdfImg(verbs::pdf_img::Args),

    /// Show overview cheatsheet (alias of `8sync` with no args)
    Help,

    /// Workflow-ordered help (lifecycle commands in chronological order)
    Flow,

    /// Search code (rg + fzf) or filenames (fd); pick → open in $EDITOR or hx
    Find(verbs::find::Args),

    /// Append a one-line note to agents/NOTES.md (AI will read it in the next session)
    Note(verbs::note::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if !matches!(
        cli.cmd,
        Some(Cmd::Up(_)) | Some(Cmd::Setup(_)) | Some(Cmd::Help)
    ) {
        verbs::selfup::auto_check_notice();
    }
    match cli.cmd {
        None => {
            verbs::root::print_cheatsheet();
            Ok(())
        }
        Some(Cmd::Setup(a))   => verbs::setup::run(a),
        Some(Cmd::Up(a))      => verbs::up::run(a),
        Some(Cmd::Doctor)     => verbs::doctor::run(),
        Some(Cmd::Here(a))    => verbs::here::run(a),
        Some(Cmd::Ai(a))      => verbs::ai::run(a),
        Some(Cmd::Ship(a))    => verbs::ship::run(a),
        Some(Cmd::Run(a))     => verbs::run::run(a),
        Some(Cmd::Sec(a))     => verbs::sec::run(a),
        Some(Cmd::Bt(a))      => verbs::bt::run(a),
        Some(Cmd::Clean(a))   => verbs::clean::run(a),
        Some(Cmd::Skill(a))   => verbs::skill::run(a),
        Some(Cmd::Harness(a)) => verbs::harness::run(a),
        Some(Cmd::Shot(a))    => verbs::shot::run(a),
        Some(Cmd::DiffImg(a)) => verbs::diff_img::run(a),
        Some(Cmd::PdfImg(a))  => verbs::pdf_img::run(a),
        Some(Cmd::Help)       => { verbs::root::print_cheatsheet(); Ok(()) }
        Some(Cmd::Flow)       => verbs::flow::run(),
        Some(Cmd::Find(a))    => verbs::find::run(a),
        Some(Cmd::Note(a))    => verbs::note::run(a),
    }
}
