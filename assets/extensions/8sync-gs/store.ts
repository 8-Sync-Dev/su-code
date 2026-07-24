// 8sync-gs — durable persistence: atomic runtime cache, cross-machine phase
// checkpoint, compact Markdown projections, ACTIVE pointer, and legacy engine
// import. Uses node:fs but every write is randomized-tmp→fsync→rename with a
// .bak rotation so a crash mid-write never loses the last good state, and a
// symlinked path component beneath the project root is refused rather than
// silently followed. Structural validation of persisted JSON is exhaustive
// and fail-closed: a present-but-malformed file is never treated as valid
// state, and never silently resets a run — the caller sees the real source
// (cache/checkpoint/backup/legacy/corrupt/none).

import { randomBytes } from "node:crypto";
import {
  closeSync,
  existsSync,
  fsyncSync,
  lstatSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  rmSync,
  type Stats,
  writeSync,
} from "node:fs";
import { basename, dirname, isAbsolute, join, relative, sep } from "node:path";
import {
  GS_SCHEMA_VERSION,
  type AcceptanceCriterion,
  type AcceptanceEvidence,
  type AgentRunEvidence,
  type Approval,
  type AuditEvent,
  type GateFinding,
  type GateResult,
  type GsState,
  type LegacyImport,
  type ModelEvidence,
  type PendingAction,
  type Plan,
  type PlanSlice,
  type PlanTask,
  type Requirement,
  type RiskAssessment,
  type Stage,
  type StageRecord,
  type VerifyCommand,
  type VerifyEvidence,
} from "./types.ts";
import { createRun, fnv1a } from "./machine.ts";
import { isGsAgent } from "./task-evidence.ts";

export const CACHE_REL = ".cache/8sync/gs/state.json";
export const LEGACY_REL = ".cache/8sync/engine/state.json";
export const LEGACY_COPY_REL = ".cache/8sync/gs/legacy-engine-state.json";
export const LEGACY_IMPORT_MARKER_REL = ".cache/8sync/gs/legacy-import-marker.json";
export const PLANNING_REL = "su-code/planning";
export const ACTIVE_REL = "su-code/planning/ACTIVE";
export const STATE_MD_REL = "su-code/STATE.md";

export interface GsPaths {
  cacheFile: string;
  backupFile: string;
  planningDir: string;
  checkpoint: string;
  active: string;
  requirementsMd: string;
  planMd: string;
  verificationMd: string;
}

export function gsPaths(root: string, slug: string): GsPaths {
  const planningDir = join(root, PLANNING_REL, slug);
  return {
    cacheFile: join(root, CACHE_REL),
    backupFile: `${join(root, CACHE_REL)}.bak`,
    planningDir,
    checkpoint: join(planningDir, "CHECKPOINT.json"),
    active: join(root, ACTIVE_REL),
    requirementsMd: join(planningDir, "REQUIREMENTS.md"),
    planMd: join(planningDir, "PLAN.md"),
    verificationMd: join(planningDir, "VERIFICATION.md"),
  };
}

// ---------------------------------------------------------------------------
// Atomic, symlink-safe writes
// ---------------------------------------------------------------------------

/**
 * Refuse to write beneath `root` when an existing path component between
 * `root` and the target's directory is a symlink. Missing components are
 * safe (mkdir creates a real directory); this only inspects what already
 * exists, so it must run before any directory is created.
 */
function assertNoSymlinkEscape(root: string, target: string): void {
  const dir = dirname(target);
  const rel = relative(root, dir);
  if (rel === "") return;
  if (rel === ".." || rel.startsWith(`..${sep}`) || isAbsolute(rel)) {
    throw new Error(`8sync-gs: refusing to write outside project root: ${target}`);
  }
  let current = root;
  for (const segment of rel.split(sep)) {
    if (!segment) continue;
    current = join(current, segment);
    let st: Stats;
    try {
      st = lstatSync(current);
    } catch {
      continue; // does not exist yet — mkdir will create a real directory
    }
    if (st.isSymbolicLink()) {
      throw new Error(`8sync-gs: refusing to write through symlinked path component: ${current}`);
    }
  }
}

