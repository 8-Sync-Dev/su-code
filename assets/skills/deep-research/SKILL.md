---
name: deep-research
description: Run a thorough, source-heavy investigation or architectural analysis on any topic or codebase. Use when the user asks for deep research, comprehensive analysis, in-depth technical report, multi-source investigation, or complex AI agent design pattern research. Produces a cited research brief with provenance tracking and loop-engineering verification.
---

# Deep Research & AI Agent Loop Engineering

Ported and enhanced for the `omp` + `8sync` harness runtime. Performs source-verified research across external domain sources and internal codebase architecture using STEP-0 code intelligence (`codegraph`, `codebase-memory-mcp`, `serena`, `headroom`).

---

## Tool Mapping & Capability Routing

| Need | Harness Tool | Pattern & Guidance |
|---|---|---|
| Web Search | `web_search` | Run ≥3 distinct queries; prefer primary sources, official docs, papers. |
| URL Fetch | `read <url>` | Reader-mode clean text. NEVER guess content. |
| Codebase Architecture | `mcp__codebase_memory_mcp_get_architecture` | Surface de-facto package/service clusters and cross-module seams. |
| Graph Symbol & Call Paths | `mcp__codebase_memory_mcp_search_graph` / `_trace_path` | Search graph definitions and callers/callees sub-ms without grep dumps. |
| Precise Symbol Lookup | `mcp__serena_find_symbol` / `_find_referencing_symbols` | LSP-precise symbol declarations and reference tracking. |
| Fast Code Graph Explore | `codegraph query` / `explore` / `impact` | Fast CLI semantic code intel; explore call paths in one call. |
| Subagent Parallel Fan-Out | `task` (batch `tasks[]` array) | Dispatch up to 32 parallel subagents in wave execution with shared `context`. |
| Large Output Compression | `mcp__headroom_compress` | Compress outputs >50 lines by 60–95% before re-emitting into context. |
| Vision & Visual Grounding | `zai-vision` MCP / `8sync locate` | Route images/diagrams/screenshots through OCR/vision; locate coords via ggml. |
| User Clarification | `ask` | Ask only when trade-offs require human decision. |
| Durable Memory | `retain` | Persist architectural decisions, user preferences, and verified facts. |

---

## AI Agent Design Patterns for Real-World Systems

### 1. Loop Engineering & State Machines
- **Plan → Execute → Verify → Advance:** Every complex research or implementation task operates on an explicit state machine (persisted in `STATE.md` and `outputs/.plans/<slug>.md`).
- **Code-Enforced Verification Gate:** Never claim completion without proof. Research requires source/tool verification; code requires build/test/execution output.
- **Fail-Fast & Self-Correction:** When a hypothesis or tool fails, record the root cause in `su-code/KNOWLEDGE.md` (`failure:` entry), adjust the plan, and retry without looping blindly.

### 2. STEP-0 Code-Intel First Discipline
- **Zero-Grep Exploration:** Code structural questions MUST use `codegraph`, `codebase-memory-mcp`, or `serena` before raw text search (`grep`/`glob`).
- **Read-Before-Edit:** Read raw file lines only when preparing an anchored edit.
- **Impact Radius Check:** Run symbol reference checks (`_find_referencing_symbols` or `codegraph impact`) before altering exported interfaces.

### 3. Wave Execution & Multi-Agent Orchestration
- **Parallel Fan-out / Single Synthesis:** Dispatch independent sub-investigations simultaneously in a single `task` call containing multiple items in `tasks[]`.
- **Verifier-Implementer Separation:** Delegate verification to a dedicated `Verifier` or `Reviewer` role subagent with a clean prompt context.
- **Shared Contracts in Context:** Pass shared API schemas, output structures, and non-goals in the batch `context` parameter.

### 4. Token Discipline & Modality Fit
- **Headroom Compression:** Use `mcp__headroom_compress` on large search outputs, transcripts, or draft briefs before re-emitting.
- **Modality-Fit Routing:** Render high-entropy structural diagrams, graphs, or dashboards with `8sync shot` and process via vision; read exact code, line numbers, and configs as text.
- **Ponytail (YAGNI):** Write the minimal code/prose necessary. Delete unnecessary abstractions. Prefer simple, boring, observable patterns.

