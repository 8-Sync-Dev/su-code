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

## 2. Plan
Call **engine_plan** with the goal + slices; each slice's atomic tasks; each task's `verify` commands = the project's REAL lint/test/build (this is the gate). Smallest-first.

## 3. Loop until done (in an autonomous run, do not yield between tasks)
1. **engine_next** → next task + scoped context. Understand before editing (codegraph callers/deps + `git log/blame` + `agents/DECISIONS.md`).
2. Implement at the right size. Prefer **serena** symbol-level edits over blind whole-file rewrites.
3. **engine_verify** `{taskId}` — the gate runs the commands. FAILED → fix the cause, call engine_verify again. BLOCKED (retries exhausted) → write a `failure:` to `agents/KNOWLEDGE.md`, move to the next unblocked task or escalate.
4. **engine_advance** `{taskId, commit:true}` only after VERIFIED (gitleaks clean).
5. Tick `agents/STATE.md` (Current/Next); distill a `validated:` runbook into `agents/PLAYBOOKS.md` when a multi-step procedure worked.
6. Loop. Use **engine_worktree** to isolate a risky/large slice (open → work → merge squash → remove).

## 4. Closeout (large / multi-slice)
When engine_status shows all done: full test suite + end-to-end QA + doc-hygiene (`8sync harness audit`) + a handoff summary; every goal item ↔ concrete evidence.

## Use omp's tools (they are strong)
- **browser** — verify any web/visual change for real (open the page, screenshot, click through) instead of guessing.
- **task** subagents — independent parallel work / context-heavy isolation / specialization you lack.
- **irc** — coordinate with siblings before editing a shared file.

## Guardrails
Verify-gate before every commit (engine_verify enforces it) · scope to the change + `agents/` memory · NO `git push` / PR unless asked · unattended needs omp `tools.approvalMode: yolo` · stop only on a true blocker (missing credential, irreversible/destructive action), never on ambiguity — pick the reversible option and log it.

Begin: ground, right-size, then act on `$ARGUMENTS`.