/** Best-effort fsync of a directory so a rename inside it is durable. */
function fsyncParentDir(file: string): void {
  try {
    const dirFd = openSync(dirname(file), "r");
    try {
      fsyncSync(dirFd);
    } finally {
      closeSync(dirFd);
    }
  } catch {
    /* directory fsync is unsupported on some platforms/filesystems */
  }
}

/**
 * Write `content` to `file` via a randomized, exclusively-created temp file
 * (fails loudly instead of following an existing name), fsync the temp file
 * before rename, rename into place (POSIX rename replaces the destination —
 * including a symlinked destination — without ever dereferencing it), then
 * fsync the parent directory so the rename survives a crash.
 */
function writeFileAtomic(file: string, content: string): void {
  const tmp = join(dirname(file), `.${basename(file)}.${randomBytes(6).toString("hex")}.tmp`);
  const fd = openSync(tmp, "wx", 0o600);
  try {
    writeSync(fd, content);
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
  renameSync(tmp, file);
  fsyncParentDir(file);
}

/**
 * Atomic write with `.bak` rotation. When `guardRoot` is given, every
 * existing path component beneath it is checked for a symlink escape first —
 * this is how every internal call site (all paths live under the project
 * root) prevents a planted symlink from redirecting a write outside it.
 */
export function atomicWrite(file: string, content: string, guardRoot?: string): void {
  if (guardRoot) assertNoSymlinkEscape(guardRoot, file);
  mkdirSync(dirname(file), { recursive: true });
  if (existsSync(file)) {
    try {
      const st = lstatSync(file);
      if (!st.isSymbolicLink()) {
        writeFileAtomic(`${file}.bak`, readFileSync(file, "utf8"));
      }
    } catch {
      /* best effort: backup rotation never blocks the primary write */
    }
  }
  writeFileAtomic(file, content);
}

// ---------------------------------------------------------------------------
// Structural validation — fail-closed. A parse failure OR a structurally
// invalid nested field must never silently reset a run; the caller decides
// (resume() falls back to checkpoint/backup/legacy, surfacing `corrupt` only
// when nothing recoverable exists).
// ---------------------------------------------------------------------------

const ALL_STAGES: Stage[] = [
  "clarify",
  "research",
  "plan",
  "plan_review",
  "implement",
  "verify",
  "review",
  "uat",
  "closeout",
  "done",
  "blocked",
  "aborted",
];

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}
function isString(v: unknown): v is string {
  return typeof v === "string";
}
function isNumber(v: unknown): v is number {
  return typeof v === "number" && Number.isFinite(v);
}
function isBoolean(v: unknown): v is boolean {
  return typeof v === "boolean";
}
function isStringArray(v: unknown): v is string[] {
  return Array.isArray(v) && v.every(isString);
}
function isArrayOf<T>(pred: (v: unknown) => v is T): (v: unknown) => v is T[] {
  return (v: unknown): v is T[] => Array.isArray(v) && v.every(pred);
}
function optionalField<T>(v: unknown, pred: (v: unknown) => v is T): v is T | undefined {
  return v === undefined || pred(v);
}
function isEnum<T extends string>(values: readonly T[]): (v: unknown) => v is T {
  const lookup: Record<string, true> = {};
  for (const value of values) lookup[value] = true;
  return (v: unknown): v is T => typeof v === "string" && lookup[v] === true;
}

