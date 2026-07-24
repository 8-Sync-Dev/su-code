// 8sync-gs — pure state machine: run creation, mutations (revision-checked,
// audited, idempotent), stage gates, and forward/backward transitions. No omp
// or node imports; every effect is a returned value.

import {
  GS_SCHEMA_VERSION,
  STAGE_ORDER,
  type AcceptanceCriterion,
  type AcceptanceEvidence,
  type AgentRunEvidence,
  type Approval,
  type GateFinding,
  type GateResult,
  type GsConfig,
  type GsState,
  type Mode,
  type PendingAction,
  type Plan,
  type PlanTask,
  type Requirement,
  type RiskAssessment,
  type Stage,
  type StageRecord,
  type TransitionResult,
  type VerifyEvidence,
} from "./types.ts";
import { agentsForStage, modelsIndependent, stageRequirement, verifyCommandIsShell } from "./policy.ts";

const ALL_STAGES: Stage[] = [
  ...STAGE_ORDER,
  "done",
  "blocked",
  "aborted",
];

function emptyStage(): StageRecord {
  return { status: "pending", attempts: 0, agentRuns: [], verify: [] };
}

export function emptyRisk(): RiskAssessment {
  return {
    trivial: false,
    externalUnknown: false,
    newArchitecture: false,
    security: false,
    destructive: false,
    outwardEffects: false,
    notes: [],
  };
}

export interface CreateRunOptions {
  runId: string;
  slug: string;
  goal: string;
  projectRoot: string;
  mode: Mode;
  now?: string;
}

export function createRun(opts: CreateRunOptions): GsState {
  const now = opts.now ?? new Date().toISOString();
  const stages = {} as Record<Stage, StageRecord>;
  for (const s of ALL_STAGES) stages[s] = emptyStage();
  stages.clarify.status = "running";
  stages.clarify.startedAt = now;
  return {
    schemaVersion: GS_SCHEMA_VERSION,
    runId: opts.runId,
    slug: opts.slug,
    goal: opts.goal,
    projectRoot: opts.projectRoot,
    mode: opts.mode,
    stage: "clarify",
    status: opts.mode === "assisted" ? "running" : "running",
    revision: 1,
    createdAt: now,
    updatedAt: now,
    requirements: [],
    acceptance: [],
    risk: emptyRisk(),
    stages,
    approvals: [],
    audit: [{ at: now, stage: "clarify", kind: "created", detail: `run ${opts.slug} (${opts.mode})` }],
  };
}

// ---------------------------------------------------------------------------
// Mutation helpers — all bump revision, stamp updatedAt, append audit.
// ---------------------------------------------------------------------------

export class RevisionError extends Error {}

function touch(state: GsState, now: string, kind: string, detail: string, eventId?: string): void {
  state.revision += 1;
  state.updatedAt = now;
  state.audit.push({ at: now, stage: state.stage, kind, detail, eventId });
  if (state.audit.length > 500) state.audit.splice(0, state.audit.length - 500);
}

/** Guard optimistic-concurrency: reject a mutation carrying a stale revision. */
export function assertRevision(state: GsState, expected: number | undefined): void {
  if (expected !== undefined && expected !== state.revision) {
    throw new RevisionError(`revision mismatch: expected ${expected}, have ${state.revision}`);
  }
}

export interface DefineInput {
  requirements: Requirement[];
  acceptance: AcceptanceCriterion[];
  risk: RiskAssessment;
  expectedRevision?: number;
  now?: string;
}

export function defineRequirements(state: GsState, input: DefineInput): GsState {
  assertRevision(state, input.expectedRevision);
  const now = input.now ?? new Date().toISOString();
  state.requirements = input.requirements;
  state.acceptance = input.acceptance.map((a) => ({ ...a, evidence: a.evidence ?? [] }));
  state.risk = input.risk;
  touch(state, now, "define", `${input.requirements.length} reqs, ${input.acceptance.length} ACs`);
  return state;
}

/** Command-hash key carried in `note` for one-shot action approvals. */
function actionHash(approval: { note?: string }): string {
  return approval.note ?? "";
}

