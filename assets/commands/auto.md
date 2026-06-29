---
name: auto
argument-hint: '[<goal> | status | resume]'
description: 8sync autonomous engine — decompose a goal into slices/tasks and run to DONE on omp core via the code-enforced engine_* tools (durable state, verify-with-retry gate, git worktree). Right-sized, token-lean (codegraph/cbm/serena/headroom), ponytail/karpathy discipline.
---

# /auto — run to done on the 8sync engine

`$ARGUMENTS` first word: `<goal>` = plan + run · `status` = report · `resume`/empty = continue the saved plan.

You drive the **8sync-engine** (model-callable `engine_*` tools). It owns the durable plan state, the verify gate, and worktrees in CODE — you supply judgement. Obey `~/.omp/agent/APPEND_SYSTEM.md` (code-intel first; always-on skills).

## 0. Ground (token-lean)
Read `agents/STATE.md` + the recent `failure:` entries in `agents/KNOWLEDGE.md`. Explore with **codegraph / codebase-memory-mcp / serena** — never grep/Read-all. `headroom_compress` any tool output > ~50 lines.

## 1. Right-size first (ponytail)
Trivial / small (a few files, clear path) → just do it, no engine ceremony. Medium / large / multi-slice → use the engine.

## 2. Research, then plan (research is INTEGRATED into planning)
First **research** the unknowns so the plan is grounded — never guess:
- Scout the codebase: **codegraph** (callers/deps/impact) + **codebase-memory-mcp** (architecture) + **serena** (symbols).
- Domain/external unknowns: **feynman** skills (`deep-research` / `autoresearch` / `literature-review`) + `web_search` / `last30days`. Log decisions under `## Assumptions` in `agents/STATE.md`.
Then call **engine_plan** with the goal + slices; each slice's atomic tasks; each task's `verify` commands = the project's REAL lint/test/build (this is the gate). Smallest-first.

## 3. Loop until done (in an autonomous run, do not yield between tasks)
1. **engine_next** → next task + scoped context. Understand before editing (codegraph callers/deps + `git log/blame` + `agents/DECISIONS.md`).
2. Implement at the right size. Prefer **serena** symbol-level edits over blind whole-file rewrites.
3. **engine_verify** `{taskId}` — the gate runs the commands. FAILED → fix the cause, call engine_verify again. BLOCKED (retries exhausted) → write a `failure:` to `agents/KNOWLEDGE.md`, move to the next unblocked task or escalate.
4. **engine_advance** `{taskId, commit:true}` only after VERIFIED (gitleaks clean).
5. Tick `agents/STATE.md` (Current/Next); distill a `validated:` runbook into `agents/PLAYBOOKS.md` when a multi-step procedure worked.
6. Loop. Use **engine_worktree** to isolate a risky/large slice (open → work → merge squash → remove).

## 4. Closeout — re-evaluate HARD before handing back (large / multi-slice)
When engine_status shows all done, hand over ONLY if green:
1. **Full test suite** (not just changed tests) + add missing coverage.
2. **QA / UAT the running thing** — exercise real end-to-end flows in a **browser** (web app or the desktop web-debug port; see below). Catch what unit tests miss.
3. **Independent re-review** — a fresh `reviewer` (+ `senior-security` for anything sensitive) subagent reads the whole diff vs the goal + Definition-of-Done.
4. Doc-hygiene (`8sync harness audit`) + handoff summary; every goal item ↔ concrete evidence. Any gap → back into the loop.

## Use omp's tools (they are strong)
- **browser** — verify any web/visual change for real (open, screenshot, click), never guess. **Web app** → point it at the dev URL. **Desktop app (Tauri v2 / WRY-WebKit)** → run it with its web-inspector/remote-debug port enabled, then point the SAME `browser` tool at that port — identical DOM/console/network verification as web.
- **task** subagents — independent parallel work / context-heavy isolation / specialization you lack.
- **irc** — coordinate with siblings before editing a shared file.

## Guardrails
Verify-gate before every commit (engine_verify enforces it) · scope to the change + `agents/` memory · NO `git push` / PR unless asked · unattended needs omp `tools.approvalMode: yolo` · stop only on a true blocker (missing credential, irreversible/destructive action), never on ambiguity — pick the reversible option and log it.

## Model + context budget
Use the models in `~/.config/8sync/models.toml` (view/edit: `8sync harness model`); route per task class — cheaper for trivial, stronger for review/debug — **never above the configured ceiling**, and let omp fall back to an authenticated model when the configured one isn't logged in. When context nears the limit (auto-compacts at 50%), write a structured handoff into `agents/STATE.md` BEFORE it fires so the next session resumes clean.

Begin: ground, right-size, then act on `$ARGUMENTS`.
