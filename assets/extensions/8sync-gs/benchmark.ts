// 8sync-gs — deterministic contract benchmark. Each scenario asserts the NEW
// pure GS control plane (machine + policy + store) against a documented "legacy
// engine" expectation. It proves control-plane CORRECTNESS (gates fire, models
// stay independent, evidence is required, state recovers) — NOT model quality.
// Live model A/B lives in scripts/gs-live-bench.ts.

import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DEFAULT_CONFIG } from "./config.ts";
import {
  advance,
  applyVerify,
  attachAcceptanceEvidence,
  createRun,
  defineRequirements,
  emptyRisk,
  evaluateGate,
  planApproved,
  recordAgentRun,
  recordApproval,
  setPlan,
} from "./machine.ts";
import { agentsForStage, classifyBashCommand, selectStageModel, type ResolvedModel } from "./policy.ts";
import { CACHE_REL, resume, saveRuntime, validateState } from "./store.ts";
import type {
  AcceptanceCriterion,
  AgentRunEvidence,
  GsState,
  Plan,
  Requirement,
  RiskAssessment,
} from "./types.ts";

const cfg = DEFAULT_CONFIG;

// legacy expectation column is documentation only; escapes are scored vs GS.
export interface ScenarioResult {
  name: string;
  legacyExpected: string;
  gsExpected: "pass" | "block";
  gsActual: "pass" | "block";
  ok: boolean;
  critical: boolean; // a "block" GS failed to block = critical escape
}

export interface BenchmarkReport {
  pass: boolean;
  criticalEscapes: number;
  falseBlocks: number;
  scenarios: ScenarioResult[];
  summary: string;
  detail: string;
}

