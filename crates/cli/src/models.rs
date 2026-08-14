//! Adaptive model routing for omp.
//!
//! Reads `~/.config/8sync/models.toml` (falling back to the embedded default
//! `assets/configs/models.toml`), classifies a prompt into a task class, and
//! emits omp CLI flags (`--model` + `--plan`/`--smol`/`--slow`). omp owns the
//! actual model catalog and resolution (fuzzy match); 8sync only steers which
//! model omp uses per prompt instead of hard-fixing a single `default`.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub roles: Roles,
    #[serde(default)]
    pub tasks: BTreeMap<String, String>,
    /// Enable omp's `--advisor` passive per-turn reviewer. Default ON (skipped
    /// for trivial prompts to stay token-optimal). Opt out: `advisor = false`
    /// in models.toml, or `8sync ai --no-advisor` for one run.
    #[serde(default = "advisor_default")]
    pub advisor: bool,
    /// STEP-0 tool-routing enforcement (default ON). When ON, 8sync launches omp
    /// with a `--config` overlay that sets `grep.enabled`/`glob.enabled` to
    /// false, so the two redundant searchers are absent from the session and
    /// code lookup MUST go through codegraph (CLI) · codebase-memory-mcp ·
    /// serena. Everything else omp offers — including MCP/xdev tools — is
    /// untouched, because this names only what to remove. `bash rg`/`grep -r`
    /// shell escapes are additionally blocked by `bashInterceptor.patterns`
    /// (deployed by `8sync harness`). Opt out: `8sync ai --no-step0`, or
    /// `step0 = false` in models.toml.
    #[serde(default = "step0_default")]
    pub step0: bool,
    /// STEP-0 shell guard: the omp `bashInterceptor` rules `8sync harness` renders
    /// into `~/.omp/agent/config.yml`. Sourced from config rather than hardcoded so
    /// the pattern set is tunable per machine without a rebuild. When the key is
    /// absent — every `models.toml` written before this section existed — the
    /// embedded default asset supplies it, so upgrading keeps the guard instead of
    /// silently dropping it. Consumed by `deploy::ensure_bash_interceptor`.
    #[serde(rename = "bashInterceptor", default = "embedded_bash_interceptor")]
    pub bash_interceptor: BashInterceptor,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            roles: Roles::default(),
            tasks: BTreeMap::new(),
            advisor: true,
            step0: true,
            bash_interceptor: embedded_bash_interceptor(),
        }
    }
}

fn advisor_default() -> bool {
    true
}

fn step0_default() -> bool {
    true
}

fn enabled_default() -> bool {
    true
}

/// omp's `bashInterceptor` config. A matching `bash` command returns a tool ERROR
/// naming the replacement, which is what closes the `bash rg` escape the `--tools`
/// allowlist leaves open (it removes the `grep`/`glob` TOOLS, not the shell).
///
/// `enabled = false` or an empty `patterns` list means "no guard": `8sync harness`
/// then REMOVES the block it previously wrote instead of leaving a stale one.
#[derive(Debug, Deserialize)]
pub struct BashInterceptor {
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The COMPLETE rule set for this layer — setting the key makes omp replace its
    /// own default array, so this is the whole guard, not an addition to omp's.
    #[serde(default)]
    pub patterns: Vec<InterceptRule>,
}

impl Default for BashInterceptor {
    fn default() -> Self {
        Self { enabled: true, patterns: Vec::new() }
    }
}

/// One interceptor rule in omp's own shape. `tool` is the availability GATE, not
/// the suggestion: omp's matcher skips any rule whose `tool` is missing from the
/// session, so it must name a tool that is always there (`lsp`) while `message`
/// names the real replacement (codegraph / serena / codebase-memory).
#[derive(Debug, Deserialize)]
pub struct InterceptRule {
    pub pattern: String,
    pub tool: String,
    pub message: String,
}

