import { describe, expect, test } from "bun:test";
import { DEFAULT_CONFIG } from "../../assets/extensions/8sync-gs/config.ts";
import {
  actionApproved,
  advance,
  applyVerify,
  assertRevision,
  attachAcceptanceEvidence,
  block,
  counts,
  createRun,
  defineRequirements,
  evaluateGate,
  findTask,
  nextStage,
  ownershipOverlapPairs,
  recordAgentRun,
  recordApproval,
  researchRequired,
  reopen,
  RevisionError,
  setPlan,
  workerEvidenceForTask,
} from "../../assets/extensions/8sync-gs/machine.ts";
import { emptyRisk } from "../../assets/extensions/8sync-gs/machine.ts";
import type {
  AcceptanceCriterion,
  AgentRunEvidence,
  GsState,
  Plan,
  Requirement,
  RiskAssessment,
  VerifyEvidence,
} from "../../assets/extensions/8sync-gs/types.ts";

const cfg = DEFAULT_CONFIG;

function req(id: string): Requirement {
  return { id, text: `requirement ${id}`, kind: "functional", required: true, status: "confirmed" };
}
function ac(id: string, reqs: string[], method: AcceptanceCriterion["method"] = "verify"): AcceptanceCriterion {
  return { id, text: `criterion ${id}`, requirements: reqs, method, required: true, status: "pending", evidence: [] };
}
function risk(over: Partial<RiskAssessment> = {}): RiskAssessment {
  return { ...emptyRisk(), ...over };
}
function agentRun(over: Partial<AgentRunEvidence> & Pick<AgentRunEvidence, "agent">): AgentRunEvidence {
  return {
    toolCallId: over.toolCallId ?? `tc-${Math.random().toString(16).slice(2)}`,
    agent: over.agent,
    assignmentHash: over.assignmentHash ?? "h",
    requestedSelector: over.requestedSelector ?? "@x",
    resolvedModel: over.resolvedModel ?? "anthropic/opus",
    resolvedModelFamily: over.resolvedModelFamily ?? "claude",
    resolvedModelIsFallback: over.resolvedModelIsFallback ?? false,
    exitCode: over.exitCode ?? 0,
    endedAt: over.endedAt ?? "2026-01-01T00:00:00Z",
    verdict: over.verdict,
    blockingFindings: over.blockingFindings,
    structuredOutput: over.structuredOutput,
    taskId: over.taskId,
  };
}
function plan(): Plan {
  return {
    hash: "planhash",
    createdAt: "2026-01-01T00:00:00Z",
    slices: [
      {
        id: "s1",
        title: "wave 1",
        tasks: [
          { id: "t1", title: "task 1", acceptance: ["AC1"], ownership: ["src/a.ts"], dependsOn: [], skills: [], verify: [{ program: "cargo", args: ["test"] }], status: "pending", attempts: 0, failStreak: 0, lastFailureHash: "", note: "" },
          { id: "t2", title: "task 2", acceptance: ["AC2"], ownership: ["src/b.ts"], dependsOn: [], skills: [], verify: [{ program: "cargo", args: ["build"] }], status: "pending", attempts: 0, failStreak: 0, lastFailureHash: "", note: "" },
        ],
      },
    ],
  };
}

/** Drive a run through clarify + plan setup in auto mode (no user gates except uat). */
function seeded(mode: "assisted" | "auto" = "auto"): GsState {
  const s = createRun({ runId: "r1", slug: "demo", goal: "g", projectRoot: "/x", mode, now: "2026-01-01T00:00:00Z" });
  defineRequirements(s, {
    requirements: [req("R1"), req("R2")],
    acceptance: [ac("AC1", ["R1"]), ac("AC2", ["R2"])],
    risk: risk({ trivial: true }),
  });
  return s;
}

describe("createRun", () => {
  test("starts in clarify", () => {
    const s = seeded();
    expect(s.stage).toBe("clarify");
    expect(s.revision).toBeGreaterThan(1);
  });
});

describe("revision guard", () => {
  test("assertRevision throws on mismatch", () => {
    const s = seeded();
    expect(() => assertRevision(s, s.revision - 1)).toThrow(RevisionError);
    expect(() => assertRevision(s, s.revision)).not.toThrow();
  });
});

