// Slice C smoke: load the extension with a fake `pi`/`ctx` (no full omp) and
// drive the /gs command + tools + hooks end to end. Proves the adapter wiring
// parses, registers, routes evidence, and enforces the agent gate.
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import gsExtension from "../../assets/extensions/8sync-gs/index.ts";
import { loadRuntime, saveRuntime } from "../../assets/extensions/8sync-gs/store.ts";

// Hermetic chainable `z` stub: the adapter only builds parameter schemas at
// registration time; this test calls execute() with pre-shaped params, so the
// schema is never asked to validate. Avoids a zod dependency in the test.
function makeZ(): unknown {
  const chain: unknown = new Proxy(
    () => chain,
    {
      get: () => (..._a: unknown[]) => chain,
      apply: () => chain,
    },
  );
  return new Proxy(
    {},
    {
      get: () => (..._a: unknown[]) => chain,
    },
  );
}
interface Tool {
  name: string;
  execute: (id: string, params: unknown, sig: unknown, u: unknown, ctx: unknown) => Promise<{ content: { text: string }[] }>;
}

function harness(root: string) {
  const tools = new Map<string, Tool>();
  const hooks = new Map<string, (event: unknown, ctx: unknown) => unknown>();
  let command: ((args: string, ctx: unknown) => Promise<void>) | undefined;
  const sent: string[] = [];
  const notified: string[] = [];

  const models = [
    { provider: "anthropic", id: "claude-opus-4-8" },
    { provider: "anthropic", id: "claude-sonnet-5" },
    { provider: "openai", id: "gpt-5.5" },
    { provider: "zai", id: "glm-5.2" },
  ];
  const resolveMap: Record<string, (typeof models)[number]> = {
    "@default": models[3],
    "@slow": models[0],
    "@plan": models[0],
    "@task": models[1],
  };
  const familyOf = (m: { provider: string }) => (m.provider === "anthropic" ? "claude" : m.provider === "openai" ? "gpt" : "glm");
  const modelsFacade = {
    list: () => models,
    current: () => models[3],
    resolve: (sel: string) => resolveMap[sel],
    family: familyOf,
  };

  const pi = {
    zod: { z: makeZ() },
    setLabel: () => {},
    registerTool: (t: Tool) => tools.set(t.name, t),
    registerCommand: (_name: string, opts: { handler: (args: string, ctx: unknown) => Promise<void> }) => {
      command = opts.handler;
    },
    on: (event: string, handler: (event: unknown, ctx: unknown) => unknown) => hooks.set(event, handler),
    setModel: async () => true,
    setThinkingLevel: () => {},
    getThinkingLevel: () => "high",
    sendUserMessage: (msg: string) => sent.push(msg),
    appendEntry: () => {},
  };

  const ui = { notify: (message: string) => notified.push(message), setWidget: () => {}, setStatus: () => {} };
  const baseCtx = { cwd: root, hasUI: false, ui, models: modelsFacade, model: models[3] };
  const toolCtx = { ...baseCtx };
  const commandCtx = { ...baseCtx, waitForIdle: async () => {} };

  // biome-ignore lint: test harness intentionally uses loose typing for fakes.
  gsExtension(pi as never);

  return { tools, hooks, command: command!, sent, notified, toolCtx, commandCtx };
}

let root: string;
beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "gs-adapter-"));
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

describe("registration", () => {
  test("registers eight gs_* tools, the /gs command, and lifecycle hooks", () => {
    const h = harness(root);
    for (const name of ["gs_status", "gs_define", "gs_plan", "gs_next", "gs_verify", "gs_acceptance", "gs_advance", "gs_worktree"]) {
      expect(h.tools.has(name)).toBe(true);
    }
    expect(typeof h.command).toBe("function");
    for (const ev of ["tool_call", "tool_result", "session_shutdown", "session_before_compact"]) {
      expect(h.hooks.has(ev)).toBe(true);
    }
  });
});

