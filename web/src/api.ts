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
};