const isStage = isEnum(ALL_STAGES);
const isMode = isEnum(["assisted", "auto"] as const);
const isRunStatus = isEnum(["running", "awaiting_user", "blocked", "done", "aborted"] as const);
const isItemStatus = isEnum(["pending", "running", "passed", "failed", "blocked", "skipped"] as const);
const isRequirementKind = isEnum(["functional", "nonfunctional", "constraint", "nongoal"] as const);
const isRequirementStatus = isEnum(["open", "confirmed", "needs_confirmation"] as const);
const isAcMethod = isEnum(["verify", "review", "uat", "manual"] as const);
const isEvidenceKind = isEnum(["verify", "review", "browser", "smoke", "api", "note"] as const);
const isModelRole = isEnum([
  "coordinator",
  "research",
  "plan",
  "implement",
  "critic",
  "verify",
  "review",
  "security",
] as const);
const isThinkingLevel = isEnum(["minimal", "low", "medium", "high", "xhigh"] as const);
const isGsVerdict = isEnum(["pass", "needs_fix", "fail", "correct", "incorrect"] as const);
const isApprovalWhat = isEnum(["requirements", "plan", "uat", "action"] as const);
const isApprovalBy = isEnum(["user", "critic"] as const);
const isPendingKind = isEnum(["agent", "verify", "user"] as const);
const isGateCode = isEnum([
  "OK",
  "UNRESOLVED_QUESTION",
  "AC_NOT_OBSERVABLE",
  "REQUIREMENTS_UNAPPROVED",
  "RESEARCH_MISSING",
  "PLANNER_MISSING",
  "AC_COVERAGE_INCOMPLETE",
  "REQ_COVERAGE_INCOMPLETE",
  "DEP_CYCLE",
  "OWNERSHIP_OVERLAP",
  "SHELL_VERIFY_COMMAND",
  "CRITIC_MISSING",
  "CRITIC_NOT_INDEPENDENT",
  "CRITIC_NEEDS_FIX",
  "PLAN_UNAPPROVED",
  "TASKS_INCOMPLETE",
  "TASK_UNVERIFIED",
  "VERIFY_FAILED",
  "VERIFIER_MISSING",
  "REVIEW_MISSING",
  "REVIEW_NOT_INDEPENDENT",
  "SECURITY_REVIEW_MISSING",
  "REVIEW_FOUND_BLOCKER",
  "AC_EVIDENCE_MISSING",
  "UAT_UNAPPROVED",
  "VERIFY_GATE_UNMET",
  "DESTRUCTIVE_UNAPPROVED",
  "MODEL_UNRESOLVABLE",
  "REVISION_MISMATCH",
  "WRONG_STAGE",
] as const);

function isModelEvidence(v: unknown): v is ModelEvidence {
  if (!isRecord(v)) return false;
  return (
    isString(v.id) &&
    isString(v.provider) &&
    isString(v.model) &&
    isString(v.family) &&
    isString(v.requestedSelector) &&
    isBoolean(v.isFallback) &&
    optionalField(v.thinking, isThinkingLevel)
  );
}

function isRequirement(v: unknown): v is Requirement {
  if (!isRecord(v)) return false;
  return isString(v.id) && isString(v.text) && isRequirementKind(v.kind) && isBoolean(v.required) && isRequirementStatus(v.status);
}

function isAcceptanceEvidence(v: unknown): v is AcceptanceEvidence {
  if (!isRecord(v)) return false;
  return isEvidenceKind(v.kind) && isString(v.ref) && isString(v.at) && optionalField(v.detail, isString);
}

function isAcceptanceCriterion(v: unknown): v is AcceptanceCriterion {
  if (!isRecord(v)) return false;
  return (
    isString(v.id) &&
    isString(v.text) &&
    isStringArray(v.requirements) &&
    isAcMethod(v.method) &&
    isBoolean(v.required) &&
    isItemStatus(v.status) &&
    isArrayOf(isAcceptanceEvidence)(v.evidence)
  );
}

function isRiskAssessment(v: unknown): v is RiskAssessment {
  if (!isRecord(v)) return false;
  return (
    isBoolean(v.trivial) &&
    isBoolean(v.externalUnknown) &&
    isBoolean(v.newArchitecture) &&
    isBoolean(v.security) &&
    isBoolean(v.destructive) &&
    isBoolean(v.outwardEffects) &&
    isStringArray(v.notes)
  );
}

function isVerifyCommand(v: unknown): v is VerifyCommand {
  if (!isRecord(v)) return false;
  return (
    isString(v.program) &&
    v.program.length > 0 &&
    isStringArray(v.args) &&
    optionalField(v.cwd, isString) &&
    optionalField(v.timeoutSeconds, isNumber)
  );
}