/** Has consent already been granted for this exact command hash? The machine
 * only STORES action approvals (no gate consumes them); the adapter's tool_call
 * hook calls this to authorize a single destructive/outward command. */
export function actionApproved(state: GsState, commandHash: string): boolean {
  return state.approvals.some((a) => a.what === "action" && actionHash(a) === commandHash);
}

export function recordApproval(state: GsState, approval: Omit<Approval, "at">, now = new Date().toISOString()): GsState {
  // Action approvals are one-shot consents keyed by command hash (in `note`):
  // the same hash is never recorded twice. Other approvals dedupe by (what, by).
  const dup =
    approval.what === "action"
      ? actionApproved(state, actionHash(approval))
      : state.approvals.some((a) => a.what === approval.what && a.by === approval.by);
  if (!dup) {
    state.approvals.push({ ...approval, at: now });
  }
  touch(state, now, "approval", `${approval.what} by ${approval.by}${approval.what === "action" ? ` hash=${actionHash(approval)}` : ""}`);
  return state;
}

export function setPlan(state: GsState, plan: Plan, expectedRevision?: number, now = new Date().toISOString()): GsState {
  assertRevision(state, expectedRevision);
  const priorHash = state.plan?.hash;
  // Re-planning invalidates any prior plan approval (user + critic).
  state.approvals = state.approvals.filter((a) => a.what !== "plan");
  state.plan = plan;
  // A changed plan hash invalidates everything bound to the old plan: the
  // critic reviewed it, workers implemented it, verifiers/reviewers audited its
  // diff, and AC evidence was collected against it. Drop it all so a downstream
  // gate can never pass on evidence bound to a superseded plan.
  if (priorHash !== undefined && priorHash !== plan.hash) {
    clearStaleDownstream(state, now);
  }
  touch(state, now, "plan", `${plan.slices.length} slices, hash ${plan.hash}`);
  return state;
}

/** Reset every stage after `plan` plus the AC evidence tied to the old plan.
 * Leaves clarify/research evidence and user requirements/uat approvals intact;
 * the caller has already dropped `plan` approvals. */
function clearStaleDownstream(state: GsState, now: string): void {
  const fromIdx = STAGE_ORDER.indexOf("plan");
  for (const st of STAGE_ORDER) {
    if (STAGE_ORDER.indexOf(st) <= fromIdx) continue;
    const rec = state.stages[st];
    rec.agentRuns = [];
    rec.verify = [];
    rec.gate = undefined;
    rec.status = "pending";
    rec.attempts = 0;
    rec.startedAt = undefined;
    rec.endedAt = undefined;
  }
  for (const a of state.acceptance) {
    a.evidence = [];
    if (a.status === "passed") a.status = "pending";
  }
  touch(state, now, "plan_invalidate", "cleared stale downstream evidence after plan hash change");
}

/** Idempotent by (toolCallId, agent, taskId). A single `task` call returns a
 * batch of results that share one toolCallId but carry distinct taskIds (one
 * per gs-worker item); deduping on toolCallId alone would silently drop every
 * worker after the first. Two results match only when all three agree. */
export function recordAgentRun(
  state: GsState,
  stage: Stage,
  evidence: AgentRunEvidence,
  now = new Date().toISOString(),
): { state: GsState; applied: boolean } {
  const rec = state.stages[stage];
  const dup = rec.agentRuns.some(
    (r) =>
      r.toolCallId === evidence.toolCallId &&
      r.agent === evidence.agent &&
      (r.taskId ?? "") === (evidence.taskId ?? ""),
  );
  if (dup) return { state, applied: false };
  rec.agentRuns.push(evidence);
  touch(state, now, "agent_run", `${evidence.agent}@${evidence.resolvedModel} (${stage})`, evidence.toolCallId);
  return { state, applied: true };
}

export function findTask(state: GsState, taskId: string): PlanTask | undefined {
  if (!state.plan) return undefined;
  for (const s of state.plan.slices) for (const t of s.tasks) if (t.id === taskId) return t;
  return undefined;
}

/** All plan tasks in slice order. */
function allTasks(state: GsState): PlanTask[] {
  return state.plan?.slices.flatMap((s) => s.tasks) ?? [];
}

