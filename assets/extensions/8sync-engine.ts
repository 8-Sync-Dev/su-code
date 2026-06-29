// 8sync-engine — a gsd-pi-style automation engine built 100% on omp core.
//
// It does NOT patch omp: it lives in omp's config dir (~/.omp/agent/extensions/
// global + <root>/.omp/extensions/ project, auto-loaded by the native extension
// provider) and exposes model-callable TOOLS that carry the parts gsd-pi enforces
// in CODE rather than prose:
//   - a durable milestone/slice/task state machine (JSON at .cache/8sync/engine/),
//   - verify-with-auto-retry (the tool runs the commands + counts attempts +
//     blocks a task once max retries is hit — the agent can't skip the gate),
//   - git worktree open / squash-merge / remove (code, not "please run git").
// The `/auto` slash command orchestrates these tools into a run-to-done loop.
// This is THE automation path: a single `/auto` command (no competing `/gs`).
import type { ExtensionAPI } from "@oh-my-pi/pi-coding-agent";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

type TaskStatus = "pending" | "in_progress" | "done" | "blocked";

interface EngineTask {
  id: string;
  title: string;
  status: TaskStatus;
  retries: number;
  verify: string[];
  note: string;
}
interface EngineSlice {
  id: string;
  title: string;
  tasks: EngineTask[];
}
interface EngineState {
  goal: string;
  createdAt: string;
  updatedAt: string;
  maxRetries: number;
  slices: EngineSlice[];
}

const STATE_REL = ".cache/8sync/engine/state.json";
const WT_REL = ".cache/8sync/engine/wt";
const MAX_OUTPUT = 2000;