describe("clarify gate", () => {
  test("fails without requirements", () => {
    const s = createRun({ runId: "r", slug: "d", goal: "g", projectRoot: "/x", mode: "auto" });
    const g = evaluateGate(s, cfg);
    expect(g.ok).toBe(false);
    expect(g.findings.map((f) => f.code)).toContain("UNRESOLVED_QUESTION");
  });
  test("assisted mode requires requirements approval", () => {
    const s = seeded("assisted");
    const g = evaluateGate(s, cfg);
    expect(g.findings.map((f) => f.code)).toContain("REQUIREMENTS_UNAPPROVED");
    recordApproval(s, { what: "requirements", by: "user" });
    expect(evaluateGate(s, cfg).ok).toBe(true);
  });
  test("auto mode passes with reqs + observable ACs", () => {
    expect(evaluateGate(seeded("auto"), cfg).ok).toBe(true);
  });
});

describe("research skip rule", () => {
  test("trivial risk skips research", () => {
    expect(researchRequired(risk({ trivial: true }))).toBe(false);
  });
  test("nontrivial unknown requires research", () => {
    expect(researchRequired(risk({ externalUnknown: true }))).toBe(true);
  });
  test("advance from clarify skips research when trivial", () => {
    const s = seeded("auto");
    const r = advance(s, cfg);
    expect(r.ok).toBe(true);
    expect(s.stage).toBe("plan");
    expect(s.stages.research.status).toBe("skipped");
  });
  test("nextStage lands on research when required", () => {
    const s = seeded("auto");
    s.risk = risk({ externalUnknown: true });
    expect(nextStage(s)).toBe("research");
  });
});

describe("plan gate", () => {
  function atPlan(): GsState {
    const s = seeded("auto");
    advance(s, cfg); // → plan
    return s;
  }
  test("requires a planner run and plan", () => {
    const s = atPlan();
    const g = evaluateGate(s, cfg);
    expect(g.findings.map((f) => f.code)).toContain("PLANNER_MISSING");
  });
  test("passes with full coverage + planner", () => {
    const s = atPlan();
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner" }));
    expect(evaluateGate(s, cfg).ok).toBe(true);
  });
  test("detects missing AC coverage", () => {
    const s = atPlan();
    const p = plan();
    p.slices[0].tasks[1].acceptance = []; // t2 covers nothing → AC2 uncovered
    setPlan(s, p);
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("AC_COVERAGE_INCOMPLETE");
  });
  test("detects a dependency cycle", () => {
    const s = atPlan();
    const p = plan();
    p.slices[0].tasks[0].dependsOn = ["t2"];
    p.slices[0].tasks[1].dependsOn = ["t1"];
    setPlan(s, p);
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("DEP_CYCLE");
  });
  test("detects ownership overlap in one wave", () => {
    const s = atPlan();
    const p = plan();
    p.slices[0].tasks[1].ownership = ["src/a.ts"]; // same as t1, no dep edge
    setPlan(s, p);
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("OWNERSHIP_OVERLAP");
  });
  test("rejects shell-string verify commands", () => {
    const s = atPlan();
    const p = plan();
    p.slices[0].tasks[0].verify = [{ program: "bash", args: ["-lc", "cargo test"] }];
    setPlan(s, p);
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("SHELL_VERIFY_COMMAND");
  });
  test("setPlan clears prior plan approvals", () => {
    const s = atPlan();
    setPlan(s, plan());
    recordApproval(s, { what: "plan", by: "user" });
    setPlan(s, plan());
    expect(s.approvals.some((a) => a.what === "plan")).toBe(false);
  });
});

describe("plan_review gate", () => {
  function atReview(criticFamily = "gpt", verdict: "pass" | "needs_fix" = "pass"): GsState {
    const s = seeded("auto");
    advance(s, cfg); // plan
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg); // plan_review
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: criticFamily, verdict }));
    return s;
  }
  test("passes with independent critic PASS (auto)", () => {
    expect(evaluateGate(atReview("gpt", "pass"), cfg).ok).toBe(true);
  });
  test("blocks a same-family critic", () => {
    expect(evaluateGate(atReview("claude", "pass"), cfg).findings.map((f) => f.code)).toContain("CRITIC_NOT_INDEPENDENT");
  });
  test("blocks a NEEDS_FIX verdict", () => {
    expect(evaluateGate(atReview("gpt", "needs_fix"), cfg).findings.map((f) => f.code)).toContain("CRITIC_NEEDS_FIX");
  });
  test("assisted mode needs plan approval", () => {
    const s = seeded("assisted");
    recordApproval(s, { what: "requirements", by: "user" });
    advance(s, cfg);
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg);
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("PLAN_UNAPPROVED");
  });
});