/**
 * Successful (exit 0) gs-worker runs that reported work for a task. A
 * legacy-imported verified task carries no worker run (it arrived `passed`
 * from the import mapping), so this returns [] for it; callers that need "is
 * this task verified" must treat a `passed` task without worker evidence as
 * legacy-verified rather than unevidenced.
 */
export function workerEvidenceForTask(state: GsState, taskId: string): AgentRunEvidence[] {
  return state.stages.implement.agentRuns.filter(
    (r) => r.agent === "gs-worker" && r.taskId === taskId && r.exitCode === 0,
  );
}

/** FNV-1a 32-bit — mirrors the legacy engine's no-progress fingerprint. */
export function fnv1a(s: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16);
}

export interface VerifyOutcome {
  state: GsState;
  status: "passed" | "failed" | "blocked";
  message: string;
}

/**
 * Apply a verify result to a task, replicating the legacy doom-loop guard:
 * identical consecutive failures block at 3; total failures block at
 * maxVerifyFailures. Success marks the task passed.
 */
export function applyVerify(
  state: GsState,
  taskId: string,
  passed: boolean,
  failureOutput: string,
  evidence: VerifyEvidence[],
  config: GsConfig,
  now = new Date().toISOString(),
): VerifyOutcome {
  const task = findTask(state, taskId);
  if (!task) return { state, status: "failed", message: `no task ${taskId}` };
  state.stages.verify.verify.push(...evidence);
  if (passed) {
    task.status = "passed";
    task.failStreak = 0;
    task.lastFailureHash = "";
    touch(state, now, "verify_pass", taskId);
    return { state, status: "passed", message: `verified ${taskId}` };
  }
  task.attempts += 1;
  const hash = fnv1a(failureOutput);
  task.failStreak = hash === task.lastFailureHash ? task.failStreak + 1 : 1;
  task.lastFailureHash = hash;
  if (task.failStreak >= 3) {
    task.status = "blocked";
    task.note = `no progress — ${task.failStreak} identical failures`;
    touch(state, now, "verify_block", `${taskId} doom-loop`);
    return { state, status: "blocked", message: `BLOCKED ${taskId}: 3 identical failures (doom-loop guard)` };
  }
  if (task.attempts >= config.limits.maxVerifyFailures) {
    task.status = "blocked";
    task.note = `blocked after ${task.attempts} failed verifies`;
    touch(state, now, "verify_block", `${taskId} maxRetries`);
    return { state, status: "blocked", message: `BLOCKED ${taskId} after ${task.attempts} attempts` };
  }
  task.status = "failed";
  touch(state, now, "verify_fail", `${taskId} attempt ${task.attempts}`);
  const warn = task.failStreak === 2 ? " WARNING: same failure twice — a third identical failure blocks the task." : "";
  return { state, status: "failed", message: `FAILED ${taskId} (attempt ${task.attempts}/${config.limits.maxVerifyFailures}).${warn}` };
}

export function attachAcceptanceEvidence(
  state: GsState,
  acId: string,
  evidence: AcceptanceEvidence,
  status: AcceptanceCriterion["status"] = "passed",
  now = new Date().toISOString(),
): GsState {
  const ac = state.acceptance.find((a) => a.id === acId);
  if (!ac) return state;
  ac.evidence.push(evidence);
  ac.status = status;
  touch(state, now, "ac_evidence", `${acId} ${status} (${evidence.kind})`);
  return state;
}

export function setPending(state: GsState, action: PendingAction, now = new Date().toISOString()): GsState {
  state.pendingAction = action;
  touch(state, now, "pending", `${action.kind}:${action.agent ?? action.stage}`);
  return state;
}

export function clearPending(state: GsState, now = new Date().toISOString()): GsState {
  if (state.pendingAction) {
    state.pendingAction = undefined;
    touch(state, now, "pending_clear", "");
  }
  return state;
}

// ---------------------------------------------------------------------------
// Gates
// ---------------------------------------------------------------------------

function latestRun(state: GsState, stage: Stage, agent: string): AgentRunEvidence | undefined {
  const runs = state.stages[stage].agentRuns.filter((r) => r.agent === agent && r.exitCode === 0);
  return runs.length ? runs[runs.length - 1] : undefined;
}

