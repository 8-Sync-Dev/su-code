// Profile system — opt-in personal customization on top of slim harness.
//
// A profile is a TOML file describing:
//   • pacman / aur packages to install
//   • systemd services to enable (system / user)
//   • post-install commands (idempotent shell)
// Profiles can `extend` other profiles to form bundles.
//
// Built-in profiles live in `assets/profiles/*.toml` (embedded).
// User profiles live in `~/.config/8sync/profiles/*.toml` (override).

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use crate::{assets, env_detect, pkg, ui};

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub extends: Vec<String>,
    #[serde(default)]
    pub requires: Requires,
    #[serde(default)]
    pub packages: Packages,
    #[serde(default)]
    pub services: Services,
    #[serde(default)]
    pub post_install: PostInstall,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Requires {
    #[serde(default)]
    pub aur_helper: bool,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Packages {
    #[serde(default)]
    pub pacman: Vec<String>,
    #[serde(default)]
    pub aur: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Services {
    #[serde(default)]
    pub system_enable: Vec<String>,
    #[serde(default)]
    pub user_enable: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct PostInstall {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub hint: String,
}

/// Load every available profile (embedded + user override).
pub fn load_all() -> Result<HashMap<String, Profile>> {
    let mut map: HashMap<String, Profile> = HashMap::new();

    // Embedded assets/profiles/*.toml
    for f in assets::Assets::iter() {
        let path = f.as_ref();
        if let Some(rel) = path.strip_prefix("profiles/") {
            if rel.ends_with(".toml") {
                if let Some(s) = assets::read(path) {
                    let p: Profile = toml::from_str(&s)
                        .with_context(|| format!("parse builtin profile {}", path))?;
                    map.insert(p.name.clone(), p);
                }
            }
        }
    }

    // User override
    let home = dirs::home_dir().context("no HOME")?;
    let user_dir = home.join(".config/8sync/profiles");
    if user_dir.is_dir() {
        for entry in std::fs::read_dir(&user_dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("toml") {
                let s = std::fs::read_to_string(&p)?;
                let prof: Profile = toml::from_str(&s)
                    .with_context(|| format!("parse user profile {}", p.display()))?;
                map.insert(prof.name.clone(), prof);
            }
        }
    }

    Ok(map)
}

/// Resolve a profile's full effective package/service set by walking `extends`.
pub fn resolve(name: &str, all: &HashMap<String, Profile>) -> Result<Profile> {
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut pacman: Vec<String> = Vec::new();
    let mut aur: Vec<String> = Vec::new();
    let mut sys: Vec<String> = Vec::new();
    let mut usr: Vec<String> = Vec::new();
    let mut cmds: Vec<String> = Vec::new();
    let mut hints: Vec<String> = Vec::new();
    let mut requires_aur = false;
    let mut description = String::new();

    fn walk(
        n: &str,
        all: &HashMap<String, Profile>,
        visited: &mut BTreeSet<String>,
        pacman: &mut Vec<String>,
        aur: &mut Vec<String>,
        sys: &mut Vec<String>,
        usr: &mut Vec<String>,
        cmds: &mut Vec<String>,
        hints: &mut Vec<String>,
        requires_aur: &mut bool,
    ) -> Result<()> {
        if !visited.insert(n.to_string()) { return Ok(()); }
        let p = all.get(n).ok_or_else(|| anyhow!("profile not found: {}", n))?;
        for e in &p.extends {
            walk(e, all, visited, pacman, aur, sys, usr, cmds, hints, requires_aur)?;
        }
        pacman.extend(p.packages.pacman.iter().cloned());
        aur.extend(p.packages.aur.iter().cloned());
        sys.extend(p.services.system_enable.iter().cloned());
        usr.extend(p.services.user_enable.iter().cloned());
        cmds.extend(p.post_install.commands.iter().cloned());
        if !p.post_install.hint.is_empty() {
            hints.push(format!("[{}] {}", p.name, p.post_install.hint));
        }
        if p.requires.aur_helper { *requires_aur = true; }
        Ok(())
    }

    walk(name, all, &mut visited, &mut pacman, &mut aur, &mut sys, &mut usr, &mut cmds, &mut hints, &mut requires_aur)?;

    if let Some(p) = all.get(name) {
        description = p.description.clone();
    }

    Ok(Profile {
        name: name.to_string(),
        description,
        extends: vec![],
        requires: Requires { aur_helper: requires_aur },
        packages: Packages { pacman: dedup(pacman), aur: dedup(aur) },
        services: Services { system_enable: dedup(sys), user_enable: dedup(usr) },
        post_install: PostInstall {
            commands: cmds,
            hint: hints.join("\n"),
        },
    })
}

fn dedup(v: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for x in v {
        if seen.insert(x.clone()) { out.push(x); }
    }
    out
}

/// Apply a resolved profile (idempotent). `yes_to_all` → unattended `--noconfirm`.
/// `dry_run` → print plan only.
pub fn apply(p: &Profile, yes_to_all: bool, dry_run: bool) -> Result<()> {
    ui::step(&format!("profile: {}", p.name));
    if !p.description.is_empty() {
        ui::info(&p.description);
    }

    // Pacman packages
    if !p.packages.pacman.is_empty() {
        if dry_run {
            ui::info(&format!("would pacman install: {}", p.packages.pacman.join(" ")));
        } else {
            let refs: Vec<&str> = p.packages.pacman.iter().map(|s| s.as_str()).collect();
            pkg::pacman_install_safe(&refs, yes_to_all)?;
        }
    }

    // AUR packages
    if !p.packages.aur.is_empty() {
        let helper = env_detect::aur_helper().ok_or_else(|| {
            anyhow!(
                "profile `{}` needs an AUR helper (paru or yay) — please install one first",
                p.name
            )
        })?;
        if dry_run {
            ui::info(&format!("would {} install: {}", helper, p.packages.aur.join(" ")));
        } else {
            let refs: Vec<&str> = p.packages.aur.iter().map(|s| s.as_str()).collect();
            pkg::aur_install_safe(helper, &refs, yes_to_all)?;
        }
    }

    // System services
    for svc in &p.services.system_enable {
        if dry_run {
            ui::info(&format!("would enable system service: {}", svc));
        } else {
            let _ = pkg::run_loud("sudo", &["systemctl", "enable", "--now", svc]);
        }
    }

    // User services
    for svc in &p.services.user_enable {
        if dry_run {
            ui::info(&format!("would enable user service: {}", svc));
        } else {
            let _ = pkg::run_loud("systemctl", &["--user", "enable", "--now", svc]);
        }
    }

    // Post-install
    for c in &p.post_install.commands {
        if dry_run {
            ui::info(&format!("would run: {}", c));
        } else {
            ui::info(&format!("$ {}", c));
            let _ = std::process::Command::new("sh").arg("-c").arg(c).status();
        }
    }

    if !p.post_install.hint.is_empty() {
        ui::info(&p.post_install.hint);
    }

    Ok(())
}

// ─── Persistence ────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default, serde::Serialize)]
pub struct State {
    #[serde(default)]
    pub applied: Vec<String>,
    #[serde(default)]
    pub last_setup: String,
}

pub fn state_path() -> Result<PathBuf> {
    let cfg = dirs::config_dir().context("no XDG_CONFIG")?;
    Ok(cfg.join("8sync/profile.toml"))
}

pub fn load_state() -> State {
    state_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_state(s: &State) -> Result<()> {
    let p = state_path()?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, toml::to_string_pretty(s)?)?;
    Ok(())
}

pub fn mark_applied(name: &str) -> Result<()> {
    let mut s = load_state();
    if !s.applied.iter().any(|x| x == name) {
        s.applied.push(name.to_string());
    }
    s.last_setup = current_ts();
    save_state(&s)
}

fn current_ts() -> String {
    // Simple ISO-ish timestamp without chrono dep
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{}", secs)
}