describe("verify doom-loop guard", () => {
  function withPlan(): GsState {
    const s = seeded("auto");
    advance(s, cfg);
    setPlan(s, plan());
    return s;
  }
  const ev: VerifyEvidence[] = [];
  test("passes and clears streak", () => {
    const s = withPlan();
    const r = applyVerify(s, "t1", true, "", ev, cfg);
    expect(r.status).toBe("passed");
    expect(findTask(s, "t1")?.status).toBe("passed");
  });
  test("three identical failures block", () => {
    const s = withPlan();
    applyVerify(s, "t1", false, "same error", ev, cfg);
    applyVerify(s, "t1", false, "same error", ev, cfg);
    const r = applyVerify(s, "t1", false, "same error", ev, cfg);
    expect(r.status).toBe("blocked");
    expect(findTask(s, "t1")?.status).toBe("blocked");
  });
  test("maxVerifyFailures blocks even with different errors", () => {
    const s = withPlan();
    for (let i = 0; i < cfg.limits.maxVerifyFailures; i++) {
      applyVerify(s, "t1", false, `error ${i}`, ev, cfg);
    }
    expect(findTask(s, "t1")?.status).toBe("blocked");
  });
});

describe("recordAgentRun idempotency", () => {
  test("a duplicate toolCallId is ignored", () => {
    const s = seeded("auto");
    const run = agentRun({ agent: "gs-planner", toolCallId: "dup" });
    const a = recordAgentRun(s, "plan", run);
    const b = recordAgentRun(s, "plan", run);
    expect(a.applied).toBe(true);
    expect(b.applied).toBe(false);
    expect(s.stages.plan.agentRuns.length).toBe(1);
  });
});

describe("full happy path (auto)", () => {
  test("advances clarify → done with all evidence", () => {
    const s = seeded("auto");
    // clarify → plan (research skipped, trivial)
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("plan");
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("plan_review");
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("implement");
    // implement all tasks
    recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", resolvedModelFamily: "claude" }));
    applyVerify(s, "t1", true, "", [], cfg);
    applyVerify(s, "t2", true, "", [], cfg);
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("verify");
    recordAgentRun(s, "verify", agentRun({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "pass" }));
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("review");
    recordAgentRun(s, "review", agentRun({ agent: "gs-reviewer", resolvedModelFamily: "gpt", verdict: "correct", blockingFindings: 0 }));
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("uat");
    // UAT: needs AC evidence + user approval even in auto mode
    expect(advance(s, cfg).ok).toBe(false);
    attachAcceptanceEvidence(s, "AC1", { kind: "verify", ref: "t1", at: "t" });
    attachAcceptanceEvidence(s, "AC2", { kind: "verify", ref: "t2", at: "t" });
    expect(advance(s, cfg).ok).toBe(false); // still needs user UAT
    recordApproval(s, { what: "uat", by: "user" });
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("closeout");
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("done");
    expect(s.status).toBe("done");
    const c = counts(s);
    expect(c.tasksPassed).toBe(2);
    expect(c.acPassed).toBe(2);
  });
});

