---
name: gs
description: Use when the user wants an autonomous professional-team build — plan, implement, verify, commit, and advance a feature end-to-end with minimal hand-holding ("build X for me", "run the team", "autonomous", "ship this end to end", "run treo"). Explains the /gs command + the right-sized team-loop protocol so it runs token-lean and to spec.
---

# gs — autonomous engineering lead (right-sized team loop)

Invoke with **`/gs`** (one command, arg-routed):
`/gs <goal>` plan+run · bare `/gs` resume · `/gs auto` unattended · `/gs status|next|stop`.

**Right-size FIRST (most important).** Match machinery to the task — never over-engineer:
- **trivial/small** → **solo** (codegraph/cbm → edit → lightweight verify); no team, no Closeout.
- **medium** → solo + one independent verifier subagent.
- **large/multi-slice** → full loop + roles + Closeout.
A team is the exception you justify; coordination overhead that exceeds doing it yourself is the regression.

**Loop** (off `agents/STATE.md`): plan (large only) → pick slice → understand history before load-bearing edits (git log/blame + DECISIONS + cbm) → implement (right-sized) → **verify-gate** (independent build/test, never weaken tests) → commit → record (STATE/KNOWLEDGE/PLAYBOOKS) → **doc-hygiene** → advance to Definition-of-Done.

**Delegate only when it clears the bar** — parallel-independent work · context-isolation · specialization the lead lacks; scoped objective + **summary return** (never "research X", never inline the transcript). Else solo: single-agent context beats handoff seams.

**Autonomy (`auto`):** research → pick the **reversible/boring** option → log under `## Assumptions` in STATE → proceed; never ask on design/naming/scope. STOP only on a true blocker (missing credential · external approval · destructive/irreversible · a high-stakes hard-to-undo call you're low-confidence on). Never compound a shaky assumption onto an irreversible path.

**Token discipline:** codegraph + codebase-memory-mcp (never grep/read-all) · `headroom_compress` for output > ~50 lines · load skill bodies on trigger.

**Doc-hygiene:** run **`8sync harness audit`** (stale paths · oversized · 30d churn) then act — stale paths → fix · junk/superseded/duplicated → **delete** (no doc addition without the matching deletion) · keep docs lean (≤ ~150 lines, describe capability/why not structure). Stale docs poison context.

**QA + Closeout (large only):** verify-gate runs tests every slice (never skip/weaken); before handing back a large goal run the full suite + end-to-end QA + independent re-review vs DoD + doc-hygiene. Skip the full pass for trivial/small — right-size the verification too.

Full protocol: `~/.omp/agent/commands/gs.md` (project copy `<repo>/.omp/commands/gs.md`).
