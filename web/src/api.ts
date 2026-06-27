// Typed fetch wrappers for the 8sync harness web API. No `any` — every response
// is typed so the UI consumes a known shape.

export type EngineStatus = { present: boolean; version: string };
export type Engines = {
  codegraph: EngineStatus;
  cbm: EngineStatus;
  headroom: EngineStatus;
  serena: EngineStatus;
  mnemopi_on: boolean;
};
export type SkillEntry = {
  name: string;
  tier: "always" | "on-demand" | "off";
  source: string;
  global: boolean;
  local: boolean;
};
export type BenchReport = {
  upfront: number;
  deferred: number;
  a1_pass: boolean;
  a2_saved_pct: number;
  force_load_prefix: number;
};
export type RoleScore = { role: string; pct: number; detail: string[] };
export type EvalReport = { overall: number; total: number; present: number; roles: RoleScore[] };
export type StateInfo = { project: string; profile: string; state_md: string };
export type WorkspaceInfo = { profiles: string[]; project: string; session: string };
export type TeamInfo = {
  roster: { type: string; role: string; skills: string }[];
  readiness: EvalReport | null;
};
export type Submodule = { name: string; path: string; url: string; initialized: boolean };
export type ContextInfo = {
  used: number;
  window: number;
  pct: number;
  threshold_pct: number;
  compact_at: number;
  over_threshold: boolean;
  last_compact_at: number | null;
  compaction_observed: boolean;
  session: string;
  model: string;
  project: string;
  note: string;
};
export type McpServer = { name: string; command: string; args: string[]; type: string; present: boolean };
export type Rule = { scope: string; name: string; path: string; bytes: number };
export type WfKind = "step" | "subagent" | "tool";
export type WfData = { label: string; kind: WfKind; ref: string };
export type WfNode = { id: string; type?: string; position: { x: number; y: number }; data: WfData };
export type WfEdge = { id: string; source: string; target: string };
export type Workflow = { name: string; nodes: WfNode[]; edges: WfEdge[] };
export type WorkflowExport = { ok: boolean; path: string; tool: string };

async function json<T>(url: string, init?: RequestInit): Promise<T> {
  const r = await fetch(url, init);
  if (!r.ok) throw new Error(`${r.status} ${await r.text().catch(() => r.statusText)}`);
  return (await r.json()) as T;
}
const POST_JSON = (body: unknown): RequestInit => ({
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify(body),
});

export const api = {
  state: () => json<StateInfo>("/api/state"),
  skills: () => json<SkillEntry[]>("/api/skills"),
  toggleSkill: (name: string, when: SkillEntry["tier"]) =>
    json<{ name: string; tier: SkillEntry["tier"] }>("/api/skills/toggle", POST_JSON({ name, when })),
  skillAdd: (spec: string) => json<{ ok: boolean; log: string }>("/api/skills/add", POST_JSON({ spec })),
  skillUpdate: (name?: string) => json<{ ok: boolean; log: string }>("/api/skills/update", POST_JSON({ name })),
  memory: (file: string) => json<{ file: string; content: string }>(`/api/memory/${file}`),
  saveMemory: (file: string, content: string) =>
    json<{ ok: boolean }>("/api/memory/" + file, POST_JSON({ content })),
  engines: () => json<Engines>("/api/engines"),
  bench: () => json<BenchReport>("/api/bench"),
  evalProject: () => json<EvalReport>("/api/eval"),
  workspaces: () => json<WorkspaceInfo>("/api/workspaces"),
  activateWorkspace: (profile?: string, project?: string) =>
    json<unknown>("/api/workspaces/activate", POST_JSON({ profile, project })),
  team: () => json<TeamInfo>("/api/team"),
  submodules: () => json<Submodule[]>("/api/submodules"),
  submoduleAdd: (url: string, path?: string) =>
    json<{ ok: boolean; path: string }>("/api/submodules/add", POST_JSON({ url, path })),
  submodulePull: (path: string) => json<{ ok: boolean }>("/api/submodules/pull", POST_JSON({ path })),
  submoduleRemove: (path: string) =>
    json<{ ok: boolean }>("/api/submodules/remove", POST_JSON({ path })),
  context: () => json<ContextInfo>("/api/context"),
  mcp: () => json<{ servers: McpServer[] }>("/api/mcp"),
  rules: () => json<Rule[]>("/api/rules"),
  ruleAdd: (name: string, content: string, scope?: string) =>
    json<{ ok: boolean; path: string }>("/api/rules/add", POST_JSON({ name, content, scope })),
  ruleDelete: (path: string) => json<{ ok: boolean }>("/api/rules/delete", POST_JSON({ path })),
  workflows: () => json<string[]>("/api/workflows"),
  workflowGet: (name: string) => json<Workflow>(`/api/workflows/${encodeURIComponent(name)}`),
  workflowSave: (name: string, nodes: WfNode[], edges: WfEdge[]) =>
    json<{ ok: boolean; path: string }>(`/api/workflows/${encodeURIComponent(name)}`, POST_JSON({ nodes, edges })),
  workflowDelete: (name: string) =>
    json<{ ok: boolean }>(`/api/workflows/${encodeURIComponent(name)}`, { method: "DELETE" }),
  workflowExport: (name: string) =>
    json<WorkflowExport>(`/api/workflows/${encodeURIComponent(name)}/export`, { method: "POST" }),
};
