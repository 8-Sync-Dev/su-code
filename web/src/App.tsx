import { useCallback, useState, type ReactNode } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type SkillEntry, type Engines, type WfData, type WfKind, type WfNode, type WfEdge } from "./api";
import {
  ReactFlow, Background, Controls, MiniMap, addEdge, Handle, Position,
  useNodesState, useEdgesState,
  type Node, type Edge, type Connection, type NodeTypes, type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import ELK, { type ElkNode } from "elkjs/lib/elk.bundled.js";
import { NavIcon, LogoMark } from "./icons";

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
        <div className="brand">
          <span className="brand-mark"><LogoMark /></span>
          <span className="brand-text">
            <span className="brand-name">8sync</span>
            <span className="brand-sub">harness</span>
          </span>
        </div>
        <div className="nav-group">
          {NAV.map((n) => (
            <button
              key={n.id}
              className="nav-item"
              onClick={() => setPage(n.id)}
              aria-current={page === n.id ? "page" : undefined}
            >
              <span className="nav-ico"><NavIcon name={n.id} /></span>
              <span className="nav-label">{n.label}</span>
            </button>
          ))}
        </div>
        <div className="side-foot">
          <span className="live-dot" aria-hidden="true" />
          agent-team dashboard
        </div>
      </nav>
      <main className="main">
        <div className="main-inner">
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
        </div>
      </main>
    </div>
  );
}

// ── Shared page scaffolding + interaction states ──────────────────────────
function Page({ title, sub, action, children }: { title: string; sub?: ReactNode; action?: ReactNode; children: ReactNode }) {
  return (
    <section className="page">
      <header className="page-head">
        <div>
          <h2>{title}</h2>
          {sub ? <p className="sub">{sub}</p> : null}
        </div>
        {action ? <div className="page-action">{action}</div> : null}
      </header>
      {children}
    </section>
  );
}

function Loading({ rows = 4 }: { rows?: number }) {
  return (
    <div className="card" aria-busy="true" aria-live="polite">
      <div className="skeleton">
        {Array.from({ length: rows }).map((_, i) => (
          <div className="skeleton-row" key={i}>
            <span className="skel" style={{ width: `${66 - i * 9}%` }} />
            <span className="skel skel-tag" />
          </div>
        ))}
      </div>
    </div>
  );
}

function ErrorState({ message }: { message: string }) {
  return (
    <div className="card state-msg state-error" role="alert">
      <span className="state-ico" aria-hidden="true">!</span>
      <div>
        <strong>Couldn’t load this</strong>
        <p className="mono">{message}</p>
      </div>
    </div>
  );
}

function EmptyState({ title, hint }: { title: string; hint?: ReactNode }) {
  return (
    <div className="card empty">
      <span className="empty-orb" aria-hidden="true" />
      <strong>{title}</strong>
      {hint ? <p className="muted">{hint}</p> : null}
    </div>
  );
}

function StatePage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["state"], queryFn: api.state });
  return (
    <Page
      title="State"
      sub={data ? <span className="mono">project {data.project} · profile {data.profile}</span> : "Project memory snapshot from agents/STATE.md."}
    >
      {isLoading || !data ? <Loading rows={5} /> : error ? <ErrorState message={(error as Error).message} /> : (
        <div className="card">
          <pre className="mono">{data.state_md || "(no agents/STATE.md)"}</pre>
        </div>
      )}
    </Page>
  );
}