function isPlanTask(v: unknown): v is PlanTask {
  if (!isRecord(v)) return false;
  return (
    isString(v.id) &&
    isString(v.title) &&
    isStringArray(v.acceptance) &&
    isStringArray(v.ownership) &&
    isStringArray(v.dependsOn) &&
    isStringArray(v.skills) &&
    isArrayOf(isVerifyCommand)(v.verify) &&
    isItemStatus(v.status) &&
    isNumber(v.attempts) &&
    isNumber(v.failStreak) &&
    isString(v.lastFailureHash) &&
    isString(v.note)
  );
}

function isPlanSlice(v: unknown): v is PlanSlice {
  if (!isRecord(v)) return false;
  return isString(v.id) && isString(v.title) && isArrayOf(isPlanTask)(v.tasks);
}

function isPlan(v: unknown): v is Plan {
  if (!isRecord(v)) return false;
  return isString(v.hash) && isString(v.createdAt) && isArrayOf(isPlanSlice)(v.slices);
}

function isGateFinding(v: unknown): v is GateFinding {
  if (!isRecord(v)) return false;
  return isGateCode(v.code) && isString(v.message);
}

function isGateResult(v: unknown): v is GateResult {
  if (!isRecord(v)) return false;
  return isBoolean(v.ok) && isStage(v.stage) && isArrayOf(isGateFinding)(v.findings) && isString(v.checkedAt);
}

function isVerifyEvidence(v: unknown): v is VerifyEvidence {
  if (!isRecord(v)) return false;
  return (
    isVerifyCommand(v.command) &&
    isNumber(v.exitCode) &&
    isString(v.stdoutHash) &&
    isString(v.stderrHash) &&
    isNumber(v.durationMs) &&
    isString(v.finishedAt) &&
    isString(v.output)
  );
}

function isAgentRunEvidence(v: unknown): v is AgentRunEvidence {
  if (!isRecord(v)) return false;
  return (
    isString(v.toolCallId) &&
    isGsAgent(v.agent) &&
    optionalField(v.batchToolCallId, isString) &&
    optionalField(v.taskId, isString) &&
    isString(v.assignmentHash) &&
    isString(v.requestedSelector) &&
    isString(v.resolvedModel) &&
    optionalField(v.resolvedModelFamily, isString) &&
    isBoolean(v.resolvedModelIsFallback) &&
    isNumber(v.exitCode) &&
    optionalField(v.outputPath, isString) &&
    optionalField(v.startedAt, isString) &&
    isString(v.endedAt) &&
    optionalField(v.verdict, isGsVerdict) &&
    optionalField(v.blockingFindings, isNumber)
    // structuredOutput is intentionally `unknown` — no shape to enforce.
  );
}

function isStageRecord(v: unknown): v is StageRecord {
  if (!isRecord(v)) return false;
  return (
    isItemStatus(v.status) &&
    isNumber(v.attempts) &&
    optionalField(v.requiredAgent, isGsAgent) &&
    optionalField(v.modelRole, isModelRole) &&
    isArrayOf(isAgentRunEvidence)(v.agentRuns) &&
    isArrayOf(isVerifyEvidence)(v.verify) &&
    optionalField(v.gate, isGateResult) &&
    optionalField(v.startedAt, isString) &&
    optionalField(v.endedAt, isString)
  );
}

function isApproval(v: unknown): v is Approval {
  if (!isRecord(v)) return false;
  return isApprovalWhat(v.what) && isApprovalBy(v.by) && isString(v.at) && optionalField(v.note, isString);
}

function isGsWorktree(v: unknown): v is NonNullable<GsState["worktrees"]>[number] {
  if (!isRecord(v)) return false;
  return isString(v.slug) && isString(v.path) && isString(v.branch) && isString(v.createdAt);
}

