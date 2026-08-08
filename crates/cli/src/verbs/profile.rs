// Profile system — opt-in personal customization on top of slim harness.
//
// A profile is a TOML file describing:
//   • packages to install, in two sibling tables:
//       [packages]         pacman / aur / aur_yay  — Arch family, canonical
//       [packages.fedora]  dnf / copr / rpmfusion / swap
//   • systemd services to enable (system / user)
//   • post-install commands (idempotent shell)
// Profiles can `extend` other profiles to form bundles.
//
// The Arch keys are canonical and are NEVER renamed — profiles also load from
// the user-owned ~/.config/8sync/profiles/*.toml, where a rename would parse
// clean, install nothing and exit 0. `[packages.fedora]` is strictly additive;
// a profile that omits it is *skipped with a printed reason* on Fedora rather
// than attempted and failed.
//
// Built-in profiles live in `assets/profiles/*.toml` (embedded).
// User profiles live in `~/.config/8sync/profiles/*.toml` (override).

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use crate::{assets, env_detect, pkg, ui};

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Community,
    Personal,
}

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
    #[serde(default)]
    pub visibility: Visibility,
    /// Profiles reached through `extends` that ask for Arch packages but ship
    /// no `[packages.fedora]`. Populated by [`resolve`], never by TOML, so a
    /// bundle can say which member it silently dropped on Fedora.
    #[serde(skip)]
    pub fedora_gaps: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Requires {
    #[serde(default)]
    pub aur_helper: bool,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Packages {
    #[serde(default)]
    pub pacman: Vec<String>,
    #[serde(default)]
    pub aur: Vec<String>,
    /// Packages that MUST be installed via `yay` specifically (not paru).
    /// Used for AUR packages that ship custom yay-only build hooks or that
    /// fail under paru's review pipeline (e.g. `lianli-linux-git`).
    #[serde(default)]
    pub aur_yay: Vec<String>,
    /// Fedora/RHEL sibling table. `None` — the table is absent — means the
    /// profile has no Fedora equivalent, which is a *different* statement from
    /// an empty table and is why this is an `Option`.
    #[serde(default)]
    pub fedora: Option<FedoraPackages>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct FedoraPackages {
    #[serde(default)]
    pub dnf: Vec<String>,
    /// `owner/project` COPR repos to enable first. Screened against
    /// `pkg::COPR_ALLOWLIST` — profile data cannot name an arbitrary repo.
    #[serde(default)]
    pub copr: Vec<String>,
    /// Enable RPM Fusion free + nonfree before installing.
    #[serde(default)]
    pub rpmfusion: bool,
    /// `[["from", "to"]]` pairs run as `dnf swap --allowerasing`, for the
    /// conflicting-provider case (rpmfusion `ffmpeg` vs Fedora `ffmpeg-free`).
    #[serde(default)]
    pub swap: Vec<(String, String)>,
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
    let user_dir = crate::brand::config_dir(&home).join("profiles");
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

/// Package/service accumulation while walking a profile's `extends` tree.
#[derive(Default)]
struct Acc {
    pacman: Vec<String>,
    aur: Vec<String>,
    aur_yay: Vec<String>,
    dnf: Vec<String>,
    copr: Vec<String>,
    rpmfusion: bool,
    swap: Vec<(String, String)>,
    /// Any `[packages.fedora]` seen at all — distinguishes "not ported" from
    /// "ported, installs nothing extra".
    has_fedora: bool,
    fedora_gaps: Vec<String>,
    sys: Vec<String>,
    usr: Vec<String>,
    cmds: Vec<String>,
    hints: Vec<String>,
    requires_aur: bool,
}

/// Resolve a profile's full effective package/service set by walking `extends`.
///
/// "Effective" is host-relative: on Fedora a member profile that ships no
/// `[packages.fedora]` contributes *nothing* — not its services and not its
/// post-install commands either, because those exist to configure software
/// that will not be installed (`warp-svc.service` and four `warp-cli` calls
/// are pure noise on a box with no `cloudflare-warp`). The member is recorded
/// in `fedora_gaps` so [`apply`] can name what it dropped.
pub fn resolve(name: &str, all: &HashMap<String, Profile>) -> Result<Profile> {
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut acc = Acc::default();
    let fedora = env_detect::distro_family() == env_detect::Family::Fedora;

    fn walk(
        n: &str,
        all: &HashMap<String, Profile>,
        visited: &mut BTreeSet<String>,
        acc: &mut Acc,
        fedora: bool,
    ) -> Result<()> {
        if !visited.insert(n.to_string()) { return Ok(()); }
        let p = all.get(n).ok_or_else(|| anyhow!("profile not found: {}", n))?;
        for e in &p.extends {
            walk(e, all, visited, acc, fedora)?;
        }
        // Packages are merged unconditionally: the two tables are already
        // family-split, and `apply` needs the Arch set intact to tell "not
        // ported" apart from "declares nothing at all".
        acc.pacman.extend(p.packages.pacman.iter().cloned());
        acc.aur.extend(p.packages.aur.iter().cloned());
        acc.aur_yay.extend(p.packages.aur_yay.iter().cloned());
        let mut unported = false;
        match &p.packages.fedora {
            Some(f) => {
                acc.has_fedora = true;
                acc.dnf.extend(f.dnf.iter().cloned());
                acc.copr.extend(f.copr.iter().cloned());
                acc.rpmfusion |= f.rpmfusion;
                acc.swap.extend(f.swap.iter().cloned());
            }
            None if declares_native(&p.packages) => {
                unported = true;
                acc.fedora_gaps.push(p.name.clone());
            }
            None => {}
        }
        if fedora && unported {
            return Ok(());
        }
        acc.sys.extend(p.services.system_enable.iter().cloned());
        acc.usr.extend(p.services.user_enable.iter().cloned());
        acc.cmds.extend(p.post_install.commands.iter().cloned());
        if !p.post_install.hint.is_empty() {
            acc.hints.push(format!("[{}] {}", p.name, p.post_install.hint));
        }
        if p.requires.aur_helper || !p.packages.aur_yay.is_empty() { acc.requires_aur = true; }
        Ok(())
    }

    walk(name, all, &mut visited, &mut acc, fedora)?;

    let description = all.get(name).map(|p| p.description.clone()).unwrap_or_default();

    Ok(Profile {
        name: name.to_string(),
        description,
        extends: vec![],
        requires: Requires { aur_helper: acc.requires_aur },
        packages: Packages {
            pacman: dedup(acc.pacman),
            aur: dedup(acc.aur),
            aur_yay: dedup(acc.aur_yay),
            fedora: acc.has_fedora.then(|| FedoraPackages {
                dnf: dedup(acc.dnf),
                copr: dedup(acc.copr),
                rpmfusion: acc.rpmfusion,
                swap: dedup_pairs(acc.swap),
            }),
        },
        services: Services { system_enable: dedup(acc.sys), user_enable: dedup(acc.usr) },
        post_install: PostInstall {
            commands: acc.cmds,
            hint: acc.hints.join("\n"),
        },
        visibility: all.get(name).map(|p| p.visibility).unwrap_or_default(),
        fedora_gaps: dedup(acc.fedora_gaps),
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

fn dedup_pairs(v: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for x in v {
        if seen.insert(x.clone()) { out.push(x); }
    }
    out
}

/// Does this profile ask for any Arch-native package?
fn declares_native(pkgs: &Packages) -> bool {
    !pkgs.pacman.is_empty() || !pkgs.aur.is_empty() || !pkgs.aur_yay.is_empty()
}

/// Whether a **resolved** profile has anything dnf can actually install.
pub fn fedora_supported(p: &Profile) -> bool {
    p.packages.fedora.as_ref().is_some_and(|f| !f.dnf.is_empty())
}

/// Names of profiles that declare packages but have nothing installable on
/// `fam`, sorted. Only Fedora can report gaps — the Arch tables are the
/// canonical set, so `Family::Arch` and `Family::Other` are always empty.
pub fn unsupported_on_family(
    all: &HashMap<String, Profile>,
    fam: env_detect::Family,
) -> Vec<String> {
    if fam != env_detect::Family::Fedora {
        return Vec::new();
    }
    let mut out: Vec<String> = all
        .keys()
        .filter(|n| {
            resolve(n.as_str(), all)
                .map(|r| declares_native(&r.packages) && !fedora_supported(&r))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    out.sort();
    out
}

/// Apply a resolved profile (idempotent). `yes_to_all` → unattended `--noconfirm`.
/// `dry_run` → print plan only.
///
/// A dry run is side-effect-free *and* infallible on a missing tool: every
/// helper lookup (AUR helper, yay bootstrap, COPR, RPM Fusion) sits on the
/// non-dry branch, so `--dry-run` always prints a plan instead of erroring.
pub fn apply(p: &Profile, yes_to_all: bool, dry_run: bool) -> Result<()> {
    let fedora = env_detect::distro_family() == env_detect::Family::Fedora;

    ui::step(&format!("profile: {}", p.name));

    if fedora && declares_native(&p.packages) && !fedora_supported(p) {
        ui::skip(
            &p.name,
            "no Fedora packages — add a [packages.fedora] table to port it",
        );
        return Ok(());
    }

    if !p.description.is_empty() {
        ui::info(&p.description);
    }
    if fedora {
        for gap in &p.fedora_gaps {
            ui::warn(&format!(
                "extended profile `{gap}` has no Fedora packages — it contributed nothing"
            ));
        }
        apply_fedora(p, yes_to_all, dry_run)?;
    } else {
        apply_arch(p, yes_to_all, dry_run)?;
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

/// Dry-run plan line for the native package set, naming the resolved backend.
fn plan_native(pkgs: &[String]) {
    match pkg::backend() {
        Some(b) => ui::info(&format!("would {} install: {}", b.name(), pkgs.join(" "))),
        None => ui::warn(&format!(
            "no native package manager — install manually: {}",
            pkgs.join(" ")
        )),
    }
}

/// Arch family (and the no-backend fallback, where `pkg::install` degrades to
/// a printed notice).
fn apply_arch(p: &Profile, yes_to_all: bool, dry_run: bool) -> Result<()> {
    if !p.packages.pacman.is_empty() {
        if dry_run {
            plan_native(&p.packages.pacman);
        } else {
            let refs: Vec<&str> = p.packages.pacman.iter().map(String::as_str).collect();
            pkg::install(&p.name, &refs, yes_to_all)?;
        }
    }

    // AUR packages. The helper lookup is on the non-dry branch: a dry run must
    // print a plan even on a box that has never had paru or yay installed.
    if !p.packages.aur.is_empty() {
        if dry_run {
            let helper = env_detect::aur_helper().unwrap_or("paru/yay");
            ui::info(&format!("would {} install: {}", helper, p.packages.aur.join(" ")));
        } else {
            let helper = env_detect::aur_helper().ok_or_else(|| {
                anyhow!(
                    "profile `{}` needs an AUR helper (paru or yay) — please install one first",
                    p.name
                )
            })?;
            let refs: Vec<&str> = p.packages.aur.iter().map(String::as_str).collect();
            pkg::aur_install_safe(helper, &refs, yes_to_all)?;
        }
    }

    // AUR packages that REQUIRE yay specifically. Bootstrap yay if missing,
    // even when paru is already present — some PKGBUILDs (e.g.
    // `lianli-linux-git`) only succeed under yay.
    if !p.packages.aur_yay.is_empty() {
        if dry_run {
            ui::info(&format!("would yay install (yay-only): {}", p.packages.aur_yay.join(" ")));
        } else {
            pkg::ensure_yay()?;
            let refs: Vec<&str> = p.packages.aur_yay.iter().map(String::as_str).collect();
            pkg::aur_install_safe("yay", &refs, yes_to_all)?;
        }
    }
    Ok(())
}

/// Fedora family: repo prep (RPM Fusion → COPR → swap) then the dnf set.
fn apply_fedora(p: &Profile, yes_to_all: bool, dry_run: bool) -> Result<()> {
    let Some(f) = p.packages.fedora.as_ref() else { return Ok(()) };

    if f.rpmfusion {
        if dry_run {
            ui::info("would enable RPM Fusion (free + nonfree)");
        } else {
            pkg::ensure_rpmfusion()?;
        }
    }

    for spec in &f.copr {
        if dry_run {
            ui::info(&format!("would copr enable: {}", spec));
        } else {
            pkg::copr_enable(spec, false)?;
        }
    }

    for (from, to) in &f.swap {
        if dry_run {
            ui::info(&format!("would dnf swap: {} -> {}", from, to));
        } else {
            pkg::swap(from, to)?;
        }
    }

    if !f.dnf.is_empty() {
        if dry_run {
            plan_native(&f.dnf);
        } else {
            let refs: Vec<&str> = f.dnf.iter().map(String::as_str).collect();
            pkg::install(&p.name, &refs, yes_to_all)?;
        }
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
    Ok(cfg.join(crate::brand::NS).join("profile.toml"))
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

/// Drop `applied` entries that no longer resolve against `load_all()` — e.g. a profile
/// deleted from the repo/override dir after it was applied (state is append-only and
/// outlives the profile definition otherwise). Rewrites state only if something changed.
/// Returns the names that were pruned.
pub fn prune_stale(all: &HashMap<String, Profile>) -> Result<Vec<String>> {
    let mut s = load_state();
    let (kept, stale): (Vec<String>, Vec<String>) =
        s.applied.drain(..).partition(|n| all.contains_key(n));
    if !stale.is_empty() {
        s.applied = kept;
        save_state(&s)?;
    }
    Ok(stale)
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
