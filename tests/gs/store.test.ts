import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { existsSync, lstatSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import {
  ACTIVE_REL,
  CACHE_REL,
  LEGACY_COPY_REL,
  LEGACY_IMPORT_MARKER_REL,
  LEGACY_REL,
  STATE_MD_REL,
  atomicWrite,
  clearActive,
  gsPaths,
  importLegacyEngine,
  loadRuntime,
  readActive,
  renderPlanMd,
  renderStateSection,
  renderVerificationMd,
  resume,
  saveCheckpoint,
  saveRuntime,
  upsertStateSection,
  validateState,
  writeActive,
  writeStateMd,
} from "../../assets/extensions/8sync-gs/store.ts";
import { createRun, defineRequirements, emptyRisk, setPlan } from "../../assets/extensions/8sync-gs/machine.ts";
import type { GsState, Plan } from "../../assets/extensions/8sync-gs/types.ts";

let root: string;
beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "gs-store-"));
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

function demo(mode: "assisted" | "auto" = "auto"): GsState {
  const s = createRun({ runId: "r1", slug: "demo", goal: "goal", projectRoot: root, mode, now: "2026-01-01T00:00:00Z" });
  defineRequirements(s, {
    requirements: [{ id: "R1", text: "req", kind: "functional", required: true, status: "confirmed" }],
    acceptance: [{ id: "AC1", text: "ac", requirements: ["R1"], method: "verify", required: true, status: "pending", evidence: [] }],
    risk: { ...emptyRisk(), trivial: true },
  });
  return s;
}
function samplePlan(): Plan {
  return {
    hash: "h",
    createdAt: "2026-01-01T00:00:00Z",
    slices: [{ id: "s1", title: "w1", tasks: [{ id: "t1", title: "t", acceptance: ["AC1"], ownership: ["a"], dependsOn: [], skills: [], verify: [{ program: "cargo", args: ["test"] }], status: "pending", attempts: 0, failStreak: 0, lastFailureHash: "", note: "" }] }],
  };
}
/** Loosely-typed mutable view of a serialized GsState for deliberate test-only
 * corruption; JSON.parse's structural shape matches GsState 1:1 here, so a
 * single named cast at this boundary replaces `any` for every mutate case. */
interface RawState {
  schemaVersion: unknown;
  stage: unknown;
  status: unknown;
  mode: unknown;
  risk: Record<string, unknown>;
  requirements: Record<string, unknown>[];
  acceptance: Record<string, unknown>[];
  plan?: { slices: Array<{ tasks: Record<string, unknown>[] }> };
  stages: Record<string, { status: unknown; agentRuns: Record<string, unknown>[]; verify: Record<string, unknown>[]; gate?: Record<string, unknown> }>;
  approvals: Record<string, unknown>[];
  pendingAction?: Record<string, unknown>;
  audit: Record<string, unknown>[];
  legacy?: Record<string, unknown>;
  worktrees?: Record<string, unknown>[];
}
function rawOf(s: GsState): RawState {
  return JSON.parse(JSON.stringify(s)) as RawState;
}

/** A state exercising every nested shape validateState must check: plan/tasks,
 * a stage's agent-run + verify + gate evidence, an approval, a pending
 * action, and legacy metadata. */
function fullDemo(): GsState {
  const s = demo();
  setPlan(s, samplePlan());
  s.stages.plan.agentRuns.push({
    toolCallId: "tc1",
    batchToolCallId: "batch1",
    agent: "gs-planner",
    assignmentHash: "h1",
    requestedSelector: "@plan",
    resolvedModel: "anthropic/claude-opus",
    resolvedModelIsFallback: false,
    exitCode: 0,
    endedAt: "2026-01-01T00:00:00Z",
    verdict: "pass",
  });
  s.stages.plan.verify.push({
    command: { program: "cargo", args: ["test"] },
    exitCode: 0,
    stdoutHash: "h",
    stderrHash: "h",
    durationMs: 10,
    finishedAt: "2026-01-01T00:00:00Z",
    output: "ok",
  });
  s.stages.plan.gate = { ok: true, stage: "plan", findings: [], checkedAt: "2026-01-01T00:00:00Z" };
  s.approvals.push({ what: "plan", by: "user", at: "2026-01-01T00:00:00Z" });
  s.approvals.push({ what: "action", by: "user", at: "2026-01-01T00:00:00Z", note: "action-hash" });
  s.pendingAction = {
    id: "p1",
    stage: "implement",
    kind: "agent",
    agent: "gs-worker",
    toolCallId: "tc1",
    leaseRevision: 2,
    boundRevision: 3,
    instruction: "do it",
    createdAt: "2026-01-01T00:00:00Z",
  };
  s.legacy = { importedAt: "2026-01-01T00:00:00Z", sourcePath: LEGACY_REL, goal: "g", taskCount: 1, verifiedCount: 1 };
  s.worktrees = [{ slug: "slice-a", path: "/tmp/slice-a", branch: "gs/slice-a", createdAt: "2026-01-01T00:00:00Z" }];
  return s;
}