function requiredAcs(state: GsState): AcceptanceCriterion[] {
  return state.acceptance.filter((a) => a.required);
}

function detectCycle(tasks: PlanTask[]): boolean {
  const ids = new Set(tasks.map((t) => t.id));
  const graph = new Map<string, string[]>();
  for (const t of tasks) graph.set(t.id, t.dependsOn.filter((d) => ids.has(d)));
  const state = new Map<string, number>(); // 0=unvisited,1=visiting,2=done
  const visit = (id: string): boolean => {
    const s = state.get(id) ?? 0;
    if (s === 1) return true;
    if (s === 2) return false;
    state.set(id, 1);
    for (const dep of graph.get(id) ?? []) if (visit(dep)) return true;
    state.set(id, 2);
    return false;
  };
  for (const id of ids) if (visit(id)) return true;
  return false;
}

export interface OwnershipConflict {
  a: string;
  b: string;
  ownership: string;
}

/**
 * Pairs of tasks that share a file-ownership glob with no dependency edge
 * between them — they would race in the same parallel wave. Dependency-aware:
 * a dependsOn link serializes the pair, so it is never reported as a conflict.
 */
export function ownershipOverlapPairs(tasks: PlanTask[]): OwnershipConflict[] {
  const out: OwnershipConflict[] = [];
  for (let i = 0; i < tasks.length; i++) {
    for (let j = i + 1; j < tasks.length; j++) {
      const a = tasks[i];
      const b = tasks[j];
      const linked = a.dependsOn.includes(b.id) || b.dependsOn.includes(a.id);
      if (linked) continue;
      const shared = a.ownership.find((o) => b.ownership.includes(o));
      if (shared) out.push({ a: a.id, b: b.id, ownership: shared });
    }
  }
  return out;
}

/** Should research run? Deterministic risk rule. */
export function researchRequired(risk: RiskAssessment): boolean {
  if (risk.trivial) return false;
  return risk.externalUnknown || risk.newArchitecture || risk.security;
}

function gateOk(stage: Stage, now: string): GateResult {
  return { ok: true, stage, findings: [], checkedAt: now };
}

function gateFail(stage: Stage, findings: GateFinding[], now: string): GateResult {
  return { ok: false, stage, findings, checkedAt: now };
}

/**
 * Evaluate the gate that governs leaving `state.stage`. Reads recorded evidence
 * only — never model prose. Returns ok=false with the exact failed findings.
 */
function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** Validate a gs-researcher structured output: non-empty findings + sources
 * and no unresolved open_unknowns. A bare task exit is insufficient. */
function researchOutputFindings(run: AgentRunEvidence): GateFinding[] {
  const o = isObj(run.structuredOutput) ? run.structuredOutput : {};
  const findings = Array.isArray(o.findings) ? o.findings : [];
  const sources = Array.isArray(o.sources) ? o.sources : [];
  const unknowns = Array.isArray(o.open_unknowns) ? o.open_unknowns : [];
  const f: GateFinding[] = [];
  if (findings.length === 0) f.push({ code: "RESEARCH_MISSING", message: "gs-researcher produced no findings" });
  if (sources.length === 0) f.push({ code: "RESEARCH_MISSING", message: "gs-researcher produced no sources" });
  if (unknowns.length > 0) f.push({ code: "UNRESOLVED_QUESTION", message: `gs-researcher left ${unknowns.length} unresolved open_unknowns` });
  return f;
}

/**
 * Findings for leaving `stage`, evaluated on RECORDED evidence only — never
 * model prose. Pure per-stage logic so closeout can re-assert the load-bearing
 * verify/review/uat gates against the CURRENT state instead of trusting a
 * recorded gate that may be stale after a resume or replan.
 */
