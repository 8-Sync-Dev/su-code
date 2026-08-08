// 8sync-engine — the automation gates omp core does NOT ship, and nothing else.
//
// Everything omp already has was removed from this extension (M4/UC-16):
//   - the plan/slice/task machine as a *planning* surface → omp's `todo`
//     (phases + tasks + `todo_reminder` + `user_todo_edit` restored on resume).
//     What survives here is only the durable ledger the gates need, which is
//     also the `8sync harness web` dashboard's read model — so the on-disk
//     `{goal, updatedAt, slices[].tasks[]}` shape is a contract, not decoration.
//   - `engine_worktree` → omp's `task.isolation.mode` (auto/apfs/btrfs/zfs/
//     reflink/overlayfs/projfs/rcopy via the pi-iso PAL; branch `omp/task/<id>`).
//   - the fnv1a identical-failure doom-loop guard → the TTSR `repeatMode` /
//     `repeatGap` rule 8sync deploys to `~/.omp/agent/rules/`.
//   - `/auto`'s "in an autonomous run, do not yield between tasks" prose → the
//     `session_stop` continuation below (code, not a wish).
//
// What is genuinely additive and therefore stays:
//   1. engine_verify runs the task's verify commands and records the verdict;
//      engine_advance REFUSES a task whose commands never passed — the agent's
//      own "done" is not a stop signal.
//   2. a gitleaks staged-diff gate before every autonomous commit (omp's
//      `security_scan` is a full OAuth-pinned review, not a staged-secret check).
//   3. the run-to-done loop, enforced via `session_stop`.
// It never patches omp: it loads from `~/.omp/agent/extensions/` (global) and
// `<root>/.omp/extensions/` (project), so omp updates stay safe.
import type { ExtensionAPI } from "@oh-my-pi/pi-coding-agent";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

