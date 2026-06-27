import { useCallback, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type SkillEntry, type Engines, type WfData, type WfKind, type WfNode, type WfEdge } from "./api";
import {
  ReactFlow, Background, Controls, MiniMap, addEdge, Handle, Position,
  useNodesState, useEdgesState,
  type Node, type Edge, type Connection, type NodeTypes, type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import ELK, { type ElkNode } from "elkjs/lib/elk.bundled.js";

type Page = "state" | "skills" | "memory" | "engines" | "context" | "bench" | "eval" | "workspaces" | "team" | "submodules" | "mcp" | "rules" | "workflow";
const NAV: { id: Page; label: string }[] = [
  { id: "state", label: "State" },
  { id: "context", label: "Context" },
  { id: "skills", label: "Skills" },
  { id: "memory", label: "Memory" },
  { id: "engines", label: "Engines" },
  { id: "bench", label: "Bench" },
  { id: "eval", label: "Readiness" },
  { id: "workspaces", label: "Workspaces" },
  { id: "team", label: "Team" },
  { id: "submodules", label: "Submodules" },
  { id: "mcp", label: "MCP" },
  { id: "rules", label: "Rules" },
  { id: "workflow", label: "Workflow" },
];
const MEMORY_FILES = ["STATE", "KNOWLEDGE", "PLAYBOOKS", "DECISIONS", "PROJECT", "NOTES"] as const;

export default function App() {
  const [page, setPage] = useState<Page>("state");
  return (
    <div className="app">
      <nav className="sidebar" aria-label="Sections">
        <h1>8sync harness</h1>
        {NAV.map((n) => (
          <button
            key={n.id}
            onClick={() => setPage(n.id)}
            aria-current={page === n.id ? "page" : undefined}
          >
            {n.label}
          </button>
        ))}
      </nav>
      <main className="main">
        {page === "state" && <StatePage />}
        {page === "skills" && <SkillsPage />}
        {page === "memory" && <MemoryPage />}
        {page === "engines" && <EnginesPage />}
        {page === "bench" && <BenchPage />}
        {page === "eval" && <EvalPage />}
        {page === "workspaces" && <WorkspacesPage />}
        {page === "team" && <TeamPage />}
        {page === "submodules" && <SubmodulesPage />}
        {page === "context" && <ContextPage />}
        {page === "mcp" && <McpPage />}
        {page === "rules" && <RulesPage />}
        {page === "workflow" && <WorkflowPage />}
      </main>
    </div>
  );
}

function StatePage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["state"], queryFn: api.state });
  if (isLoading) return <p className="muted">Loading…</p>;
  if (error) return <p className="err">Error: {(error as Error).message}</p>;
  if (!data) return <p className="muted">Loading…</p>;
  return (
    <>
      <h2>State</h2>
      <p className="sub mono">
        project: {data.project} · profile: {data.profile}
      </p>
      <div className="card">
        <pre className="mono">{data.state_md || "(no agents/STATE.md)"}</pre>
      </div>
    </>
  );
}

function SkillsPage() {
  const qc = useQueryClient();
  const { data, isLoading, error } = useQuery({ queryKey: ["skills"], queryFn: api.skills });
  const toggle = useMutation({
    mutationFn: (v: { name: string; when: SkillEntry["tier"] }) => api.toggleSkill(v.name, v.when),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["skills"] }),
  });
  if (isLoading) return <p className="muted">Loading…</p>;
  if (error) return <p className="err">Error: {(error as Error).message}</p>;
  if (!data) return <p className="muted">Loading…</p>;
  const cycle = (t: SkillEntry["tier"]): SkillEntry["tier"] =>
    t === "always" ? "on-demand" : t === "on-demand" ? "off" : "always";
  return (
    <>
      <h2>Skills</h2>
      <p className="sub">{data.length} skill(s). Click tier to cycle always → on-demand → off.</p>
      <div className="card">
        {data.map((s) => (
          <div className="row" key={s.name}>
            <div>
              <div className="mono">{s.name}</div>
              <div className="muted mono" style={{ fontSize: 11 }}>
                {s.source || "—"} {s.global ? "· global" : ""} {s.local ? "· project" : ""}
              </div>
            </div>
            <button
              className={`tag ${s.tier === "always" ? "ok" : s.tier === "off" ? "warn" : ""}`}
              onClick={() => toggle.mutate({ name: s.name, when: cycle(s.tier) })}
              disabled={toggle.isPending}
              title="Cycle tier"
            >
              {s.tier}
            </button>
          </div>
        ))}
      </div>
    </>
  );
}