describe("atomicWrite", () => {
  test("writes then rotates a .bak on overwrite", () => {
    const f = join(root, "sub/x.json");
    atomicWrite(f, "one");
    expect(readFileSync(f, "utf8")).toBe("one");
    expect(existsSync(`${f}.bak`)).toBe(false);
    atomicWrite(f, "two");
    expect(readFileSync(f, "utf8")).toBe("two");
    expect(readFileSync(`${f}.bak`, "utf8")).toBe("one");
  });
});

describe("runtime round-trip", () => {
  test("saveRuntime + loadRuntime preserves state", () => {
    const s = demo();
    saveRuntime(root, s);
    const back = loadRuntime(root);
    expect(back?.slug).toBe("demo");
    expect(back?.revision).toBe(s.revision);
  });
});

describe("validateState", () => {
  test("rejects wrong schema version / shape", () => {
    expect(validateState({ schemaVersion: 99 })).toBeUndefined();
    expect(validateState(null)).toBeUndefined();
    expect(validateState({ schemaVersion: 1, stage: "clarify" })).toBeUndefined();
  });
});

describe("ACTIVE pointer", () => {
  test("write + read", () => {
    writeActive(root, "myslug");
    expect(readActive(root)).toBe("myslug");
  });
});

describe("resume ordering", () => {
  test("prefers a valid cache", () => {
    const s = demo();
    saveRuntime(root, s);
    const r = resume(root);
    expect(r.source).toBe("cache");
  });

  test("falls back to the durable checkpoint when cache is absent", () => {
    const s = demo();
    saveCheckpoint(root, s);
    const r = resume(root);
    expect(r.source).toBe("checkpoint");
    expect(r.state?.slug).toBe("demo");
  });

  test("recovers from .bak when the primary is corrupt", () => {
    const s = demo();
    saveRuntime(root, s); // writes cache
    saveRuntime(root, s); // rotates .bak (valid)
    // Corrupt the primary cache.
    writeFileSync(join(root, CACHE_REL), "{ not json");
    const r = resume(root);
    expect(r.source).toBe("backup");
    expect(r.state?.slug).toBe("demo");
  });

  test("corrupt primary with no fallback yields a diagnostic, never a reset", () => {
    mkdirSync(dirname(join(root, CACHE_REL)), { recursive: true });
    writeFileSync(join(root, CACHE_REL), "{ garbage");
    const r = resume(root);
    expect(r.state).toBeUndefined();
    expect(r.source).toBe("corrupt");
  });

  test("no run → none", () => {
    expect(resume(root).state).toBeUndefined();
    expect(resume(root).source).toBe("none");
  });
});