interface EngineTask {
  id: string;
  title: string;
  status: "pending" | "in_progress" | "done" | "blocked";
  retries: number;
  verified: boolean;
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
const MAX_OUTPUT = 2000;

export default function (pi: ExtensionAPI) {
  const { z } = pi.zod;
  pi.setLabel("8sync engine (verify gate · secret gate · run-to-done loop)");

  const stateSchema = z.object({
    goal: z.string(),
    createdAt: z.string(),
    updatedAt: z.string(),
    maxRetries: z.number(),
    slices: z.array(
      z.object({
        id: z.string(),
        title: z.string(),
        tasks: z.array(
          z.object({
            id: z.string(),
            title: z.string(),
            status: z.enum(["pending", "in_progress", "done", "blocked"]),
            retries: z.number(),
            verified: z.boolean().default(false),
            verify: z.array(z.string()),
            note: z.string(),
          }),
        ),
      }),
    ),
  });

  function load(): EngineState | null {
    const p = join(process.cwd(), STATE_REL);
    if (!existsSync(p)) return null;
    try {
      const parsed = stateSchema.safeParse(JSON.parse(readFileSync(p, "utf8")));
      return parsed.success ? (parsed.data as EngineState) : null;
    } catch {
      return null;
    }
  }

  function save(state: EngineState): void {
    state.updatedAt = new Date().toISOString();
    mkdirSync(join(process.cwd(), ".cache/8sync/engine"), { recursive: true });
    writeFileSync(join(process.cwd(), STATE_REL), JSON.stringify(state, null, 2));
  }

  function counts(state: EngineState): { total: number; done: number; blocked: number } {
    let total = 0;
    let done = 0;
    let blocked = 0;
    for (const s of state.slices) {
      for (const t of s.tasks) {
        total += 1;
        if (t.status === "done") done += 1;
        else if (t.status === "blocked") blocked += 1;
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

  function find(state: EngineState, id: string): EngineTask | undefined {
    for (const s of state.slices) for (const t of s.tasks) if (t.id === id) return t;
    return undefined;
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

  // The run-to-done loop, armed only once THIS session has actually driven the
  // engine — a session that merely happens to sit in a repo with a stale plan is
  // never hijacked. omp awaits `session_stop` before the main session settles,
  // never fires it for subagents, and hard-caps consecutive continuations at 8.
  let armed = false;

  pi.registerTool({
    name: "engine_plan",
    label: "Engine: plan",
    description:
      "Record the run-to-done plan: a goal, its slices, each slice's atomic tasks and their verify commands (the project's real lint/test/build). This is the durable gate ledger at .cache/8sync/engine/state.json — mirror the same tasks into `todo` for turn-by-turn tracking.",
    parameters: z.object({
      goal: z.string(),
      maxRetries: z.number().int().min(0).max(10).default(3),
      slices: z
        .array(
          z.object({
            title: z.string(),
            tasks: z.array(
              z.object({ title: z.string(), verify: z.array(z.string()).default(() => []) }),
            ),
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
              return {
                id: `s${si}.t${ti}`,
                title: t.title,
                status: "pending" as const,
                retries: 0,
                verified: false,
                verify: t.verify,
                note: "",
              };
            }),
          };
        }),
      };
      save(state);
      armed = true;
      const c = counts(state);
      return text(
        `Plan saved: "${params.goal}" — ${state.slices.length} slices, ${c.total} tasks. Call engine_next to start.`,
      );
    },
  });

  pi.registerTool({
    name: "engine_status",
    label: "Engine: status",
    description: "Report the plan: per-slice task statuses + progress (done/total, blocked).",
    parameters: z.object({}),
    async execute() {
      const state = load();
      if (!state) return text("No plan yet. Call engine_plan first.");
      const c = counts(state);
      const lines = [`Goal: ${state.goal}`, `Progress: ${c.done}/${c.total} done, ${c.blocked} blocked`, ""];
      for (const s of state.slices) {
        lines.push(`# ${s.id} ${s.title}`);
        for (const t of s.tasks) {
          const why = t.status === "blocked" && t.note ? ` — ${t.note}` : "";
          lines.push(`  [${t.status}] ${t.id} ${t.title}${t.retries ? ` (retries:${t.retries})` : ""}${why}`);
        }
      }
      return text(lines.join("\n"));
    },
  });

  pi.registerTool({
    name: "engine_next",
    label: "Engine: next task",
    description:
      "Return the next unfinished task (with its slice) and mark it in_progress. Reports DONE when every task is done/blocked.",
    parameters: z.object({}),
    async execute() {
      const state = load();
      if (!state) return text("No plan yet. Call engine_plan first.");
      armed = true;
      const next = findNext(state);
      if (!next) {
        armed = false;
        const c = counts(state);
        return text(
          c.blocked
            ? `All tasks resolved but ${c.blocked} BLOCKED — review engine_status.`
            : "DONE — every task is complete.",
        );
      }
      next.task.status = "in_progress";
      save(state);
      const verify = next.task.verify.length ? `\nVerify with: ${next.task.verify.join(" && ")}` : "";
      return text(
        `NEXT ${next.task.id} (slice ${next.slice.title}): ${next.task.title}${verify}\nImplement it, then call engine_verify, then engine_advance.`,
      );
    },
  });

  pi.registerTool({
    name: "engine_verify",
    label: "Engine: verify (the gate)",
    description:
      "Run the task's verify commands (or the ones passed). ALL must pass. Each failure increments the retry counter; at maxRetries the task is BLOCKED. The verdict is recorded in code — engine_advance refuses a task this never passed for.",
    parameters: z.object({ taskId: z.string(), commands: z.array(z.string()).optional() }),
    async execute(_id, params) {
      const state = load();
      if (!state) return text("No plan yet. Call engine_plan first.");
      const target = find(state, params.taskId);
      if (!target) return text(`No task ${params.taskId}.`);
      const cmds = params.commands?.length ? params.commands : target.verify;
      if (!cmds.length) {
        return text(`Task ${target.id} has no verify commands — add some or advance manually if truly trivial.`);
      }

      const failures: string[] = [];
      for (const cmd of cmds) {
        const r = run(cmd);
        if (!r.ok) failures.push(`$ ${cmd}\n${r.output}`);
      }
      if (!failures.length) {
        target.verified = true;
        target.note = "";
        save(state);
        return text(`VERIFIED ${target.id}: all ${cmds.length} checks passed. Call engine_advance.`);
      }

      target.verified = false;
      target.retries += 1;
      const detail = failures.join("\n\n");
      if (target.retries >= state.maxRetries) {
        target.status = "blocked";
        target.note = `blocked after ${target.retries} failed verifies`;
        save(state);
        return text(
          `BLOCKED ${target.id} after ${target.retries} attempts (maxRetries=${state.maxRetries}). Record a failure: in su-code/KNOWLEDGE.md and move on / escalate.\n\n${detail}`,
        );
      }
      save(state);
      return text(
        `FAILED ${target.id} (attempt ${target.retries}/${state.maxRetries}). Fix the CAUSE with a different approach, then call engine_verify again:\n\n${detail}`,
      );
    },
  });

  pi.registerTool({
    name: "engine_advance",
    label: "Engine: advance",
    description:
      "Mark a verified task done and optionally commit it. REFUSES a task with verify commands but no passing engine_verify run. With commit:true a gitleaks staged-diff gate runs first — a finding aborts the commit and unstages.",
    parameters: z.object({
      taskId: z.string(),
      commit: z.boolean().default(false),
      message: z.string().optional(),
    }),
    async execute(_id, params) {
      const state = load();
      if (!state) return text("No plan yet. Call engine_plan first.");
      const target = find(state, params.taskId);
      if (!target) return text(`No task ${params.taskId}.`);
      if (target.verify.length && !target.verified) {
        return text(
          `REFUSED ${target.id}: it has ${target.verify.length} verify command(s) but no passing engine_verify run. The gate is code-enforced — call engine_verify {taskId:"${target.id}"} first.`,
        );
      }
      target.status = "done";
      save(state);
      armed = true;
      let committed = "";
      if (params.commit) {
        run("git add -A");
        // Secret gate before every autonomous commit — same check as the 8sync
        // pre-commit hook. gitleaks absent → the `if` runs no branch and exits 0
        // (best-effort skip); present + a finding → non-zero → abort and unstage.
        const scan = run("if command -v gitleaks >/dev/null 2>&1; then gitleaks protect --staged --no-banner; fi");
        if (!scan.ok) {
          run("git reset");
          committed = `\nCommit ABORTED: gitleaks flagged a secret in the staged diff. Task is done; resolve the leak, then commit manually.\n${scan.output}`;
        } else {
          const msg = params.message ?? `feat: ${target.title}`;
          const r = run(`git commit -m ${JSON.stringify(msg)}`);
          committed = r.ok ? `\nCommitted: ${msg}` : `\nCommit skipped/failed: ${r.output}`;
        }
      }
      const c = counts(state);
      return text(`DONE ${target.id}. Progress ${c.done}/${c.total}.${committed}\nCall engine_next for the next task.`);
    },
  });

  pi.on("session_stop", async () => {
    if (!armed) return undefined;
    const state = load();
    if (!state) return undefined;
    const next = findNext(state);
    if (!next) {
      armed = false;
      return undefined;
    }
    const c = counts(state);
    return {
      continue: true,
      additionalContext:
        `8sync engine: the run is NOT done — ${c.done}/${c.total} tasks complete, ${c.blocked} blocked. ` +
        `Next up is ${next.task.id} "${next.task.title}" (slice: ${next.slice.title}). ` +
        `Call engine_next and keep working; stop only when engine_next reports DONE, every remaining task is BLOCKED, or you hit a true blocker (missing credential, irreversible action).`,
    };
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
