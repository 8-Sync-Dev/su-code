// 8sync — vibe coding harness for CachyOS + Kitty + Helix
// Org: 8-Sync-Dev

mod ui;
mod env_detect;
mod pkg;
mod assets;
mod verbs;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "8sync",
    bin_name = "8sync",
    version,
    about = "vibe coding harness for CachyOS + Kitty + Helix",
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
  8sync setup                       install harness, then ask y/N per profile
  8sync setup --yall                install harness + ALL profiles, no prompts
  8sync .                           open project session (kitty + forge)
  8sync ai \"add dark mode toggle\"   ai prompt
  8sync ship \"feat: dark mode\"      commit + push + PR
  8sync end                         capture knowledge, close session
  8sync sec on                      WARP VPN + ufw firewall on

Every verb supports -h:
  8sync setup -h    8sync ai -h    8sync sec -h
";

#[derive(Subcommand)]
enum Cmd {
    /// Install harness (helix/lazygit/abduco/gh + forge + configs + skills) then prompt per personal profile
    Setup(verbs::setup::Args),

    /// Update managed tools (only if newer version available). Self-updates 8sync binary from GitHub first.
    #[command(alias = "update")]
    Up,

    /// Health-check; report what's installed and what's missing
    Doctor,

    /// Open project session: kitty 3-pane (if remote control on) + forge in abduco. Subcommands: ls/to/new/rm/mv/wipe/kick
    #[command(name = ".", alias = "here")]
    Here(verbs::here::Args),

    /// AI session / one-shot prompt (forge)
    Ai(verbs::ai::Args),

    /// Commit + push + PR (smart shortcut)
    Ship(verbs::ship::Args),

    /// Run project command per recipe (dev/build/test/fmt/lint)
    Run(verbs::run::Args),

    /// Security toggle: WARP VPN + ufw firewall (on/off/status/toggle)
    Sec(verbs::sec::Args),

    /// Capture session knowledge, save state, close panes
    End,

    /// Manage skill library (list/add/sync)
    Skill(verbs::skill::Args),

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

    /// Workflow-ordered help (lifecycle commands theo thứ tự dùng)
    Flow,

    /// Search code (rg + fzf) or filenames (fd); pick → open in $EDITOR or helix
    Find(verbs::find::Args),

    /// Append a one-line note to agents/NOTES.md (AI sẽ đọc lại session sau)
    Note(verbs::note::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if !matches!(
        cli.cmd,
        Some(Cmd::Up) | Some(Cmd::Setup(_)) | Some(Cmd::Help)
    ) {
        verbs::selfup::auto_check_notice();
    }
    match cli.cmd {
        None => {
            verbs::root::print_cheatsheet();
            Ok(())
        }
        Some(Cmd::Setup(a))   => verbs::setup::run(a),
        Some(Cmd::Up)         => verbs::up::run(),
        Some(Cmd::Doctor)     => verbs::doctor::run(),
        Some(Cmd::Here(a))    => verbs::here::run(a),
        Some(Cmd::Ai(a))      => verbs::ai::run(a),
        Some(Cmd::Ship(a))    => verbs::ship::run(a),
        Some(Cmd::Run(a))     => verbs::run::run(a),
        Some(Cmd::Sec(a))     => verbs::sec::run(a),
        Some(Cmd::End)        => verbs::end::run(),
        Some(Cmd::Skill(a))   => verbs::skill::run(a),
        Some(Cmd::Shot(a))    => verbs::shot::run(a),
        Some(Cmd::DiffImg(a)) => verbs::diff_img::run(a),
        Some(Cmd::PdfImg(a))  => verbs::pdf_img::run(a),
        Some(Cmd::Help)       => { verbs::root::print_cheatsheet(); Ok(()) }
        Some(Cmd::Flow)       => verbs::flow::run(),
        Some(Cmd::Find(a))    => verbs::find::run(a),
        Some(Cmd::Note(a))    => verbs::note::run(a),
    }
}
