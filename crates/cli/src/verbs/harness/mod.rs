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

mod auto;
pub(crate) mod audit;
mod bench;
mod eval;
mod external;
mod init;
mod memory;
mod up;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync harness                   ONE command — deploy/update skills + mirror + inject + memory + index (idempotent)
      8sync harness init              explicit full bootstrap with progress UI (force re-deploy everything)
      8sync harness up                refresh skills/AGENTS.md/memory + re-index codegraph to current state
      8sync harness up --pull        refresh AND re-pull registered skills from their source repos
      8sync harness up --commit      refresh AND git-commit agent memory (portable; default off)
      8sync harness up --loop 10m     foreground: refresh every 10m (Ctrl-C to stop)
      8sync harness up --timer 30m    install a systemd USER timer (recommended for background)
      8sync harness up --timer off    remove the timer
      8sync harness help             cheatsheet: commands, skill tiers, file taxonomy, new-machine runbook
      8sync harness bench            benchmark the loop-engineering context budget (upfront vs deferred tokens + KV-cache gate)
      8sync harness audit             scan docs for stale paths / oversized / junk + churn hotspots (doc-hygiene)
      8sync harness eval [--baseline] run the quality task-suite through omp (pass/fail + wall-time; --baseline saves the reference)

    WHAT init DEPLOYS
      always-on : codegraph · karpathy · ponytail · assp · impeccable · taste · 8sync-cli · image-routing
      on-demand : code-review-and-quality · senior-security · senior-frontend · full-flow · last30days
      tech-gated: encore-deploy (only surfaced when the project uses Encore)
      external  : ponytail (full) + addyosmani/agent-skills (best-effort clone → ~/.omp/skills)
"})]
pub struct Args {
    /// init (default) | up | audit | bench | eval | help
    pub sub: Option<String>,
    /// `up --loop <dur>`: refresh every <dur> in the foreground (e.g. 10m, 1h, 30s)
    #[arg(long = "loop", value_name = "DUR")]
    pub loop_every: Option<String>,
    /// `up --timer <dur|off>`: install/remove a systemd user timer
    #[arg(long, value_name = "DUR|off")]
    pub timer: Option<String>,
    /// `up --pull`: also re-pull every registered skill from its source repo
    /// before re-injecting (network; default off keeps `up` fast + offline-safe)
    #[arg(long)]
    pub pull: bool,
    /// `up --commit`: also `git commit` the refreshed agent memory (scoped to
    /// agents/ + AGENTS.md/CLAUDE.md/CHANGELOG.md/.gitignore; never your code)
    #[arg(long)]
    pub commit: bool,
    /// `init --force`: re-mirror skills into agents/skills/, overwriting existing.
    /// Default is additive — never clobber an already-vendored (maybe edited) skill.
    #[arg(long)]
    pub force: bool,
    /// `eval --baseline`: save this run as outputs/eval-baseline.json (the
    /// reference future `eval` runs diff against).
    #[arg(long)]
    pub baseline: bool,
    /// `eval --project`: score agent-team READINESS on the current repo
    /// (per-role capability coverage %), instead of running loop fixtures.
    #[arg(long)]
    pub project: bool,
}

pub fn run(a: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    match a.sub.as_deref() {
        None => auto::harness_auto(&env, a.force),
        Some("init") => init::harness_init(&env, a.force),
        Some("up") => up::harness_up(&env, a.loop_every.as_deref(), a.timer.as_deref(), a.pull, a.commit),
        Some("bench") => bench::harness_bench(&env),
        Some("audit") => audit::harness_audit(&env),
        Some("eval") if a.project => eval::harness_eval_project(&env),
        Some("eval") => eval::harness_eval(&env, a.baseline),
        Some("help") => {
            print_help();
            Ok(())
        }
        Some(other) => {
            ui::warn(&format!("unknown subcommand: {}", other));
            ui::info("try: 8sync harness init | up [--pull|--commit|--loop DUR|--timer DUR|off] | audit | eval | bench | help");
            Ok(())
        }
    }
}

/// `8sync harness help` — one-screen cheatsheet: harness/skill commands, skill
/// tiers, the commit-vs-ignore file taxonomy, and the new-machine runbook.
fn print_help() {
    ui::header("8sync harness help");

    println!("COMMANDS");
    println!("  8sync harness                   ONE command — skills+update+mirror+inject+memory+index (idempotent, re-run anytime)");
    println!("  8sync harness init              full bootstrap: skills + codegraph + AGENTS.md + memory + CHANGELOG + .gitignore");
    println!("  8sync harness up                refresh: re-inject rules + KNOWLEDGE breadcrumb + codegraph index");
    println!("  8sync harness up --pull         …and re-pull registered skills from their source repos (network)");
    println!("  8sync harness up --commit       …and git-commit the refreshed agent memory (portable; default off)");
    println!("  8sync harness up --loop <dur>   foreground refresh every <dur> (10m, 1h, 30s)");
    println!("  8sync harness up --timer <dur>  install a systemd USER timer (background); `--timer off` removes it");
    println!("  8sync harness help              this cheatsheet");
    println!("  8sync harness audit             scan docs for stale paths / oversized / junk + churn (doc-hygiene)");
    println!("  8sync harness bench             benchmark the loop context budget (upfront vs deferred tokens + KV-cache gate)");
    println!("  8sync harness eval [--baseline] run the quality task-suite through omp; --baseline saves the reference");
    println!("  8sync skill [list|add|gen|update]   manage the library (`skill update [name]` re-pulls from skills.toml)");

    println!("\nSKILLS (deployed by init)");
    println!("  always-on (read order): codegraph → karpathy → ponytail → assp → impeccable → taste → 8sync-cli → image-routing");
    println!("  on-demand : code-review-and-quality · senior-security · senior-frontend · full-flow · last30days");
    println!("  tech-gated: encore-deploy (only when the project uses Encore)");
    println!("  opt-in    : social-growth — enable with `8sync skill add builtin:social-growth`");
    println!("  external  : ponytail (full) + addyosmani/agent-skills (best-effort clone → ~/.omp/skills)");

    println!("\nFILE TAXONOMY (portability — survives a move to a new machine)");
    println!("  COMMIT : AGENTS.md · CLAUDE.md · agents/*.md · CHANGELOG.md · agents/skills/   (learned/decided)");
    println!("  IGNORE : .codegraph/ · .cache/8sync/                                           (derived → rebuilt by init)");
    println!("  SECRET : .env · .env.* (keep .env.example)                                     (NEVER commit)");
    println!("  → init seeds these into a managed .gitignore block; `8sync doctor` warns if memory is ignored.");

    println!("\nNEW MACHINE (nothing lost)");
    println!("  1) git clone <repo> && cd <repo>     # agents/*.md + agents/skills/ arrive with the clone");
    println!("  2) 8sync up                          # install/refresh the 8sync binary + omp");
    println!("  3) 8sync harness init                # rebuild .codegraph + global skills, re-inject rules");
    println!("  4) prior memory (KNOWLEDGE/DECISIONS/STATE) is already present — the agent resumes context.");
}