function MemoryPage() {
  const [file, setFile] = useState<(typeof MEMORY_FILES)[number]>("STATE");
  const [draft, setDraft] = useState("");
  const qc = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: ["memory", file],
    queryFn: () => api.memory(file),
    enabled: MEMORY_FILES.includes(file),
  });
  if (data && data.content !== draft && draft === "") setDraft(data.content);
  const save = useMutation({
    mutationFn: (content: string) => api.saveMemory(file, content),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["memory", file] }),
  });
  return (
    <>
      <h2>Memory</h2>
      <p className="sub">Edit the project memory spine (agents/*.md). Writes are scoped — no path escape.</p>
      <div className="card">
        <div className="row" style={{ borderBottom: 0 }}>
          <select value={file} onChange={(e) => { setFile(e.target.value as typeof file); setDraft(""); }}>
            {MEMORY_FILES.map((f) => (
              <option key={f} value={f}>
                {f}.md
              </option>
            ))}
          </select>
          <button
            className="primary"
            disabled={save.isPending || isLoading}
            onClick={() => save.mutate(draft)}
          >
            {save.isPending ? "Saving…" : "Save"}
          </button>
        </div>
        <textarea
          value={isLoading ? "Loading…" : draft}
          onChange={(e) => setDraft(e.target.value)}
          aria-label={`${file}.md content`}
        />
        {save.isSuccess && <p className="muted" style={{ marginTop: 8 }}>Saved.</p>}
        {save.isError && <p className="err" style={{ marginTop: 8 }}>Save failed: {(save.error as Error).message}</p>}
      </div>
    </>
  );
}

function EnginesPage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["engines"], queryFn: api.engines });
  if (isLoading) return <p className="muted">Loading…</p>;
  if (error) return <p className="err">Error: {(error as Error).message}</p>;
  if (!data) return <p className="muted">Loading…</p>;
  const engines: { key: keyof Omit<Engines, "mnemopi_on">; label: string; hint: string }[] = [
    { key: "codegraph", label: "codegraph", hint: "local code index (read/find)" },
    { key: "cbm", label: "codebase-memory-mcp", hint: "semantic graph" },
    { key: "headroom", label: "headroom", hint: "token compression" },
    { key: "serena", label: "serena", hint: "full-CRUD file tool" },
  ];
  return (
    <>
      <h2>Engines</h2>
      <p className="sub">Token-opt + file-CRUD stack. Absent engines fall back to slow grep/read.</p>
      <div className="grid">
        {engines.map((e) => {
          const st = data[e.key];
          return (
            <div className="card" key={e.key}>
              <div className="row" style={{ borderBottom: 0 }}>
                <strong>{e.label}</strong>
                <span className={`tag ${st.present ? "ok" : "warn"}`}>{st.present ? `ON ${st.version}`.trim() : "OFF"}</span>
              </div>
              <p className="muted" style={{ margin: "6px 0 0" }}>{e.hint}</p>
            </div>
          );
        })}
        <div className="card">
          <div className="row" style={{ borderBottom: 0 }}>
            <strong>mnemopi memory</strong>
            <span className={`tag ${data.mnemopi_on ? "ok" : "warn"}`}>{data.mnemopi_on ? "ON" : "OFF"}</span>
          </div>
          <p className="muted" style={{ margin: "6px 0 0" }}>long-term recall/retain</p>
        </div>
      </div>
    </>
  );
}

function BenchPage() {
  const { data, isLoading, error, refetch, isFetching } = useQuery({
    queryKey: ["bench"],
    queryFn: api.bench,
    enabled: false,
  });
  return (
    <>
      <h2>Bench</h2>
      <p className="sub">Token budget of the harness prefix.</p>
      <div className="card">
        <button className="primary" onClick={() => refetch()} disabled={isFetching}>
          {isFetching ? "Running…" : "Run bench"}
        </button>
        {isLoading ? <p className="muted">Not run yet.</p> : null}
        {error ? <p className="err">Error: {(error as Error).message}</p> : null}
        {data ? (
          <div style={{ marginTop: 12 }}>
            <div className="row">
              <span>UPFRONT (paid every session)</span>
              <span className="pct mono">~{data.upfront} tok</span>
            </div>
            <div className="row">
              <span>DEFERRED (on trigger)</span>
              <span className="mono">~{data.deferred} tok</span>
            </div>
            <div className="row">
              <span>force-load prefix</span>
              <span className="mono">~{data.force_load_prefix} tok</span>
            </div>
            <div className="row">
              <span>A2 progressive disclosure saved</span>
              <span className="pct">{data.a2_saved_pct}%</span>
            </div>
            <div className="row">
              <span>A1 stable-prefix (KV-cache)</span>
              <span className={`tag ${data.a1_pass ? "ok" : "warn"}`}>{data.a1_pass ? "PASS" : "FAIL"}</span>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}

function EvalPage() {
  const { data, isLoading, error, refetch, isFetching } = useQuery({
    queryKey: ["eval"],
    queryFn: api.evalProject,
    enabled: false,
  });
  return (
    <>
      <h2>Readiness</h2>
      <p className="sub">Agent-team capability coverage on this project (deterministic, not output quality).</p>
      <div className="card">
        <button className="primary" onClick={() => refetch()} disabled={isFetching}>
          {isFetching ? "Scoring…" : "Score readiness"}
        </button>
        {error ? <p className="err" style={{ marginTop: 8 }}>Error: {(error as Error).message}</p> : null}
        {data ? (
          <div style={{ marginTop: 12 }}>
            <div className="row">
              <strong>OVERALL</strong>
              <span className="pct">{data.overall}% ({data.present}/{data.total})</span>
            </div>
            {data.roles.map((r) => (
              <div className="row" key={r.role}>
                <span className="mono">{r.role}</span>
                <span style={{ display: "flex", alignItems: "center", gap: 8, flex: "0 0 240px" }}>
                  <span className="bar"><span style={{ width: `${r.pct}%` }} /></span>
                  <span className="pct" style={{ width: 40, textAlign: "right" }}>{r.pct}%</span>
                </span>
              </div>
            ))}
          </div>
        ) : isLoading ? <p className="muted">Not run yet.</p> : null}
      </div>
    </>
  );
}

function WorkspacesPage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces });
  const qc = useQueryClient();
  const activate = useMutation({
    mutationFn: (profile: string) => api.activateWorkspace(profile),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["workspaces"] }),
  });
  if (isLoading) return <p className="muted">Loading…</p>;
  if (error) return <p className="err">Error: {(error as Error).message}</p>;
  if (!data) return <p className="muted">No data.</p>;
  return (
    <>
      <h2>Workspaces</h2>
      <p className="sub">omp profiles + current project. Activate records the choice (advisory — run omp with <code>--profile</code> in that dir to isolate).</p>
      <div className="card">
        <div className="row" style={{ borderBottom: 0 }}>
          <strong>Current project</strong>
          <span className="mono">{data.project || "—"}</span>
        </div>
      </div>
      <div className="card">
        <strong style={{ display: "block", marginBottom: 8 }}>Profiles</strong>
        {data.profiles.map((p) => (
          <div className="row" key={p}>
            <span className="mono">{p}</span>
            <button className="primary" onClick={() => activate.mutate(p)} disabled={activate.isPending}>
              Activate
            </button>
          </div>
        ))}
      </div>
    </>
  );
}

function TeamPage() {
  const { data, isLoading, error, refetch, isFetching } = useQuery({ queryKey: ["team"], queryFn: api.team, enabled: false });
  return (
    <>
      <h2>Team</h2>
      <p className="sub">omp subagent roster + per-project readiness.</p>
      <div className="card">
        <button className="primary" onClick={() => refetch()} disabled={isFetching}>
          {isFetching ? "Loading…" : "Load team"}
        </button>
        {error ? <p className="err" style={{ marginTop: 8 }}>Error: {(error as Error).message}</p> : null}
        {data ? (
          <div style={{ marginTop: 12 }}>
            <div className="row">
              <strong>Readiness</strong>
              <span className="pct">{data.readiness ? `${data.readiness.overall}%` : "—"}</span>
            </div>
            {data.roster.map((r) => (
              <div className="row" key={r.type}>
                <span className="mono">{r.type} <span className="muted">— {r.role}</span></span>
                <span className="muted mono" style={{ fontSize: 11 }}>{r.skills}</span>
              </div>
            ))}
          </div>
        ) : isLoading ? <p className="muted">Not loaded.</p> : null}
      </div>
    </>
  );
}

function SubmodulesPage() {
  const qc = useQueryClient();
  const { data, isLoading, error, refetch } = useQuery({ queryKey: ["submodules"], queryFn: api.submodules });
  const [url, setUrl] = useState("");
  const add = useMutation({
    mutationFn: (u: string) => api.submoduleAdd(u),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["submodules"] }); setUrl(""); },
  });
  const pull = useMutation({ mutationFn: (p: string) => api.submodulePull(p), onSuccess: () => qc.invalidateQueries({ queryKey: ["submodules"] }) });
  const remove = useMutation({ mutationFn: (p: string) => api.submoduleRemove(p), onSuccess: () => qc.invalidateQueries({ queryKey: ["submodules"] }) });
  return (
    <>
      <h2>Submodules</h2>
      <p className="sub">Reference repos (gstack · gsd-pi · agent-reach · …). Add/pull/remove.</p>
      <div className="card">
        <div className="row" style={{ borderBottom: 0 }}>
          <input type="text" placeholder="https://github.com/owner/repo" value={url} onChange={(e) => setUrl(e.target.value)} style={{ flex: 1 }} aria-label="submodule URL" />
          <button className="primary" disabled={!url || add.isPending} onClick={() => add.mutate(url)}>Add</button>
          <button onClick={() => refetch()}>Refresh</button>
        </div>
        {add.isError ? <p className="err" style={{ marginTop: 8 }}>Add failed: {(add.error as Error).message}</p> : null}
      </div>
      <div className="card">
        {isLoading ? <p className="muted">Loading…</p> : null}
        {error ? <p className="err">Error: {(error as Error).message}</p> : null}
        {data && data.length === 0 ? <p className="muted">No submodules.</p> : null}
        {data && data.length > 0
          ? data.map((s) => (
              <div className="row" key={s.path}>
                <div>
                  <div className="mono">{s.name}</div>
                  <div className="muted mono" style={{ fontSize: 11 }}>{s.url}</div>
                </div>
                <span style={{ display: "flex", gap: 6 }}>
                  <span className={`tag ${s.initialized ? "ok" : "warn"}`}>{s.initialized ? "init" : "deinit"}</span>
                  <button onClick={() => pull.mutate(s.path)} disabled={pull.isPending}>pull</button>
                  <button onClick={() => remove.mutate(s.path)} disabled={remove.isPending}>remove</button>
                </span>
              </div>
            ))
          : null}
      </div>
    </>
  );
}

function ContextPage() {
  const { data, isLoading, error } = useQuery({
    queryKey: ["context"],
    queryFn: api.context,
    refetchInterval: 4000,
  });
  if (isLoading) return <p className="muted">Loading…</p>;
  if (error) return <p className="err">Error: {(error as Error).message}</p>;
  if (!data) return <p className="muted">No data.</p>;
  const pct = Math.min(data.pct, 100);
  const near = data.pct >= data.threshold_pct - 10;
  const over = data.over_threshold;
  const barColor = over ? "var(--err)" : near ? "var(--warn)" : "var(--accent)";
  return (
    <>
      <h2>Context</h2>
      <p className="sub">Live omp session token usage. Auto-compacts at {data.threshold_pct}% (snapcompact).</p>
      <div className="card">
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
          <strong className="mono">{(data.used / 1000).toFixed(0)}k / {(data.window / 1000).toFixed(0)}k tok</strong>
          <span className="pct" style={{ color: barColor }}>{data.pct}%</span>
        </div>
        <div style={{ position: "relative", height: 22, background: "var(--panel-2)", borderRadius: 6, overflow: "hidden" }} role="progressbar" aria-valuenow={data.pct} aria-valuemin={0} aria-valuemax={100}>
          <div style={{ width: `${pct}%`, height: "100%", background: barColor, transition: "width .4s ease-out" }} />
          {/* 50% threshold marker */}
          <div title={`compact at ${data.threshold_pct}%`} style={{ position: "absolute", left: `${data.threshold_pct}%`, top: 0, bottom: 0, width: 2, background: "var(--text)", opacity: 0.7 }} />
        </div>
        <p className="muted" style={{ marginTop: 8, fontSize: 12 }}>
          compact at {data.threshold_pct}% = {(data.compact_at / 1000).toFixed(0)}k tok · {over ? "OVER — will compact next turn" : `${data.threshold_pct - data.pct > 0 ? data.threshold_pct - data.pct : 0}% headroom`}
        </p>
        {data.compaction_observed && (
          <p style={{ marginTop: 6 }}>
            <span className="tag ok">✓ compaction observed</span>
            <span className="muted" style={{ marginLeft: 8, fontSize: 12 }}>last fired at {((data.last_compact_at ?? 0) / 1000).toFixed(0)}k tok</span>
          </p>
        )}
      </div>
      <div className="card">
        <div className="row" style={{ borderBottom: 0 }}>
          <span>model</span><span className="mono">{data.model || "—"}</span>
        </div>
        <div className="row" style={{ borderBottom: 0 }}>
          <span>project</span><span className="mono" style={{ fontSize: 11 }}>{data.project || "—"}</span>
        </div>
        <div className="row" style={{ borderBottom: 0 }}>
          <span>session</span><span className="mono" style={{ fontSize: 11 }}>{data.session || "—"}</span>
        </div>
        <p className="muted" style={{ marginTop: 8, fontSize: 11 }}>{data.note}</p>
      </div>
    </>
  );
}