function isPendingAction(v: unknown): v is PendingAction {
  if (!isRecord(v)) return false;
  return (
    isString(v.id) &&
    isStage(v.stage) &&
    isPendingKind(v.kind) &&
    optionalField(v.agent, isGsAgent) &&
    optionalField(v.modelRole, isModelRole) &&
    optionalField(v.modelSelector, isString) &&
    optionalField(v.modelId, isString) &&
    optionalField(v.modelFamily, isString) &&
    optionalField(v.assignmentHash, isString) &&
    optionalField(v.taskIds, isStringArray) &&
    optionalField(v.toolCallId, isString) &&
    optionalField(v.leaseRevision, isNumber) &&
    optionalField(v.boundRevision, isNumber) &&
    isString(v.instruction) &&
    isString(v.createdAt)
  );
}

function isAuditEvent(v: unknown): v is AuditEvent {
  if (!isRecord(v)) return false;
  return isString(v.at) && isStage(v.stage) && isString(v.kind) && isString(v.detail) && optionalField(v.eventId, isString);
}

function isLegacyImport(v: unknown): v is LegacyImport {
  if (!isRecord(v)) return false;
  return isString(v.importedAt) && isString(v.sourcePath) && isString(v.goal) && isNumber(v.taskCount) && isNumber(v.verifiedCount);
}

/** Structural validation — a parse failure OR any invalid nested field must never silently reset a run. */
export function validateState(obj: unknown): GsState | undefined {
  if (!isRecord(obj)) return undefined;
  if (obj.schemaVersion !== GS_SCHEMA_VERSION) return undefined;
  if (!isString(obj.runId) || !isString(obj.slug) || !isString(obj.goal) || !isString(obj.projectRoot)) return undefined;
  if (!isMode(obj.mode) || !isStage(obj.stage) || !isRunStatus(obj.status)) return undefined;
  if (!isNumber(obj.revision) || !isString(obj.createdAt) || !isString(obj.updatedAt)) return undefined;
  if (!optionalField(obj.originalCoordinator, isModelEvidence)) return undefined;
  if (!optionalField(obj.activeCoordinator, isModelEvidence)) return undefined;
  if (!isArrayOf(isRequirement)(obj.requirements)) return undefined;
  if (!isArrayOf(isAcceptanceCriterion)(obj.acceptance)) return undefined;
  if (!isRiskAssessment(obj.risk)) return undefined;
  if (!optionalField(obj.plan, isPlan)) return undefined;
  if (!isRecord(obj.stages)) return undefined;
  for (const stage of ALL_STAGES) {
    if (!isStageRecord(obj.stages[stage])) return undefined;
  }
  if (!isArrayOf(isApproval)(obj.approvals)) return undefined;
  if (!optionalField(obj.pendingAction, isPendingAction)) return undefined;
  if (!isArrayOf(isAuditEvent)(obj.audit)) return undefined;
  if (!optionalField(obj.legacy, isLegacyImport)) return undefined;
  if (!optionalField(obj.worktrees, isArrayOf(isGsWorktree))) return undefined;
  return obj as unknown as GsState;
}

function readJson(file: string): unknown | undefined {
  if (!existsSync(file)) return undefined;
  try {
    return JSON.parse(readFileSync(file, "utf8"));
  } catch {
    return undefined;
  }
}

export function saveRuntime(root: string, state: GsState): void {
  atomicWrite(gsPaths(root, state.slug).cacheFile, JSON.stringify(state, null, 2), root);
}

export function loadRuntime(root: string): GsState | undefined {
  return validateState(readJson(join(root, CACHE_REL)));
}

/** Write the durable checkpoint + Markdown projections + ACTIVE pointer + managed STATE.md block. */
export function saveCheckpoint(root: string, state: GsState): void {
  const p = gsPaths(root, state.slug);
  atomicWrite(p.checkpoint, JSON.stringify(state, null, 2), root);
  atomicWrite(p.requirementsMd, renderRequirementsMd(state), root);
  atomicWrite(p.planMd, renderPlanMd(state), root);
  atomicWrite(p.verificationMd, renderVerificationMd(state), root);
  writeActive(root, state.slug);
  writeStateMd(root, state);
}

export function writeActive(root: string, slug: string): void {
  atomicWrite(join(root, ACTIVE_REL), `${slug}\n`, root);
}

