// 8sync-gs — parse omp `task` tool results into AgentRunEvidence. Records the
// OBSERVED agent + resolved model, never model prose. Verdict/blocking-finding
// normalization reads the structured output the agent yielded under its schema.

import type { AgentRunEvidence, GsAgent, GsVerdict } from "./types.ts";
import { fnv1a } from "./machine.ts";

const GS_AGENTS: GsAgent[] = [
  "gs-researcher",
  "gs-planner",
  "gs-critic",
  "gs-worker",
  "gs-verifier",
  "gs-reviewer",
  "gs-security",
];

export function isGsAgent(name: unknown): name is GsAgent {
  return typeof name === "string" && (GS_AGENTS as string[]).includes(name);
}

/** Shape of a single omp subagent result (subset we consume). */
export interface SingleResultLike {
  id?: string;
  agent?: string;
  agentSource?: unknown;
  exitCode?: number;
  aborted?: boolean;
  resolvedModel?: string;
  resolvedModelIsFallback?: boolean;
  structuredOutput?: unknown;
  outputPath?: string;
}

export interface TaskDetailsLike {
  results?: SingleResultLike[];
}

/** Stable assignment fingerprint so recorded evidence can match one exact lease. */
export function assignmentHash(input: {
  agent: string;
  taskIds?: string[];
  instruction?: string;
  modelId?: string;
  revision?: number;
  planHash?: string;
  taskSpecs?: Array<{ id: string; acceptance: string[]; ownership: string[] }>;
}): string {
  const specs = input.taskSpecs?.map((task) => ({
    id: task.id,
    acceptance: [...task.acceptance],
    ownership: [...task.ownership],
  }));
  return fnv1a(
    JSON.stringify({
      agent: input.agent,
      taskIds: input.taskIds ?? [],
      instruction: input.instruction ?? "",
      modelId: input.modelId ?? "",
      revision: input.revision ?? -1,
      planHash: input.planHash ?? "",
      taskSpecs: specs ?? [],
    }),
  );
}

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function structuredRecord(structured: unknown): Record<string, unknown> | undefined {
  if (!isRecord(structured)) return undefined;
  if (isRecord(structured.data)) return structured.data;
  return structured;
}

/** Extract the planned task ID from a worker's structured result. */
export function structuredTaskId(structured: unknown): string | undefined {
  const record = structuredRecord(structured);
  if (!record) return undefined;
  const value = record.task_id ?? record.taskId;
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

/**
 * Normalize a specialist verdict from structured output. Understands the
 * reviewer schema (`overall_correctness`), an explicit `verdict`, and a
 * `status`/`pass` boolean. Returns undefined when nothing is present.
 */
export function normalizeVerdict(structured: unknown): GsVerdict | undefined {
  if (!isRecord(structured)) return undefined;
  const oc = structured.overall_correctness;
  if (oc === "correct" || oc === "incorrect") return oc;
  const v = structured.verdict;
  if (typeof v === "string") {
    const low = v.toLowerCase();
    if (low === "pass" || low === "passed" || low === "approve" || low === "approved") return "pass";
    if (low === "needs_fix" || low === "needs-fix" || low === "changes" || low === "revise") return "needs_fix";
    if (low === "fail" || low === "failed" || low === "reject" || low === "rejected") return "fail";
    if (low === "correct" || low === "incorrect") return low as GsVerdict;
  }
  const status = structured.status;
  if (typeof status === "string") {
    const low = status.toLowerCase();
    if (low === "pass" || low === "passed") return "pass";
    if (low === "fail" || low === "failed") return "fail";
  }
  if (typeof structured.pass === "boolean") return structured.pass ? "pass" : "fail";
  return undefined;
}

/** Count P0/P1 findings (priority <= 1) the agent reported. */
export function countBlockingFindings(structured: unknown): number {
  if (!isRecord(structured)) return 0;
  const findings = structured.findings;
  if (!Array.isArray(findings)) return 0;
  let n = 0;
  for (const f of findings) {
    if (isRecord(f) && typeof f.priority === "number" && f.priority <= 1) n += 1;
  }
  return n;
}

export interface BuildEvidenceInput {
  /** Composite per-result identity used for idempotent machine recording. */
  toolCallId: string;
  /** Original task batch call ID, used to prove the pending lease binding. */
  batchToolCallId?: string;
  result: SingleResultLike;
  assignmentHash: string;
  requestedSelector: string;
  /** Family lineage of the resolved model, from the omp adapter's family(). */
  resolvedModelFamily?: string;
  now?: string;
}

/** Build one AgentRunEvidence from an observed subagent result. */
export function buildAgentEvidence(input: BuildEvidenceInput): AgentRunEvidence | undefined {
  const r = input.result;
  if (!isGsAgent(r.agent)) return undefined;
  return {
    toolCallId: input.toolCallId,
    batchToolCallId: input.batchToolCallId ?? input.toolCallId,
    agent: r.agent,
    taskId: structuredTaskId(r.structuredOutput),
    assignmentHash: input.assignmentHash,
    requestedSelector: input.requestedSelector,
    resolvedModel: r.resolvedModel ?? "unknown",
    resolvedModelFamily: input.resolvedModelFamily,
    resolvedModelIsFallback: r.resolvedModelIsFallback ?? false,
    exitCode: r.aborted ? 1 : (r.exitCode ?? 0),
    structuredOutput: r.structuredOutput,
    outputPath: r.outputPath,
    endedAt: input.now ?? new Date().toISOString(),
    verdict: normalizeVerdict(r.structuredOutput),
    blockingFindings: countBlockingFindings(r.structuredOutput),
  };
}

/**
 * Extract every gs-agent result from a task tool `details` payload. Non-gs
 * agents are ignored. The caller matches each against its outstanding pending
 * action via assignmentHash before recording.
 */
export function evidenceFromTaskDetails(
  toolCallId: string,
  details: TaskDetailsLike,
  ctx: { assignmentHash: string; requestedSelector: string; familyOf?: (model: string) => string | undefined; now?: string },
): AgentRunEvidence[] {
  const results = Array.isArray(details.results) ? details.results : [];
  const out: AgentRunEvidence[] = [];
  for (const [index, result] of results.entries()) {
    const taskId = structuredTaskId(result.structuredOutput);
    const resultId = typeof result.id === "string" && result.id.length > 0 ? result.id : taskId ?? String(index);
    const ev = buildAgentEvidence({
      toolCallId: `${toolCallId}:${resultId}`,
      batchToolCallId: toolCallId,
      result,
      assignmentHash: ctx.assignmentHash,
      requestedSelector: ctx.requestedSelector,
      resolvedModelFamily: result.resolvedModel ? ctx.familyOf?.(result.resolvedModel) : undefined,
      now: ctx.now,
    });
    if (ev) out.push(ev);
  }
  return out;
}