function SkillsPage() {
  const qc = useQueryClient();
  const { data, isLoading, error } = useQuery({ queryKey: ["skills"], queryFn: api.skills });
  const toggle = useMutation({
    mutationFn: (v: { name: string; when: SkillEntry["tier"] }) => api.toggleSkill(v.name, v.when),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["skills"] }),
  });
  const cycle = (t: SkillEntry["tier"]): SkillEntry["tier"] =>
    t === "always" ? "on-demand" : t === "on-demand" ? "off" : "always";
  return (
    <Page title="Skills" sub={data ? `${data.length} registered · click a tier chip to cycle always → on-demand → off.` : "Click a tier chip to cycle always → on-demand → off."}>
      {isLoading ? <Loading rows={6} /> : error ? <ErrorState message={(error as Error).message} /> : !data ? <Loading /> : data.length === 0 ? (
        <EmptyState title="No skills registered" hint="Add a skill spec to make it loadable from the harness." />
      ) : (
        <div className="card list">
          {data.map((s) => (
            <div className="row" key={s.name}>
              <div className="row-main">
                <div className="row-title mono">{s.name}</div>
                <div className="row-meta mono">
                  {s.source || "—"}{s.global ? " · global" : ""}{s.local ? " · project" : ""}
                </div>
              </div>
              <button
                className={`tag tag-btn ${s.tier === "always" ? "ok" : s.tier === "off" ? "warn" : ""}`}
                onClick={() => toggle.mutate({ name: s.name, when: cycle(s.tier) })}
                disabled={toggle.isPending}
                title="Cycle tier"
              >
                {s.tier}
              </button>
            </div>
          ))}
        </div>
      )}
    </Page>
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
    <Page title="Memory" sub="Edit the project memory spine (agents/*.md). Writes are scoped — no path escape.">
      <div className="card">
        <div className="toolbar">
          <label className="field">
            <span className="field-label">File</span>
            <select value={file} onChange={(e) => { setFile(e.target.value as typeof file); setDraft(""); }}>
              {MEMORY_FILES.map((f) => (
                <option key={f} value={f}>{f}.md</option>
              ))}
            </select>
          </label>
          <button
            className="primary"
            disabled={save.isPending || isLoading}
            onClick={() => save.mutate(draft)}
          >
            {save.isPending ? "Saving…" : "Save changes"}
          </button>
        </div>
        <textarea
          value={isLoading ? "Loading…" : draft}
          onChange={(e) => setDraft(e.target.value)}
          aria-label={`${file}.md content`}
          spellCheck={false}
        />
        {save.isSuccess && <p className="hint hint-ok">Saved {file}.md.</p>}
        {save.isError && <p className="hint hint-err">Save failed: {(save.error as Error).message}</p>}
      </div>
    </Page>
  );
}

function EnginesPage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["engines"], queryFn: api.engines });
  const engines: { key: keyof Omit<Engines, "mnemopi_on">; label: string; hint: string }[] = [
    { key: "codegraph", label: "codegraph", hint: "local code index (read/find)" },
    { key: "cbm", label: "codebase-memory-mcp", hint: "semantic graph" },
    { key: "headroom", label: "headroom", hint: "token compression" },
    { key: "serena", label: "serena", hint: "full-CRUD file tool" },
  ];
  return (
    <Page title="Engines" sub="Token-opt + file-CRUD stack. Absent engines fall back to slow grep/read.">
      {isLoading || !data ? <Loading rows={4} /> : error ? <ErrorState message={(error as Error).message} /> : (
        <div className="grid">
          {engines.map((e) => {
            const st = data[e.key];
            return (
              <div className="card tile" key={e.key}>
                <div className="tile-head">
                  <strong>{e.label}</strong>
                  <span className={`tag ${st.present ? "ok" : "warn"}`}>{st.present ? `on ${st.version}`.trim() : "off"}</span>
                </div>
                <p className="muted tile-hint">{e.hint}</p>
              </div>
            );
          })}
          <div className="card tile">
            <div className="tile-head">
              <strong>mnemopi memory</strong>
              <span className={`tag ${data.mnemopi_on ? "ok" : "warn"}`}>{data.mnemopi_on ? "on" : "off"}</span>
            </div>
            <p className="muted tile-hint">long-term recall / retain</p>
          </div>
        </div>
      )}
    </Page>
  );
}

function BenchPage() {
  const { data, error, refetch, isFetching } = useQuery({
    queryKey: ["bench"],
    queryFn: api.bench,
    enabled: false,
  });
  return (
    <Page
      title="Bench"
      sub="Token budget of the harness prefix."
      action={
        <button className="primary" onClick={() => refetch()} disabled={isFetching}>
          {isFetching ? "Running…" : "Run bench"}
        </button>
      }
    >
      {error ? <ErrorState message={(error as Error).message} /> : null}
      {isFetching && !data ? <Loading rows={5} /> : null}
      {!data && !isFetching && !error ? (
        <EmptyState title="No bench run yet" hint="Run the bench to measure upfront vs. deferred prefix tokens." />
      ) : null}
      {data ? (
        <div className="card list">
          <div className="row">
            <span>Upfront <span className="muted">(paid every session)</span></span>
            <span className="pct mono">~{data.upfront} tok</span>
          </div>
          <div className="row">
            <span>Deferred <span className="muted">(on trigger)</span></span>
            <span className="mono">~{data.deferred} tok</span>
          </div>
          <div className="row">
            <span>Force-load prefix</span>
            <span className="mono">~{data.force_load_prefix} tok</span>
          </div>
          <div className="row">
            <span>A2 progressive disclosure saved</span>
            <span className="pct">{data.a2_saved_pct}%</span>
          </div>
          <div className="row">
            <span>A1 stable-prefix (KV-cache)</span>
            <span className={`tag ${data.a1_pass ? "ok" : "warn"}`}>{data.a1_pass ? "pass" : "fail"}</span>
          </div>
        </div>
      ) : null}
    </Page>
  );
}