function McpPage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["mcp"], queryFn: api.mcp });
  if (isLoading) return <p className="muted">Loading…</p>;
  if (error) return <p className="err">Error: {(error as Error).message}</p>;
  if (!data || data.servers.length === 0) return <p className="muted">No MCP servers registered.</p>;
  return (
    <>
      <h2>MCP servers</h2>
      <p className="sub">From <code>~/.omp/agent/mcp.json</code>. omp loads these each session.</p>
      <div className="card">
        {data.servers.map((s) => (
          <div className="row" key={s.name}>
            <div>
              <div className="mono">{s.name}</div>
              <div className="muted mono" style={{ fontSize: 11 }}>{s.command} {s.args.join(" ")}</div>
            </div>
            <span className={`tag ${s.present ? "ok" : "warn"}`}>{s.present ? "present" : "missing"} · {s.type}</span>
          </div>
        ))}
      </div>
    </>
  );
}

function RulesPage() {
  const qc = useQueryClient();
  const { data, isLoading, error } = useQuery({ queryKey: ["rules"], queryFn: api.rules });
  const [name, setName] = useState("");
  const [content, setContent] = useState("");
  const [scope, setScope] = useState<"project" | "global">("project");
  const add = useMutation({
    mutationFn: () => api.ruleAdd(name, content, scope),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["rules"] }); setName(""); setContent(""); },
  });
  const del = useMutation({ mutationFn: (p: string) => api.ruleDelete(p), onSuccess: () => qc.invalidateQueries({ queryKey: ["rules"] }) });
  return (
    <>
      <h2>Rules</h2>
      <p className="sub">omp rule files (<code>.omp/rules/*.md</code> project, <code>~/.omp/agent/rules/*.md</code> global).</p>
      <div className="card">
        <div className="row" style={{ borderBottom: 0 }}>
          <input type="text" placeholder="rule-name" value={name} onChange={(e) => setName(e.target.value)} aria-label="rule name" />
          <select value={scope} onChange={(e) => setScope(e.target.value as "project" | "global")}>
            <option value="project">project</option>
            <option value="global">global</option>
          </select>
        </div>
        <textarea value={content} onChange={(e) => setContent(e.target.value)} placeholder="Rule content (markdown)…" style={{ minHeight: 140, marginTop: 8 }} aria-label="rule content" />
        <div className="row" style={{ borderBottom: 0, marginTop: 6 }}>
          <span className="muted" style={{ fontSize: 11 }}>Tip: paste from a link/file/folder source here.</span>
          <button className="primary" disabled={!content || add.isPending} onClick={() => add.mutate()}>Add rule</button>
        </div>
        {add.isError ? <p className="err" style={{ marginTop: 8 }}>Add failed: {(add.error as Error).message}</p> : null}
      </div>
      <div className="card">
        {isLoading ? <p className="muted">Loading…</p> : null}
        {error ? <p className="err">Error: {(error as Error).message}</p> : null}
        {data && data.length === 0 ? <p className="muted">No rules.</p> : null}
        {data && data.length > 0 ? data.map((r) => (
          <div className="row" key={r.path}>
            <div>
              <div className="mono">{r.name}</div>
              <div className="muted mono" style={{ fontSize: 11 }}>{r.scope} · {r.bytes} B</div>
            </div>
            <button onClick={() => del.mutate(r.path)} disabled={del.isPending}>delete</button>
          </div>
        )) : null}
      </div>
    </>
  );
}
// WorkflowPage — react-flow visual editor. Node = one workflow step (skill /
// subagent / tool call). Export generates a standalone omp extension file
// registering a model-callable `<name>_run` tool that dispatches the steps as
// followUp messages (skills/subagents can't be spawned directly from a tool ctx).
const WF_KIND_LABEL: Record<WfKind, string> = { step: "skill", subagent: "subagent", tool: "tool" };
let wfCounter = 0;

function makeWfNode(kind: WfKind): Node<WfData> {
  wfCounter += 1;
  return {
    id: `n${Date.now()}_${wfCounter}`,
    type: "wf",
    position: { x: 80 + Math.round(Math.random() * 160), y: 80 + Math.round(Math.random() * 120) },
    data: { label: `New ${WF_KIND_LABEL[kind]}`, kind, ref: "" },
  };
}

function WfNodeView({ data }: NodeProps<Node<WfData>>) {
  const color = data.kind === "subagent" ? "var(--accent)" : data.kind === "tool" ? "var(--warn)" : "var(--ok)";
  return (
    <div className="wf-node" style={{ borderColor: color }}>
      <Handle type="target" position={Position.Top} />
      <div className="wf-node-kind mono" style={{ color }}>{WF_KIND_LABEL[data.kind]}</div>
      <div className="wf-node-label">{data.label}</div>
      {data.ref ? <div className="wf-node-ref mono">{data.ref}</div> : null}
      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}

// react-flow's NodeTypes is invariant over the Node generic, so a concrete-node
// map cannot be assigned directly — cast through unknown (no `any`).
const wfNodeTypes = { wf: WfNodeView } as unknown as NodeTypes;

const elk = new ELK();

function WorkflowPage() {
  const qc = useQueryClient();
  const [name, setName] = useState("demo");
  const [nodes, setNodes, onNodesChange] = useNodesState<Node<WfData>>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selId, setSelId] = useState<string | null>(null);

  const list = useQuery({ queryKey: ["workflows"], queryFn: api.workflows });
  const saveMut = useMutation({
    mutationFn: (v: { name: string; nodes: WfNode[]; edges: WfEdge[] }) =>
      api.workflowSave(v.name, v.nodes, v.edges),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["workflows"] }),
  });
  const exportMut = useMutation({ mutationFn: (n: string) => api.workflowExport(n) });

  const selected = nodes.find((n) => n.id === selId) ?? null;
  const onConnect = useCallback((c: Connection) => setEdges((eds) => addEdge(c, eds)), [setEdges]);
  const addNode = (kind: WfKind) => setNodes((ns) => [...ns, makeWfNode(kind)]);
  const updateSel = (patch: Partial<WfData>) => {
    if (!selected) return;
    setNodes((ns) =>
      ns.map((n) => (n.id === selected.id ? { ...n, data: { ...n.data, ...patch } } : n)),
    );
  };
  const load = async (n: string) => {
    setName(n);
    const wf = await api.workflowGet(n);
    setNodes((wf.nodes ?? []).map((x) => ({ id: x.id, type: "wf", position: x.position, data: x.data })));
    setEdges(wf.edges ?? []);
    setSelId(null);
  };
  const autoLayout = async () => {
    const graph: ElkNode = {
      id: "root",
      layoutOptions: { "elk.algorithm": "layered", "elk.direction": "DOWN" },
      children: nodes.map((n) => ({ id: n.id, width: 180, height: 72 })),
      edges: edges.map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] })),
    };
    const laid = await elk.layout(graph);
    const pos = new Map((laid.children ?? []).map((c) => [c.id, { x: c.x ?? 0, y: c.y ?? 0 }]));
    setNodes((ns) => ns.map((n) => ({ ...n, position: pos.get(n.id) ?? n.position })));
  };
  const save = () => {
    const wfNodes: WfNode[] = nodes.map((n) => ({ id: n.id, type: n.type, position: n.position, data: n.data }));
    const wfEdges: WfEdge[] = edges.map((e) => ({ id: e.id, source: e.source, target: e.target }));
    saveMut.mutate({ name, nodes: wfNodes, edges: wfEdges });
  };

  return (
    <>
      <h2>Workflow</h2>
      <p className="sub">Visual agent workflow. Export generates a model-callable omp extension tool.</p>
      <div className="wf-toolbar card">
        <label className="mono">name
          <input className="mono" value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <button onClick={() => addNode("step")}>Add skill step</button>
        <button onClick={() => addNode("subagent")}>Add subagent</button>
        <button onClick={() => addNode("tool")}>Add tool call</button>
        <button onClick={autoLayout}>Auto-layout</button>
        <button onClick={save} disabled={saveMut.isPending}>Save workflow</button>
        <button onClick={() => exportMut.mutate(name)} disabled={exportMut.isPending}>Export to omp</button>
      </div>
      {exportMut.data ? (
        <p className="ok mono" style={{ fontSize: 11 }}>exported → {exportMut.data.path} (tool: {exportMut.data.tool})</p>
      ) : saveMut.data ? (
        <p className="ok mono" style={{ fontSize: 11 }}>saved → {saveMut.data.path}</p>
      ) : null}
      <div className="wf-layout">
        <div className="wf-canvas card">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={wfNodeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={(_, n) => setSelId(n.id)}
            fitView
          >
            <Background />
            <Controls />
            <MiniMap />
          </ReactFlow>
        </div>
        <div className="wf-side card">
          <h3>Edit node</h3>
          {selected ? (
            <>
              <label className="mono">label
                <input className="mono" value={selected.data.label} onChange={(e) => updateSel({ label: e.target.value })} />
              </label>
              <label className="mono">kind
                <select value={selected.data.kind} onChange={(e) => updateSel({ kind: e.target.value as WfKind })}>
                  <option value="step">skill</option>
                  <option value="subagent">subagent</option>
                  <option value="tool">tool</option>
                </select>
              </label>
              <label className="mono">ref (skill / tool / subagent id)
                <input className="mono" value={selected.data.ref} onChange={(e) => updateSel({ ref: e.target.value })} />
              </label>
            </>
          ) : (
            <p className="muted">Select a node to edit, or add one.</p>
          )}
          <h3>Saved workflows</h3>
          {list.isLoading ? <p className="muted">Loading…</p> : (
            <ul className="wf-list">
              {(list.data ?? []).length === 0 ? <li className="muted">(none yet)</li> : list.data?.map((n) => (
                <li key={n}><button className="link" onClick={() => load(n)}>{n}</button></li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </>
  );
}
