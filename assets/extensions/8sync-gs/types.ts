// 8sync-gs — shared types for the native /gs agent-team engine.
//
// Pure contract module: NO omp / node imports. Every other pure module
// (config, policy, machine, store, task-evidence) and the Bun tests import
// from here; only index.ts (the omp adapter) pulls in the runtime.

export const GS_SCHEMA_VERSION = 1 as const;

export type Stage =
  | "clarify"
  | "research"
  | "plan"
  | "plan_review"
  | "implement"
  | "verify"
  | "review"
  | "uat"
  | "closeout"
  | "done"
  | "blocked"
  | "aborted";

/** Ordered forward path through the workflow (terminal states excluded). */
export const STAGE_ORDER: Stage[] = [
  "clarify",
  "research",
  "plan",
  "plan_review",
  "implement",
  "verify",
  "review",
  "uat",
  "closeout",
];

export const TERMINAL_STAGES: Stage[] = ["done", "blocked", "aborted"];

export type RunStatus = "running" | "awaiting_user" | "blocked" | "done" | "aborted";

export type ItemStatus = "pending" | "running" | "passed" | "failed" | "blocked" | "skipped";

/** The seven specialist agents /gs delegates to. */
export type GsAgent =
  | "gs-researcher"
  | "gs-planner"
  | "gs-critic"
  | "gs-worker"
  | "gs-verifier"
  | "gs-reviewer"
  | "gs-security";

/** Model role keys resolved from gs.json `models`. */
export type GsModelRole =
  | "coordinator"
  | "research"
  | "plan"
  | "implement"
  | "critic"
  | "verify"
  | "review"
  | "security";

export type Mode = "assisted" | "auto";

/** Normalized specialist verdict extracted from a task result. */
export type GsVerdict = "pass" | "needs_fix" | "fail" | "correct" | "incorrect";

export interface ModelEvidence {
  /** Concrete authenticated model, e.g. "anthropic/claude-opus-4-8". */
  id: string;
  provider: string;
  model: string;
  /** Opaque family lineage token used for independence checks. */
  family: string;
  /** The selector that resolved to this model (e.g. "@plan", "family:opus"). */
  requestedSelector: string;
  /** True when the primary selector was unavailable and a fallback was used. */
  isFallback: boolean;
  thinking?: ThinkingLevel;
}

export type ThinkingLevel = "minimal" | "low" | "medium" | "high" | "xhigh";

export interface Requirement {
  id: string;
  text: string;
  kind: "functional" | "nonfunctional" | "constraint" | "nongoal";
  /** true = the run must satisfy this before closeout. */
  required: boolean;
  status: "open" | "confirmed" | "needs_confirmation";
}

export interface AcceptanceCriterion {
  id: string;
  text: string;
  /** Requirement IDs this AC covers (>=1 for required ACs). */
  requirements: string[];
  /** How the AC is proven: deterministic command, review, or user UAT. */
  method: "verify" | "review" | "uat" | "manual";
  required: boolean;
  status: ItemStatus;
  /** Objective evidence references (command hashes, review verdicts, paths). */
  evidence: AcceptanceEvidence[];
}

export interface AcceptanceEvidence {
  kind: "verify" | "review" | "browser" | "smoke" | "api" | "note";
  ref: string;
  detail?: string;
  at: string;
}

export interface RiskAssessment {
  /** Deterministic classification driving research/security gates. */
  trivial: boolean;
  externalUnknown: boolean;
  newArchitecture: boolean;
  security: boolean;
  destructive: boolean;
  outwardEffects: boolean;
  notes: string[];
}

export interface PlanTask {
  id: string;
  title: string;
  /** AC IDs this task serves (>=1). */
  acceptance: string[];
  /** Exclusive file/dir ownership globs for parallel-safety checks. */
  ownership: string[];
  /** Task IDs that must complete first. */
  dependsOn: string[];
  /** Skill dirs that govern the task. */
  skills: string[];
  /** Direct-argv verify commands (NO shell strings). */
  verify: VerifyCommand[];
  status: ItemStatus;
  attempts: number;
  failStreak: number;
  lastFailureHash: string;
  note: string;
}

export interface PlanSlice {
  id: string;
  title: string;
  tasks: PlanTask[];
}

export interface Plan {
  hash: string;
  slices: PlanSlice[];
  createdAt: string;
}

export interface VerifyCommand {
  program: string;
  args: string[];
  cwd?: string;
  timeoutSeconds?: number;
}

export interface VerifyEvidence {
  command: VerifyCommand;
  exitCode: number;
  stdoutHash: string;
  stderrHash: string;
  durationMs: number;
  finishedAt: string;
  /** Truncated combined output for the model. */
  output: string;
}

export interface AgentRunEvidence {
  toolCallId: string;
  /** Original `task` batch call ID; `toolCallId` is unique per result. */
  batchToolCallId?: string;
  agent: GsAgent;
  taskId?: string;
  assignmentHash: string;
  requestedSelector: string;
  resolvedModel: string;
  resolvedModelFamily?: string;
  resolvedModelIsFallback: boolean;
  exitCode: number;
  structuredOutput?: unknown;
  outputPath?: string;
  startedAt?: string;
  endedAt: string;
  /** Normalized verdict extracted from structuredOutput by the adapter. */
  verdict?: GsVerdict;
  /** Count of P0/P1 (blocking) findings the agent reported. */
  blockingFindings?: number;
}