describe("legacy import", () => {
  test("maps verified tasks to passed, preserves original, needs_confirmation reqs", () => {
    const legacy = {
      goal: "old goal",
      slices: [
        { id: "s1", title: "w", tasks: [
          { id: "s1.t1", title: "done task", status: "done", verified: true, verify: ["cargo test"] },
          { id: "s1.t2", title: "pending task", status: "pending", verified: false, verify: [] },
        ] },
      ],
    };
    mkdirSync(dirname(join(root, LEGACY_REL)), { recursive: true });
    writeFileSync(join(root, LEGACY_REL), JSON.stringify(legacy));
    const s = importLegacyEngine(root, "2026-02-02T00:00:00Z");
    expect(s).toBeDefined();
    expect(existsSync(join(root, LEGACY_COPY_REL))).toBe(true);
    expect(s?.stage).toBe("clarify");
    const tasks = s?.plan?.slices.flatMap((x) => x.tasks) ?? [];
    expect(tasks.find((t) => t.id === "s1.t1")?.status).toBe("passed");
    expect(tasks.find((t) => t.id === "s1.t2")?.status).toBe("pending");
    expect(s?.requirements[0].status).toBe("needs_confirmation");
    expect(s?.legacy?.verifiedCount).toBe(1);
  });

  test("resume uses legacy import when only legacy state exists", () => {
    const legacy = { goal: "g", slices: [] };
    mkdirSync(dirname(join(root, LEGACY_REL)), { recursive: true });
    writeFileSync(join(root, LEGACY_REL), JSON.stringify(legacy));
    const r = resume(root);
    expect(r.source).toBe("legacy");
  });
});

describe("projections", () => {
  test("render plan + verification + state section", () => {
    const s = demo();
    setPlan(s, samplePlan());
    expect(renderPlanMd(s)).toContain("t1");
    expect(renderVerificationMd(s)).toContain("AC1");
    const section = renderStateSection(s, "run gs-planner");
    expect(section).toContain("## GS run: demo");
    expect(section).toContain("Next: run gs-planner");
  });
  test("saveCheckpoint writes projections + ACTIVE", () => {
    const s = demo();
    setPlan(s, samplePlan());
    saveCheckpoint(root, s);
    const p = gsPaths(root, "demo");
    expect(existsSync(p.checkpoint)).toBe(true);
    expect(existsSync(p.planMd)).toBe(true);
    expect(readActive(root)).toBe("demo");
  });
});

describe("upsertStateSection", () => {
  test("inserts when absent and replaces when present", () => {
    const first = upsertStateSection("# STATE\n\nbody\n", "<!-- 8sync:gs:begin -->\nX\n<!-- 8sync:gs:end -->");
    expect(first).toContain("X");
    const second = upsertStateSection(first, "<!-- 8sync:gs:begin -->\nY\n<!-- 8sync:gs:end -->");
    expect(second).toContain("Y");
    expect(second).not.toContain("\nX\n");
    // No duplicate blocks.
    expect(second.match(/8sync:gs:begin/g)?.length).toBe(1);
  });
});

describe("clearActive", () => {
  test("removes the pointer + cache", () => {
    const s = demo();
    saveRuntime(root, s);
    writeActive(root, "demo");
    clearActive(root);
    expect(existsSync(join(root, ACTIVE_REL))).toBe(false);
    expect(existsSync(join(root, CACHE_REL))).toBe(false);
  });
});