function EvalPage() {
  const { data, error, refetch, isFetching } = useQuery({
    queryKey: ["eval"],
    queryFn: api.evalProject,
    enabled: false,
  });
  return (
    <Page
      title="Readiness"
      sub="Agent-team capability coverage on this project (deterministic, not output quality)."
      action={
        <button className="primary" onClick={() => refetch()} disabled={isFetching}>
          {isFetching ? "Scoring…" : "Score readiness"}
        </button>
      }
    >
      {error ? <ErrorState message={(error as Error).message} /> : null}
      {isFetching && !data ? <Loading rows={6} /> : null}
      {!data && !isFetching && !error ? (
        <EmptyState title="Not scored yet" hint="Run the score to see per-role coverage for this project." />
      ) : null}
      {data ? (
        <div className="card list">
          <div className="row row-total">
            <strong>Overall</strong>
            <span className="pct">{data.overall}% <span className="muted">({data.present}/{data.total})</span></span>
          </div>
          {data.roles.map((r) => (
            <div className="row" key={r.role}>
              <span className="mono row-role">{r.role}</span>
              <span className="meter">
                <span className="bar"><span style={{ width: `${r.pct}%` }} /></span>
                <span className="pct meter-val">{r.pct}%</span>
              </span>
            </div>
          ))}
        </div>
      ) : null}
    </Page>
  );
}

function WorkspacesPage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces });
  const qc = useQueryClient();
  const activate = useMutation({
    mutationFn: (profile: string) => api.activateWorkspace(profile),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["workspaces"] }),
  });
  return (
    <Page title="Workspaces" sub={<>omp profiles + current project. Activate records the choice (advisory — run omp with <code>--profile</code> in that dir to isolate).</>}>
      {isLoading ? <Loading rows={4} /> : error ? <ErrorState message={(error as Error).message} /> : !data ? (
        <EmptyState title="No workspace data" />
      ) : (
        <>
          <div className="card">
            <div className="row">
              <strong>Current project</strong>
              <span className="mono">{data.project || "—"}</span>
            </div>
          </div>
          <div className="card list">
            <div className="card-title">Profiles</div>
            {data.profiles.length === 0 ? (
              <p className="muted">No profiles defined.</p>
            ) : data.profiles.map((p) => (
              <div className="row" key={p}>
                <span className="mono">{p}</span>
                <button className="primary" onClick={() => activate.mutate(p)} disabled={activate.isPending}>
                  Activate
                </button>
              </div>
            ))}
          </div>
        </>
      )}
    </Page>
  );
}