describe("review gate", () => {
  function atReview(): GsState {
    const s = seeded("auto");
    s.risk = risk({ security: true, trivial: false, externalUnknown: false });
    // manually place at review with worker evidence
    s.stage = "review";
    recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", resolvedModelFamily: "claude" }));
    return s;
  }
  test("security-sensitive change requires gs-security", () => {
    const s = atReview();
    recordAgentRun(s, "review", agentRun({ agent: "gs-reviewer", resolvedModelFamily: "gpt", verdict: "correct" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("SECURITY_REVIEW_MISSING");
  });
  test("reviewer sharing impl family is not independent", () => {
    const s = atReview();
    recordAgentRun(s, "review", agentRun({ agent: "gs-reviewer", resolvedModelFamily: "claude", verdict: "correct" }));
    recordAgentRun(s, "review", agentRun({ agent: "gs-security", resolvedModelFamily: "gpt", verdict: "correct" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("REVIEW_NOT_INDEPENDENT");
  });
  test("a P0 blocker fails review", () => {
    const s = atReview();
    recordAgentRun(s, "review", agentRun({ agent: "gs-reviewer", resolvedModelFamily: "gpt", verdict: "incorrect", blockingFindings: 1 }));
    recordAgentRun(s, "review", agentRun({ agent: "gs-security", resolvedModelFamily: "gpt", verdict: "correct" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("REVIEW_FOUND_BLOCKER");
  });
});

describe("reopen + block", () => {
  test("reopen sends the run back to a stage and bumps attempts", () => {
    const s = seeded("auto");
    advance(s, cfg); // plan
    reopen(s, "plan", "critic needs fix");
    expect(s.stage).toBe("plan");
    expect(s.stages.plan.attempts).toBe(1);
  });
  test("block terminates the run", () => {
    const s = seeded("auto");
    block(s, "no legal model");
    expect(s.stage).toBe("blocked");
    expect(s.status).toBe("blocked");
  });
});

// ---------------------------------------------------------------------------
// Research gate: structured-output validation (not mere task exit)
// ---------------------------------------------------------------------------

describe("research gate structured output", () => {
  function atResearch(): GsState {
    const s = seeded("auto");
    s.risk = risk({ externalUnknown: true, trivial: false });
    advance(s, cfg); // clarify → research (not skipped: externalUnknown)
    expect(s.stage).toBe("research");
    return s;
  }
  test("fails when researcher exited clean but produced no structured output", () => {
    const s = atResearch();
    recordAgentRun(s, "research", agentRun({ agent: "gs-researcher" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("RESEARCH_MISSING");
  });
  test("fails when findings or sources are empty", () => {
    const s = atResearch();
    recordAgentRun(s, "research", agentRun({ agent: "gs-researcher", structuredOutput: { findings: [], sources: [] } }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("RESEARCH_MISSING");
  });
  test("fails when open_unknowns remain unresolved", () => {
    const s = atResearch();
    recordAgentRun(s, "research", agentRun({
      agent: "gs-researcher",
      structuredOutput: { findings: ["x"], sources: ["doc.md"], open_unknowns: ["?"] },
    }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("UNRESOLVED_QUESTION");
  });
  test("passes with non-empty findings + sources and no open_unknowns", () => {
    const s = atResearch();
    recordAgentRun(s, "research", agentRun({
      agent: "gs-researcher",
      structuredOutput: { findings: ["x"], sources: ["doc.md"], open_unknowns: [] },
    }));
    expect(evaluateGate(s, cfg).ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Plan gate: dependency integrity
// ---------------------------------------------------------------------------

describe("plan gate dependency integrity", () => {
  test("flags a dependsOn pointing at an unknown task", () => {
    const s = seeded("auto");
    advance(s, cfg); // plan
    const p = plan();
    p.slices[0].tasks[0].dependsOn = ["nope"];
    setPlan(s, p);
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner" }));
    expect(evaluateGate(s, cfg).findings.map((f) => f.code)).toContain("DEP_CYCLE");
  });
});

// ---------------------------------------------------------------------------
// recordAgentRun: a task batch shares one toolCallId but distinct taskIds
// ---------------------------------------------------------------------------

describe("recordAgentRun batch idempotency", () => {
  test("distinct taskIds under one toolCallId are all kept", () => {
    const s = seeded("auto");
    advance(s, cfg);
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg); // plan_review
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
    advance(s, cfg); // implement
    const batch = "batch-1";
    const a = recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", toolCallId: batch, taskId: "t1" }));
    const b = recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", toolCallId: batch, taskId: "t2" }));
    expect(a.applied).toBe(true);
    expect(b.applied).toBe(true);
    expect(s.stages.implement.agentRuns.filter((r) => r.toolCallId === batch).length).toBe(2);
    // a true duplicate (same taskId under the same toolCallId) is still rejected
    const dup = recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", toolCallId: batch, taskId: "t1" }));
    expect(dup.applied).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// setPlan: a changed hash invalidates stale downstream evidence
// ---------------------------------------------------------------------------

describe("setPlan invalidates stale downstream evidence", () => {
  test("a changed plan hash clears critic/worker/verifier runs + AC evidence", () => {
    const s = seeded("auto");
    advance(s, cfg); // plan
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg); // plan_review
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
    advance(s, cfg); // implement
    recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", resolvedModelFamily: "claude", taskId: "t1" }));
    attachAcceptanceEvidence(s, "AC1", { kind: "verify", ref: "t1", at: "t" });
    expect(s.stages.plan_review.agentRuns.length).toBe(1);
    expect(s.acceptance[0].evidence.length).toBe(1);
    // a critic bounce reopens plan, then a revised plan arrives (new hash)
    reopen(s, "plan", "critic bounce");
    const p2 = plan();
    p2.hash = "different";
    setPlan(s, p2);
    expect(s.stages.plan_review.agentRuns.length).toBe(0);
    expect(s.stages.implement.agentRuns.length).toBe(0);
    expect(s.stages.verify.agentRuns.length).toBe(0);
    expect(s.acceptance[0].evidence.length).toBe(0);
    expect(s.acceptance[0].status).toBe("pending");
    expect(s.approvals.some((a) => a.what === "plan")).toBe(false);
  });
  test("the same plan hash does not clear downstream", () => {
    const s = seeded("auto");
    advance(s, cfg);
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner" }));
    const before = s.stages.plan.agentRuns.length;
    setPlan(s, plan()); // identical hash
    expect(s.stages.plan.agentRuns.length).toBe(before);
  });
});

// ---------------------------------------------------------------------------
// Auto-reopen: critic NEEDS_FIX / verifier FAIL / reviewer blocker
// ---------------------------------------------------------------------------

describe("plan_review auto-reopen + loop limit", () => {
  function toPlanReviewNeedFix(): GsState {
    const s = seeded("auto");
    advance(s, cfg); // plan
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg); // plan_review
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "needs_fix" }));
    return s;
  }
  test("NEEDS_FIX reopens plan (auto mode is not stranded)", () => {
    const s = toPlanReviewNeedFix();
    const r = advance(s, cfg);
    expect(s.stage).toBe("plan");
    expect(s.stages.plan.attempts).toBe(1);
    expect(r.message).toContain("reopened plan");
  });
  test("after reopen, a fresh critic PASS advances to implement", () => {
    const s = toPlanReviewNeedFix();
    advance(s, cfg); // reopen → plan (attempts 1)
    const p2 = plan();
    p2.hash = "p2";
    setPlan(s, p2); // new hash clears the stale NEEDS_FIX critic
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg); // plan_review
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
    expect(advance(s, cfg).ok).toBe(true);
    expect(s.stage).toBe("implement");
  });
  test("a same-family critic is a config issue, not a fix loop (no reopen)", () => {
    const s = seeded("auto");
    advance(s, cfg);
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg);
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "claude", verdict: "pass" }));
    const r = advance(s, cfg); // CRITIC_NOT_INDEPENDENT only → plain gate fail
    expect(r.ok).toBe(false);
    expect(s.stage).toBe("plan_review");
    expect(s.stages.plan.attempts).toBe(0);
  });
  test("maxPlanReviewLoops blocks after the limit", () => {
    const s = toPlanReviewNeedFix();
    advance(s, cfg); // reopen, attempts 1
    expect(s.stages.plan.attempts).toBe(1);
    advance(s, cfg); // plan → plan_review
    advance(s, cfg); // reopen, attempts 2
    expect(s.stages.plan.attempts).toBe(2);
    advance(s, cfg); // plan → plan_review
    const r = advance(s, cfg); // attempts 2 >= limit → block
    expect(s.stage).toBe("blocked");
    expect(s.status).toBe("blocked");
    expect(r.ok).toBe(false);
    expect(r.message).toContain("plan-review loop limit");
  });
});

describe("verify/review auto-reopen implement", () => {
  // Drive to the verify stage with BOTH tasks passed (the implement gate clears
  // only when every task is resolved). Only t1 carries worker evidence, so on a
  // reopen resetTasksForFixPass resets t1 (built this run) and preserves t2
  // (legacy-verified-style: passed with no worker run).
  function toVerifyStage(): GsState {
    const s = seeded("auto");
    advance(s, cfg); // plan
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg); // plan_review
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
    advance(s, cfg); // implement
    recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", resolvedModelFamily: "claude", taskId: "t1" }));
    applyVerify(s, "t1", true, "", [], cfg);
    applyVerify(s, "t2", true, "", [], cfg);
    advance(s, cfg); // verify
    return s;
  }
  test("VERIFY_FAILED reopens implement and resets the passed task (no no-op)", () => {
    const s = toVerifyStage();
    recordAgentRun(s, "verify", agentRun({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "fail" }));
    expect(findTask(s, "t1")?.status).toBe("passed");
    const r = advance(s, cfg);
    expect(s.stage).toBe("implement");
    expect(s.stages.implement.attempts).toBe(1);
    expect(findTask(s, "t1")?.status).toBe("pending");
    expect(r.message).toContain("reopened implement");
  });
  test("VERIFIER_MISSING is a setup issue, not a fix loop (no reopen)", () => {
    const s = toVerifyStage(); // no verifier recorded
    const r = advance(s, cfg); // VERIFIER_MISSING → plain gate fail
    expect(r.ok).toBe(false);
    expect(s.stage).toBe("verify");
    expect(s.stages.implement.attempts).toBe(0);
  });
  test("REVIEW_FOUND_BLOCKER reopens implement", () => {
    const s = toVerifyStage();
    recordAgentRun(s, "verify", agentRun({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "pass" }));
    advance(s, cfg); // review
    recordAgentRun(s, "review", agentRun({ agent: "gs-reviewer", resolvedModelFamily: "gpt", verdict: "incorrect", blockingFindings: 1 }));
    const r = advance(s, cfg);
    expect(s.stage).toBe("implement");
    expect(findTask(s, "t1")?.status).toBe("pending");
    expect(r.message).toContain("reopened implement");
  });
  test("maxReviewLoops blocks after the limit", () => {
    const s = toVerifyStage();
    recordAgentRun(s, "verify", agentRun({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "fail" }));
    advance(s, cfg); // reopen, attempts 1, t1 reset (t2 preserved)
    expect(s.stages.implement.attempts).toBe(1);
    applyVerify(s, "t1", true, "", [], cfg); // re-fix t1
    advance(s, cfg); // implement → verify
    recordAgentRun(s, "verify", agentRun({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "fail" }));
    advance(s, cfg); // reopen, attempts 2
    expect(s.stages.implement.attempts).toBe(2);
    applyVerify(s, "t1", true, "", [], cfg);
    advance(s, cfg); // implement → verify
    recordAgentRun(s, "verify", agentRun({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "fail" }));
    const r = advance(s, cfg); // attempts 2 >= limit → block
    expect(s.stage).toBe("blocked");
    expect(r.ok).toBe(false);
    expect(r.message).toContain("implement loop limit");
  });
});

// ---------------------------------------------------------------------------
// Closeout recomputes verify/review/uat from CURRENT evidence
// ---------------------------------------------------------------------------

describe("closeout re-checks current evidence", () => {
  function toCloseout(): GsState {
    const s = seeded("auto");
    advance(s, cfg); // plan
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg); // plan_review
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
    advance(s, cfg); // implement
    recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", resolvedModelFamily: "claude", taskId: "t1" }));
    recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", resolvedModelFamily: "claude", taskId: "t2" }));
    applyVerify(s, "t1", true, "", [], cfg);
    applyVerify(s, "t2", true, "", [], cfg);
    advance(s, cfg); // verify
    recordAgentRun(s, "verify", agentRun({ agent: "gs-verifier", resolvedModelFamily: "gpt", verdict: "pass" }));
    advance(s, cfg); // review
    recordAgentRun(s, "review", agentRun({ agent: "gs-reviewer", resolvedModelFamily: "gpt", verdict: "correct", blockingFindings: 0 }));
    advance(s, cfg); // uat
    attachAcceptanceEvidence(s, "AC1", { kind: "verify", ref: "t1", at: "t" });
    attachAcceptanceEvidence(s, "AC2", { kind: "verify", ref: "t2", at: "t" });
    recordApproval(s, { what: "uat", by: "user" });
    advance(s, cfg); // closeout
    expect(s.stage).toBe("closeout");
    return s;
  }
  test("passes when all current evidence is present", () => {
    expect(evaluateGate(toCloseout(), cfg).ok).toBe(true);
  });
  test("fails when verifier evidence went stale (verdict flipped to fail)", () => {
    const s = toCloseout();
    s.stages.verify.agentRuns[s.stages.verify.agentRuns.length - 1].verdict = "fail";
    const g = evaluateGate(s, cfg);
    expect(g.ok).toBe(false);
    expect(g.findings.map((f) => f.code)).toContain("VERIFY_FAILED");
  });
  test("fails when AC evidence was removed", () => {
    const s = toCloseout();
    s.acceptance[0].evidence = [];
    s.acceptance[0].status = "pending";
    const g = evaluateGate(s, cfg);
    expect(g.ok).toBe(false);
    expect(g.findings.map((f) => f.code)).toContain("AC_EVIDENCE_MISSING");
  });
  test("fails when the reviewer run is missing", () => {
    const s = toCloseout();
    s.stages.review.agentRuns = [];
    const g = evaluateGate(s, cfg);
    expect(g.ok).toBe(false);
    expect(g.findings.map((f) => f.code)).toContain("REVIEW_MISSING");
  });
});

// ---------------------------------------------------------------------------
// Pure helpers: worker evidence by task + dependency-aware ownership
// ---------------------------------------------------------------------------

describe("pure evidence/ownership helpers", () => {
  test("workerEvidenceForTask returns only successful worker runs for the task", () => {
    const s = seeded("auto");
    advance(s, cfg);
    setPlan(s, plan());
    recordAgentRun(s, "plan", agentRun({ agent: "gs-planner", resolvedModelFamily: "claude" }));
    advance(s, cfg);
    recordAgentRun(s, "plan_review", agentRun({ agent: "gs-critic", resolvedModelFamily: "gpt", verdict: "pass" }));
    advance(s, cfg); // implement
    recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", taskId: "t1" }));
    recordAgentRun(s, "implement", agentRun({ agent: "gs-worker", taskId: "t1", exitCode: 1 }));
    expect(workerEvidenceForTask(s, "t1").length).toBe(1); // only exit 0
    expect(workerEvidenceForTask(s, "t2").length).toBe(0);
  });
  test("ownershipOverlapPairs is dependency-aware", () => {
    const t = (id: string, ownership: string[], dependsOn: string[] = []) => ({
      id, title: id, acceptance: ["AC1"], ownership, dependsOn, skills: [],
      verify: [{ program: "true", args: [] }], status: "pending" as const,
      attempts: 0, failStreak: 0, lastFailureHash: "", note: "",
    });
    // a & b share ownership, no dep edge → conflict
    expect(ownershipOverlapPairs([t("a", ["src/x.ts"]), t("b", ["src/x.ts"])])).toHaveLength(1);
    // same ownership but b depends on a → serialized, no conflict
    expect(ownershipOverlapPairs([t("a", ["src/x.ts"]), t("b", ["src/x.ts"], ["a"])])).toHaveLength(0);
    // disjoint ownership → no conflict
    expect(ownershipOverlapPairs([t("a", ["src/x.ts"]), t("b", ["src/y.ts"])])).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Action approvals: one-shot, hash-deduped (stored only, not gate-consumed)
// ---------------------------------------------------------------------------

describe("action approval (one-shot, hash-deduped)", () => {
  test("records and dedupes action approvals by command hash", () => {
    const s = seeded("auto");
    recordApproval(s, { what: "action", by: "user", note: "h1" });
    expect(s.approvals.filter((a) => a.what === "action").length).toBe(1);
    recordApproval(s, { what: "action", by: "user", note: "h1" }); // same hash → no-op
    expect(s.approvals.filter((a) => a.what === "action").length).toBe(1);
    recordApproval(s, { what: "action", by: "user", note: "h2" }); // different hash → kept
    expect(s.approvals.filter((a) => a.what === "action").length).toBe(2);
  });
  test("actionApproved queries by hash and is unaffected by other approvals", () => {
    const s = seeded("auto");
    recordApproval(s, { what: "requirements", by: "user" });
    recordApproval(s, { what: "action", by: "user", note: "h1" });
    expect(actionApproved(s, "h1")).toBe(true);
    expect(actionApproved(s, "h2")).toBe(false);
  });
});