export interface StageRecord {
  status: ItemStatus;
  attempts: number;
  requiredAgent?: GsAgent;
  modelRole?: GsModelRole;
  agentRuns: AgentRunEvidence[];
  verify: VerifyEvidence[];
  gate?: GateResult;
  startedAt?: string;
  endedAt?: string;
}

export interface Approval {
  /** "action" = one-shot destructive/outward-command consent bound by hash in `note`. */
  what: "requirements" | "plan" | "uat" | "action";
  by: "user" | "critic";
  at: string;
  note?: string;
}

/** A leased next-action the coordinator must satisfy before advancing. */
export interface PendingAction {
  id: string;
  stage: Stage;
  kind: "agent" | "verify" | "user";
  /** For agent actions: the exact required agent + assignment. */
  agent?: GsAgent;
  modelRole?: GsModelRole;
  /** Concrete model the subagent should run (selected at gs_next). */
  modelSelector?: string;
  modelId?: string;
  modelFamily?: string;
  assignmentHash?: string;
  taskIds?: string[];
  /** The exact `task` tool_call this lease was bound to (stamped by the tool_call hook). */
  toolCallId?: string;
  /** Revision that was hashed into the lease and revision at dispatch binding. */
  leaseRevision?: number;
  boundRevision?: number;
  instruction: string;
  createdAt: string;
}

export interface AuditEvent {
  at: string;
  stage: Stage;
  kind: string;
  detail: string;
  eventId?: string;
}

export interface LegacyImport {
  importedAt: string;
  sourcePath: string;
  goal: string;
  taskCount: number;
  verifiedCount: number;
}

export interface GsWorktree {
  slug: string;
  path: string;
  branch: string;
  createdAt: string;
}

export interface GsState {
  schemaVersion: typeof GS_SCHEMA_VERSION;
  runId: string;
  slug: string;
  goal: string;
  projectRoot: string;
  mode: Mode;
  stage: Stage;
  status: RunStatus;
  revision: number;
  createdAt: string;
  updatedAt: string;
  originalCoordinator?: ModelEvidence;
  activeCoordinator?: ModelEvidence;
  requirements: Requirement[];
  acceptance: AcceptanceCriterion[];
  risk: RiskAssessment;
  plan?: Plan;
  stages: Record<Stage, StageRecord>;
  approvals: Approval[];
  pendingAction?: PendingAction;
  audit: AuditEvent[];
  legacy?: LegacyImport;
  /** Worktrees successfully created by this run and safe for managed removal. */
  worktrees?: GsWorktree[];
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

export interface GsConfig {
  schemaVersion: number;
  models: Record<GsModelRole, string[]>;
  thinking: Partial<Record<Stage, ThinkingLevel>>;
  limits: {
    maxPlanReviewLoops: number;
    maxVerifyFailures: number;
    maxReviewLoops: number;
    maxParallelWorkers: number;
    commandTimeoutSeconds: number;
  };
  safety: {
    requireUserForDestructive: boolean;
    requireUserForExternalEffects: boolean;
    requireFinalUat: boolean;
  };
}

// ---------------------------------------------------------------------------
// Gate result
// ---------------------------------------------------------------------------

export type GateCode =
  | "OK"
  | "UNRESOLVED_QUESTION"
  | "AC_NOT_OBSERVABLE"
  | "REQUIREMENTS_UNAPPROVED"
  | "RESEARCH_MISSING"
  | "PLANNER_MISSING"
  | "AC_COVERAGE_INCOMPLETE"
  | "REQ_COVERAGE_INCOMPLETE"
  | "DEP_CYCLE"
  | "OWNERSHIP_OVERLAP"
  | "SHELL_VERIFY_COMMAND"
  | "CRITIC_MISSING"
  | "CRITIC_NOT_INDEPENDENT"
  | "CRITIC_NEEDS_FIX"
  | "PLAN_UNAPPROVED"
  | "TASKS_INCOMPLETE"
  | "TASK_UNVERIFIED"
  | "VERIFY_FAILED"
  | "VERIFIER_MISSING"
  | "REVIEW_MISSING"
  | "REVIEW_NOT_INDEPENDENT"
  | "SECURITY_REVIEW_MISSING"
  | "REVIEW_FOUND_BLOCKER"
  | "AC_EVIDENCE_MISSING"
  | "UAT_UNAPPROVED"
  | "VERIFY_GATE_UNMET"
  | "DESTRUCTIVE_UNAPPROVED"
  | "MODEL_UNRESOLVABLE"
  | "REVISION_MISMATCH"
  | "WRONG_STAGE";

export interface GateFinding {
  code: GateCode;
  message: string;
}

export interface GateResult {
  ok: boolean;
  stage: Stage;
  findings: GateFinding[];
  checkedAt: string;
}

/** Result of a machine mutation attempt. */
export interface TransitionResult {
  ok: boolean;
  state: GsState;
  gate?: GateResult;
  message: string;
}
