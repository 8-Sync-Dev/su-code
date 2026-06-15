//! `8sync harness up` — refresh the harness to the project's CURRENT state:
//! re-inject AGENTS.md + sub-folder indexes, refresh the KNOWLEDGE breadcrumb,
//! and re-index codegraph so the agent keeps learning as the project grows.
//! `--loop <dur>` runs it in the foreground; `--timer <dur|off>` installs a
//! systemd user timer (the recommended background option).
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};

use super::memory::seed_harness_memory;
use crate::verbs::skill::{discover, inject_agents_md, inject_subfolder_indexes};
use crate::{env_detect, ui};

pub(crate) fn harness_up(
    env: &env_detect::Env,
    loop_every: Option<&str>,
    timer: Option<&str>,
) -> Result<()> {
    if let Some(spec) = timer {
        return manage_timer(env, spec);
    }
    if let Some(dur) = loop_every {
        let secs = parse_dur_secs(dur).max(60);
        ui::header(&format!("8sync harness up --loop ({}s interval)", secs));
        ui::info("Ctrl-C to stop. Each pass re-injects rules + refreshes memory + re-indexes codegraph.");
        loop {
            let _ = refresh_once(env);
            ui::step(&format!("sleeping {}s …", secs));
            std::thread::sleep(Duration::from_secs(secs));
        }
    }
    ui::header("8sync harness up");
    refresh_once(env)
}

fn refresh_once(env: &env_detect::Env) -> Result<()> {
    let Some(root) = discover::detect_current_project_root() else {
        ui::warn("not inside a project — nothing to refresh");
        return Ok(());
    };
    inject_agents_md(&env.home, &root)?;
    inject_subfolder_indexes(&root)?;
    seed_harness_memory(&root)?;
    if which::which("codegraph").is_ok() {
        ui::step("codegraph index (re-learn current state)");
        let _ = Command::new("codegraph").arg("index").arg(&root).status();
    }
    ui::ok(&format!("harness up to date → {}", root.display()));
    Ok(())
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