function TeamPage() {
  const { data, error, refetch, isFetching } = useQuery({ queryKey: ["team"], queryFn: api.team, enabled: false });
  return (
    <Page
      title="Team"
      sub="omp subagent roster + per-project readiness."
      action={
        <button className="primary" onClick={() => refetch()} disabled={isFetching}>
          {isFetching ? "Loading…" : "Load team"}
        </button>
      }
    >
      {error ? <ErrorState message={(error as Error).message} /> : null}
      {isFetching && !data ? <Loading rows={5} /> : null}
      {!data && !isFetching && !error ? (
        <EmptyState title="Team not loaded" hint="Load the roster to see each subagent’s role and skills." />
      ) : null}
      {data ? (
        <div className="card list">
          <div className="row row-total">
            <strong>Readiness</strong>
            <span className="pct">{data.readiness ? `${data.readiness.overall}%` : "—"}</span>
          </div>
          {data.roster.map((r) => (
            <div className="row" key={r.type}>
              <div className="row-main">
                <div className="row-title mono">{r.type}</div>
                <div className="row-meta">{r.role}</div>
              </div>
              <span className="muted mono row-skills">{r.skills}</span>
            </div>
          ))}
        </div>
      ) : null}
    </Page>
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
    <Page title="Submodules" sub="Reference repos (gstack · gsd-pi · agent-reach · …). Add, pull, or remove.">
      <div className="card">
        <div className="toolbar">
          <input type="text" placeholder="https://github.com/owner/repo" value={url} onChange={(e) => setUrl(e.target.value)} className="grow" aria-label="submodule URL" />
          <button className="primary" disabled={!url || add.isPending} onClick={() => add.mutate(url)}>Add</button>
          <button onClick={() => refetch()}>Refresh</button>
        </div>
        {add.isError ? <p className="hint hint-err">Add failed: {(add.error as Error).message}</p> : null}
      </div>
      {isLoading ? <Loading rows={3} /> : error ? <ErrorState message={(error as Error).message} /> : data && data.length === 0 ? (
        <EmptyState title="No submodules" hint="Paste a repo URL above to add a reference submodule." />
      ) : data && data.length > 0 ? (
        <div className="card list">
          {data.map((s) => (
            <div className="row" key={s.path}>
              <div className="row-main">
                <div className="row-title mono">{s.name}</div>
                <div className="row-meta mono">{s.url}</div>
              </div>
              <span className="row-actions">
                <span className={`tag ${s.initialized ? "ok" : "warn"}`}>{s.initialized ? "init" : "deinit"}</span>
                <button onClick={() => pull.mutate(s.path)} disabled={pull.isPending}>Pull</button>
                <button onClick={() => remove.mutate(s.path)} disabled={remove.isPending}>Remove</button>
              </span>
            </div>
          ))}
        </div>
      ) : null}
    </Page>
  );
}

function ContextPage() {
  const { data, isLoading, error } = useQuery({
    queryKey: ["context"],
    queryFn: api.context,
    refetchInterval: 4000,
  });
  if (isLoading) return <Page title="Context" sub="Live omp session token usage."><Loading rows={3} /></Page>;
  if (error) return <Page title="Context" sub="Live omp session token usage."><ErrorState message={(error as Error).message} /></Page>;
  if (!data) return <Page title="Context" sub="Live omp session token usage."><EmptyState title="No context data" /></Page>;
  const pct = Math.min(data.pct, 100);
  const near = data.pct >= data.threshold_pct - 10;
  const over = data.over_threshold;
  const barColor = over ? "var(--err)" : near ? "var(--warn)" : "var(--accent)";
  return (
    <Page title="Context" sub={`Live omp session token usage. Auto-compacts at ${data.threshold_pct}% (snapcompact).`}>
      <div className="card">
        <div className="gauge-head">
          <strong className="mono">{(data.used / 1000).toFixed(0)}k / {(data.window / 1000).toFixed(0)}k tok</strong>
          <span className="pct gauge-pct" style={{ color: barColor }}>{data.pct}%</span>
        </div>
        <div className="gauge" role="progressbar" aria-valuenow={data.pct} aria-valuemin={0} aria-valuemax={100}>
          <div className="gauge-fill" style={{ width: `${pct}%`, background: barColor, boxShadow: `0 0 16px ${barColor}` }} />
          <div className="gauge-mark" title={`compact at ${data.threshold_pct}%`} style={{ left: `${data.threshold_pct}%` }} />
        </div>
        <p className="muted gauge-note">
          compact at {data.threshold_pct}% = {(data.compact_at / 1000).toFixed(0)}k tok · {over ? "over — will compact next turn" : `${data.threshold_pct - data.pct > 0 ? data.threshold_pct - data.pct : 0}% headroom`}
        </p>
        {data.compaction_observed && (
          <p className="gauge-observed">
            <span className="tag ok">✓ compaction observed</span>
            <span className="muted">last fired at {((data.last_compact_at ?? 0) / 1000).toFixed(0)}k tok</span>
          </p>
        )}
      </div>
      <div className="card list">
        <div className="row"><span>model</span><span className="mono">{data.model || "—"}</span></div>
        <div className="row"><span>project</span><span className="mono row-meta">{data.project || "—"}</span></div>
        <div className="row"><span>session</span><span className="mono row-meta">{data.session || "—"}</span></div>
        {data.note ? <p className="muted gauge-note">{data.note}</p> : null}
      </div>
    </Page>
  );
}