export function readActive(root: string): string | undefined {
  const raw = readJson(join(root, ACTIVE_REL));
  if (typeof raw === "string") return raw.trim();
  // ACTIVE is plain text, not JSON — read directly.
  const file = join(root, ACTIVE_REL);
  if (!existsSync(file)) return undefined;
  const txt = readFileSync(file, "utf8").trim();
  return txt.length ? txt : undefined;
}

export interface ResumeResult {
  state: GsState;
  source: "cache" | "checkpoint" | "backup" | "legacy";
}

export interface ResumeDiagnostic {
  state?: undefined;
  source: "corrupt" | "none";
  detail: string;
}

/**
 * Resume the active run. Order: runtime cache → durable checkpoint → cache .bak
 * → legacy engine import. An invalid primary (parse failure OR any structural
 * validation failure) never suppresses checkpoint/backup recovery; only when
 * NONE of cache/checkpoint/backup/legacy recover does a present-but-invalid
 * cache surface as a `corrupt` diagnostic — the caller must never reset.
 */
export function resume(root: string): ResumeResult | ResumeDiagnostic {
  const cacheFile = join(root, CACHE_REL);
  const cacheExists = existsSync(cacheFile);
  const cacheState = validateState(readJson(cacheFile));
  if (cacheState) return { state: cacheState, source: "cache" };
  const cacheInvalid = cacheExists; // present but failed validateState (parse or structural)

  const slug = readActive(root);
  if (slug) {
    const cp = validateState(readJson(gsPaths(root, slug).checkpoint));
    if (cp) return { state: cp, source: "checkpoint" };
  }

  const bak = validateState(readJson(`${cacheFile}.bak`));
  if (bak) return { state: bak, source: "backup" };

  if (existsSync(join(root, LEGACY_REL))) {
    const imported = importLegacyEngine(root);
    if (imported) return { state: imported, source: "legacy" };
  }

  if (cacheInvalid) {
    return {
      source: "corrupt",
      detail: `${CACHE_REL} is present but invalid (parse or structural validation failure) and no valid checkpoint/backup exists`,
    };
  }
  return { source: "none", detail: "no active GS run" };
}

/** Archive the active run: clear the ACTIVE pointer + runtime cache. Leaves the legacy import marker intact. */
export function clearActive(root: string): void {
  try {
    rmSync(join(root, ACTIVE_REL), { force: true });
  } catch {
    /* ignore */
  }
  try {
    rmSync(join(root, CACHE_REL), { force: true });
  } catch {
    /* ignore */
  }
}

// ---------------------------------------------------------------------------
// Legacy engine import (one-time per source content, non-destructive)
// ---------------------------------------------------------------------------

interface LegacyTask {
  id: string;
  title: string;
  status: string;
  retries?: number;
  verified?: boolean;
  verify?: string[];
}
interface LegacySlice {
  id: string;
  title: string;
  tasks: LegacyTask[];
}
interface LegacyState {
  goal: string;
  slices: LegacySlice[];
}

interface LegacyImportMarker {
  sourceHash: string;
  importedAt: string;
  slug: string;
}

function isLegacyImportMarker(v: unknown): v is LegacyImportMarker {
  if (!isRecord(v)) return false;
  return isString(v.sourceHash) && isString(v.importedAt) && isString(v.slug);
}

/**
 * Import the legacy 8sync-engine state once per distinct source content. The
 * original is preserved (copied to legacy-engine-state.json for rollback);
 * verified tasks become passed implementation tasks, everything else stays
 * non-passed. Requirements/ACs are synthesized as needs_confirmation
 * placeholders — never PASS. Resumes at clarify so the critic/review/UAT
 * gates still run.
 *
 * A provenance marker (content hash of the legacy source) is written after a
 * successful import and survives `clearActive`. If the marker already
 * matches the current legacy source's hash, the import is skipped — the same
 * legacy content is never re-imported after its GS run is archived/reset,
 * which would otherwise recreate the run forever. Legacy content that
 * genuinely changes (a new engine run) hashes differently and imports again.
 */
