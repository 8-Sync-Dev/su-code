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
mod compaction;
pub(crate) mod audit;
mod bench;
mod eval;
mod external;
mod global;
mod init;
pub(crate) mod memory;
mod model;
mod local_model;
mod gateway;
mod up;
mod web;
mod marketplace;
pub(crate) mod knowledge;
mod toolstats;

#[derive(ClapArgs, Debug)]
#[command(after_help = indoc::indoc! {"
    EXAMPLES
      8sync harness                   ONE command — deploy/update skills + mirror + inject + memory + index (idempotent)
      8sync harness global            apply omp rules MACHINE-WIDE (all projects) + Anthropic token-optimizer defaults
      8sync harness global --sweep    …and stamp skills/memory into every omp project (has su-code/ or AGENTS.md) under ~/Projects
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
      8sync harness compaction [pct]  view/set omp auto-compaction threshold (default 50% — anti-forget)
      8sync harness gateway [apply|key <KEY>|verify|status]  deploy/verify the omp model-gateway (9router + thinking fix)
      8sync harness add-local-model <path.gguf|org/repo|url> [name]  serve a local GGUF via mistral.rs + register with omp
      8sync harness add-local-model list|rm <name>  list / remove registered local models

    WHAT init DEPLOYS
      always-on : codegraph · karpathy · ponytail · assp · impeccable · taste · 8sync-cli · image-routing
      on-demand : code-review-and-quality · senior-security · senior-frontend · full-flow · last30days
      tech-gated: encore-deploy (only surfaced when the project uses Encore)
      external  : ponytail (full) + addyosmani/agent-skills (best-effort clone → ~/.omp/skills)
"})]
pub struct Args {
    /// init (default) | up | global | audit | bench | eval | toolstats | model | gateway | web | compaction | help
    pub sub: Option<String>,
    /// Optional value for value-taking sub-commands (e.g. `compaction <pct>`).
    pub value: Option<String>,
    /// Second positional value (e.g. `model <key> <value>`).
    pub value2: Option<String>,
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
    /// su-code/ + AGENTS.md/CLAUDE.md/CHANGELOG.md/.gitignore; never your code)
    #[arg(long)]
    pub commit: bool,
    /// `init --force`: re-mirror skills into su-code/skills/, overwriting existing.
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
    /// `web`: launch the local dashboard (axum + Vite FE) at http://127.0.0.1:8731.
    #[arg(long)]
    pub web: bool,
    /// `web --port <PORT>`: override the dashboard port (default 8731).
    #[arg(long, value_name = "PORT")]
    pub port: Option<u16>,
    /// `web --no-open`: do not auto-open the browser.
    #[arg(long)]
    pub no_open: bool,
    /// `global --sweep [DIR]`: also stamp the per-project layer (skills mirror +
    /// AGENTS.md inject + memory seed + gitleaks hook) into every omp project
    /// (repo with su-code/ or AGENTS.md/CLAUDE.md) under DIR (default ~/Projects).
    #[arg(long, value_name = "DIR", num_args = 0..=1, default_missing_value = "")]
    pub sweep: Option<String>,
}

pub fn run(a: Args) -> Result<()> {
    let env = env_detect::Env::detect()?;
    // Shorthand: `8sync harness <sub>=<value>` == `8sync harness <sub> <value>`
    // (e.g. `model=claude+glm`, `compaction=50`). Splits the first `=` in the token.
    let (sub, v1) = match a.sub.as_deref().and_then(|s| s.split_once('=')) {
        Some((k, v)) => (Some(k.to_string()), Some(v.to_string())),
        None => (a.sub.clone(), a.value.clone()),
    };
    match sub.as_deref() {
        None => auto::harness_auto(&env, a.force),
        Some("init") => init::harness_init(&env, a.force),
        Some("up") => up::harness_up(&env, a.loop_every.as_deref(), a.timer.as_deref(), a.pull, a.commit),
        Some("global") => global::harness_global(&env, a.sweep.as_deref(), a.pull, a.force),
        Some("bench") => bench::harness_bench(&env),
        Some("audit") => audit::harness_audit(&env),
        Some("eval") if a.project => eval::harness_eval_project(&env),
        Some("eval") => eval::harness_eval(&env, a.baseline),
        Some("web") => web::harness_web(&env.home, a.port.unwrap_or(8731), a.no_open),
        Some("toolstats") | Some("tools") => toolstats::harness_toolstats(&env),
        Some("compaction") => compaction::harness_compaction(&env.home, v1.as_deref()),
        Some("model") => {
            let args: Vec<String> = [v1.clone(), a.value2.clone()].into_iter().flatten().collect();
            model::harness_model(&env, &args)
        }
        Some("gateway") => {
            let args: Vec<String> = [v1.clone(), a.value2.clone()].into_iter().flatten().collect();
            gateway::harness_gateway(&env, &args)
        }
        Some("add-local-model") | Some("add-model") => {
            let args: Vec<String> =
                [v1.clone(), a.value2.clone()].into_iter().flatten().collect();
            local_model::harness_add_local_model(&env, &args, a.port)
        }
        Some("help") => {
            print_help();
            Ok(())
        }
        Some(other) => {
            ui::warn(&format!("unknown subcommand: {}", other));
            ui::info("try: 8sync harness init | up [--pull|--commit|--loop DUR|--timer DUR|off] | global [--sweep DIR] | gateway | audit | eval | bench | help");
            Ok(())
        }
    }
}

