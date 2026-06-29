//! `8sync harness up` — refresh the harness to the project's CURRENT state:
//! re-inject AGENTS.md + sub-folder indexes, refresh the KNOWLEDGE breadcrumb,
//! and re-index codegraph so the agent keeps learning as the project grows.
//! `--loop <dur>` runs it in the foreground; `--timer <dur|off>` installs a
//! systemd user timer (the recommended background option). Per tick the harness
//! refreshes context (re-inject + re-index + consolidate); the agent then drives
//! the L1→L3 loop off STATE.md (read STATE → Next → verify-gate → update spine →
//! `--commit`), per the loop-engineering rules in 00-force-load.md.
use std::process::Command;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};

use super::memory::{consolidate_learnings, seed_gitleaks_hook, seed_harness_memory};
use crate::verbs::skill::{discover, inject_agents_md, inject_subfolder_indexes};
use crate::{env_detect, ui};

pub(crate) fn harness_up(
    env: &env_detect::Env,
    loop_every: Option<&str>,
    timer: Option<&str>,
    pull: bool,
    commit: bool,
) -> Result<()> {
    if let Some(spec) = timer {
        return manage_timer(env, spec);
    }
    if let Some(dur) = loop_every {
        let secs = parse_dur_secs(dur).max(60);
        ui::header(&format!("8sync harness up --loop ({}s interval)", secs));
        ui::info("Ctrl-C to stop. Each pass re-injects rules + refreshes memory + re-indexes codegraph.");
        loop {
            let _ = refresh_once(env, pull, commit);
            ui::step(&format!("sleeping {}s …", secs));
            std::thread::sleep(Duration::from_secs(secs));
        }
    }
    ui::header("8sync harness up");
    refresh_once(env, pull, commit)
}

fn refresh_once(env: &env_detect::Env, pull: bool, commit: bool) -> Result<()> {
    let Some(root) = discover::detect_current_project_root() else {
        ui::warn("not inside a project — nothing to refresh");
        return Ok(());
    };
    if pull {
        ui::step("re-pull registered skills (--pull)");
        let registry = env.xdg_config.join("8sync/skills.toml");
        let _ = crate::verbs::skill::update::update_skills(env, &registry, None);
    }
    inject_agents_md(&env.home, &root)?;
    inject_subfolder_indexes(&root)?;
    let _ = crate::verbs::skill::deploy::ensure_append_system(&env.home);
    let _ = crate::verbs::skill::deploy::ensure_serena_mcp(env);
    let _ = crate::verbs::skill::deploy::ensure_engine(&env.home, Some(&root));
    let _ = crate::verbs::skill::deploy::cleanup_legacy_gs(&env.home, Some(&root));
    let _ = crate::verbs::skill::deploy::ensure_workflow_extension(&env.home, Some(&root));
    seed_harness_memory(&root)?;
    let _ = consolidate_learnings(&root);
    seed_gitleaks_hook(&root);
    if which::which("codegraph").is_ok() {
        ui::step("codegraph index (re-learn current state)");
        let _ = Command::new("codegraph").arg("index").arg(&root).status();
    }
    crate::verbs::skill::deploy::index_codebase_memory(&root);
    if commit {
        commit_memory(&root);
    }
    ui::ok(&format!("harness up to date → {}", root.display()));
    Ok(())
}