describe("validateState — nested structural validation", () => {
  test("accepts a fully valid nested state", () => {
    expect(validateState(rawOf(fullDemo()))).toBeDefined();
  });

  const cases: Array<[string, (r: RawState) => void]> = [
    ["requirement kind", (r) => { r.requirements[0].kind = "bogus"; }],
    ["acceptance evidence kind", (r) => { r.acceptance[0].evidence = [{ kind: "bogus", ref: "x", at: "t" }]; }],
    ["risk boolean field", (r) => { r.risk.trivial = "yes"; }],
    ["plan task empty verify program", (r) => { r.plan!.slices[0].tasks[0].verify = [{ program: "", args: [] }]; }],
    ["plan task non-numeric attempts", (r) => { r.plan!.slices[0].tasks[0].attempts = "0"; }],
    ["stage record bad status", (r) => { r.stages.plan.status = "bogus"; }],
    ["stage agent-run bad agent name", (r) => { r.stages.plan.agentRuns[0].agent = "not-a-gs-agent"; }],
    ["stage agent-run missing endedAt", (r) => { delete r.stages.plan.agentRuns[0].endedAt; }],
    ["stage agent-run bad batchToolCallId", (r) => { r.stages.plan.agentRuns[0].batchToolCallId = 1; }],
    ["stage verify evidence bad exitCode", (r) => { r.stages.plan.verify[0].exitCode = "0"; }],
    ["stage gate bad stage", (r) => { r.stages.plan.gate!.stage = "not-a-stage"; }],
    ["missing stage key entirely", (r) => { delete r.stages.plan; }],
    ["approval bad what", (r) => { r.approvals[0].what = "bogus"; }],
    ["approval bad by", (r) => { r.approvals[0].by = "nobody"; }],
    ["pendingAction missing instruction", (r) => { delete r.pendingAction!.instruction; }],
    ["pendingAction bad kind", (r) => { r.pendingAction!.kind = "bogus"; }],
    ["pendingAction non-numeric leaseRevision", (r) => { r.pendingAction!.leaseRevision = "2"; }],
    ["pendingAction non-numeric boundRevision", (r) => { r.pendingAction!.boundRevision = "3"; }],
    ["pendingAction bad toolCallId", (r) => { r.pendingAction!.toolCallId = 4; }],
    ["audit bad stage", (r) => { r.audit = [{ at: "t", stage: "nope", kind: "x", detail: "y" }]; }],
    ["legacy missing taskCount", (r) => { delete r.legacy!.taskCount; }],
    ["legacy non-numeric verifiedCount", (r) => { r.legacy!.verifiedCount = "1"; }],
    ["worktree missing createdAt", (r) => { delete r.worktrees![0].createdAt; }],
    ["worktree non-string path", (r) => { r.worktrees![0].path = 1; }],
    ["top-level stage enum", (r) => { r.stage = "not-a-stage"; }],
    ["top-level status enum", (r) => { r.status = "not-a-status"; }],
    ["top-level mode enum", (r) => { r.mode = "sometimes"; }],
    ["schemaVersion mismatch", (r) => { r.schemaVersion = 2; }],
  ];

  for (const [name, mutate] of cases) {
    test(`rejects malformed ${name}`, () => {
      const r = rawOf(fullDemo());
      mutate(r);
      expect(validateState(r)).toBeUndefined();
    });
  }
});

describe("resume — invalid cache never suppresses recovery", () => {
  test("structurally invalid cache (valid JSON) still falls back to a valid checkpoint", () => {
    const s = demo();
    saveCheckpoint(root, s);
    const badCache = rawOf(s);
    badCache.risk.trivial = "not-a-boolean";
    mkdirSync(dirname(join(root, CACHE_REL)), { recursive: true });
    writeFileSync(join(root, CACHE_REL), JSON.stringify(badCache));
    const r = resume(root);
    expect(r.source).toBe("checkpoint");
    expect(r.state?.slug).toBe("demo");
  });

  test("structurally invalid cache with nothing recoverable returns corrupt, never a reset", () => {
    const s = demo();
    const badCache = rawOf(s);
    badCache.stages.plan.status = "bogus";
    mkdirSync(dirname(join(root, CACHE_REL)), { recursive: true });
    writeFileSync(join(root, CACHE_REL), JSON.stringify(badCache));
    const r = resume(root);
    expect(r.state).toBeUndefined();
    expect(r.source).toBe("corrupt");
  });
});

describe("atomicWrite — symlink hardening", () => {
  test("refuses to write through a symlinked directory component beneath the guarded root", () => {
    const outside = mkdtempSync(join(tmpdir(), "gs-outside-"));
    symlinkSync(outside, join(root, ".cache"));
    expect(() => saveRuntime(root, demo())).toThrow();
    expect(existsSync(join(outside, "8sync"))).toBe(false);
  });

  test("a symlinked destination file is atomically replaced, never followed", () => {
    const s = demo();
    const p = gsPaths(root, s.slug);
    mkdirSync(dirname(p.cacheFile), { recursive: true });
    const outsideDir = mkdtempSync(join(tmpdir(), "gs-target-"));
    const outsideTarget = join(outsideDir, "victim.json");
    writeFileSync(outsideTarget, "untouched");
    symlinkSync(outsideTarget, p.cacheFile);
    saveRuntime(root, s);
    expect(lstatSync(p.cacheFile).isSymbolicLink()).toBe(false);
    expect(readFileSync(outsideTarget, "utf8")).toBe("untouched");
    expect(JSON.parse(readFileSync(p.cacheFile, "utf8")).slug).toBe("demo");
  });
});

