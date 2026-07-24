// 8sync-gs — pure policy: stage requirements, model selection + independence,
// and deterministic safety classification (security triggers, destructive
// commands, shell-string verify detection). No omp / node imports.

import type {
  GateFinding,
  GsAgent,
  GsConfig,
  GsModelRole,
  ModelEvidence,
  RiskAssessment,
  Stage,
  VerifyCommand,
} from "./types.ts";

/** A model identity resolved by the omp adapter (or a test fake). */
export interface ResolvedModel {
  id: string;
  provider: string;
  model: string;
  family: string;
}

export type ModelResolver = (selector: string) => ResolvedModel | undefined;

export interface StageRequirement {
  agent?: GsAgent;
  modelRole?: GsModelRole;
  /** true = the stage additionally requires gs-security when risk.security. */
  securityIfRisky?: boolean;
}

/** What each stage demands. Terminal stages have no requirement. */
export function stageRequirement(stage: Stage): StageRequirement {
  switch (stage) {
    case "clarify":
      return { modelRole: "coordinator" };
    case "research":
      return { agent: "gs-researcher", modelRole: "research" };
    case "plan":
      return { agent: "gs-planner", modelRole: "plan" };
    case "plan_review":
      return { agent: "gs-critic", modelRole: "critic" };
    case "implement":
      return { agent: "gs-worker", modelRole: "implement" };
    case "verify":
      return { agent: "gs-verifier", modelRole: "verify" };
    case "review":
      return { agent: "gs-reviewer", modelRole: "review", securityIfRisky: true };
    case "uat":
      return { modelRole: "coordinator" };
    case "closeout":
      return { modelRole: "coordinator" };
    default:
      return {};
  }
}

/** Which specialist agents are legal at a given stage (for the tool_call hook). */
export function agentsForStage(stage: Stage, risk: RiskAssessment): GsAgent[] {
  const req = stageRequirement(stage);
  const out: GsAgent[] = [];
  if (req.agent) out.push(req.agent);
  if (req.securityIfRisky && risk.security) out.push("gs-security");
  return out;
}

export interface ModelSelection {
  model?: ModelEvidence;
  finding?: GateFinding;
}

/**
 * Resolve a stage's model from its config fallback chain. The first selector
 * that resolves to an authenticated model NOT excluded by independence wins.
 * `isFallback` is true when the winner is not the primary selector. Returns a
 * MODEL_UNRESOLVABLE finding when no legal candidate exists — the caller blocks.
 */
export function selectStageModel(
  role: GsModelRole,
  config: GsConfig,
  resolve: ModelResolver,
  opts: { excludeFamilies?: string[]; excludeIds?: string[]; thinking?: ModelEvidence["thinking"] } = {},
): ModelSelection {
  const selectors = config.models[role] ?? [];
  const excludeFamilies = new Set(opts.excludeFamilies ?? []);
  const excludeIds = new Set(opts.excludeIds ?? []);
  const tried: string[] = [];
  let excludedByIndependence = false;

  for (let i = 0; i < selectors.length; i++) {
    const selector = selectors[i];
    tried.push(selector);
    const resolved = resolve(selector);
    if (!resolved) continue;
    if (excludeIds.has(resolved.id) || excludeFamilies.has(resolved.family)) {
      excludedByIndependence = true;
      continue;
    }
    return {
      model: {
        id: resolved.id,
        provider: resolved.provider,
        model: resolved.model,
        family: resolved.family,
        requestedSelector: selector,
        isFallback: i > 0,
        thinking: opts.thinking,
      },
    };
  }

  const reason = excludedByIndependence
    ? `every authenticated candidate for role "${role}" collides with an excluded model/family (independence); add a distinct model to gs.json models.${role}`
    : `no authenticated model resolves for role "${role}" (tried ${tried.join(", ") || "nothing"}); log in and/or fix gs.json models.${role}`;
  return { finding: { code: "MODEL_UNRESOLVABLE", message: reason } };
}

/** Two models are independent when their family lineage tokens differ. */
export function modelsIndependent(a: ModelEvidence | undefined, b: ModelEvidence | undefined): boolean {
  if (!a || !b) return false;
  if (a.family && b.family) return a.family !== b.family;
  // No family token — fall back to concrete id inequality.
  return a.id !== b.id;
}

// ---------------------------------------------------------------------------
// Verify command safety
// ---------------------------------------------------------------------------