/// The `[bashInterceptor]` section of the EMBEDDED `assets/configs/models.toml`.
/// Parsed through a local struct instead of `ModelConfig` so this serde default
/// can never recurse into itself. Unparseable/absent ⇒ empty ⇒ no guard written
/// (fail-open: a broken config must not dead-end the shell).
fn embedded_bash_interceptor() -> BashInterceptor {
    #[derive(Deserialize)]
    struct Section {
        #[serde(rename = "bashInterceptor")]
        bash_interceptor: BashInterceptor,
    }
    crate::assets::read("configs/models.toml")
        .and_then(|s| toml::from_str::<Section>(&s).ok())
        .map(|s| s.bash_interceptor)
        .unwrap_or_default()
}

/// STEP-0 tool enforcement, expressed as a DENY-list.
///
/// omp's `--tools` is an ALLOWLIST, so steering it forces 8sync to mirror omp's
/// ENTIRE built-in tool set — and that mirror rotted the moment omp 17.3
/// renamed `ast_grep` and dropped `github`/`checkpoint`/`rewind`/
/// `security_scan`: every `8sync .` and `8sync ai` launch died instantly with
/// `CliUsageError: Unknown tools in --tools`, which sent the user back to a bare
/// `omp --continue` — a DIFFERENT session store, so the named session they had
/// just opened looked lost. The registry omp validates against is also built
/// asynchronously, so the accepted set is not even stable within one version:
/// no mirrored list can be correct.
///
/// omp exposes per-tool switches instead (`grep.enabled`, `glob.enabled` —
/// settings.md), which name only what we want GONE and stay valid however many
/// tools omp adds or renames. Shipping them as a `--config` overlay keeps the
/// scope per-launch, so `8sync ai --no-step0` and the user's own bare `omp` are
/// untouched.
const STEP0_OVERLAY: &str = "# 8sync STEP-0 — managed file, rewritten whenever it drifts.\n\
                             # Code lookup must go through codegraph / codebase-memory-mcp / serena.\n\
                             grep:\n  enabled: false\nglob:\n  enabled: false\n";

/// Path to the STEP-0 overlay, written only when its bytes differ (omp treats a
/// `--config` file as strict input, so it must exist and parse).
///
/// `None` when it cannot be written — the caller then launches WITHOUT STEP-0
/// rather than pointing `--config` at a file omp would hard-error on. Losing the
/// tool drop for one run is recoverable; a launch that refuses to start is what
/// this whole change exists to prevent.
pub(crate) fn step0_overlay() -> Option<PathBuf> {
    let path = dirs::config_dir()?.join(crate::brand::NS).join("omp-step0.yml");
    if std::fs::read_to_string(&path).ok().as_deref() != Some(STEP0_OVERLAY) {
        std::fs::create_dir_all(path.parent()?).ok()?;
        std::fs::write(&path, STEP0_OVERLAY).ok()?;
    }
    Some(path)
}

/// Whether STEP-0 still bites, asked of omp itself: with the overlay applied,
/// `--tools grep,glob` must be REJECTED, because the two names are no longer in
/// omp's registry. `--tools` is validated before any provider is contacted, so
/// the probe is offline and free.
///
/// `Some(false)` means omp stopped honouring the `grep.enabled`/`glob.enabled`
/// keys (renamed, removed) and the agent silently regained the searchers.
/// `None` when omp is absent or the overlay could not be written.
pub(crate) fn step0_effective() -> Option<bool> {
    let overlay = step0_overlay()?;
    let out = std::process::Command::new("omp")
        .arg("--config")
        .arg(&overlay)
        .args(["--tools", "grep,glob", "-p", ""])
        .output()
        .ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    Some(text.contains("Unknown tool") && text.contains("grep"))
}

#[derive(Debug, Default, Deserialize)]
pub struct Roles {
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub plan: String,
    #[serde(default)]
    pub smol: String,
    #[serde(default)]
    pub slow: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskClass {
    Plan,
    Review,
    Debug,
    Code,
    Trivial,
}

impl TaskClass {
    pub fn key(self) -> &'static str {
        match self {
            TaskClass::Plan => "plan",
            TaskClass::Review => "review",
            TaskClass::Debug => "debug",
            TaskClass::Code => "code",
            TaskClass::Trivial => "trivial",
        }
    }
}

