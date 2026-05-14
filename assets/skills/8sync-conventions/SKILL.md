# 8sync-conventions — how forge should behave inside an 8sync project

**Active in any project that contains `AGENTS.md` referencing 8sync.**

## Project memory protocol

8sync maintains a `.gsd/` directory in each project:

| File              | Purpose                                              |
|-------------------|------------------------------------------------------|
| `PROJECT.md`      | Stack, entrypoints, conventions detected             |
| `KNOWLEDGE.md`    | Cumulative learning (architecture, gotchas, recipes) |
| `DECISIONS.md`    | One-line entries: "we chose X over Y because …"      |
| `PREFERENCES.md`  | User style: naming, formatting, libraries, etc.      |
| `STATE.md`        | In-flight work: where we left off, next steps        |

**Read these at session start.** Append to them when learning something the user would want next session. Do NOT overwrite — append with `<!-- date -->` markers.

## End-of-session capture

When user runs `8sync end` (or the session is ending), output a single message with four fenced blocks:

```
<DECISIONS>
- one-line decisions made in this session
</DECISIONS>

<KNOWLEDGE>
- facts about the codebase you now know
</KNOWLEDGE>

<PREFERENCES>
- user style preferences observed
</PREFERENCES>

<STATE>
- what is in-flight, next step
</STATE>
```

8sync parses these and appends to `.gsd/*.md`.

## Token discipline

- Prefer `8sync mcp get_project_outline` over reading whole files in big repos.
- Use `8sync shot`, `8sync pdf-img`, `8sync diff-img` for visual content.
- Summarize tool output before re-reading.

## Conventions

- Cite code with `filepath:start-end`.
- Use `8sync ship` (not raw `git push`) for the final commit/push/PR — it integrates message linting and PR template.
- Never modify files in `.gsd/` directly; let 8sync write them.
