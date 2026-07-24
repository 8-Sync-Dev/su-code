// 8sync-gs — pure terminal status rendering. Two lines: a stage/progress
// summary and the exact next action. The adapter feeds these to setWidget.

import { counts } from "./machine.ts";
import type { GsState } from "./types.ts";

export const WIDGET_KEY = "8sync-gs";

function shortModel(id: string | undefined): string {
  if (!id) return "—";
  const tail = id.split("/").pop() ?? id;
  return tail.length > 22 ? `…${tail.slice(-20)}` : tail;
}

/** The one-line status-bar string. */
export function statusLine(state: GsState): string {
  const c = counts(state);
  return `GS ${state.stage} ${c.tasksPassed}/${c.tasksTotal} tasks · AC ${c.acPassed}/${c.acTotal}`;
}

/** The two-line widget content. */
export function widgetLines(state: GsState): string[] {
  const c = counts(state);
  const model = shortModel(state.activeCoordinator?.id);
  const fallback = state.activeCoordinator?.isFallback ? " (fallback)" : "";
  const head = `GS  ${state.stage}  ${c.tasksPassed}/${c.tasksTotal} tasks  AC ${c.acPassed}/${c.acTotal}  ${state.mode}  model ${model}${fallback}`;
  const p = state.pendingAction;
  let next: string;
  if (state.status === "done") {
    next = "done — run complete";
  } else if (state.status === "blocked") {
    next = `blocked — ${state.audit[state.audit.length - 1]?.detail ?? "see /gs status"}`;
  } else if (state.status === "aborted") {
    next = "aborted";
  } else if (p) {
    const who = p.agent ? `${p.agent}${p.modelSelector ? ` (${p.modelSelector})` : ""}` : p.kind;
    next = `next  ${who} · ${trim(p.instruction, 90)}`;
  } else {
    next = "next  call gs_status";
  }
  return [head, next];
}

function trim(s: string, n: number): string {
  return s.length > n ? `${s.slice(0, n - 1)}…` : s;
}
