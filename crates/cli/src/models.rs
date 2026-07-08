//! Adaptive model routing for omp.
//!
//! Reads `~/.config/8sync/models.toml` (falling back to the embedded default
//! `assets/configs/models.toml`), classifies a prompt into a task class, and
//! emits omp CLI flags (`--model` + `--plan`/`--smol`/`--slow`). omp owns the
//! actual model catalog and resolution (fuzzy match); 8sync only steers which
//! model omp uses per prompt instead of hard-fixing a single `default`.

use serde::Deserialize;
use std::collections::BTreeMap;

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
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            roles: Roles::default(),
            tasks: BTreeMap::new(),
            advisor: true,
        }
    }
}

fn advisor_default() -> bool {
    true
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
        out
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
}
