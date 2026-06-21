use anyhow::Result;
use crate::{env_detect, ui, verbs::{profile, sec, bt}};

pub fn run() -> Result<()> {
    ui::header("8sync doctor");
    let env = env_detect::Env::detect()?;

    // OS / desktop stack
    check("OS", &env.os_id);
    if env_detect::is_hyde() {
        ui::ok("HyDE detected (Hyprland + wallbash theme engine)");
    }

    // AUR helper
    match env_detect::aur_helper() {
        Some(h) => ui::ok(&format!("AUR helper: {}", h)),
        None    => ui::info("AUR helper: none (paru or yay needed for AUR profiles: hardware-lianli, warp, ...)"),
    }

    // Core harness
    check_cmd("git",     &["--version"]);
    check_cmd("omp",     &["--version"]);

    // gh is REQUIRED for `8sync ship`
    match env_detect::cmd_version("gh", &["--version"]) {
        Some(v) => ui::ok(&format!("gh: {}", v)),
        None    => ui::err("gh: MISSING — `8sync ship` needs github-cli (run `8sync setup`)"),
    }
    if let Some(out) = env_detect::cmd_version("gh", &["auth", "status"]) {
        ui::info(&format!("gh auth: {}", out));
    }

    // Configs present?
    for path in [
        env.xdg_config.join("8sync/global.toml"),
        env.xdg_config.join("8sync/skills.toml"),
        env.home.join(".omp/skills/00-force-load.md"),
    ] {
        if path.exists() {
            ui::ok(&format!("{}", path.display()));
        } else {
            ui::warn(&format!("missing: {}", path.display()));
        }
    }

    // Project portability: durable agent memory MUST be git-tracked so it
    // survives `git clone` to a new machine.
    check_portability();

    // Secret scanning for safe auto-commit (`harness up --commit`).
    if which::which("gitleaks").is_ok() {
        ui::ok("gitleaks present (`harness up --commit` scans staged memory before committing)");
    } else {
        ui::info("gitleaks not found — recommended for `harness up --commit` (pre-commit secret scan; GitGuardian 2026)");
    }

    // Fish PATH bootstrap (only relevant if fish is present)
    if which::which("fish").is_ok() {
        let fish_snippet = env.home.join(".config/fish/conf.d/8sync-path.fish");
        if fish_snippet.exists() {
            ui::ok(&format!("fish PATH bootstrap: {}", fish_snippet.display()));
        } else {
            ui::warn(&format!(
                "fish installed but missing {} — re-run `8sync setup`",
                fish_snippet.display()
            ));
        }
    }

    // Bluetooth (bluez) — compact status
    bt::status_quiet();

    // Security (warp + ufw) — compact one-liners
    sec::status_quiet();

    // Profiles applied
    let st = profile::load_state();
    if st.applied.is_empty() {
        ui::info("profiles: none applied (run `8sync setup`)");
    } else {
        ui::ok(&format!("profiles applied: {}", st.applied.join(", ")));
    }

    Ok(())
}

fn check(label: &str, value: &str) {
    ui::ok(&format!("{}: {}", label, value));
}

fn check_cmd(name: &str, args: &[&str]) {
    match env_detect::cmd_version(name, args) {
        Some(v) => ui::ok(&format!("{}: {}", name, v)),
        None => ui::warn(&format!("{}: missing", name)),
    }
}

/// Warn if any durable agent-memory file in the current project is gitignored
/// (learnings would be lost on a new machine). Silent when not in a project or
/// not a git repo.
fn check_portability() {
    let Some(root) = crate::verbs::skill::discover::detect_current_project_root() else {
        return;
    };
    let durable = [
        "AGENTS.md",
        "CHANGELOG.md",
        "agents/PROJECT.md",
        "agents/KNOWLEDGE.md",
        "agents/DECISIONS.md",
        "agents/STATE.md",
        "agents/PREFERENCES.md",
        "agents/NOTES.md",
    ];
    let mut present = false;
    let mut ignored_any = false;
    for rel in durable {
        if !root.join(rel).exists() {
            continue;
        }
        present = true;
        // `git check-ignore -q` exits 0 only when the path IS ignored.
        let ignored = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["check-ignore", "-q", rel])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ignored {
            ignored_any = true;
            ui::err(&format!(
                "MEMORY IGNORED: {} is gitignored — learnings won't persist or move to a new machine; remove it from .gitignore",
                rel
            ));
        }
    }
    if present && !ignored_any {
        ui::ok("project memory is git-tracked (portable)");
    }
    // Context budget: the injected force-load block must stay lean (Gloaguen
    // 2026, arXiv 2602.11988 — bloated/auto context cuts success + ~20% cost).
    if let Ok(s) = std::fs::read_to_string(root.join("AGENTS.md")) {
        if let (Some(b), Some(e)) = (
            s.find("<!-- 8sync:skills:begin -->"),
            s.find("<!-- 8sync:skills:end -->"),
        ) {
            if b < e {
                let lines = s[b..e].lines().count();
                if lines > 120 {
                    ui::warn(&format!(
                        "AGENTS.md force-load block is {} lines (>120) — trim on-demand entries; rely on progressive disclosure",
                        lines
                    ));
                }
            }
        }
    }
}
