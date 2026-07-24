// 8sync-gs — compact, stage-specific coordinator instructions. These drive the
// MAIN agent (the coordinator) turn by turn. They are deliberately short: the
// hard rules live in the machine + hooks, not in prose.

import type { GsState, PendingAction, Stage } from "./types.ts";

const COMMON = "Obey ~/.omp/agent/APPEND_SYSTEM.md (code-intel first; always-on skills). Never git push / open a PR unless the goal explicitly asked. Call gs_status any time.";

export function stageInstruction(state: GsState, pending: PendingAction | undefined): string {
  const stage: Stage = state.stage;
  switch (stage) {
    case "clarify":
      return [
        `GS clarify — goal: ${state.goal}`,
        "Ground with codegraph/codebase-memory/serena + su-code memory. Then call gs_define with:",
        "- requirements (functional/nonfunctional/constraint/nongoal, mark required)",
        "- observable acceptance criteria (each maps to >=1 requirement; method verify|review|uat)",
        "- a deterministic risk assessment (trivial? external unknown? new architecture? security? destructive/outward effects?).",
        state.mode === "assisted"
          ? "Then STOP and wait for the user to run `/gs approve requirements`."
          : "Then call gs_advance. (auto mode: no user gate here.)",
        COMMON,
      ].join("\n");
    case "research":
      return [
        "GS research — risk is nontrivial. Spawn the `gs-researcher` task (model per gs_next).",
        "Its structured output must carry findings, source URLs/files, constraints, open unknowns, confidence.",
        "Then call gs_advance.",
        COMMON,
      ].join("\n");
    case "plan":
      return [
        "GS plan — spawn exactly one `gs-planner` task (Opus by policy).",
        "Record its plan via gs_plan: slices→tasks, each task with acceptance IDs, file ownership, dependsOn, skills, and DIRECT-ARGV verify commands (program+args, never a shell string).",
        "Every required AC needs >=1 task; every AC maps to >=1 requirement. Then gs_advance.",
        COMMON,
      ].join("\n");
    case "plan_review":
      return [
        "GS plan_review — spawn `gs-critic` (a DIFFERENT model family from the planner; gs_next picks it).",
        "Critic returns a verdict (pass|needs_fix) + P0-P3 findings + missing ACs + unsafe commands.",
        "needs_fix → call gs_reject plan <reason> to reopen planning. pass →",
        state.mode === "assisted" ? "wait for `/gs approve plan`." : "gs_advance (auto: critic PASS is the gate).",
        COMMON,
      ].join("\n");
    case "implement":
      return [
        "GS implement — call gs_next for the next wave. Spawn `gs-worker` task(s) with the exact task IDs, allowed paths, AC IDs, and model from gs_next.",
        "Independent tasks in ONE task batch; dependent tasks wait. Workers supply evidence; they do not mark tasks done.",
        "After a wave lands, run gs_verify {taskId} for each task. All tasks passed → gs_advance.",
        COMMON,
      ].join("\n");
    case "verify":
      return [
        "GS verify — spawn `gs-verifier` to audit the diff + command evidence (independent model).",
        "It returns pass|fail. pass → gs_advance. fail → gs_reject implement <reason>.",
        COMMON,
      ].join("\n");
    case "review":
      return [
        "GS review — spawn `gs-reviewer` (model family distinct from the implementation model).",
        state.risk.security ? "Security-sensitive: ALSO spawn `gs-security`." : "",
        "P0/P1 or overall_correctness=incorrect → gs_reject implement <reason>. Clean → gs_advance.",
        COMMON,
      ].filter(Boolean).join("\n");
    case "uat":
      return [
        "GS uat — build the AC matrix. Attach objective evidence per AC via gs_acceptance:",
        "UI → browser evidence; CLI → a real command smoke run; API → an end-to-end request/response.",
        "Pending/blocked/skipped ACs block closure. When every required AC has PASS evidence, STOP and ask the user to run `/gs approve uat`.",
        COMMON,
      ].join("\n");
    case "closeout":
      return [
        "GS closeout — gs_advance performs the final gate. Commit locally only if all gates passed and gitleaks is clean.",
        "Update su-code CHANGELOG/KNOWLEDGE/STATE as cleanup. Original coordinator model is restored automatically.",
        COMMON,
      ].join("\n");
    default:
      return `GS ${stage}: ${state.status}. Call gs_status.`;
  }
}

export const HELP = [
  "/gs <goal>              start a bounded-autonomous run (assisted gates)",
  "/gs --auto <goal>       autonomous, independent-critic gates (still stops for UAT + danger)",
  "/gs                     continue the active run (or this help)",
  "/gs status              stage, evidence, blockers, next action",
  "/gs continue            resume a paused/awaiting run",
  "/gs approve requirements|plan|uat   human locks",
  "/gs reject <stage> <reason>          reopen a stage",
  "/gs pause               durable checkpoint + stop",
  "/gs abort               restore model, keep artifacts, mark aborted",
  "/gs reset --yes         archive the active run + clear the pointer",
  "/gs benchmark          run the deterministic contract benchmark",
].join("\n");