describe("command + tool flow (auto mode)", () => {
  test("start → define → advance → plan → next → evidence → advance", async () => {
    const h = harness(root);
    await h.command("--auto add a fixture behavior", h.commandCtx);
    let s = loadRuntime(root);
    expect(s?.stage).toBe("clarify");
    expect(s?.mode).toBe("auto");

    const call = (name: string, params: unknown) => h.tools.get(name)!.execute("tc", params, undefined, undefined, h.toolCtx);

    const status = await call("gs_status", {});
    expect(status.content[0].text).toContain("clarify");

    await call("gs_define", {
      requirements: [{ id: "R1", text: "do the thing", kind: "functional", required: true, status: "confirmed" }],
      acceptance: [{ id: "AC1", text: "thing works", requirements: ["R1"], method: "verify", required: true }],
      risk: { trivial: true },
    });
    const adv1 = await call("gs_advance", {});
    expect(adv1.content[0].text).toContain("plan");
    s = loadRuntime(root);
    expect(s?.stage).toBe("plan");

    await call("gs_plan", {
      slices: [
        {
          id: "s1",
          title: "wave 1",
          tasks: [{ id: "t1", title: "impl", acceptance: ["AC1"], ownership: ["a.ts"], dependsOn: [], skills: [], verify: [{ program: "true", args: [] }] }],
        },
      ],
    });

    const next = await call("gs_next", {});
    expect(next.content[0].text).toContain("gs-planner");
    s = loadRuntime(root);
    expect(s?.pendingAction?.agent).toBe("gs-planner");

    // tool_call hook: wrong agent at plan stage is blocked.
    const wrong = h.hooks.get("tool_call")!({ toolName: "task", toolCallId: "x", input: { tasks: [{ agent: "gs-hacker" }] } }, h.toolCtx);
    expect((wrong as { block?: boolean } | undefined)?.block).toBe(true);

    const leased = loadRuntime(root)?.pendingAction;
    const missingHash = h.hooks.get("tool_call")!(
      {
        toolName: "task",
        toolCallId: "missing-hash",
        input: { tasks: [{ agent: "gs-planner", model: leased?.modelSelector, task: "plan without lease binding" }] },
      },
      h.toolCtx,
    ) as { block?: boolean };
    expect(missingHash.block).toBe(true);
    const wrongModel = h.hooks.get("tool_call")!(
      {
        toolName: "task",
        toolCallId: "wrong-model",
        input: {
          tasks: [{ agent: "gs-planner", model: "@default", task: `Assignment hash: ${leased?.assignmentHash}` }],
        },
      },
      h.toolCtx,
    ) as { block?: boolean };
    expect(wrongModel.block).toBe(true);

    // The exact leased agent/model/hash passes and binds this tool call.
    const pending = loadRuntime(root)?.pendingAction;
    const okCall = h.hooks.get("tool_call")!(
      {
        toolName: "task",
        toolCallId: "x",
        input: {
          tasks: [
            {
              agent: "gs-planner",
              model: pending?.modelSelector,
              task: `Assignment hash: ${pending?.assignmentHash}`,
            },
          ],
        },
      },
      h.toolCtx,
    );
    expect(okCall).toBeUndefined();

    // A stale result cannot satisfy the now-bound lease.
    h.hooks.get("tool_result")!(
      {
        toolName: "task",
        toolCallId: "stale",
        isError: false,
        details: { results: [{ agent: "gs-planner", resolvedModel: pending?.modelId, exitCode: 0, structuredOutput: {} }] },
      },
      h.toolCtx,
    );
    expect(loadRuntime(root)?.pendingAction?.toolCallId).toBe("x");

    // A wrong observed agent/model also leaves the lease outstanding.
    h.hooks.get("tool_result")!(
      {
        toolName: "task",
        toolCallId: "x",
        isError: false,
        details: { results: [{ agent: "gs-critic", resolvedModel: "openai/gpt-5.5", exitCode: 0, structuredOutput: {} }] },
      },
      h.toolCtx,
    );
    expect(loadRuntime(root)?.pendingAction?.toolCallId).toBe("x");

    // The exact result records evidence + clears pending.
    h.hooks.get("tool_result")!(
      {
        toolName: "task",
        toolCallId: "x",
        isError: false,
        details: { results: [{ id: "planner", agent: "gs-planner", resolvedModel: pending?.modelId, exitCode: 0, structuredOutput: {} }] },
      },
      h.toolCtx,
    );
    s = loadRuntime(root);
    expect(s?.stages.plan.agentRuns.length).toBe(1);
    expect(s?.stages.plan.agentRuns[0].batchToolCallId).toBe("x");
    expect(s?.pendingAction).toBeUndefined();

    const adv2 = await call("gs_advance", {});
    expect(adv2.content[0].text).toContain("plan_review");
  });

  test("edit before an approved plan is blocked by the tool_call hook", async () => {
    const h = harness(root);
    await h.command("add a thing", h.commandCtx); // assisted mode
    const res = h.hooks.get("tool_call")!({ toolName: "write", toolCallId: "w", input: { path: "x", content: "y" } }, h.toolCtx);
    expect((res as { block?: boolean }).block).toBe(true);
  });

  test("gs_verify requires current worker evidence and attaches objective verify evidence", async () => {
    const h = harness(root);
    await h.command("--auto verify a fixture", h.commandCtx);
    const call = (name: string, params: unknown) => h.tools.get(name)!.execute("tc", params, undefined, undefined, h.toolCtx);
    const early = await call("gs_verify", { taskId: "t1" });
    expect(early.content[0].text).toContain("gated");

    const s = loadRuntime(root)!;
    s.stage = "implement";
    s.stages.implement.status = "running";
    s.requirements = [{ id: "R1", text: "fixture works", kind: "functional", required: true, status: "confirmed" }];
    s.acceptance = [
      { id: "AC1", text: "fixture command passes", requirements: ["R1"], method: "verify", required: true, status: "pending", evidence: [] },
    ];
    s.plan = {
      hash: "approved-plan",
      createdAt: new Date().toISOString(),
      slices: [
        {
          id: "s1",
          title: "fixture",
          tasks: [
            {
              id: "t1",
              title: "fixture",
              acceptance: ["AC1"],
              ownership: ["fixture.ts"],
              dependsOn: [],
              skills: [],
              verify: [{ program: "true", args: [] }],
              status: "pending",
              attempts: 0,
              failStreak: 0,
              lastFailureHash: "",
              note: "",
            },
          ],
        },
      ],
    };
    s.approvals.push({ what: "plan", by: "critic", at: new Date().toISOString() });
    s.stages.implement.agentRuns.push({
      toolCallId: "worker:t1",
      batchToolCallId: "worker",
      agent: "gs-worker",
      taskId: "t1",
      assignmentHash: "worker-lease",
      requestedSelector: "family:sonnet",
      resolvedModel: "anthropic/claude-sonnet-5",
      resolvedModelFamily: "claude",
      resolvedModelIsFallback: false,
      exitCode: 0,
      endedAt: new Date().toISOString(),
    });
    saveRuntime(root, s);

    const verified = await call("gs_verify", { taskId: "t1" });
    expect(verified.content[0].text).toContain("VERIFIED t1");
    const after = loadRuntime(root)!;
    expect(after.acceptance[0].status).toBe("passed");
    expect(after.acceptance[0].evidence[0].kind).toBe("verify");

    after.stage = "uat";
    after.stages.uat.status = "running";
    saveRuntime(root, after);
    const incompatible = await call("gs_acceptance", {
      acId: "AC1",
      evidence: { kind: "note", ref: "model prose" },
      status: "passed",
    });
    expect(incompatible.content[0].text).toContain("incompatible");
  });

  test("commit remains blocked before verification, review, and UAT", async () => {
    const h = harness(root);
    await h.command("change a fixture", h.commandCtx);
    const result = h.hooks.get("tool_call")!(
      { toolName: "bash", toolCallId: "commit", input: { command: "git commit -m fixture" } },
      h.toolCtx,
    ) as { block?: boolean; reason?: string };
    expect(result.block).toBe(true);
    expect(result.reason).toContain("review/security");
  });

  test("one-shot action approval is exact and generic UAT cannot authorize bash", async () => {
    const h = harness(root);
    await h.command("safely update a fixture", h.commandCtx);
    await h.command("approve uat", h.commandCtx);
    expect(h.notified.some((message) => message.includes("only valid at uat"))).toBe(true);

    const event = { toolName: "bash", toolCallId: "b1", input: { command: "rm -rf /tmp/gs-action-fixture" } };
    const first = h.hooks.get("tool_call")!(event, h.toolCtx) as { block?: boolean; reason?: string };
    expect(first.block).toBe(true);
    const hash = first.reason?.match(/approve action ([0-9a-f]+)/)?.[1];
    expect(hash).toBeTruthy();

    await h.command(`approve action ${hash}`, h.commandCtx);
    expect(h.hooks.get("tool_call")!(event, h.toolCtx)).toBeUndefined();
    const replay = h.hooks.get("tool_call")!(event, h.toolCtx) as { block?: boolean };
    expect(replay.block).toBe(true);
  });

  test("refuses to overwrite an active run and rejects unsupported live benchmark", async () => {
    const h = harness(root);
    await h.command("first goal", h.commandCtx);
    await h.command("second goal", h.commandCtx);
    expect(loadRuntime(root)?.goal).toBe("first goal");
    expect(h.notified.some((message) => message.includes("already active"))).toBe(true);

    await h.command("benchmark --live", h.commandCtx);
    expect(h.sent.some((message) => message.includes("explicitly unsupported"))).toBe(true);
  });

  test("/gs benchmark reports a pass through the command", async () => {
    const h = harness(root);
    await h.command("benchmark", h.commandCtx);
    expect(h.sent.some((m) => m.includes("contract benchmark"))).toBe(true);
  });
});