export function importLegacyEngine(root: string, now = new Date().toISOString()): GsState | undefined {
  const legacyFile = join(root, LEGACY_REL);
  let rawLegacy: string;
  try {
    rawLegacy = readFileSync(legacyFile, "utf8");
  } catch {
    return undefined;
  }
  let raw: unknown;
  try {
    raw = JSON.parse(rawLegacy);
  } catch {
    return undefined;
  }
  if (typeof raw !== "object" || raw === null) return undefined;
  const legacy = raw as LegacyState;
  if (typeof legacy.goal !== "string" || !Array.isArray(legacy.slices)) return undefined;

  const sourceHash = fnv1a(rawLegacy);
  const marker = readJson(join(root, LEGACY_IMPORT_MARKER_REL));
  if (isLegacyImportMarker(marker) && marker.sourceHash === sourceHash) {
    // Already imported this exact legacy content (possibly since archived) —
    // never re-import it; the original rollback copy from that import stands.
    return undefined;
  }

  // Preserve the original for rollback. Uses the guarded atomic writer, not
  // copyFileSync, so the destination can't be redirected through a symlink.
  try {
    atomicWrite(join(root, LEGACY_COPY_REL), rawLegacy, root);
  } catch {
    /* best effort */
  }

  const slug = "imported-engine-run";
  const state = createRun({ runId: `legacy-${Date.now()}`, slug, goal: legacy.goal, projectRoot: root, mode: "assisted", now });

  let taskCount = 0;
  let verifiedCount = 0;
  state.plan = {
    hash: "legacy-import",
    createdAt: now,
    slices: legacy.slices.map((s) => ({
      id: s.id,
      title: s.title,
      tasks: (s.tasks ?? []).map((t): PlanTask => {
        taskCount += 1;
        const verified = t.status === "done" && t.verified === true;
        if (verified) verifiedCount += 1;
        return {
          id: t.id,
          title: t.title,
          acceptance: [],
          ownership: [],
          dependsOn: [],
          skills: [],
          verify: (t.verify ?? []).map((cmd) => ({ program: "bash", args: ["-lc", cmd] })),
          status: verified ? "passed" : "pending",
          attempts: t.retries ?? 0,
          failStreak: 0,
          lastFailureHash: "",
          note: verified ? "imported: verified" : "imported: unverified",
        };
      }),
    })),
  };

  state.requirements = [
    { id: "R-IMPORT", text: legacy.goal, kind: "functional", required: true, status: "needs_confirmation" },
  ];
  state.acceptance = [];
  state.legacy = { importedAt: now, sourcePath: LEGACY_REL, goal: legacy.goal, taskCount, verifiedCount };
  state.stage = "clarify";
  state.audit.push({ at: now, stage: "clarify", kind: "legacy_import", detail: `${taskCount} tasks, ${verifiedCount} verified` });
  saveRuntime(root, state);

  const marker2: LegacyImportMarker = { sourceHash, importedAt: now, slug };
  atomicWrite(join(root, LEGACY_IMPORT_MARKER_REL), JSON.stringify(marker2, null, 2), root);

  return state;
}

// ---------------------------------------------------------------------------
// Markdown projections (pure)
// ---------------------------------------------------------------------------

export function renderRequirementsMd(state: GsState): string {
  const lines = [`# Requirements — ${state.slug}`, "", `Goal: ${state.goal}`, ""];
  lines.push("## Requirements");
  for (const r of state.requirements) {
    lines.push(`- **${r.id}** (${r.kind}${r.required ? ", required" : ""}, ${r.status}): ${r.text}`);
  }
  lines.push("", "## Acceptance criteria");
  for (const a of state.acceptance) {
    lines.push(`- **${a.id}** [${a.status}] via ${a.method} → covers ${a.requirements.join(", ") || "—"}: ${a.text}`);
  }
  return `${lines.join("\n")}\n`;
}

