---
name: jobs
description: Inspect visible research run state, background subagent jobs, and durable watch/autoresearch/replication artifacts. Use when the user asks what's running for a research workflow or wants research-run status.
---

# Jobs

Ported from `companion-inc/feynman`'s `/jobs` slash-command. Self-contained omp-native version.

## Workflow

1. **Background jobs** — Use omp's `job` tool with `list: true` to show active background subagents/tasks (this is omp's equivalent of feynman's `process` tool).
2. **Scheduling** — omp has no built-in scheduler (see the `watch` skill). Record `Schedule state: BLOCKED - no scheduler tool available` rather than claiming a recurring watch exists.
3. **Durable state** — Inspect `outputs/.plans/`, `outputs/`, `experiments/`, and `notes/` (via `glob`/`read`) for watch baselines, autoresearch logs (`autoresearch.md`/`.jsonl`), replication runs, and recent research artifacts.
4. **Summarize** — active background jobs (from step 1); durable watch/autoresearch/replication artifacts found on disk (from step 3); failures that need attention; the next concrete command the user should run for logs or detailed status. Be concise and operational.