const SHELL_PROGRAMS = new Set(["bash", "sh", "zsh", "fish", "dash", "ksh"]);
const SHELL_METACHARS = /[;&|`$><]|\$\(|&&|\|\|/;

/**
 * A verify command is "shell" (rejected by the plan gate) when it invokes a
 * shell with -c/-lc, or smuggles shell metacharacters through its args instead
 * of using a clean argv. Direct `{program:"cargo", args:["test"]}` is allowed.
 */
export function verifyCommandIsShell(cmd: VerifyCommand): boolean {
  const prog = (cmd.program ?? "").split("/").pop() ?? "";
  if (SHELL_PROGRAMS.has(prog) && cmd.args.some((a) => a === "-c" || a === "-lc" || a === "-ic")) {
    return true;
  }
  return cmd.args.some((a) => SHELL_METACHARS.test(a));
}

// ---------------------------------------------------------------------------
// Security + destructive classification
// ---------------------------------------------------------------------------

const SECURITY_KEYWORDS: RegExp[] = [
  /\bauth(entication|orization)?\b/i,
  /\bcredential|secret|token|password|api[_-]?key\b/i,
  /\bfilesystem|\bpath traversal|\.\.\//i,
  /\bspawn|exec|child_process|subprocess|command injection\b/i,
  /\bnetwork|http|fetch|socket|request\b/i,
  /\bdeserial|pickle|yaml\.load|eval\(/i,
  /\bpermission|chmod|chown|setuid\b/i,
  /\b(npm|pip|cargo|apt|pacman|brew) install\b/i,
  /\bproduction|deploy|release\b/i,
];

/** Does the given free text imply a security-sensitive surface? */
export function textTriggersSecurity(text: string): boolean {
  return SECURITY_KEYWORDS.some((re) => re.test(text));
}

const SECURITY_PATH_HINTS: RegExp[] = [
  /auth/i,
  /secret|credential|token|password/i,
  /security|crypto|hash/i,
  /(^|\/)(exec|spawn|shell|command)/i,
  /net(work)?|http|fetch|socket/i,
  /deploy|release|install|setup/i,
];

export function pathTriggersSecurity(path: string): boolean {
  return SECURITY_PATH_HINTS.some((re) => re.test(path));
}

export interface DestructiveClass {
  destructive: boolean;
  outward: boolean;
  reason: string;
}

const DESTRUCTIVE_PATTERNS: Array<{ re: RegExp; outward: boolean; reason: string }> = [
  { re: /\bgit\b[^\n;&|]*\bpush\b/, outward: true, reason: "git push" },
  { re: /\bgit\b[^\n;&|]*--force\b/, outward: false, reason: "git force operation" },
  { re: /\bgh\b[^\n;&|]*\b(pr|release)\b/, outward: true, reason: "GitHub PR/release" },
  { re: /\brm\b[^\n;&|]*(?:--recursive\b|--force\b|-[a-z]*r[a-z]*\b|-[a-z]*f[a-z]*\b)/, outward: false, reason: "recursive/forced rm" },
  { re: /\b(drop|truncate|delete)\s+(table|database|from)\b/i, outward: false, reason: "destructive SQL" },
  { re: /\b(npm|pip|cargo|apt|pacman|paru|yay|brew)\s+(install|add|-S)\b/, outward: true, reason: "package install" },
  { re: /\bcurl\b.*\|(\s*)(sh|bash)\b/, outward: true, reason: "pipe-to-shell" },
  { re: /\b(kubectl|docker)\s+(apply|push|run|rm)\b/, outward: true, reason: "container/cluster mutation" },
  { re: /\bsystemctl\b[^\n;&|]*\b(start|stop|restart|disable|enable)\b/, outward: false, reason: "systemd mutation" },
];

/**
 * Classify a bash command string as destructive and/or outward-visible. Used by
 * the tool_call hook to block unapproved dangerous actions. Conservative: any
 * match trips the gate.
 */
export function classifyBashCommand(command: string): DestructiveClass {
  for (const p of DESTRUCTIVE_PATTERNS) {
    if (p.re.test(command)) {
      return { destructive: true, outward: p.outward, reason: p.reason };
    }
  }
  return { destructive: false, outward: false, reason: "" };
}

/** A git commit is only allowed once the current work is verified. */
export function isGitCommit(command: string): boolean {
  return /\bgit\s+commit\b/.test(command);
}

// ---------------------------------------------------------------------------
// Direct-argv verify classification
// ---------------------------------------------------------------------------
//
// Verify commands are structured {program, args, cwd?} executed WITHOUT a shell
// (verify.ts spawnSync shell:false). The helpers below classify such an argv
// deterministically — independent of flag order or leading prefixes — and share
// the destructive vocabulary of classifyBashCommand so the plan gate and the
// tool_call hook reason consistently.

const PREFIX_RUNNERS: Record<string, true> = {
  sudo: true, env: true, pkexec: true, doas: true, nice: true, nohup: true, time: true, command: true,
};
const PKG_PROGRAMS: Record<string, true> = {
  npm: true, pnpm: true, yarn: true, pip: true, pip3: true, uv: true, pipx: true, cargo: true,
  apt: true, "apt-get": true, pacman: true, paru: true, yay: true, brew: true, gem: true, go: true, dotnet: true,
};
const PKG_INSTALL_TOKENS: Record<string, true> = {
  install: true, add: true, i: true, in: true, "-S": true, sync: true, upgrade: true, up: true,
};
const CONTAINER_PROGRAMS: Record<string, true> = {
  kubectl: true, docker: true, podman: true, nerdctl: true, helm: true,
};
const CONTAINER_MUTATE: Record<string, true> = {
  apply: true, push: true, run: true, rm: true, deploy: true, delete: true, create: true, scale: true, rollout: true,
};
const SYSTEMD_MUTATE: Record<string, true> = {
  start: true, stop: true, restart: true, reload: true, disable: true, enable: true, mask: true, kill: true,
};

function progBasename(program: string): string {
  return (program ?? "").split("/").pop() ?? "";
}

interface Flags {
  short: Set<string>;
  long: Set<string>;
}

/** Extract short flag bundles (-rf -> r,f) and long options (--force / --force=x). */
function extractFlags(args: string[]): Flags {
  const short = new Set<string>();
  const long = new Set<string>();
  for (const a of args) {
    if (a === "--") break;
    if (a.startsWith("--")) {
      const name = a.slice(2).split("=")[0];
      if (name) long.add(name);
    } else if (a.startsWith("-") && a.length > 1 && !/^-?\d+(\.\d+)?$/.test(a)) {
      for (const ch of a.slice(1)) short.add(ch);
    }
  }
  return { short, long };
}

/**
 * Canonical destructive/outward classification for a single argv pair
 * (program + args). Token + flag based, so it is robust to reordered options
 * and to leading prefixes (sudo/env) that survive argv splitting. Shared by the
 * verify surface (classifyVerifyCommand) and the string surface
 * (classifyBashCommand) so both reason identically.
 */
export function classifyArgv(programRaw: string, args: string[]): DestructiveClass {
  let program = progBasename(programRaw);
  // Strip leading command runners / env assignments (sudo rm ..., env VAR=x rm ...)
  // so a prefixed destructive argv is still classified instead of smuggled past.
  while ((PREFIX_RUNNERS[program] === true || /^[A-Za-z_][A-Za-z0-9_]*=/.test(program)) && args.length > 0) {
    program = progBasename(args[0]);
    args = args.slice(1);
  }
  const flags = extractFlags(args);

  if (program === "git") {
    // push checked first so `git push --force` reports outward (matches classifyBashCommand order)
    if (args.includes("push")) return { destructive: true, outward: true, reason: "git push" };
    if (flags.long.has("force")) return { destructive: true, outward: false, reason: "git force operation" };
  }
  if (program === "rm") {
    const recursive = flags.long.has("recursive") || flags.short.has("r") || flags.short.has("R");
    const force = flags.long.has("force") || flags.short.has("f");
    if (recursive || force) {
      return { destructive: true, outward: false, reason: recursive ? "recursive rm" : "forced rm" };
    }
  }
  if (args.some((a) => /\b(drop|truncate|delete)\s+(table|database|from)\b/i.test(a))) {
    return { destructive: true, outward: false, reason: "destructive SQL" };
  }
  if (PKG_PROGRAMS[program] === true && args.some((a) => PKG_INSTALL_TOKENS[a] === true)) {
    return { destructive: true, outward: true, reason: "package install" };
  }
  if (program === "gh" && args.some((a) => a === "pr" || a === "release")) {
    return { destructive: true, outward: true, reason: "GitHub PR/release" };
  }
  if (CONTAINER_PROGRAMS[program] === true && args.some((a) => CONTAINER_MUTATE[a] === true)) {
    return { destructive: true, outward: true, reason: "container/cluster mutation" };
  }
  if (program === "systemctl" && args.some((a) => SYSTEMD_MUTATE[a] === true)) {
    return { destructive: true, outward: false, reason: "systemd mutation" };
  }
  return { destructive: false, outward: false, reason: "" };
}

/** Classify a structured verify command argv (no shell). */
export function classifyVerifyCommand(cmd: VerifyCommand): DestructiveClass {
  return classifyArgv(cmd.program, cmd.args);
}

// ---------------------------------------------------------------------------
// Verify cwd containment
// ---------------------------------------------------------------------------

function isAbs(p: string): boolean {
  return p.startsWith("/");
}

/** Lexically normalize a path, collapsing "." and "..". */
function normalizeSegments(p: string): string[] {
  const out: string[] = [];
  for (const part of p.split("/")) {
    if (part === "" || part === ".") continue;
    if (part === "..") {
      if (out.length > 0 && out[out.length - 1] !== "..") out.pop();
      else out.push("..");
    } else {
      out.push(part);
    }
  }
  return out;
}

/** True when `target` is lexically under `root` (segment compare, not prefix). */
function lexicallyContained(root: string, target: string): boolean {
  if (!isAbs(root) || !isAbs(target)) return false;
  const r = normalizeSegments(root);
  const t = normalizeSegments(target);
  if (t.length < r.length) return false;
  for (let i = 0; i < r.length; i++) {
    if (t[i] !== r[i]) return false;
  }
  return true;
}

/** Resolve a possibly-relative cwd against the project root. */
function anchorUnder(root: string, cwd: string): string {
  if (isAbs(cwd)) return cwd;
  const base = root.endsWith("/") ? root.slice(0, -1) : root;
  return base + "/" + cwd;
}

function tryRealpath(realpath: (p: string) => string | undefined, p: string): string | undefined {
  try {
    return realpath(p);
  } catch {
    return undefined;
  }
}

/**
 * Is a verify command's optional `cwd` override safely contained under the
 * project root?
 *
 * - No cwd (undefined/empty) → contained (runs at the project root).
 * - Relative cwd is resolved against projectRoot before checking.
 * - Lexical containment is always enforced: absolute escapes and ".."
 *   traversal are rejected, and a segment (not string-prefix) compare defeats
 *   the `/proj-evil` sibling trick.
 * - When a `realpath` resolver is injected, canonical containment is ALSO
 *   required, which defeats symlink escapes. If resolution fails for either
 *   path (missing target / broken symlink) containment is DENIED — fail closed.
 *
 * Pure: verify.ts injects node's realpathSync at runtime; tests inject a fake.
 */
export function isCwdContained(
  projectRoot: string,
  cwd: string | undefined,
  opts: { realpath?: (p: string) => string | undefined } = {},
): boolean {
  if (cwd === undefined || cwd === "") return true;
  const anchored = anchorUnder(projectRoot, cwd);
  if (!lexicallyContained(projectRoot, anchored)) return false;
  if (opts.realpath) {
    const rootReal = tryRealpath(opts.realpath, projectRoot);
    const cwdReal = tryRealpath(opts.realpath, anchored);
    if (rootReal === undefined || cwdReal === undefined) return false;
    if (!lexicallyContained(rootReal, cwdReal)) return false;
  }
  return true;
}

export interface VerifyCommandClass {
  /** Shell invocation or metacharacter smuggling (reject outright). */
  shell: boolean;
  /** Destructive local mutation (rm, SQL, git force, systemd...). */
  destructive: boolean;
  /** Outward-visible effect (push, PR, install, deploy...). */
  outward: boolean;
  /** cwd override escapes the project root. */
  cwdEscape: boolean;
  reasons: string[];
}

/**
 * Single reusable audit of a plan-proposed verify command: shell smuggling,
 * destructive/outward argv, and cwd containment. The plan gate and gs_verify
 * both consult this so classification stays consistent end to end.
 */
export function assessVerifyCommand(
  cmd: VerifyCommand,
  opts: { projectRoot?: string; realpath?: (p: string) => string | undefined } = {},
): VerifyCommandClass {
  const reasons: string[] = [];
  const shell = verifyCommandIsShell(cmd);
  if (shell) reasons.push("shell command / metacharacter smuggling");
  const dc = classifyVerifyCommand(cmd);
  if (dc.destructive) reasons.push(dc.reason);
  const cwdEscape = opts.projectRoot !== undefined && !isCwdContained(opts.projectRoot, cmd.cwd, { realpath: opts.realpath });
  if (cwdEscape) reasons.push("cwd escapes project root");
  return { shell, destructive: dc.destructive, outward: dc.outward, cwdEscape, reasons };
}