export function renderPlanMd(state: GsState): string {
  const lines = [`# Plan — ${state.slug}`, ""];
  if (!state.plan) {
    lines.push("_No plan yet._");
    return `${lines.join("\n")}\n`;
  }
  lines.push(`Plan hash: \`${state.plan.hash}\``, "");
  for (const s of state.plan.slices) {
    lines.push(`## ${s.id} — ${s.title}`);
    for (const t of s.tasks) {
      const verify = t.verify.map((v) => `${v.program} ${v.args.join(" ")}`).join(" · ") || "—";
      lines.push(
        `- [${statusMark(t.status)}] **${t.id}** ${t.title} — AC ${t.acceptance.join(",") || "—"} · owns ${t.ownership.join(",") || "—"} · deps ${t.dependsOn.join(",") || "—"} · verify \`${verify}\``,
      );
    }
    lines.push("");
  }
  return `${lines.join("\n")}\n`;
}

export function renderVerificationMd(state: GsState): string {
  const lines = [`# Verification — ${state.slug}`, "", `Stage: ${state.stage} · status: ${state.status}`, "", "## AC matrix"];
  for (const a of requiredAccept(state)) {
    const ev = a.evidence.map((e) => `${e.kind}:${e.ref}`).join(", ") || "—";
    lines.push(`- **${a.id}** [${a.status.toUpperCase()}] — ${a.text} — evidence: ${ev}`);
  }
  lines.push("", "## Model evidence");
  for (const stage of Object.keys(state.stages) as Stage[]) {
    for (const run of state.stages[stage].agentRuns) {
      lines.push(
        `- ${stage} · ${run.agent} · ${run.resolvedModel}${run.resolvedModelIsFallback ? " (fallback)" : ""} · verdict ${run.verdict ?? "—"}`,
      );
    }
  }
  return `${lines.join("\n")}\n`;
}

function requiredAccept(state: GsState): AcceptanceCriterion[] {
  return state.acceptance.filter((a) => a.required);
}

function statusMark(status: PlanTask["status"]): string {
  return status === "passed" ? "x" : status === "blocked" ? "!" : status === "skipped" ? "-" : " ";
}

// STATE.md managed section --------------------------------------------------

export const STATE_BEGIN = "<!-- 8sync:gs:begin -->";
export const STATE_END = "<!-- 8sync:gs:end -->";

/** Compact GS block for su-code/STATE.md (goal, stage, AC progress, next). */
export function renderStateSection(state: GsState, nextAction: string): string {
  const ac = requiredAccept(state);
  const passed = ac.filter((a) => a.status === "passed").length;
  return [
    STATE_BEGIN,
    `## GS run: ${state.slug}`,
    `- Goal: ${state.goal}`,
    `- Stage: ${state.stage} (${state.status})`,
    `- AC: ${passed}/${ac.length} passed`,
    `- Blocker: ${state.status === "blocked" ? state.audit[state.audit.length - 1]?.detail ?? "—" : "none"}`,
    `- Next: ${nextAction}`,
    STATE_END,
  ].join("\n");
}

/** Upsert the managed GS block into an existing STATE.md body. */
export function upsertStateSection(body: string, section: string): string {
  const start = body.indexOf(STATE_BEGIN);
  const end = body.indexOf(STATE_END);
  if (start >= 0 && end > start) {
    return `${body.slice(0, start)}${section}${body.slice(end + STATE_END.length)}`;
  }
  const sep2 = body.endsWith("\n") || body.length === 0 ? "" : "\n";
  return `${body}${sep2}\n${section}\n`;
}

function nextActionText(state: GsState): string {
  if (state.status === "done") return "done — run complete";
  if (state.status === "blocked") return state.pendingAction?.instruction || "blocked — see audit for detail";
  return state.pendingAction?.instruction || "call gs_next";
}

/**
 * Upsert the compact GS status block into project `su-code/STATE.md`, using
 * the same managed-section helpers as `renderStateSection`/`upsertStateSection`
 * above. Every byte outside the `STATE_BEGIN`/`STATE_END` sentinel — including
 * a STATE.md that predates any GS run — is preserved verbatim.
 */
export function writeStateMd(root: string, state: GsState): void {
  const file = join(root, STATE_MD_REL);
  const body = existsSync(file) ? readFileSync(file, "utf8") : "";
  const section = renderStateSection(state, nextActionText(state));
  atomicWrite(file, upsertStateSection(body, section), root);
}