function McpPage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["mcp"], queryFn: api.mcp });
  return (
    <Page title="MCP servers" sub={<>From <code>~/.omp/agent/mcp.json</code>. omp loads these each session.</>}>
      {isLoading ? <Loading rows={3} /> : error ? <ErrorState message={(error as Error).message} /> : !data || data.servers.length === 0 ? (
        <EmptyState title="No MCP servers registered" hint="Add a server to ~/.omp/agent/mcp.json to surface it here." />
      ) : (
        <div className="card list">
          {data.servers.map((s) => (
            <div className="row" key={s.name}>
              <div className="row-main">
                <div className="row-title mono">{s.name}</div>
                <div className="row-meta mono">{s.command} {s.args.join(" ")}</div>
              </div>
              <span className={`tag ${s.present ? "ok" : "warn"}`}>{s.present ? "present" : "missing"} · {s.type}</span>
            </div>
          ))}
        </div>
      )}
    </Page>
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
    <Page title="Rules" sub={<>omp rule files (<code>.omp/rules/*.md</code> project, <code>~/.omp/agent/rules/*.md</code> global).</>}>
      <div className="card">
        <div className="toolbar">
          <input type="text" placeholder="rule-name" value={name} onChange={(e) => setName(e.target.value)} className="grow" aria-label="rule name" />
          <label className="field">
            <span className="field-label">Scope</span>
            <select value={scope} onChange={(e) => setScope(e.target.value as "project" | "global")}>
              <option value="project">project</option>
              <option value="global">global</option>
            </select>
          </label>
        </div>
        <textarea value={content} onChange={(e) => setContent(e.target.value)} placeholder="Rule content (markdown)…" className="textarea-sm" aria-label="rule content" spellCheck={false} />
        <div className="toolbar toolbar-split">
          <span className="muted hint-inline">Tip: paste from a link / file / folder source here.</span>
          <button className="primary" disabled={!content || add.isPending} onClick={() => add.mutate()}>Add rule</button>
        </div>
        {add.isError ? <p className="hint hint-err">Add failed: {(add.error as Error).message}</p> : null}
      </div>
      {isLoading ? <Loading rows={3} /> : error ? <ErrorState message={(error as Error).message} /> : data && data.length === 0 ? (
        <EmptyState title="No rules yet" hint="Add a project or global rule with the form above." />
      ) : data && data.length > 0 ? (
        <div className="card list">
          {data.map((r) => (
            <div className="row" key={r.path}>
              <div className="row-main">
                <div className="row-title mono">{r.name}</div>
                <div className="row-meta mono">{r.scope} · {r.bytes} B</div>
              </div>
              <button onClick={() => del.mutate(r.path)} disabled={del.isPending}>Delete</button>
            </div>
          ))}
        </div>
      ) : null}
    </Page>
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
    <div className={`wf-node wf-node-${data.kind}`} style={{ borderColor: color }}>
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
    <Page title="Workflow" sub="Visual agent workflow. Export generates a model-callable omp extension tool.">
      <div className="wf-toolbar card">
        <label className="field">
          <span className="field-label">Name</span>
          <input className="mono" value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <span className="wf-toolbar-spacer" />
        <button onClick={() => addNode("step")}>+ Skill step</button>
        <button onClick={() => addNode("subagent")}>+ Subagent</button>
        <button onClick={() => addNode("tool")}>+ Tool call</button>
        <button onClick={autoLayout}>Auto-layout</button>
        <button className="primary" onClick={save} disabled={saveMut.isPending}>Save workflow</button>
        <button onClick={() => exportMut.mutate(name)} disabled={exportMut.isPending}>Export to omp</button>
      </div>
      {exportMut.data ? (
        <p className="hint hint-ok mono">exported → {exportMut.data.path} (tool: {exportMut.data.tool})</p>
      ) : saveMut.data ? (
        <p className="hint hint-ok mono">saved → {saveMut.data.path}</p>
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
              <label className="field">
                <span className="field-label">Label</span>
                <input className="mono" value={selected.data.label} onChange={(e) => updateSel({ label: e.target.value })} />
              </label>
              <label className="field">
                <span className="field-label">Kind</span>
                <select value={selected.data.kind} onChange={(e) => updateSel({ kind: e.target.value as WfKind })}>
                  <option value="step">skill</option>
                  <option value="subagent">subagent</option>
                  <option value="tool">tool</option>
                </select>
              </label>
              <label className="field">
                <span className="field-label">Ref (skill / tool / subagent id)</span>
                <input className="mono" value={selected.data.ref} onChange={(e) => updateSel({ ref: e.target.value })} />
              </label>
            </>
          ) : (
            <p className="muted">Select a node to edit, or add one from the toolbar.</p>
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
    </Page>
  );
}
