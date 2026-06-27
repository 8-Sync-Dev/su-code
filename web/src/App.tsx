import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type SkillEntry, type Engines } from "./api";

type Page = "state" | "skills" | "memory" | "engines" | "bench" | "eval" | "workspaces" | "team" | "submodules";
const NAV: { id: Page; label: string }[] = [
  { id: "state", label: "State" },
  { id: "skills", label: "Skills" },
  { id: "memory", label: "Memory" },
  { id: "engines", label: "Engines" },
  { id: "bench", label: "Bench" },
  { id: "eval", label: "Readiness" },
  { id: "workspaces", label: "Workspaces" },
  { id: "team", label: "Team" },
  { id: "submodules", label: "Submodules" },
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