function stageFindings(state: GsState, config: GsConfig, stage: Stage, now: string): GateFinding[] {
  const f: GateFinding[] = [];
  const assisted = state.mode === "assisted";

  switch (stage) {
    case "clarify": {
      if (state.requirements.length === 0) {
        f.push({ code: "UNRESOLVED_QUESTION", message: "no requirements captured" });
      }
      if (requiredAcs(state).length === 0) {
        f.push({ code: "AC_NOT_OBSERVABLE", message: "no required acceptance criteria defined" });
      }
      for (const a of state.acceptance) {
        if (!a.text.trim() || !a.method) {
          f.push({ code: "AC_NOT_OBSERVABLE", message: `AC ${a.id} is not observable (missing text/method)` });
        }
      }
      if (assisted && !state.approvals.some((a) => a.what === "requirements" && a.by === "user")) {
        f.push({ code: "REQUIREMENTS_UNAPPROVED", message: "awaiting /gs approve requirements" });
      }
      break;
    }
    case "research": {
      if (!researchRequired(state.risk)) break;
      const run = latestRun(state, "research", "gs-researcher");
      if (!run) {
        f.push({ code: "RESEARCH_MISSING", message: "risk is nontrivial but no gs-researcher result recorded" });
        break;
      }
      // A clean task exit is not enough: the structured output must actually
      // resolve the unknowns with backed findings + sources.
      f.push(...researchOutputFindings(run));
      break;
    }
    case "plan": {
      if (!state.plan) {
        f.push({ code: "PLANNER_MISSING", message: "no plan recorded" });
        break;
      }
      if (!latestRun(state, "plan", "gs-planner")) {
        f.push({ code: "PLANNER_MISSING", message: "no gs-planner task result recorded" });
      }
      const tasks = state.plan.slices.flatMap((s) => s.tasks);
      const acIds = new Set(state.acceptance.map((a) => a.id));
      const reqIds = new Set(state.requirements.map((r) => r.id));
      const ids = new Set(tasks.map((t) => t.id));
      for (const t of tasks) {
        if (t.acceptance.length === 0 || !t.acceptance.some((id) => acIds.has(id))) {
          f.push({ code: "AC_COVERAGE_INCOMPLETE", message: `task ${t.id} maps to no known AC` });
        }
        for (const v of t.verify) {
          if (verifyCommandIsShell(v)) {
            f.push({ code: "SHELL_VERIFY_COMMAND", message: `task ${t.id} has a shell-string verify command` });
          }
        }
        for (const dep of t.dependsOn) {
          if (!ids.has(dep)) {
            f.push({ code: "DEP_CYCLE", message: `task ${t.id} depends on unknown task ${dep}` });
          }
        }
      }
      const covered = new Set(tasks.flatMap((t) => t.acceptance));
      for (const a of requiredAcs(state)) {
        if (!covered.has(a.id)) {
          f.push({ code: "AC_COVERAGE_INCOMPLETE", message: `required AC ${a.id} has no task` });
        }
        if (!a.requirements.some((r) => reqIds.has(r))) {
          f.push({ code: "REQ_COVERAGE_INCOMPLETE", message: `AC ${a.id} maps to no requirement` });
        }
      }
      if (detectCycle(tasks)) f.push({ code: "DEP_CYCLE", message: "task dependency graph has a cycle" });
      for (const s of state.plan.slices) {
        const conflicts = ownershipOverlapPairs(s.tasks);
        if (conflicts.length) {
          const c = conflicts[0];
          f.push({ code: "OWNERSHIP_OVERLAP", message: `slice ${s.id}: tasks ${c.a} & ${c.b} share ownership "${c.ownership}" with no dependency edge` });
        }
      }
      break;
    }
    case "plan_review": {
      const planner = latestRun(state, "plan", "gs-planner");
      const critic = latestRun(state, "plan_review", "gs-critic");
      if (!critic) {
        f.push({ code: "CRITIC_MISSING", message: "no gs-critic result recorded" });
      } else {
        if (!independentByFamily(planner, critic)) {
          f.push({ code: "CRITIC_NOT_INDEPENDENT", message: "planner and critic resolved to the same model family" });
        }
        if (critic.verdict !== "pass") {
          f.push({ code: "CRITIC_NEEDS_FIX", message: `critic verdict is ${critic.verdict ?? "unknown"}` });
        }
      }
      if (assisted && !state.approvals.some((a) => a.what === "plan" && a.by === "user")) {
        f.push({ code: "PLAN_UNAPPROVED", message: "awaiting /gs approve plan" });
      }
      break;
    }
    case "implement": {
      const tasks = state.plan?.slices.flatMap((s) => s.tasks) ?? [];
      for (const t of tasks) {
        if (t.status === "passed" || t.status === "skipped") continue;
        if (t.status === "blocked") {
          f.push({ code: "TASKS_INCOMPLETE", message: `task ${t.id} is blocked` });
          continue;
        }
        f.push({ code: "TASKS_INCOMPLETE", message: `task ${t.id} is ${t.status}` });
      }
      break;
    }
    case "verify": {
      const verifier = latestRun(state, "verify", "gs-verifier");
      if (!verifier) {
        f.push({ code: "VERIFIER_MISSING", message: "no gs-verifier audit recorded" });
      } else if (verifier.verdict !== "pass") {
        f.push({ code: "VERIFY_FAILED", message: `verifier verdict is ${verifier.verdict ?? "unknown"}` });
      }
      break;
    }
    case "review": {
      const impl = latestRun(state, "implement", "gs-worker");
      const reviewer = latestRun(state, "review", "gs-reviewer");
      if (!reviewer) {
        f.push({ code: "REVIEW_MISSING", message: "no gs-reviewer result recorded" });
      } else {
        if (impl && !independentByFamily(impl, reviewer)) {
          f.push({ code: "REVIEW_NOT_INDEPENDENT", message: "reviewer shares the implementation model family" });
        }
        if (reviewer.verdict === "incorrect" || (reviewer.blockingFindings ?? 0) > 0) {
          f.push({ code: "REVIEW_FOUND_BLOCKER", message: "reviewer flagged a P0/P1 blocker" });
        }
      }
      if (state.risk.security) {
        const sec = latestRun(state, "review", "gs-security");
        if (!sec) {
          f.push({ code: "SECURITY_REVIEW_MISSING", message: "security-sensitive change lacks a gs-security review" });
        } else if (sec.verdict === "incorrect" || (sec.blockingFindings ?? 0) > 0) {
          f.push({ code: "REVIEW_FOUND_BLOCKER", message: "security review flagged a P0/P1 blocker" });
        }
      }
      break;
    }
    case "uat": {
      for (const a of requiredAcs(state)) {
        if (a.status !== "passed" || a.evidence.length === 0) {
          f.push({ code: "AC_EVIDENCE_MISSING", message: `AC ${a.id} lacks objective PASS evidence (${a.status})` });
        }
      }
      if (config.safety.requireFinalUat && !state.approvals.some((a) => a.what === "uat" && a.by === "user")) {
        f.push({ code: "UAT_UNAPPROVED", message: "awaiting /gs approve uat" });
      }
      break;
    }
    case "closeout": {
      // Re-assert the load-bearing gates from CURRENT evidence. A recorded gate
      // (or a corrupted resume) must never let closeout skip verify, review, or
      // UAT — recompute each against the live state.
      const seen = new Set<string>();
      for (const st of ["verify", "review", "uat"] as Stage[]) {
        for (const finding of stageFindings(state, config, st, now)) {
          if (seen.has(finding.code)) continue;
          seen.add(finding.code);
          f.push({ code: finding.code, message: `closeout re-check: ${finding.message}` });
        }
      }
      break;
    }
    default:
      break;
  }

  return f;
}

