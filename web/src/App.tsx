import { useCallback, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  api,
  EndpointMissingError,
  type SkillEntry,
  type Engines,
  type ModelConfig,
  type ModelRole,
  type WfData,
  type WfKind,
  type WfNode,
  type WfEdge,
} from "./api";
import { Markdown } from "./markdown";
import { NavIcon, LogoMark, Glyph } from "./icons";
import {
  ReactFlow, Background, Controls, MiniMap, addEdge, Handle, Position,
  useNodesState, useEdgesState,
  type Node, type Edge, type Connection, type NodeTypes, type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import ELK, { type ElkNode } from "elkjs/lib/elk.bundled.js";

type Page =
  | "state" | "context" | "models" | "skills" | "memory" | "rules"
  | "engines" | "mcp" | "submodules"
  | "bench" | "eval" | "team" | "workspaces" | "workflow";

const NAV_GROUPS: { label: string; items: { id: Page; label: string }[] }[] = [
  { label: "Session", items: [{ id: "state", label: "State" }, { id: "context", label: "Context" }] },
  { label: "Configure", items: [{ id: "models", label: "Models" }, { id: "skills", label: "Skills" }, { id: "memory", label: "Memory" }, { id: "rules", label: "Rules" }] },
  { label: "Runtime", items: [{ id: "engines", label: "Engines" }, { id: "mcp", label: "MCP" }, { id: "submodules", label: "Submodules" }] },
  { label: "Quality", items: [{ id: "bench", label: "Bench" }, { id: "eval", label: "Readiness" }, { id: "team", label: "Team" }] },
  { label: "Projects", items: [{ id: "workspaces", label: "Workspaces" }] },
  { label: "Build", items: [{ id: "workflow", label: "Workflow" }] },
];

const MEMORY_FILES = ["STATE", "KNOWLEDGE", "PLAYBOOKS", "DECISIONS", "PROJECT", "NOTES"] as const;

// Known model shorthands surfaced as quick-pick options in the Models page.
const MODEL_HINTS = ["opus", "glm", "haiku", "codex"] as const;

const ROLE_HINTS: Record<string, string> = {
  default: "fallback", plan: "planning", smol: "cheap calls", slow: "deep", vision: "images",
};
const TASK_HINTS: Record<string, string> = {
  plan: "thinking", review: "thinking", debug: "thinking", code: "mechanical", trivial: "mechanical",
};

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
        <ProjectSwitcher />
        <div className="nav-scroll">
          {NAV_GROUPS.map((g) => (
            <div className="nav-section" key={g.label}>
              <div className="nav-section-label">{g.label}</div>
              {g.items.map((n) => (
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
          {page === "context" && <ContextPage />}
          {page === "models" && <ModelsPage />}
          {page === "skills" && <SkillsPage />}
          {page === "memory" && <MemoryPage />}
          {page === "rules" && <RulesPage />}
          {page === "engines" && <EnginesPage />}
          {page === "mcp" && <McpPage />}
          {page === "submodules" && <SubmodulesPage />}
          {page === "bench" && <BenchPage />}
          {page === "eval" && <EvalPage />}
          {page === "team" && <TeamPage />}
          {page === "workspaces" && <WorkspacesPage />}
          {page === "workflow" && <WorkflowPage />}
        </div>
      </main>
    </div>
  );
}

// ── Project switcher (sidebar-top, persistent) ────────────────────────────
// Lists every omp project with a status dot (green = active, dim = off).
// Selecting one POSTs activate + invalidates every query so pages refetch in
// the new project's context. The menu uses position: fixed to escape the
// sidebar's overflow:auto clipping.
function ProjectSwitcher() {
  const qc = useQueryClient();
  const projects = useQuery({ queryKey: ["projects"], queryFn: api.projects });
  const ws = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces, staleTime: 8000 });
  const [open, setOpen] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number } | null>(null);
  const trigRef = useRef<HTMLButtonElement>(null);

  const activate = useMutation({
    mutationFn: (path: string) => api.activateProject(path),
    onSuccess: () => qc.invalidateQueries(),
  });

  useLayoutEffect(() => {
    if (!open || !trigRef.current) return;
    const r = trigRef.current.getBoundingClientRect();
    setCoords({ top: r.bottom + 6, left: r.left });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as HTMLElement;
      if (!t.closest(".proj-menu") && !t.closest(".proj-trigger")) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") setOpen(false); };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const list = projects.data ?? [];
  // Prefer the project matching the current cwd (what State/Context show), else
  // the backend's first "active". Keeps the trigger label consistent with the
  // page content even when several projects report active.
  const currentPath = ws.data?.project;
  const current = list.find((p) => p.current) ?? (currentPath ? list.find((p) => p.path === currentPath) : null) ?? list.find((p) => p.active) ?? null;
  const currentName = current?.name ?? (currentPath ? currentPath.split("/").pop() : null) ?? "no project";
  const missing = projects.error instanceof EndpointMissingError;

  return (
    <div className="proj-switch">
      <button
        ref={trigRef}
        className="proj-trigger"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        aria-haspopup="listbox"
        title={current?.path ?? currentPath ?? "Select project"}
      >
        <span className="proj-ic"><Glyph name="folder" /></span>
        <span className="proj-name">{currentName}</span>
        <span className="proj-chev"><Glyph name="chevron" /></span>
      </button>
      {open && coords && (
        <div className="proj-menu" role="listbox" style={coords}>
          <div className="proj-menu-head">omp projects</div>
          <div className="proj-menu-list">
            {missing ? (
              <div style={{ padding: "4px 6px" }}><EndpointMissing endpoint="/api/projects" /></div>
            ) : projects.isLoading ? (
              <div style={{ padding: "12px 14px" }} className="muted">Loading…</div>
            ) : list.length === 0 ? (
              <div style={{ padding: "12px 14px" }} className="muted">No projects found.</div>
            ) : (
              list.map((p) => (
                <button
                  key={p.path}
                  className={`proj-item ${current && p.path === current.path ? "active" : ""}`}
                  onClick={() => { activate.mutate(p.path); setOpen(false); }}
                  disabled={activate.isPending}
                  title={p.path}
                >
                  <span className={`dot ${p.active ? "on" : "off"}`} />
                  <span className="proj-item-main">
                    <span className="proj-item-name">{p.name}</span>
                    <span className="proj-item-path">{p.path}</span>
                  </span>
                </button>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Shared scaffolding + interaction states ───────────────────────────────
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
            <span className="skel" style={{ width: `${68 - i * 9}%` }} />
            <span className="skel skel-tag" />
          </div>
        ))}
      </div>
    </div>
  );
}

// Type guard — keeps EndpointMissingError narrowing live at call sites.
function isMissing(e: unknown): boolean {
  return e instanceof EndpointMissingError;
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

// Honest "endpoint not ready" banner — used when the backend route isn't live.
function EndpointMissing({ endpoint }: { endpoint: string }) {
  return (
    <div className="card missing state-msg">
      <span className="state-ico" aria-hidden="true"><Glyph name="alert" /></span>
      <div>
        <strong>Waiting on the backend</strong>
        <p>{endpoint} isn’t live in the running server yet. The screen fills in automatically once it ships; nothing to do here.</p>
      </div>
    </div>
  );
}

function EmptyState({ title, hint, icon }: { title: string; hint?: ReactNode; icon?: ReactNode }) {
  return (
    <div className="card empty">
      <span className="empty-orb" aria-hidden="true">{icon}</span>
      <strong>{title}</strong>
      {hint ? <p className="muted">{hint}</p> : null}
    </div>
  );
}

// Compact k-formatting for token counts (the magic 1k boundary).
function fmt(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(n >= 100000 ? 0 : 1)}k` : String(n);
}

// ── State (markdown rendered) ─────────────────────────────────────────────
function StatePage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["state"], queryFn: api.state });
  return (
    <Page
      title="State"
      sub={data ? <span>Project memory snapshot · <span className="mono">{data.project}</span> · profile {data.profile}</span> : "Live plan from agents/STATE.md, rewritten at each phase boundary."}
    >
      {isLoading ? <Loading rows={6} /> : error ? <ErrorState message={(error as Error).message} /> : !data ? <Loading /> : (
        <div className="card">
          {data.state_md ? <Markdown source={data.state_md} /> : (
            <EmptyState title="No STATE.md yet" hint="agents/STATE.md is empty. It fills in once the harness writes a plan." />
          )}
        </div>
      )}
    </Page>
  );
}

// ── Context (honest about assumed window + willCompact) ───────────────────
function ContextPage() {
  const { data, isLoading, error } = useQuery({
    queryKey: ["context"],
    queryFn: api.context,
    refetchInterval: 4000,
  });
  if (isLoading) return <Page title="Context" sub="Live omp session token usage."><Loading rows={3} /></Page>;
  if (error) return <Page title="Context" sub="Live omp session token usage."><ErrorState message={(error as Error).message} /></Page>;
  if (!data) return <Page title="Context"><EmptyState title="No context data" /></Page>;
  const d = data;
  const pct = Math.min(d.pct, 100);
  const near = d.pct >= d.thresholdPct - 10;
  const over = d.willCompact;
  const stale = Boolean(d.stale);
  const barColor = over ? (stale ? "var(--warn)" : "var(--err)") : near ? "var(--warn)" : "var(--accent)";
  const headroom = Math.max(0, d.thresholdPct - d.pct);
  return (
    <Page
      title="Context"
      sub="Live omp session token usage. The window is the active model's real context window (or an estimate if the model isn't in omp's catalog)."
    >
      <div className="card">
        <div className="gauge-head">
          <span className="big">{fmt(d.usedTok)} / {fmt(d.windowTok)} tok</span>
          <span className="gauge-pct" style={{ color: barColor }}>{d.pct}%</span>
        </div>
        <div className="gauge" role="progressbar" aria-valuenow={Math.round(d.pct)} aria-valuemin={0} aria-valuemax={100}>
          <div className="gauge-fill" style={{ width: `${pct}%`, background: barColor }} />
          <div className="gauge-mark" title={`compact at ${d.thresholdPct}%`} style={{ left: `${d.thresholdPct}%` }} />
        </div>
        <p className="gauge-note">
          {d.assumed && <span className="tag warn" style={{ marginRight: 8 }}>assumed window</span>}
          {stale && <span className="tag info" style={{ marginRight: 8 }}>idle snapshot</span>}
          compact at <strong>{d.thresholdPct}%</strong> ({fmt((d.thresholdPct * d.windowTok) / 100)} tok) ·{" "}
          {over ? (
            <span className={stale ? "warn" : "err"}>over threshold — compacts on next turn</span>
          ) : (
            <span>{headroom}% headroom</span>
          )}
        </p>
        {d.compactionObserved && d.lastCompactAt != null && (
          <p className="gauge-observed">
            <span className="tag ok"><Glyph name="check" /> compaction observed</span>
            <span className="muted">last fired near {fmt(d.lastCompactAt)} tok</span>
          </p>
        )}
        {d.note && <p className="gauge-note faint">{d.note}</p>}
      </div>
      <div className="card">
        <div className="card-title">Session</div>
        <div className="kv"><span className="kv-k">model</span><span className="kv-v mono">{d.model || "—"}</span></div>
        <div className="kv"><span className="kv-k">project</span><span className="kv-v mono">{d.project || "—"}</span></div>
        <div className="kv"><span className="kv-k">session</span><span className="kv-v mono">{d.session || "—"}</span></div>
      </div>
    </Page>
  );
}

// ── Models (new) — inline-editable role/task routing ──────────────────────
function ModelsPage() {
  const qc = useQueryClient();
  const { data, isLoading, error } = useQuery({ queryKey: ["models"], queryFn: api.models });
  const setMut = useMutation({
    mutationFn: (v: { section: "roles" | "tasks"; key: string; value: string }) =>
      api.modelSet(v.section, v.key, v.value),
    onSuccess: (updated: ModelConfig) => qc.setQueryData(["models"], updated),
  });

  if (isLoading) return <Page title="Models" sub="Which model each task runs on."><Loading rows={5} /></Page>;
  if (error) {
    return (
      <Page title="Models" sub="Which model each task runs on.">
        {isMissing(error) ? <EndpointMissing endpoint="/api/models" /> : <ErrorState message={(error as Error).message} />}
      </Page>
    );
  }
  if (!data) return <Page title="Models"><EmptyState title="No model config" /></Page>;

  const roleOrder: ModelRole[] = ["default", "plan", "smol", "slow", "vision"];
  const roles = roleOrder.filter((r) => data.roles[r] !== undefined || r === "default");
  const classes = data.classes ?? ["plan", "review", "debug", "code", "trivial"];
  return (
    <Page
      title="Models"
      sub={<>Routing config at <code>{data.path || "~/.config/8sync/models.toml"}</code>. Thinking work goes to Opus; mechanical work to GLM.</>}
    >
      <div className="card">
        <div className="card-title">Routing philosophy</div>
        <div className="model-philosophy">
          <div className="model-track">
            <span className="model-track-label">thinking</span>
            <span className="model-track-models">
              <span className="tag accent">plan</span>
              <span className="tag accent">review</span>
              <span className="tag accent">debug</span>
              <span className="muted">→</span>
              <span className="tag accent">opus</span>
            </span>
          </div>
          <div className="model-track">
            <span className="model-track-label">mechanical</span>
            <span className="model-track-models">
              <span className="tag">code</span>
              <span className="tag">edit</span>
              <span className="tag">default</span>
              <span className="tag">trivial</span>
              <span className="muted">→</span>
              <span className="tag">glm</span>
            </span>
          </div>
        </div>
        <p className="hint faint">Quick picks: opus · glm · haiku · codex. Changes write to the config immediately.</p>
      </div>

      <div className="models-grid">
        <div className="card">
          <div className="card-title">Roles</div>
          <div className="model-table">
            {roles.map((r) => (
              <ModelRow
                key={r}
                name={r}
                hint={ROLE_HINTS[r]}
                value={data.roles[r] ?? ""}
                saving={setMut.isPending && setMut.variables?.section === "roles" && setMut.variables?.key === r}
                onChange={(v) => setMut.mutate({ section: "roles", key: r, value: v })}
              />
            ))}
          </div>
        </div>
        <div className="card">
          <div className="card-title">Tasks by class</div>
          <div className="model-table">
            {classes.map((cls) => (
              <ModelRow
                key={cls}
                name={cls}
                hint={TASK_HINTS[cls] ?? "per-class"}
                value={data.tasks[cls] ?? ""}
                saving={setMut.isPending && setMut.variables?.section === "tasks" && setMut.variables?.key === cls}
                onChange={(v) => setMut.mutate({ section: "tasks", key: cls, value: v })}
              />
            ))}
          </div>
        </div>
      </div>
    </Page>
  );
}

function ModelRow({ name, hint, value, saving, onChange }: {
  name: string; hint?: string; value: string; saving: boolean; onChange: (v: string) => void;
}) {
  const opts = Array.from(new Set([...MODEL_HINTS, value].filter(Boolean) as string[]));
  return (
    <div className="model-row">
      <div className="model-key">
        <span className="model-key-name">{name}</span>
        {hint ? <span className="model-key-hint">{hint}</span> : null}
      </div>
      <div className="model-val">
        <select className="model-select" value={value} onChange={(e) => onChange(e.target.value)} disabled={saving}>
          {opts.map((o) => <option key={o} value={o}>{o}</option>)}
        </select>
        {saving && <span className="model-saving">saving…</span>}
      </div>
    </div>
  );
}

// ── Skills ─────────────────────────────────────────────────────────────────
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
    <Page title="Skills" sub="Click a tier chip to cycle always → on-demand → off.">
      {isLoading ? <Loading rows={6} /> : error ? <ErrorState message={(error as Error).message} /> : !data ? <Loading /> : data.length === 0 ? (
        <EmptyState title="No skills registered" hint="Add a skill spec to make it loadable from the harness." />
      ) : (
        <div className="card list">
          {data.map((s) => (
            <div className="row" key={s.name}>
              <div className="row-main">
                <div className="row-title mono">{s.name}</div>
                <div className="row-meta mono">{s.source || "—"}{s.global ? " · global" : ""}{s.local ? " · project" : ""}</div>
              </div>
              <button
                className={`tag tag-btn ${s.tier === "always" ? "ok" : s.tier === "off" ? "muted" : "accent"}`}
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

// ── Memory (edit + markdown preview) ──────────────────────────────────────
function MemoryPage() {
  const [file, setFile] = useState<(typeof MEMORY_FILES)[number]>("STATE");
  const [draft, setDraft] = useState("");
  const [preview, setPreview] = useState(false);
  const qc = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: ["memory", file],
    queryFn: () => api.memory(file),
  });
  if (data && data.content !== draft && draft === "") setDraft(data.content);
  const save = useMutation({
    mutationFn: (content: string) => api.saveMemory(file, content),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["memory", file] }),
  });
  return (
    <Page
      title="Memory"
      sub="The project memory spine (agents/*.md). Writes are sandboxed — no path escape."
      action={
        <button className={preview ? "ghost" : ""} onClick={() => setPreview((p) => !p)}>
          {preview ? "Edit" : "Preview"}
        </button>
      }
    >
      <div className="card">
        <div className="toolbar">
          <label className="field">
            <span className="field-label">File</span>
            <select value={file} onChange={(e) => { setFile(e.target.value as typeof file); setDraft(""); }}>
              {MEMORY_FILES.map((f) => <option key={f} value={f}>{f}.md</option>)}
            </select>
          </label>
          {!preview && (
            <button className="primary" disabled={save.isPending || isLoading} onClick={() => save.mutate(draft)}>
              {save.isPending ? "Saving…" : "Save changes"}
            </button>
          )}
        </div>
        {preview ? (
          draft.trim() ? <div style={{ marginTop: 12 }}><Markdown source={draft} /></div> : <p className="muted" style={{ marginTop: 12 }}>Nothing to preview.</p>
        ) : (
          <textarea value={isLoading ? "Loading…" : draft} onChange={(e) => setDraft(e.target.value)} aria-label={`${file}.md`} spellCheck={false} />
        )}
        {save.isSuccess && <p className="hint hint-ok">Saved {file}.md.</p>}
        {save.isError && <p className="hint hint-err">Save failed: {(save.error as Error).message}</p>}
      </div>
    </Page>
  );
}

// ── Engines (serena now present + registered/runner) ──────────────────────
function EnginesPage() {
  const { data, isLoading, error } = useQuery({ queryKey: ["engines"], queryFn: api.engines });
  const engines: { key: keyof Omit<Engines, "mnemopi_on">; label: string; hint: string }[] = [
    { key: "codegraph", label: "codegraph", hint: "local code index (read/find)" },
    { key: "cbm", label: "codebase-memory", hint: "semantic graph" },
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
                  <span className={`tag ${st.present ? "ok" : "warn"}`}>
                    {st.present ? (st.version ? `on ${st.version}` : "on") : "off"}
                  </span>
                </div>
                <p className="tile-hint">{e.hint}</p>
                {e.key === "serena" && st.registered != null && (
                  <div className="tile-sub">
                    <span className={`tag ${st.registered ? "ok" : "warn"}`}>{st.registered ? "registered" : "unregistered"}</span>
                    {st.runner != null && (
                      <span className={`tag ${st.runner ? "ok" : "warn"}`}>{st.runner ? "runner ready" : "no runner"}</span>
                    )}
                  </div>
                )}
              </div>
            );
          })}
          <div className="card tile">
            <div className="tile-head">
              <strong>mnemopi memory</strong>
              <span className={`tag ${data.mnemopi_on ? "ok" : "warn"}`}>{data.mnemopi_on ? "on" : "off"}</span>
            </div>
            <p className="tile-hint">long-term recall / retain</p>
          </div>
        </div>
      )}
    </Page>
  );
}

// ── Bench ──────────────────────────────────────────────────────────────────
function BenchPage() {
  const { data, error, refetch, isFetching } = useQuery({ queryKey: ["bench"], queryFn: api.bench, enabled: false });
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
          <div className="row"><span>Upfront <span className="muted">(paid every session)</span></span><span className="pct mono">~{data.upfront} tok</span></div>
          <div className="row"><span>Deferred <span className="muted">(on trigger)</span></span><span className="mono">~{data.deferred} tok</span></div>
          <div className="row"><span>Force-load prefix</span><span className="mono">~{data.force_load_prefix} tok</span></div>
          <div className="row"><span>A2 progressive disclosure saved</span><span className="pct">{data.a2_saved_pct}%</span></div>
          <div className="row"><span>A1 stable-prefix (KV-cache)</span><span className={`tag ${data.a1_pass ? "ok" : "warn"}`}>{data.a1_pass ? "pass" : "fail"}</span></div>
        </div>
      ) : null}
    </Page>
  );
}

// ── Readiness (eval) ───────────────────────────────────────────────────────
function EvalPage() {
  const { data, error, refetch, isFetching } = useQuery({ queryKey: ["eval"], queryFn: api.evalProject, enabled: false });
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
          <div className="row row-total"><strong>Overall</strong><span className="pct">{data.overall}% <span className="muted">({data.present}/{data.total})</span></span></div>
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

// ── Workspaces ─────────────────────────────────────────────────────────────
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
            <div className="kv"><span className="kv-k">current project</span><span className="kv-v mono">{data.project || "—"}</span></div>
            <div className="kv"><span className="kv-k">session</span><span className="kv-v mono">{data.session || "—"}</span></div>
          </div>
          <div className="card">
            <div className="card-title">Profiles</div>
            {data.profiles.length === 0 ? <p className="muted">No profiles defined.</p> : (
              <div className="list">
                {data.profiles.map((p) => (
                  <div className="row" key={p}>
                    <span className="mono">{p}</span>
                    <button className="primary" onClick={() => activate.mutate(p)} disabled={activate.isPending}>Activate</button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </>
      )}
    </Page>
  );
}

// ── Team ───────────────────────────────────────────────────────────────────
function TeamPage() {
  const { data, error, refetch, isFetching } = useQuery({ queryKey: ["team"], queryFn: api.team, enabled: false });
  return (
    <Page
      title="Team"
      sub="omp subagent roster + per-project readiness."
      action={<button className="primary" onClick={() => refetch()} disabled={isFetching}>{isFetching ? "Loading…" : "Load team"}</button>}
    >
      {error ? <ErrorState message={(error as Error).message} /> : null}
      {isFetching && !data ? <Loading rows={5} /> : null}
      {!data && !isFetching && !error ? (
        <EmptyState title="Team not loaded" hint="Load the roster to see each subagent’s role and skills." />
      ) : null}
      {data ? (
        <div className="card list">
          <div className="row row-total"><strong>Readiness</strong><span className="pct">{data.readiness ? `${data.readiness.overall}%` : "—"}</span></div>
          {data.roster.map((r) => (
            <div className="row" key={r.type}>
              <div className="row-main">
                <div className="row-title mono">{r.type}</div>
                <div className="row-meta">{r.role}</div>
              </div>
              <span className="mono row-skills">{r.skills}</span>
            </div>
          ))}
        </div>
      ) : null}
    </Page>
  );
}

// ── Submodules ─────────────────────────────────────────────────────────────
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
    <Page title="Submodules" sub="Reference repos. Add, pull, or remove.">
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
                <button className="danger" onClick={() => remove.mutate(s.path)} disabled={remove.isPending}>Remove</button>
              </span>
            </div>
          ))}
        </div>
      ) : null}
    </Page>
  );
}

// ── MCP ────────────────────────────────────────────────────────────────────
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

// ── Rules ──────────────────────────────────────────────────────────────────
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
          <span className="muted hint-inline">Paste from a link, file, or folder source.</span>
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
              <button className="danger" onClick={() => del.mutate(r.path)} disabled={del.isPending}>Delete</button>
            </div>
          ))}
        </div>
      ) : null}
    </Page>
  );
}

// ── Workflow (react-flow editor + sample templates) ────────────────────────
// Node = one workflow step (skill / subagent / tool call). Export generates a
// standalone omp extension registering a model-callable `<name>_run` tool.
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
  const color = data.kind === "subagent" ? "var(--accent-bright)" : data.kind === "tool" ? "var(--warn)" : "var(--ok)";
  return (
    <div className="wf-node" style={{ borderColor: color }}>
      <Handle type="target" position={Position.Top} />
      <div className="wf-node-kind" style={{ color }}>{WF_KIND_LABEL[data.kind]}</div>
      <div className="wf-node-label">{data.label}</div>
      {data.ref ? <div className="wf-node-ref">{data.ref}</div> : null}
      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}

// react-flow's NodeTypes is invariant over the Node generic; cast through unknown.
const wfNodeTypes = { wf: WfNodeView } as unknown as NodeTypes;
const elk = new ELK();

function WorkflowPage() {
  const qc = useQueryClient();
  const [name, setName] = useState("demo");
  const [nodes, setNodes, onNodesChange] = useNodesState<Node<WfData>>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selId, setSelId] = useState<string | null>(null);

  const list = useQuery({ queryKey: ["workflows"], queryFn: api.workflows });
  const templates = useQuery({ queryKey: ["wf-templates"], queryFn: api.workflowTemplates });
  const saveMut = useMutation({
    mutationFn: (v: { name: string; nodes: WfNode[]; edges: WfEdge[] }) => api.workflowSave(v.name, v.nodes, v.edges),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["workflows"] }),
  });
  const exportMut = useMutation({ mutationFn: (n: string) => api.workflowExport(n) });

  const selected = nodes.find((n) => n.id === selId) ?? null;
  const onConnect = useCallback((c: Connection) => setEdges((eds) => addEdge(c, eds)), [setEdges]);
  const addNode = (kind: WfKind) => setNodes((ns) => [...ns, makeWfNode(kind)]);
  const updateSel = (patch: Partial<WfData>) => {
    if (!selected) return;
    setNodes((ns) => ns.map((n) => (n.id === selected.id ? { ...n, data: { ...n.data, ...patch } } : n)));
  };
  const loadGraph = (g: { nodes?: WfNode[]; edges?: WfEdge[] } | null) => {
    if (!g) return;
    setNodes((g.nodes ?? []).map((x) => ({ id: x.id, type: "wf", position: x.position, data: x.data })));
    setEdges(g.edges ?? []);
    setSelId(null);
  };
  const load = async (n: string) => { setName(n); loadGraph(await api.workflowGet(n)); };
  const autoLayout = async () => {
    const graph: ElkNode = {
      id: "root",
      layoutOptions: { "elk.algorithm": "layered", "elk.direction": "DOWN" },
      children: nodes.map((n) => ({ id: n.id, width: 184, height: 74 })),
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
            minZoom={0.2}
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
            <p className="muted wf-empty">Select a node to edit, or add one from the toolbar.</p>
          )}
          <h3>Templates</h3>
          {templates.error instanceof EndpointMissingError ? (
            <p className="wf-empty">Templates endpoint not live yet.</p>
          ) : templates.isLoading ? (
            <p className="wf-empty">Loading…</p>
          ) : (templates.data ?? []).length === 0 ? (
            <p className="wf-empty">(none)</p>
          ) : (
            <ul className="wf-list">
              {templates.data?.map((t) => (
                <li key={t.name}><button className="link" onClick={() => { setName(t.name); loadGraph(t); }}>{t.name}</button></li>
              ))}
            </ul>
          )}
          <h3>Saved workflows</h3>
          {list.isLoading ? <p className="wf-empty">Loading…</p> : (
            <ul className="wf-list">
              {(list.data ?? []).length === 0 ? <li className="wf-empty">(none yet)</li> : list.data?.map((n) => (
                <li key={n}><button className="link" onClick={() => load(n)}>{n}</button></li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </Page>
  );
}