/// `8sync harness help` — one-screen cheatsheet: harness/skill commands, skill
/// tiers, the commit-vs-ignore file taxonomy, and the new-machine runbook.
fn print_help() {
    ui::header("8sync harness help");

    println!("COMMANDS");
    println!("{}", crate::brand::render("  8sync harness                   ONE command — skills+update+mirror+inject+memory+index (idempotent, re-run anytime)"));
    println!("{}", crate::brand::render("  8sync harness global            apply omp rules MACHINE-WIDE: ~/.omp skills+APPEND_SYSTEM+MCP → ALL projects, + Anthropic token defaults"));
    println!("{}", crate::brand::render("  8sync harness global --sweep [DIR]  …and stamp skills/memory into every omp project (su-code/ or AGENTS.md) under DIR (default ~/Projects)"));
    println!("{}", crate::brand::render("  8sync harness init              full bootstrap: skills + codegraph + AGENTS.md + memory + CHANGELOG + .gitignore"));
    println!("{}", crate::brand::render("  8sync harness up                refresh: re-inject rules + KNOWLEDGE breadcrumb + codegraph index"));
    println!("{}", crate::brand::render("  8sync harness up --pull         …and re-pull registered skills from their source repos (network)"));
    println!("{}", crate::brand::render("  8sync harness up --commit       …and git-commit the refreshed agent memory (portable; default off)"));
    println!("{}", crate::brand::render("  8sync harness up --loop <dur>   foreground refresh every <dur> (10m, 1h, 30s)"));
    println!("{}", crate::brand::render("  8sync harness up --timer <dur>  install a systemd USER timer (background); `--timer off` removes it"));
    println!("{}", crate::brand::render("  8sync harness help              this cheatsheet"));
    println!("{}", crate::brand::render("  8sync harness audit             scan docs for stale paths / oversized / junk + churn (doc-hygiene)"));
    println!("{}", crate::brand::render("  8sync harness bench             benchmark the loop context budget (upfront vs deferred tokens + KV-cache gate)"));
    println!("{}", crate::brand::render("  8sync harness eval [--baseline] run the quality task-suite through omp; --baseline saves the reference"));
    println!("{}", crate::brand::render("  8sync harness compaction [pct]  view/set omp auto-compaction threshold (anti-forget; default 50%)"));
    println!("{}", crate::brand::render("  8sync harness model [combo|k v]  view/set model routing; combo `model=claude+glm` sets ALL omp roles (opus=think, glm=work)"));
    println!("{}", crate::brand::render("  8sync harness gateway [apply|key|verify]  deploy/verify omp model-gateway (9router + sonnet-5 thinking fix)"));
    println!("{}", crate::brand::render("  8sync harness add-local-model <path> [name]  serve a local GGUF via mistral.rs (Rust) + register as omp `local/<name>`"));
    println!("{}", crate::brand::render("  8sync harness web [--port N]    local dashboard (axum+Vite): skills/memory/engines/team/submodules"));
    println!("{}", crate::brand::render("  8sync harness toolstats         SQLite tracker: optimizer (codegraph/cbm/serena) vs fallback (grep/read) call ratio + fails"));
    println!("{}", crate::brand::render("  8sync skill [list|add|gen|update]   manage the library (`skill update [name]` re-pulls from skills.toml)"));
    println!("{}", crate::brand::render("  8sync feature [new|switch|status|list]  large multi-phase GSD scope (planning tree + ACTIVE switch); /feature plan|go|ship in omp"));

    println!("\nLOCAL GGUF MODEL — real flow (mistral.rs serves it, omp sees `local/<name>`)");
    println!("{}", crate::brand::render("  8sync harness add-local-model ~/models/qwen2.5-coder-7b-instruct-q4_k_m.gguf qwen-coder"));
    println!("{}", crate::brand::render("  8sync harness add-local-model bartowski/Qwen2.5-Coder-7B-Instruct-GGUF        # HF repo id — auto-download"));
    println!("{}", crate::brand::render("  8sync harness add-local-model https://host/model.gguf my-model                # direct .gguf URL"));
    println!("{}", crate::brand::render("  8sync harness add-local-model list                                            # registered models + ports + status"));
    println!("{}", crate::brand::render("  8sync ai --model local/qwen-coder \"refactor this function\"                     # use it for one prompt"));
    println!("{}", crate::brand::render("  8sync harness model default local/qwen-coder                                  # make it the daily-driver model"));
    println!("{}", crate::brand::render("  8sync harness model code local/qwen-coder                                     # or only for the `code` task class"));
    println!("{}", crate::brand::render("  8sync harness add-local-model rm qwen-coder                                   # stop service + unregister"));
    println!("{}", crate::brand::render("  → registry: ~/.config/8sync/local-models.tsv · provider block: ~/.omp/agent/models.yml (sentinel-managed)"));

    println!("\nSKILLS (deployed by init)");
    println!("{}", crate::brand::render("  always-on (read order): codegraph → karpathy → ponytail → assp → impeccable → taste → 8sync-cli → image-routing"));
    println!("  on-demand : code-review-and-quality · senior-security · senior-frontend · full-flow · last30days");
    println!("  tech-gated: encore-deploy (only when the project uses Encore)");
    println!("{}", crate::brand::render("  opt-in    : social-growth — enable with `8sync skill add builtin:social-growth`"));
    println!("  external  : ponytail (full) + addyosmani/agent-skills (best-effort clone → ~/.omp/skills)");

    println!("\nFILE TAXONOMY (portability — survives a move to a new machine)");
    println!("  COMMIT : AGENTS.md · CLAUDE.md · su-code/*.md · CHANGELOG.md · su-code/skills/   (learned/decided)");
    println!("{}", crate::brand::render("  IGNORE : .codegraph/ · .cache/8sync/                                           (derived → rebuilt by init)"));
    println!("  SECRET : .env · .env.* (keep .env.example)                                     (NEVER commit)");
    println!("{}", crate::brand::render("  → init seeds these into a managed .gitignore block; `8sync doctor` warns if memory is ignored."));

    println!("\nOVERWRITE POLICY (default = NEVER overwrite — only add what's missing)");
    println!("  user-owned : su-code/*.md · CHANGELOG.md · su-code/skills/ · AGENTS.md outside sentinels · hooks · your config keys");
    println!("               → seed-if-missing or sentinel-block updates ONLY; your edits are never clobbered");
    println!("  managed    : ~/.omp/skills (bundled) · 00-force-load.md · APPEND_SYSTEM.md · extensions/commands");
    println!("{}", crate::brand::render("               → 8sync-shipped copies, refreshed when the binary updates (edit the PROJECT copy, not these)"));
    println!("  overwrite  : ONLY with an explicit flag — `--force` re-mirrors su-code/skills/ over local edits");

    println!("\nNEW MACHINE (nothing lost)");
    println!("  1) git clone <repo> && cd <repo>     # su-code/*.md + su-code/skills/ arrive with the clone");
    println!("{}", crate::brand::render("  2) 8sync up                          # install/refresh the 8sync binary + omp"));
    println!("{}", crate::brand::render("  3) 8sync harness init                # rebuild .codegraph + global skills, re-inject rules"));
    println!("{}", crate::brand::render("  3b) 8sync harness gateway apply     # deploy omp gateway config (set $NINE_ROUTER_KEY first)"));
    println!("  4) prior memory (KNOWLEDGE/DECISIONS/STATE) is already present — the agent resumes context.");
}