impl ModelConfig {
    /// Load user config, else the embedded default, else an empty config (omp
    /// decides everything — graceful when nothing is configured).
    pub fn load() -> Self {
        if let Some(dir) = dirs::config_dir() {
            let p = dir.join(crate::brand::NS).join("models.toml");
            if let Ok(s) = std::fs::read_to_string(&p) {
                if let Ok(c) = toml::from_str::<ModelConfig>(&s) {
                    return c;
                }
            }
        }
        crate::assets::read("configs/models.toml")
            .and_then(|s| toml::from_str::<ModelConfig>(&s).ok())
            .unwrap_or_default()
    }

    /// Model for a task class, falling back to `roles.default`.
    fn model_for(&self, class: TaskClass) -> &str {
        let task = self.tasks.get(class.key()).map(String::as_str).unwrap_or("");
        if task.is_empty() {
            self.roles.default.as_str()
        } else {
            task
        }
    }

    /// omp flags for a fresh prompt: `--model <classified>` + role flags.
    /// `override_model` (from `8sync ai --model X`) wins for the main model.
    /// Empty values are skipped so omp keeps its own defaults.
    pub fn omp_flags(&self, prompt: &str, override_model: Option<&str>) -> Vec<String> {
        let class = classify(prompt);
        let main = match override_model {
            Some(m) if !m.trim().is_empty() => m.trim().to_string(),
            _ => self.model_for(class).to_string(),
        };
        let mut out = Vec::new();
        push_flag(&mut out, "--model", &main);
        self.push_role_flags(&mut out);
        // Advisor: passive per-turn rule/tool reviewer. On for substantive work,
        // skipped for trivial prompts to stay token-optimal.
        if self.advisor && class != TaskClass::Trivial {
            out.push("--advisor".to_string());
        }
        self.push_step0(&mut out);
        out
    }

    /// Role flags (+ default `--model`) for resume/continue, where there is no
    /// new prompt to classify.
    pub fn resume_flags(&self) -> Vec<String> {
        let mut out = Vec::new();
        push_flag(&mut out, "--model", &self.roles.default);
        self.push_role_flags(&mut out);
        // Interactive dev session (`8sync .` / resume): advisor on.
        if self.advisor {
            out.push("--advisor".to_string());
        }
        self.push_step0(&mut out);
        out
    }