// ---- builders (mirror the unit-test fixtures) ------------------------------
function req(id: string): Requirement {
  return { id, text: `req ${id}`, kind: "functional", required: true, status: "confirmed" };
}
function ac(id: string, reqs: string[]): AcceptanceCriterion {
  return { id, text: `ac ${id}`, requirements: reqs, method: "verify", required: true, status: "pending", evidence: [] };
}
function risk(over: Partial<RiskAssessment> = {}): RiskAssessment {
  return { ...emptyRisk(), ...over };
}
function run(over: Partial<AgentRunEvidence> & Pick<AgentRunEvidence, "agent">): AgentRunEvidence {
  return {
    toolCallId: over.toolCallId ?? `tc-${Math.random().toString(16).slice(2)}`,
    agent: over.agent,
    assignmentHash: "h",
    requestedSelector: "@x",
    resolvedModel: over.resolvedModel ?? "anthropic/opus",
    resolvedModelFamily: over.resolvedModelFamily ?? "claude",
    resolvedModelIsFallback: false,
    exitCode: over.exitCode ?? 0,
    endedAt: "t",
    verdict: over.verdict,
    blockingFindings: over.blockingFindings,
  };
}
function plan(): Plan {
  return {
    hash: "h",
    createdAt: "t",
    slices: [
      {
        id: "s1",
        title: "w1",
        tasks: [
          { id: "t1", title: "task 1", acceptance: ["AC1"], ownership: ["a"], dependsOn: [], skills: [], verify: [{ program: "true", args: [] }], status: "pending", attempts: 0, failStreak: 0, lastFailureHash: "", note: "" },
        ],
      },
    ],
  };
}
function seeded(mode: "assisted" | "auto" = "auto"): GsState {
  const s = createRun({ runId: "r", slug: "d", goal: "g", projectRoot: "/x", mode, now: "t" });
  defineRequirements(s, { requirements: [req("R1")], acceptance: [ac("AC1", ["R1"])], risk: risk({ trivial: true }) });
  return s;
}
/** Drive to plan_review PASS in auto mode, ready to enter implement. */
function toImplement(): GsState {
  const s = seeded("auto");
  advance(s, cfg); // plan
  setPlan(s, plan());
  recordAgentRun(s, "plan", run({ agent: "gs-planner", resolvedModelFamily: "claude" }));
  advance(s, cfg); // plan_review
  recordAgentRun(s, "plan_review", run({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
  advance(s, cfg); // implement
  return s;
}

const opus: ResolvedModel = { id: "anthropic/opus", provider: "anthropic", model: "opus", family: "claude" };

function verdictOf(state: GsState): "pass" | "block" {
  return evaluateGate(state, cfg).ok ? "pass" : "block";
}

type Case = { name: string; legacy: string; expect: "pass" | "block"; actual: () => "pass" | "block" };

function scenarios(): Case[] {
  return [
    {
      name: "happy path with verified task",
      legacy: "pass",
      expect: "pass",
      actual: () => {
        const s = toImplement();
        recordAgentRun(s, "implement", run({ agent: "gs-worker", resolvedModelFamily: "claude" }));
        applyVerify(s, "t1", true, "", [], cfg);
        return verdictOf(s);
      },
    },
    {
      name: "retry then changed fix",
      legacy: "pass",
      expect: "pass",
      actual: () => {
        const s = toImplement();
        applyVerify(s, "t1", false, "err A", [], cfg);
        applyVerify(s, "t1", true, "", [], cfg);
        recordAgentRun(s, "implement", run({ agent: "gs-worker" }));
        return verdictOf(s);
      },
    },
    {
      name: "advance unverified",
      legacy: "block",
      expect: "block",
      actual: () => verdictOf(toImplement()), // t1 still pending
    },
    {
      name: "plan has no AC mapping",
      legacy: "accept",
      expect: "block",
      actual: () => {
        const s = seeded("auto");
        advance(s, cfg);
        const p = plan();
        p.slices[0].tasks[0].acceptance = [];
        setPlan(s, p);
        recordAgentRun(s, "plan", run({ agent: "gs-planner" }));
        return verdictOf(s);
      },
    },
    {
      name: "edit before approved plan",
      legacy: "accept",
      expect: "block",
      actual: () => {
        const s = seeded("assisted");
        return planApproved(s) ? "pass" : "block";
      },
    },
    {
      name: "planner and critic resolve same model",
      legacy: "no check",
      expect: "block",
      actual: () => {
        const s = seeded("auto");
        advance(s, cfg);
        setPlan(s, plan());
        recordAgentRun(s, "plan", run({ agent: "gs-planner", resolvedModelFamily: "claude" }));
        advance(s, cfg);
        recordAgentRun(s, "plan_review", run({ agent: "gs-critic", resolvedModelFamily: "claude", verdict: "pass" }));
        return verdictOf(s);
      },
    },
    {
      name: "invented/wrong agent name",
      legacy: "no check",
      expect: "block",
      actual: () => {
        const s = toImplement();
        const allowed = agentsForStage(s.stage, s.risk) as string[];
        return allowed.includes("gs-hacker") ? "pass" : "block";
      },
    },
    {
      name: "all tasks done but review missing",
      legacy: "done",
      expect: "block",
      actual: () => {
        const s = toImplement();
        recordAgentRun(s, "implement", run({ agent: "gs-worker" }));
        applyVerify(s, "t1", true, "", [], cfg);
        advance(s, cfg); // verify
        recordAgentRun(s, "verify", run({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "pass" }));
        advance(s, cfg); // review
        return verdictOf(s); // no reviewer recorded
      },
    },
    {
      name: "all tasks done but UAT missing",
      legacy: "done",
      expect: "block",
      actual: () => {
        const s = fullToUat();
        attachAcceptanceEvidence(s, "AC1", { kind: "verify", ref: "t1", at: "t" });
        return verdictOf(s); // AC evidence present but no user UAT approval
      },
    },
    {
      name: "one required AC skipped/pending",
      legacy: "unaware",
      expect: "block",
      actual: () => {
        const s = fullToUat();
        recordApproval(s, { what: "uat", by: "user" });
        return verdictOf(s); // AC1 has no evidence → still blocks
      },
    },
    {
      name: "destructive/outward action unapproved",
      legacy: "no check",
      expect: "block",
      actual: () => {
        const s = toImplement();
        const cls = classifyBashCommand("git push origin main");
        const approved = s.approvals.some((a) => a.what === "uat");
        return cls.outward && !approved ? "block" : "pass";
      },
    },
    {
      name: "duplicate/stale task result",
      legacy: "may mutate",
      expect: "block",
      actual: () => {
        const s = seeded("auto");
        const r = run({ agent: "gs-planner", toolCallId: "dup" });
        const a = recordAgentRun(s, "plan", r);
        const b = recordAgentRun(s, "plan", r);
        // "block" here means the duplicate was rejected (not applied).
        return a.applied && !b.applied && s.stages.plan.agentRuns.length === 1 ? "block" : "pass";
      },
    },
    {
      name: "corrupt primary state with valid backup",
      legacy: "lose state",
      expect: "block",
      actual: () => {
        const root = mkdtempSync(join(tmpdir(), "gs-bench-"));
        try {
          const s = seeded("auto");
          s.projectRoot = root;
          saveRuntime(root, s);
          saveRuntime(root, s); // rotate a valid .bak
          writeFileSync(join(root, CACHE_REL), "{ corrupt");
          const r = resume(root);
          // "block" = recovered from backup (did NOT reset to null).
          return r.source === "backup" && !!validateState(r.state) ? "block" : "pass";
        } finally {
          rmSync(root, { recursive: true, force: true });
        }
      },
    },
    {
      name: "fallback causes model collision",
      legacy: "no check",
      expect: "block",
      actual: () => {
        const sel = selectStageModel(
          "critic",
          cfg,
          (s) => (s === "family:gpt" || s === "@slow" || s === "@default" ? opus : undefined),
          { excludeFamilies: ["claude"] },
        );
        return sel.finding?.code === "MODEL_UNRESOLVABLE" ? "block" : "pass";
      },
    },
  ];
}

/** Drive a full run to the uat stage (no AC evidence / approval yet). */
function fullToUat(): GsState {
  const s = toImplement();
  recordAgentRun(s, "implement", run({ agent: "gs-worker", resolvedModelFamily: "claude" }));
  applyVerify(s, "t1", true, "", [], cfg);
  advance(s, cfg); // verify
  recordAgentRun(s, "verify", run({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "pass" }));
  advance(s, cfg); // review
  recordAgentRun(s, "review", run({ agent: "gs-reviewer", resolvedModelFamily: "gpt", verdict: "correct", blockingFindings: 0 }));
  advance(s, cfg); // uat
  return s;
}

export function runContractBenchmark(): BenchmarkReport {
  const results: ScenarioResult[] = [];
  for (const c of scenarios()) {
    let actual: "pass" | "block";
    try {
      actual = c.actual();
    } catch {
      actual = c.expect === "pass" ? "block" : "pass"; // a throw counts as a mismatch
    }
    const ok = actual === c.expect;
    results.push({
      name: c.name,
      legacyExpected: c.legacy,
      gsExpected: c.expect,
      gsActual: actual,
      ok,
      critical: c.expect === "block" && actual === "pass",
    });
  }
  const criticalEscapes = results.filter((r) => r.critical).length;
  const falseBlocks = results.filter((r) => r.gsExpected === "pass" && r.gsActual === "block").length;
  const pass = criticalEscapes === 0 && falseBlocks === 0 && results.every((r) => r.ok);
  const detail = results
    .map((r) => `${r.ok ? "OK  " : "FAIL"} ${r.name}: legacy=${r.legacyExpected} gs-expect=${r.gsExpected} gs-actual=${r.gsActual}`)
    .join("\n");
  const summary = pass
    ? `PASS — ${results.length} scenarios, 0 critical escapes, 0 false blocks`
    : `FAIL — ${criticalEscapes} critical escapes, ${falseBlocks} false blocks of ${results.length}`;
  return { pass, criticalEscapes, falseBlocks, scenarios: results, summary, detail };
}