/// `up --commit`: stage ONLY 8sync-managed memory artifacts (never the user's
/// code) and commit them, so learnings persist to git in the same pass. No-op
/// when nothing changed; best-effort — warns, never bails.
fn commit_memory(root: &Path) {
    let candidates = ["agents", "AGENTS.md", "CLAUDE.md", "CHANGELOG.md", ".gitignore"];
    let present: Vec<&str> = candidates
        .into_iter()
        .filter(|p| root.join(p).exists())
        .collect();
    if present.is_empty() {
        return;
    }
    let added = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["add", "--"])
        .args(&present)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !added {
        ui::warn("git add failed — skipping memory commit (not a git repo?)");
        return;
    }
    // Commit only when memory is actually staged (avoids empty-commit spam on timers).
    let nothing_staged = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "--cached", "--quiet", "--"])
        .args(&present)
        .status()
        .map(|s| s.success())
        .unwrap_or(true);
    if nothing_staged {
        ui::skip("git commit", "no agent-memory changes");
        return;
    }
    // Secret scan before committing (GitGuardian 2026: AI-assisted commits leak
    // ~2× baseline). Abort only on a positive detection; tolerate tool/version
    // errors so a quirky gitleaks build can't wedge every commit.
    if which::which("gitleaks").is_ok() {
        let code = Command::new("gitleaks")
            .args(["protect", "--staged", "--no-banner"])
            .current_dir(root)
            .status()
            .ok()
            .and_then(|s| s.code());
        match code {
            Some(0) => {}
            Some(1) => {
                ui::err("gitleaks detected a secret in staged memory — commit ABORTED. Remove it, then retry.");
                return;
            }
            other => ui::warn(&format!(
                "gitleaks scan inconclusive (exit {:?}) — committing without scan; verify manually",
                other
            )),
        }
    } else {
        ui::warn("gitleaks not installed — committing memory WITHOUT secret scan (install gitleaks to harden)");
    }
    let committed = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["commit", "-m", "chore(8sync): refresh agent memory + harness"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if committed {
        ui::ok("committed agent memory (portable)");
    } else {
        ui::warn("git commit failed — memory left staged");
    }
}

/// Parse a human duration (`10m`, `1h`, `30s`, or bare seconds) into seconds.
fn parse_dur_secs(s: &str) -> u64 {
    let s = s.trim();
    let (num, mult) = if let Some(n) = s.strip_suffix('h') {
        (n, 3600)
    } else if let Some(n) = s.strip_suffix('m') {
        (n, 60)
    } else if let Some(n) = s.strip_suffix('s') {
        (n, 1)
    } else {
        (s, 1)
    };
    num.trim().parse::<u64>().unwrap_or(600).saturating_mul(mult)
}

/// Install/remove a systemd USER timer that runs `8sync harness up` in the
/// project directory every `<dur>` (proven pattern, mirrors `8sync clean --timer`).
fn manage_timer(env: &env_detect::Env, spec: &str) -> Result<()> {
    let unit_dir = env.home.join(".config/systemd/user");
    let svc = unit_dir.join("8sync-harness-up.service");
    let timer = unit_dir.join("8sync-harness-up.timer");

    if spec.eq_ignore_ascii_case("off") {
        ui::header("8sync harness up --timer off");
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "--now", "8sync-harness-up.timer"])
            .status();
        let _ = std::fs::remove_file(&svc);
        let _ = std::fs::remove_file(&timer);
        let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
        ui::ok("timer removed");
        return Ok(());
    }

    ui::header(&format!("8sync harness up --timer {}", spec));
    let root = discover::detect_current_project_root()
        .context("`harness up --timer` must run inside a project (it sets WorkingDirectory)")?;
    let exe = std::env::current_exe().context("current_exe")?;
    std::fs::create_dir_all(&unit_dir)?;

    let svc_body = format!(
        "[Unit]\nDescription=8sync harness up ({proj})\n\n[Service]\nType=oneshot\nTimeoutStartSec=300\nWorkingDirectory={wd}\nExecStart={exe} harness up\n",
        proj = root.file_name().and_then(|s| s.to_str()).unwrap_or("project"),
        wd = root.display(),
        exe = exe.display(),
    );
    let timer_body = format!(
        "[Unit]\nDescription=8sync harness up timer (every {dur})\n\n[Timer]\nOnBootSec=5min\nOnUnitActiveSec={dur}\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n",
        dur = spec
    );
    std::fs::write(&svc, svc_body)?;
    std::fs::write(&timer, timer_body)?;
    ui::ok(&format!("wrote {} + .timer", svc.display()));

    let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
    let st = Command::new("systemctl")
        .args(["--user", "enable", "--now", "8sync-harness-up.timer"])
        .status();
    match st {
        Ok(s) if s.success() => {
            ui::ok(&format!("timer enabled — refreshes `{}` every {}", root.display(), spec));
            ui::info("status: systemctl --user list-timers 8sync-harness-up.timer");
            ui::info("note: boot-time runs need `loginctl enable-linger $USER`");
        }
        _ => ui::warn("could not enable timer (is `systemctl --user` available?)"),
    }
    Ok(())
}
