import { describe, expect, test } from "bun:test";
import {
  assignmentHash,
  buildAgentEvidence,
  countBlockingFindings,
  evidenceFromTaskDetails,
  isGsAgent,
  normalizeVerdict,
  structuredTaskId,
} from "../../assets/extensions/8sync-gs/task-evidence.ts";

describe("isGsAgent", () => {
  test("accepts the seven gs agents, rejects others", () => {
    expect(isGsAgent("gs-planner")).toBe(true);
    expect(isGsAgent("gs-security")).toBe(true);
    expect(isGsAgent("reviewer")).toBe(false);
    expect(isGsAgent(undefined)).toBe(false);
  });
});

describe("normalizeVerdict", () => {
  test("reviewer overall_correctness", () => {
    expect(normalizeVerdict({ overall_correctness: "correct" })).toBe("correct");
    expect(normalizeVerdict({ overall_correctness: "incorrect" })).toBe("incorrect");
  });
  test("explicit verdict strings", () => {
    expect(normalizeVerdict({ verdict: "PASS" })).toBe("pass");
    expect(normalizeVerdict({ verdict: "needs-fix" })).toBe("needs_fix");
    expect(normalizeVerdict({ verdict: "reject" })).toBe("fail");
  });
  test("status + pass boolean", () => {
    expect(normalizeVerdict({ status: "failed" })).toBe("fail");
    expect(normalizeVerdict({ pass: true })).toBe("pass");
  });
  test("absent → undefined", () => {
    expect(normalizeVerdict({})).toBeUndefined();
    expect(normalizeVerdict("nope")).toBeUndefined();
  });
});

describe("countBlockingFindings", () => {
  test("counts P0/P1 only", () => {
    const structured = { findings: [{ priority: 0 }, { priority: 1 }, { priority: 2 }, { priority: 3 }] };
    expect(countBlockingFindings(structured)).toBe(2);
  });
  test("no findings → 0", () => {
    expect(countBlockingFindings({})).toBe(0);
    expect(countBlockingFindings({ findings: "x" })).toBe(0);
  });
});

describe("assignmentHash", () => {
  test("stable for identical input, differs on change", () => {
    const a = assignmentHash({ agent: "gs-worker", taskIds: ["t1"], instruction: "do it" });
    const b = assignmentHash({ agent: "gs-worker", taskIds: ["t1"], instruction: "do it" });
    const c = assignmentHash({ agent: "gs-worker", taskIds: ["t2"], instruction: "do it" });
    expect(a).toBe(b);
    expect(a).not.toBe(c);
  });
});

describe("buildAgentEvidence", () => {
  test("records observed agent + resolved model, ignores prose", () => {
    const ev = buildAgentEvidence({
      toolCallId: "tc1",
      result: {
        agent: "gs-reviewer",
        resolvedModel: "openai/gpt-5.5",
        resolvedModelIsFallback: true,
        exitCode: 0,
        structuredOutput: { overall_correctness: "incorrect", findings: [{ priority: 0 }] },
        outputPath: "/tmp/out",
      },
      assignmentHash: "h1",
      requestedSelector: "family:gpt",
      resolvedModelFamily: "gpt",
    });
    expect(ev?.agent).toBe("gs-reviewer");
    expect(ev?.resolvedModel).toBe("openai/gpt-5.5");
    expect(ev?.resolvedModelIsFallback).toBe(true);
    expect(ev?.verdict).toBe("incorrect");
    expect(ev?.blockingFindings).toBe(1);
    expect(ev?.resolvedModelFamily).toBe("gpt");
  });
  test("non-gs agent yields no evidence", () => {
    const ev = buildAgentEvidence({
      toolCallId: "tc",
      result: { agent: "scout", resolvedModel: "x" },
      assignmentHash: "h",
      requestedSelector: "@x",
    });
    expect(ev).toBeUndefined();
  });
  test("aborted result is a nonzero exit", () => {
    const ev = buildAgentEvidence({
      toolCallId: "tc",
      result: { agent: "gs-planner", aborted: true, resolvedModel: "m" },
      assignmentHash: "h",
      requestedSelector: "@plan",
    });
    expect(ev?.exitCode).toBe(1);
  });
  test("extracts task ID and keeps parent batch identity", () => {
    const ev = buildAgentEvidence({
      toolCallId: "batch:t1",
      batchToolCallId: "batch",
      result: {
        agent: "gs-worker",
        resolvedModel: "anthropic/sonnet",
        structuredOutput: { task_id: "t1" },
      },
      assignmentHash: "lease",
      requestedSelector: "family:sonnet",
    });
    expect(ev?.taskId).toBe("t1");
    expect(ev?.toolCallId).toBe("batch:t1");
    expect(ev?.batchToolCallId).toBe("batch");
    expect(structuredTaskId({ data: { taskId: "nested" } })).toBe("nested");
  });
});

describe("evidenceFromTaskDetails", () => {
  test("extracts only gs-agent results and resolves families", () => {
    const evs = evidenceFromTaskDetails(
      "tc9",
      {
        results: [
          { agent: "gs-planner", resolvedModel: "anthropic/opus", exitCode: 0, structuredOutput: {} },
          { agent: "scout", resolvedModel: "zai/glm", exitCode: 0 },
        ],
      },
      { assignmentHash: "h", requestedSelector: "@plan", familyOf: (m) => (m.startsWith("anthropic") ? "claude" : "other") },
    );
    expect(evs.length).toBe(1);
    expect(evs[0].agent).toBe("gs-planner");
    expect(evs[0].resolvedModelFamily).toBe("claude");
  });

  test("preserves every worker in a batch with unique evidence identities", () => {
    const evs = evidenceFromTaskDetails(
      "batch",
      {
        results: [
          { id: "r1", agent: "gs-worker", resolvedModel: "anthropic/sonnet", structuredOutput: { task_id: "t1" } },
          { id: "r2", agent: "gs-worker", resolvedModel: "anthropic/sonnet", structuredOutput: { task_id: "t2" } },
        ],
      },
      { assignmentHash: "exact-lease", requestedSelector: "family:sonnet", familyOf: () => "claude" },
    );
    expect(evs.map((evidence) => evidence.taskId)).toEqual(["t1", "t2"]);
    expect(evs.map((evidence) => evidence.toolCallId)).toEqual(["batch:r1", "batch:r2"]);
    expect(evs.every((evidence) => evidence.batchToolCallId === "batch")).toBe(true);
    expect(evs.every((evidence) => evidence.assignmentHash === "exact-lease")).toBe(true);
  });
});