export default function (pi: ExtensionAPI) {
  const { z } = pi.zod;
  pi.setLabel("8sync engine (gsd-pi-style, on omp core)");

  const taskSchema = z.object({
    id: z.string(),
    title: z.string(),
    status: z.enum(["pending", "in_progress", "done", "blocked"]),
    retries: z.number(),
    verify: z.array(z.string()),
    note: z.string(),
  });
  const stateSchema = z.object({
    goal: z.string(),
    createdAt: z.string(),
    updatedAt: z.string(),
    maxRetries: z.number(),
    slices: z.array(z.object({ id: z.string(), title: z.string(), tasks: z.array(taskSchema) })),
  });

  function load(): EngineState | null {
    const p = join(process.cwd(), STATE_REL);
    if (!existsSync(p)) return null;
    try {
      const parsed = stateSchema.safeParse(JSON.parse(readFileSync(p, "utf8")));
      return parsed.success ? parsed.data : null;
    } catch {
      return null;
    }
  }

  function save(state: EngineState): void {
    state.updatedAt = new Date().toISOString();
    const p = join(process.cwd(), STATE_REL);
    mkdirSync(join(process.cwd(), ".cache/8sync/engine"), { recursive: true });
    writeFileSync(p, JSON.stringify(state, null, 2));
  }

  function counts(state: EngineState): { total: number; done: number; blocked: number } {
    let total = 0;
    let done = 0;
    let blocked = 0;
    for (const s of state.slices) {
      for (const t of s.tasks) {
        total += 1;
        if (t.status === "done") done += 1;
        if (t.status === "blocked") blocked += 1;
      }
    }
    return { total, done, blocked };
  }

  function findNext(state: EngineState): { slice: EngineSlice; task: EngineTask } | null {
    for (const s of state.slices) {
      for (const t of s.tasks) {
        if (t.status === "pending" || t.status === "in_progress") return { slice: s, task: t };
      }
    }
    return null;
  }

  function run(cmd: string): { ok: boolean; output: string } {
    const r = spawnSync("bash", ["-lc", cmd], { cwd: process.cwd(), encoding: "utf8" });
    const raw = `${r.stdout ?? ""}${r.stderr ?? ""}`.trim();
    const output = raw.length > MAX_OUTPUT ? `${raw.slice(0, MAX_OUTPUT)}\n…[truncated]` : raw;
    return { ok: r.status === 0, output };
  }

  function text(s: string) {
    return { content: [{ type: "text" as const, text: s }] };
  }

  pi.registerTool({
    name: "engine_plan",
    label: "Engine: plan",
    description:
      "Create/replace the run-to-done plan: a goal decomposed into slices, each with atomic tasks and optional verify commands (lint/test). Persists to .cache/8sync/engine/state.json.",
    parameters: z.object({
      goal: z.string(),
      maxRetries: z.number().int().min(0).max(10).default(3),
      slices: z
        .array(
          z.object({
            title: z.string(),
            tasks: z.array(z.object({ title: z.string(), verify: z.array(z.string()).default([]) })),
          }),
        )
        .min(1),
    }),
    async execute(_id, params) {
      const now = new Date().toISOString();
      let si = 0;
      const state: EngineState = {
        goal: params.goal,
        createdAt: now,
        updatedAt: now,
        maxRetries: params.maxRetries,
        slices: params.slices.map((s) => {
          si += 1;
          let ti = 0;
          return {
            id: `s${si}`,
            title: s.title,
            tasks: s.tasks.map((t) => {
              ti += 1;
              return { id: `s${si}.t${ti}`, title: t.title, status: "pending", retries: 0, verify: t.verify, note: "" };
            }),
          };
        }),
      };
      save(state);
      const c = counts(state);
      return text(`Plan saved: "${params.goal}" — ${state.slices.length} slices, ${c.total} tasks. Call engine_next to start.`);
    },
  });

  pi.registerTool({
    name: "engine_status",
    label: "Engine: status",
    description: "Report the current plan: per-slice task statuses + overall progress (done/total, blocked).",
    parameters: z.object({}),
    async execute() {
      const state = load();
      if (!state) return text("No plan yet. Call engine_plan first.");
      const c = counts(state);
      const lines = [`Goal: ${state.goal}`, `Progress: ${c.done}/${c.total} done, ${c.blocked} blocked`, ""];
      for (const s of state.slices) {
        lines.push(`# ${s.id} ${s.title}`);
        for (const t of s.tasks) lines.push(`  [${t.status}] ${t.id} ${t.title}${t.retries ? ` (retries:${t.retries})` : ""}`);
      }
      return text(lines.join("\n"));
    },
  });

  pi.registerTool({
    name: "engine_next",
    label: "Engine: next task",
    description: "Return the next pending task (with its slice) and mark it in_progress. Returns done when every task is done/blocked.",
    parameters: z.object({}),
    async execute() {
      const state = load();
      if (!state) return text("No plan yet. Call engine_plan first.");
      const next = findNext(state);
      if (!next) {
        const c = counts(state);
        return text(c.blocked ? `All tasks resolved but ${c.blocked} BLOCKED — review engine_status.` : "DONE — every task is complete.");
      }
      next.task.status = "in_progress";
      save(state);
      const verify = next.task.verify.length ? `\nVerify with: ${next.task.verify.join(" && ")}` : "";
      return text(`NEXT ${next.task.id} (slice ${next.slice.title}): ${next.task.title}${verify}\nImplement it, then call engine_verify, then engine_advance.`);
    },
  });

  pi.registerTool({
    name: "engine_verify",
    label: "Engine: verify (auto-retry gate)",
    description:
      "Run the task's verify commands (or the ones passed). All must pass to advance. On failure the retry counter increments; once it reaches maxRetries the task is BLOCKED. The gate is code-enforced — you cannot advance an unverified task.",
    parameters: z.object({ taskId: z.string(), commands: z.array(z.string()).optional() }),
    async execute(_id, params) {
      const state = load();
      if (!state) return text("No plan yet. Call engine_plan first.");
      let target: EngineTask | undefined;
      for (const s of state.slices) for (const t of s.tasks) if (t.id === params.taskId) target = t;
      if (!target) return text(`No task ${params.taskId}.`);
      const cmds = params.commands?.length ? params.commands : target.verify;
      if (!cmds.length) return text(`Task ${target.id} has no verify commands — add some or advance manually if truly trivial.`);

      const failures: string[] = [];
      for (const cmd of cmds) {
        const r = run(cmd);
        if (!r.ok) failures.push(`$ ${cmd}\n${r.output}`);
      }
      if (!failures.length) return text(`VERIFIED ${target.id}: all ${cmds.length} checks passed. Call engine_advance.`);

      target.retries += 1;
      if (target.retries >= state.maxRetries) {
        target.status = "blocked";
        target.note = `blocked after ${target.retries} failed verifies`;
        save(state);
        return text(`BLOCKED ${target.id} after ${target.retries} attempts (maxRetries=${state.maxRetries}). Record a failure: in agents/KNOWLEDGE.md and move on / escalate.\n\n${failures.join("\n\n")}`);
      }
      save(state);
      return text(`FAILED ${target.id} (attempt ${target.retries}/${state.maxRetries}). Fix the cause, then call engine_verify again:\n\n${failures.join("\n\n")}`);
    },
  });

  pi.registerTool({
    name: "engine_advance",
    label: "Engine: advance",
    description: "Mark a verified task done and optionally commit the change (gitleaks-free commit is the caller's responsibility). Advances the plan.",
    parameters: z.object({ taskId: z.string(), commit: z.boolean().default(false), message: z.string().optional() }),
    async execute(_id, params) {
      const state = load();
      if (!state) return text("No plan yet. Call engine_plan first.");
      let target: EngineTask | undefined;
      for (const s of state.slices) for (const t of s.tasks) if (t.id === params.taskId) target = t;
      if (!target) return text(`No task ${params.taskId}.`);
      target.status = "done";
      save(state);
      let committed = "";
      if (params.commit) {
        run("git add -A");
        const msg = params.message ?? `feat: ${target.title}`;
        const r = run(`git commit -m ${JSON.stringify(msg)}`);
        committed = r.ok ? `\nCommitted: ${msg}` : `\nCommit skipped/failed: ${r.output}`;
      }
      const c = counts(state);
      return text(`DONE ${target.id}. Progress ${c.done}/${c.total}.${committed}\nCall engine_next for the next task.`);
    },
  });

  pi.registerTool({
    name: "engine_worktree",
    label: "Engine: git worktree",
    description:
      "Isolate a slice in its own git worktree (open), squash-merge it back to the current branch (merge), or discard it (remove). open: git worktree add .cache/8sync/engine/wt/<slug> -b 8sync/<slug>.",
    parameters: z.object({ action: z.enum(["open", "merge", "remove"]), slug: z.string() }),
    async execute(_id, params) {
      const slug = params.slug.replace(/[^a-zA-Z0-9._-]/g, "-");
      const wt = join(WT_REL, slug);
      const branch = `8sync/${slug}`;
      if (params.action === "open") {
        mkdirSync(join(process.cwd(), WT_REL), { recursive: true });
        const r = run(`git worktree add ${wt} -b ${branch}`);
        return text(r.ok ? `Worktree ${wt} on ${branch}. cd there to work the slice.` : `worktree add failed:\n${r.output}`);
      }
      if (params.action === "merge") {
        const m = run(`git merge --squash ${branch}`);
        if (!m.ok) return text(`squash-merge failed (resolve, then retry):\n${m.output}`);
        const c = run(`git commit -m ${JSON.stringify(`merge ${branch} (squash)`)}`);
        run(`git worktree remove ${wt} --force`);
        run(`git branch -D ${branch}`);
        return text(c.ok ? `Squash-merged ${branch} and removed the worktree.` : `merged but commit failed:\n${c.output}`);
      }
      run(`git worktree remove ${wt} --force`);
      const r = run(`git branch -D ${branch}`);
      return text(`Removed worktree ${wt}${r.ok ? ` and branch ${branch}` : ""}.`);
    },
  });

  pi.registerCommand("engine", {
    description: "8sync engine status — show plan progress",
    handler: async (_args, ctx) => {
      const state = load();
      if (!state) {
        ctx.ui.notify("8sync engine: no plan. Use /auto <goal> or call engine_plan.", "info");
        return;
      }
      const c = counts(state);
      ctx.ui.notify(`8sync engine: ${c.done}/${c.total} done, ${c.blocked} blocked — goal: ${state.goal}`, "info");
    },
  });
}
