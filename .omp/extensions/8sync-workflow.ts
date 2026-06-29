// 8sync-workflow — gsd-pi-grade omp extension (no core patch).
//
// Deploys a model-callable workflow surface that survives omp updates: it lives
// in omp's config dir (~/.omp/agent/extensions/ global + <root>/.omp/extensions/
// project) and is auto-loaded by omp's native extension provider (prio 100). The
// `8sync harness web` Workflow page appends more `registerTool` blocks (exported
// workflows) to the project copy of this file.
//
// Surface (MVP): durable workflow state across compaction + a status command.
//   - wf_state_get / wf_state_set: read/write {goal, slice, progress}, persisted
//     via a custom session entry (pi.appendEntry) and rebuilt on session_start.
//   - /wf: human status line.
// Full gsd-pi parity (slice orchestration, auto loop, worktree mgmt) = more
// registerTool/handlers added to THIS same module; the surface is what ships.
import type { ExtensionAPI } from "@oh-my-pi/pi-coding-agent";

interface WfState {
  goal?: string;
  slice?: string;
  progress?: string;
}

const CUSTOM_TYPE = "8sync-wf-state";

// Per-session in-memory state. Rebuilt from the session branch on session_start
// so it survives compaction (the custom entry persists across the summary).
let state: WfState = {};

export default function (pi: ExtensionAPI) {
  const { z } = pi.zod;
  pi.setLabel("8sync workflow (gsd-pi-grade)");

  pi.on("session_start", async (_event, ctx) => {
    let latest: WfState | undefined;
    for (const entry of ctx.sessionManager.getBranch()) {
      if (entry.type === "custom" && entry.customType === CUSTOM_TYPE && entry.data) {
        latest = entry.data as WfState;
      }
    }
    if (latest) {
      state = { ...latest };
    }
  });

  pi.registerTool({
    name: "wf_state_get",
    label: "Workflow state get",
    description:
      "Read the current 8sync-workflow state (goal, slice, progress). Persists across compaction.",
    parameters: z.object({}),
    async execute(_toolCallId, _params, _signal, _onUpdate, _ctx) {
      return {
        content: [{ type: "text", text: JSON.stringify(state) }],
        details: { state },
      };
    },
  });

  pi.registerTool({
    name: "wf_state_set",
    label: "Workflow state set",
    description:
      "Update the 8sync-workflow state (goal/slice/progress). Merges partial fields. Persists across compaction via a custom session entry.",
    parameters: z.object({
      goal: z.string().optional(),
      slice: z.string().optional(),
      progress: z.string().optional(),
    }),
    async execute(_toolCallId, params, _signal, _onUpdate, _ctx) {
      state = { ...state, ...params };
      await pi.appendEntry(CUSTOM_TYPE, state);
      return {
        content: [
          { type: "text", text: `8sync-workflow state updated: ${JSON.stringify(state)}` },
        ],
        details: { state },
      };
    },
  });

  pi.registerCommand("wf", {
    description: "8sync workflow status — show current goal/slice/progress",
    handler: async (_args, ctx) => {
      ctx.ui.notify(
        `8sync-workflow: goal=${state.goal ?? "-"} slice=${state.slice ?? "-"} progress=${state.progress ?? "-"}`,
        "info",
      );
    },
  });
}
