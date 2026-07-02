---
name: watch
description: Create a research watch baseline for a topic, with a documented follow-up plan since omp has no built-in scheduler. Use when the user wants to monitor a topic for changes over time.
---

# Watch

Ported from `companion-inc/feynman`'s `/watch` slash-command. Self-contained omp-native version.

**No scheduler**: feynman's `/watch` used a `schedule_prompt` tool for recurring follow-ups — omp has no equivalent built-in scheduler. Never claim a recurring watch was scheduled. Options to actually recur: a `cron`/systemd-timer entry the user sets up themselves (give them the exact command), or `8sync harness up --timer` if this is a su-code-managed loop. Record `Scheduling: BLOCKED - no scheduler tool available; see manual follow-up command below` when neither applies.

Derive a slug from the watch topic (lowercase, hyphens, ≤5 words).

## Workflow

1. **Plan** — What to monitor, what signals matter, what counts as a meaningful change, sensible check frequency. Write to `outputs/.plans/<slug>.md`, summarize briefly, continue immediately (don't wait for confirmation unless asked).
2. **Baseline** — Run a baseline sweep with `web_search`/`read`.
3. **Follow-up** — Give the user the exact command to re-run this watch later (e.g. "run `/watch <topic>` again next week", or a `cron` line if they want it automated). Do not claim scheduling happened.
4. **Deliver** — Save exactly one baseline artifact to `outputs/<slug>-baseline.md`, ending with a `Sources` section (direct URL per source).