/**
 * Evaluate the gate that governs leaving `state.stage`. Reads recorded evidence
 * only — never model prose. Returns ok=false with the exact failed findings.
 */
export function evaluateGate(state: GsState, config: GsConfig, now = new Date().toISOString()): GateResult {
  const f = stageFindings(state, config, state.stage, now);
  return f.length ? gateFail(state.stage, f, now) : gateOk(state.stage, now);
}

function independentByFamily(a: AgentRunEvidence | undefined, b: AgentRunEvidence | undefined): boolean {
  return modelsIndependent(
    a ? { id: a.resolvedModel, provider: "", model: "", family: a.resolvedModelFamily ?? "", requestedSelector: "", isFallback: false } : undefined,
    b ? { id: b.resolvedModel, provider: "", model: "", family: b.resolvedModelFamily ?? "", requestedSelector: "", isFallback: false } : undefined,
  );
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

/** Next forward stage, skipping research when the risk rule says so. */
export function nextStage(state: GsState): Stage {
  const idx = STAGE_ORDER.indexOf(state.stage);
  if (idx < 0) return state.stage;
  let next = STAGE_ORDER[idx + 1];
  if (next === undefined) return "done";
  if (state.stage === "clarify" && next === "research" && !researchRequired(state.risk)) {
    next = "plan";
  }
  return next;
}

/**
 * Attempt to advance out of the current stage. Evaluates the gate; on pass,
 * records the gate on the stage, moves to the next stage (or `done`), and
 * initializes the new stage. On fail, returns the gate findings unchanged.
 */
export function advance(state: GsState, config: GsConfig, expectedRevision?: number, now = new Date().toISOString()): TransitionResult {
  assertRevision(state, expectedRevision);
  const gate = evaluateGate(state, config, now);
  const cur = state.stages[state.stage];
  cur.gate = gate;
  if (!gate.ok) {
    const handled = tryAutoReopen(state, config, gate, now);
    if (handled) return handled;
    touch(state, now, "gate_fail", `${state.stage}: ${gate.findings.map((x) => x.code).join(",")}`);
    return { ok: false, state, gate, message: `gate ${state.stage} failed: ${gate.findings.map((x) => x.message).join("; ")}` };
  }
  cur.status = "passed";
  cur.endedAt = now;
  const target = state.stage === "closeout" ? "done" : nextStage(state);
  const skippedResearch = state.stage === "clarify" && target === "plan";
  if (skippedResearch) {
    state.stages.research.status = "skipped";
    state.stages.research.gate = { ok: true, stage: "research", findings: [], checkedAt: now };
  }
  state.stage = target;
  state.pendingAction = undefined;
  if (target === "done") {
    state.status = "done";
    state.stages.done.status = "passed";
  } else {
    const rec = state.stages[target];
    rec.status = "running";
    rec.startedAt = now;
    const req = stageRequirement(target);
    rec.requiredAgent = req.agent;
    rec.modelRole = req.modelRole;
  }
  touch(state, now, "advance", `→ ${target}`);
  return { ok: true, state, gate, message: `advanced to ${target}` };
}

/** Reopen an earlier stage after a critic NEEDS_FIX / review blocker. */
export function reopen(state: GsState, target: Stage, reason: string, now = new Date().toISOString()): GsState {
  state.stage = target;
  const rec = state.stages[target];
  rec.status = "running";
  rec.attempts += 1;
  rec.gate = undefined;
  rec.endedAt = undefined;
  state.pendingAction = undefined;
  const req = stageRequirement(target);
  rec.requiredAgent = req.agent;
  rec.modelRole = req.modelRole;
  touch(state, now, "reopen", `${target}: ${reason}`);
  return state;
}

/**
 * After a verify/review failure reopens `implement`, clear the "passed" mark
 * from tasks this run actually built (those with worker evidence) so the next
 * implement pass is a real rework, not a no-op that re-advances immediately.
 * Legacy-imported verified tasks (no worker run) are preserved.
 */
function resetTasksForFixPass(state: GsState, now: string, reason: string): void {
  const workerTaskIds = new Set(
    state.stages.implement.agentRuns
      .filter((r) => r.agent === "gs-worker" && r.taskId)
      .map((r) => r.taskId as string),
  );
  for (const t of allTasks(state)) {
    if (t.status === "passed" && workerTaskIds.has(t.id)) {
      t.status = "pending";
      t.failStreak = 0;
      t.lastFailureHash = "";
      t.note = `reopened for fix pass: ${reason}`;
    }
  }
}

/**
 * On a gate failure that represents a fixable downstream defect (not a missing
 * run or a model-independence/config problem), automatically reopen the source
 * stage so auto mode is never stranded at a gate it cannot satisfy itself.
 * Bounded by maxPlanReviewLoops (plan <-> plan_review) and maxReviewLoops
 * (implement <-> verify/review); at the limit the run blocks for escalation.
 * Returns a TransitionResult when it handled the failure, or undefined to let
 * advance report a plain gate failure.
 */
function tryAutoReopen(
  state: GsState,
  config: GsConfig,
  gate: GateResult,
  now: string,
): TransitionResult | undefined {
  const codes = new Set(gate.findings.map((x) => x.code));
  const stage = state.stage;

  // plan_review: a critic NEEDS_FIX (and only that) sends the run back to plan.
  if (
    stage === "plan_review" &&
    codes.has("CRITIC_NEEDS_FIX") &&
    !codes.has("CRITIC_NOT_INDEPENDENT") &&
    !codes.has("CRITIC_MISSING")
  ) {
    const limit = config.limits.maxPlanReviewLoops;
    if (state.stages.plan.attempts >= limit) {
      block(state, `plan-review loop limit (${limit}) reached; critic still NEEDS_FIX`, now);
      return { ok: false, state, gate, message: `BLOCKED: plan-review loop limit (${limit}) reached — critic still NEEDS_FIX` };
    }
    const attempt = state.stages.plan.attempts + 1;
    reopen(state, "plan", `critic NEEDS_FIX (fix attempt ${attempt}/${limit})`, now);
    return { ok: true, state, gate, message: `plan_review failed (critic NEEDS_FIX); reopened plan for fix attempt ${attempt}/${limit}` };
  }

  // verify/review: a verifier FAIL or a reviewer P0/P1 blocker sends the run
  // back to implement for a real fix pass. Missing runs and independence issues
  // are setup problems, not fix loops — left as plain gate failures.
  const fixable =
    (stage === "verify" && codes.has("VERIFY_FAILED")) ||
    (stage === "review" && codes.has("REVIEW_FOUND_BLOCKER"));
  if (fixable) {
    const limit = config.limits.maxReviewLoops;
    if (state.stages.implement.attempts >= limit) {
      block(state, `implement loop limit (${limit}) reached; ${stage} still failing`, now);
      return { ok: false, state, gate, message: `BLOCKED: implement loop limit (${limit}) reached — ${stage} still failing` };
    }
    const attempt = state.stages.implement.attempts + 1;
    resetTasksForFixPass(state, now, `${stage} failure`);
    reopen(state, "implement", `${stage} failure (fix attempt ${attempt}/${limit})`, now);
    return { ok: true, state, gate, message: `${stage} failed; reopened implement for fix attempt ${attempt}/${limit}` };
  }

  return undefined;
}

export function block(state: GsState, reason: string, now = new Date().toISOString()): GsState {
  state.stage = "blocked";
  state.status = "blocked";
  state.stages.blocked.status = "blocked";
  state.pendingAction = undefined;
  touch(state, now, "blocked", reason);
  return state;
}

export function abort(state: GsState, reason: string, now = new Date().toISOString()): GsState {
  state.stage = "aborted";
  state.status = "aborted";
  state.stages.aborted.status = "blocked";
  state.pendingAction = undefined;
  touch(state, now, "aborted", reason);
  return state;
}

export interface Counts {
  tasksTotal: number;
  tasksPassed: number;
  tasksBlocked: number;
  acTotal: number;
  acPassed: number;
}

export function counts(state: GsState): Counts {
  const tasks = state.plan?.slices.flatMap((s) => s.tasks) ?? [];
  const req = requiredAcs(state);
  return {
    tasksTotal: tasks.length,
    tasksPassed: tasks.filter((t) => t.status === "passed").length,
    tasksBlocked: tasks.filter((t) => t.status === "blocked").length,
    acTotal: req.length,
    acPassed: req.filter((a) => a.status === "passed").length,
  };
}

export function agentsAllowedNow(state: GsState) {
  return agentsForStage(state.stage, state.risk);
}

/** Forward-position comparison over the canonical stage order. */
export function afterStage(stage: Stage, ref: Stage): boolean {
  return STAGE_ORDER.indexOf(stage) > STAGE_ORDER.indexOf(ref);
}

/**
 * Is the plan approved (so edits may begin)? Auto mode accepts a critic PASS or
 * being past plan_review; assisted mode requires the user's plan approval.
 */
export function planApproved(state: GsState): boolean {
  if (state.mode === "auto") {
    return state.approvals.some((a) => a.what === "plan" && a.by === "critic") || afterStage(state.stage, "plan_review");
  }
  return state.approvals.some((a) => a.what === "plan" && a.by === "user");
}