describe("atomicWrite — no stray temp files after write", () => {
  test("leaves only the target and its .bak, never a leftover randomized temp file", () => {
    const f = join(root, "durable/x.json");
    atomicWrite(f, "one", root);
    atomicWrite(f, "two", root);
    const entries = readdirSync(dirname(f));
    expect(entries.sort()).toEqual(["x.json", "x.json.bak"]);
    expect(readFileSync(f, "utf8")).toBe("two");
  });
});

describe("writeStateMd — managed STATE.md block", () => {
  test("inserts the GS block while preserving surrounding content", () => {
    const stateMdPath = join(root, STATE_MD_REL);
    mkdirSync(dirname(stateMdPath), { recursive: true });
    writeFileSync(stateMdPath, "# STATE\n\nSome unrelated user content.\n");
    writeStateMd(root, demo());
    const body = readFileSync(stateMdPath, "utf8");
    expect(body).toContain("Some unrelated user content.");
    expect(body).toContain("## GS run: demo");
  });

  test("updates idempotently on repeated checkpoints without duplicating the block or clobbering user text", () => {
    const stateMdPath = join(root, STATE_MD_REL);
    mkdirSync(dirname(stateMdPath), { recursive: true });
    writeFileSync(stateMdPath, "# STATE\n\nKeep me.\n");
    const s = demo();
    writeStateMd(root, s);
    s.stage = "plan";
    writeStateMd(root, s);
    const body = readFileSync(stateMdPath, "utf8");
    expect(body).toContain("Keep me.");
    expect(body.match(/8sync:gs:begin/g)?.length).toBe(1);
    expect(body).toContain("Stage: plan");
  });

  test("saveCheckpoint writes the managed STATE.md block", () => {
    const stateMdPath = join(root, STATE_MD_REL);
    mkdirSync(dirname(stateMdPath), { recursive: true });
    writeFileSync(stateMdPath, "# STATE\n\nRoot notes.\n");
    const s = demo();
    setPlan(s, samplePlan());
    saveCheckpoint(root, s);
    const body = readFileSync(stateMdPath, "utf8");
    expect(body).toContain("Root notes.");
    expect(body).toContain("## GS run: demo");
  });

  test("creates STATE.md when absent", () => {
    writeStateMd(root, demo());
    const body = readFileSync(join(root, STATE_MD_REL), "utf8");
    expect(body).toContain("## GS run: demo");
  });
});

describe("legacy import provenance", () => {
  function writeLegacy(content: unknown): void {
    mkdirSync(dirname(join(root, LEGACY_REL)), { recursive: true });
    writeFileSync(join(root, LEGACY_REL), JSON.stringify(content));
  }

  test("does not re-import the same legacy content after the imported run is archived", () => {
    writeLegacy({ goal: "g", slices: [] });
    const first = resume(root);
    expect(first.source).toBe("legacy");
    expect(existsSync(join(root, LEGACY_IMPORT_MARKER_REL))).toBe(true);
    const rollbackCopy = readFileSync(join(root, LEGACY_COPY_REL), "utf8");

    clearActive(root);
    const second = resume(root);
    expect(second.source).not.toBe("legacy");
    expect(second.source).toBe("none");
    // The rollback copy from the original import is untouched.
    expect(readFileSync(join(root, LEGACY_COPY_REL), "utf8")).toBe(rollbackCopy);
  });

  test("re-imports when the legacy source content genuinely changes after an archived import", () => {
    writeLegacy({ goal: "g1", slices: [] });
    expect(resume(root).source).toBe("legacy");
    clearActive(root);
    writeLegacy({ goal: "g2", slices: [] });
    const r = resume(root);
    expect(r.source).toBe("legacy");
    expect(r.state?.goal).toBe("g2");
  });

  test("calling importLegacyEngine directly a second time with unchanged content is a no-op", () => {
    writeLegacy({ goal: "g", slices: [] });
    const s1 = importLegacyEngine(root, "2026-01-01T00:00:00Z");
    expect(s1).toBeDefined();
    const s2 = importLegacyEngine(root, "2026-01-02T00:00:00Z");
    expect(s2).toBeUndefined();
  });
});