    /// STEP-0: hand omp the deny-list overlay. Skipped (with a warning) when the
    /// file cannot be written — never send `--config` at a path omp will reject.
    fn push_step0(&self, out: &mut Vec<String>) {
        if !self.step0 {
            return;
        }
        match step0_overlay() {
            Some(p) => {
                out.push("--config".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            None => crate::ui::warn(
                "STEP-0 overlay unwritable — launching with grep/glob ENABLED for the agent",
            ),
        }
    }

    fn push_role_flags(&self, out: &mut Vec<String>) {
        push_flag(out, "--plan", &self.roles.plan);
        push_flag(out, "--smol", &self.roles.smol);
        push_flag(out, "--slow", &self.roles.slow);
    }
}

fn push_flag(out: &mut Vec<String>, flag: &str, val: &str) {
    let v = val.trim();
    if !v.is_empty() {
        out.push(flag.to_string());
        out.push(v.to_string());
    }
}

/// Heuristic prompt → task class. Specific intents (review/plan/debug) beat the
/// generic `code` default; a very short prompt with no build verb is `trivial`.
pub fn classify(prompt: &str) -> TaskClass {
    let p = prompt.to_lowercase();
    let has = |kws: &[&str]| kws.iter().any(|k| p.contains(k));

    if has(&["review", "audit", "critique", "vulnerab", "security", "code smell"]) {
        return TaskClass::Review;
    }
    if has(&[
        "plan", "architect", "design ", "approach", "strategy", "how should",
        "trade-off", "tradeoff", "decompose",
    ]) {
        return TaskClass::Plan;
    }
    if has(&[
        "debug", "fix ", "bug", "error", "crash", "failing", "stack trace",
        "why does", "why is", "broken", "regression",
    ]) {
        return TaskClass::Debug;
    }
    let build_verb = has(&["implement", "build", "add ", "refactor", "write", "create", "migrate"]);
    if p.split_whitespace().count() <= 4 && !build_verb {
        return TaskClass::Trivial;
    }
    TaskClass::Code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_intents() {
        assert_eq!(classify("review the auth module for security holes"), TaskClass::Review);
        assert_eq!(classify("plan the architecture for a job queue"), TaskClass::Plan);
        assert_eq!(classify("fix the failing login test"), TaskClass::Debug);
        assert_eq!(classify("implement a dark mode toggle"), TaskClass::Code);
        assert_eq!(classify("rename foo"), TaskClass::Trivial);
    }

    #[test]
    fn embedded_default_parses_and_routes() {
        let cfg: ModelConfig = toml::from_str(
            r#"
            [roles]
            default = "codex"
            plan = "glm"
            smol = "haiku"
            slow = "opus"
            [tasks]
            plan = "glm"
            review = "opus"
            code = "codex"
            "#,
        )
        .unwrap();
        // review prompt → opus, role flags appended.
        let f = cfg.omp_flags("audit this for vulnerabilities", None);
        assert!(f.windows(2).any(|w| w == ["--model", "opus"]));
        assert!(f.windows(2).any(|w| w == ["--plan", "glm"]));
        // explicit override wins.
        let f2 = cfg.omp_flags("plan something", Some("glm"));
        assert!(f2.windows(2).any(|w| w == ["--model", "glm"]));
    }

    /// The regression this replaced: STEP-0 used to send omp an ALLOWLIST of
    /// every built-in tool, so omp 17.3 renaming `ast_grep` and dropping
    /// `github`/`checkpoint`/`rewind`/`security_scan` made omp exit with
    /// `CliUsageError: Unknown tools in --tools` on EVERY `8sync .` launch.
    /// `--tools` must never be emitted again: the flag is what coupled us to a
    /// list we do not own.
    #[test]
    fn step0_never_sends_a_tool_allowlist() {
        let cfg = ModelConfig::default();
        assert!(cfg.step0, "STEP-0 defaults ON");
        for flags in [cfg.resume_flags(), cfg.omp_flags("refactor the parser", None)] {
            assert!(
                !flags.iter().any(|f| f == "--tools"),
                "STEP-0 must deny grep/glob by config, never allowlist tools: {flags:?}"
            );
            let i = flags.iter().position(|f| f == "--config").expect("overlay passed");
            assert!(flags[i + 1].ends_with("omp-step0.yml"), "{flags:?}");
        }
    }

    /// The overlay is the whole enforcement: it must name the two searchers and
    /// nothing else, or STEP-0 either stops biting or starts disabling tools the
    /// agent needs.
    #[test]
    fn step0_overlay_disables_exactly_grep_and_glob() {
        let path = step0_overlay().expect("overlay written");
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body, STEP0_OVERLAY);
        for tool in ["grep", "glob"] {
            assert!(body.contains(&format!("{tool}:\n  enabled: false")), "{body}");
        }
        assert_eq!(body.matches("enabled: false").count(), 2, "{body}");
    }

    /// Rewritten only when it drifts, so an unchanged overlay keeps its mtime and
    /// omp's config layer stays byte-stable across launches (prompt-cache hit).
    #[test]
    fn step0_overlay_is_idempotent() {
        let a = step0_overlay().expect("overlay written");
        let m1 = std::fs::metadata(&a).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let b = step0_overlay().expect("overlay written");
        assert_eq!(a, b);
        assert_eq!(m1, std::fs::metadata(&b).unwrap().modified().unwrap());
    }
}
