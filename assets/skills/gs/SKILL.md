---
name: gs
description: Use when the user wants an autonomous professional-team build — plan, implement, verify, commit, and advance a feature end-to-end with minimal hand-holding ("build X for me", "run the team", "autonomous", "ship this end to end", "run treo"). Explains the /gs command and the team-loop protocol so the loop runs token-lean and to spec.
---

# gs — autonomous engineering team loop

Invoke with the **`/gs`** slash command (one command, arg-routed):
`/gs <goal>` plan+run · bare `/gs` resume · `/gs auto` unattended · `/gs status|next|stop`.

**Loop** (driven off `agents/STATE.md`, the live plan): plan -> pick slice -> delegate to a specialist role (subagent, own context) -> **verify-gate** (independent build/test) -> commit -> record (STATE / KNOWLEDGE / PLAYBOOKS) -> advance. Runs until Definition-of-Done or a blocker; in `auto` it never yields between slices.

**Token discipline (always):** explore via codegraph + codebase-memory-mcp (never grep / read-all); compress any output over ~50 lines with `headroom_compress`; load skill bodies on trigger only.

**Roles:** planner / eng-manager / designer / implementer / reviewer / QA / security / release — use gstack role skills if installed, else bundled (`plan` agent, `code-review-and-quality`, `senior-security`, `impeccable`, `taste`) + `task` subagents.

**Guardrails:** verify-gate before every commit · no push/PR unless asked · L3/unattended uses an isolated git worktree + hard-stop via `/gs stop` or `.gs/STOP`. Run 24/7 with `8sync harness up --timer 30m`.

Full protocol lives in the `/gs` command body: `~/.omp/agent/commands/gs.md` (project copy: `<repo>/.omp/commands/gs.md`).
