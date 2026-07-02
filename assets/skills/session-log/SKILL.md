---
name: session-log
description: Write a durable session log capturing completed work, findings, open questions, and next steps. Use when the user asks to log progress, save session notes, write up what was done, or create a research diary entry.
---

# Session Log

Ported from `companion-inc/feynman`'s `/log` slash-command. Self-contained omp-native version.

## Workflow

1. Summarize what was done this session.
2. Capture the strongest findings or decisions. Durable, reusable facts (decisions, preferences, project context) → also `retain` them so future sessions have them without re-reading this log.
3. List open questions, unresolved risks, concrete next steps.
4. Reference important artifacts written to `notes/`, `outputs/`, `experiments/`, or `papers/` this session.
5. Include direct source URLs for any external claims that matter.
6. Save the log to `notes/` as Markdown with a date-oriented filename (e.g. `notes/2026-07-02-session.md`).

If this project already uses `agents/STATE.md`/`agents/KNOWLEDGE.md` (8sync harness convention), prefer updating those over a bespoke `notes/` file — check for their existence first.
