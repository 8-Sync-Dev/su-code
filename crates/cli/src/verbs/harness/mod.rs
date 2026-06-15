//! `8sync harness` — stand up & refresh the project's agent harness.
//!
//! `init` (default): full bootstrap — deploy bundled skills + codegraph binary +
//! external skill packs, mirror into the project, init codegraph, seed memory +
//! CHANGELOG, inject force-load into AGENTS.md/CLAUDE.md + every sub-folder.
//! `up`: refresh to the current project state (re-inject + re-index codegraph),
//! with `--loop`/`--timer` for periodic runs.
use anyhow::Result;
use clap::Args as ClapArgs;

use crate::{env_detect, ui};

mod external;
mod init;
mod memory;
mod up;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync harness init              full bootstrap: skills + codegraph + AGENTS.md + memory + CHANGELOG
      8sync harness                   same as `harness init`
      8sync harness up                refresh skills/AGENTS.md/memory + re-index codegraph to current state
      8sync harness up --loop 10m     foreground: refresh every 10m (Ctrl-C to stop)
      8sync harness up --timer 30m    install a systemd USER timer (recommended for background)
      8sync harness up --timer off    remove the timer

    WHAT init DEPLOYS
      always-on : codegraph · karpathy · ponytail · assp · impeccable · taste · 8sync-cli · image-routing
      on-demand : code-review-and-quality · senior-security · senior-frontend · full-flow · last30days
      tech-gated: encore-deploy (only surfaced when the project uses Encore)
      external  : ponytail (full) + addyosmani/agent-skills (best-effort clone → ~/.omp/skills)
"})]
pub struct Args {
    /// init (default) | up
    pub sub: Option<String>,
    /// `up --loop <dur>`: refresh every <dur> in the foreground (e.g. 10m, 1h, 30s)
    #[arg(long = "loop", value_name = "DUR")]
    pub loop_every: Option<String>,
    /// `up --timer <dur|off>`: install/remove a systemd user timer
    #[arg(long, value_name = "DUR|off")]
    pub timer: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    match a.sub.as_deref() {
        None | Some("init") => init::harness_init(&env),
        Some("up") => up::harness_up(&env, a.loop_every.as_deref(), a.timer.as_deref()),
        Some(other) => {
            ui::warn(&format!("unknown subcommand: {}", other));
            ui::info("try: 8sync harness init | 8sync harness up [--loop DUR | --timer DUR|off]");
            Ok(())
        }
    }
}