### 5. Native & Binary-Weight Audits (measure, never assume)
When the question is *"can Zig/Rust/native tooling make this leaner or faster?"*, opinions are worthless — only A/B byte counts and timings count. Protocol:

1. **Ground the artifact.** `stat -c%s <bin>` and `size -A <bin>` (`.text` = code, `.rodata` = embedded data, `.eh_frame`/`.rela.dyn` = link overhead). Compare against the project's own stated budget before proposing anything.
2. **Attribute before optimising.** `cargo bloat --release --crates -n 25` for per-crate `.text`. Treat its `[Unknown]` row as C code and chase it with `du -h` on the `build/*/out/*.a` blobs. The tool prints *"numbers are a result of guesswork"* — quote that caveat rather than laundering its output as exact.
3. **A/B every proposed flag.** Build variants into a scratch `--target-dir`, always with an explicit `--target` so `RUSTFLAGS` do not reach host proc-macros (`-C relocation-model=static` breaks them otherwise). Use `CARGO_PROFILE_RELEASE_*` env overrides to test profile changes without editing `Cargo.toml`.
4. **Report negative results.** A falsified knob is a durable `failure:` entry in `su-code/KNOWLEDGE.md` that stops the next session re-litigating it. Size folklore (`opt-level="s"` is smaller, `panic="abort"` frees `.eh_frame`) is frequently false on real binaries.
5. **Trace surprise dependencies.** `cargo tree -i <crate>` — shared build/runtime crates routinely link compress-side or codegen-side code into a binary that only needs the read side.
6. **Separate the real levers.** Optional subsystems and embedded asset trees usually dwarf every compiler flag: prefer `[features]` gating and un-embedding over rewrites. Per ponytail, gate before you rewrite.
7. **Right-size the language question.** Reach for Zig/C/asm only with a profile showing a compute hot path. For IO/orchestration CLIs the answer is build tooling (`cargo-zigbuild` as cross-linker, `universal2` fat binaries) — never a second language runtime. Verify platform support in the upstream README before promising a CI simplification.

---

## Required Artifacts

Derive a slug from the topic (lowercase, hyphenated, ≤5 words). Every deep research run leaves:
- `outputs/.plans/<slug>.md` — The Plan (key questions, scale decision, task ledger, verification log)
- `outputs/.drafts/<slug>-draft.md` — First Draft (findings by theme, evidence-backed caveats)
- `outputs/.drafts/<slug>-cited.md` — Cited Draft (verified URLs / code references + Sources section)
- `outputs/<slug>.md` — Final Research Brief
- `outputs/<slug>.provenance.md` — Provenance Sidecar (date, rounds, source counts, verification status)

---

## 7-Step Execution Workflow

1. **Plan** — Write `outputs/.plans/<slug>.md`: key questions, evidence needed, scale decision, task ledger, verification log. Summarize briefly to user.
2. **Scale Decision:**
   - *Explainer (3-10 tool calls):* Execute directly inline, no subagents.
   - *Comparison (2-3 items):* Spawn 2 `task` subagents in one batch.
   - *Broad Survey:* Spawn 3-4 parallel `task` subagents.
   - *Complex Multi-Domain System:* Spawn 4-6 parallel `task` subagents with role specs (`Researcher`, `Verifier`, `Architect`).
3. **Gather Evidence:**
   - *Web Research:* Execute ≥3 distinct `web_search` queries across primary sources.
   - *Codebase Research:* Use `codebase-memory-mcp` (`get_architecture`, `search_graph`, `trace_path`) or `codegraph` to trace real call chains.
4. **Draft Synthesis:**
   - Write `outputs/.drafts/<slug>-draft.md` yourself.
   - Categorize findings by theme, state evidence-backed caveats, and highlight open questions.
   - Compress intermediate drafts exceeding 50 lines with `mcp__headroom_compress`.
5. **Citations & Provenance:**
   - Verify every cited URL (`read <url>`) or codebase symbol (`mcp__serena_find_symbol`).
   - Copy to `<slug>-cited.md` with explicit inline links and a Sources section.
6. **Review & Self-Correction:**
   - Run verification pass (`Verifier` subagent or self-review). Classify findings into FATAL / MAJOR / MINOR.
   - Fix FATAL issues immediately before final synthesis.
7. **Deliver:**
   - Write final candidate to `outputs/<slug>.md`.
   - Write `outputs/<slug>.provenance.md`. Confirm all files exist on disk before yielding.
