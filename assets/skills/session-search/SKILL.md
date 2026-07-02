---
name: session-search
description: Recover prior omp work from session transcripts. Use when the user asks to find something discussed in a past session, resume a lost thread, or search across previous conversations.
---

# Session Search

Ported from `companion-inc/feynman`'s `session-search` skill (referenced feynman's own `/search` command and `~/.feynman/sessions/` path — neither applies to omp). omp-native version below.

## Direct file search

omp session transcripts are JSONL files under `~/.omp/agent/sessions/<project-slug>/`, one subdirectory per project (slug = the project's absolute path with every `/` replaced by `-`, e.g. `/home/alexdev/Projects/foo` → `-home-alexdev-Projects-foo`).

Search across every project:

```bash
grep -ril "scaling laws" ~/.omp/agent/sessions/
```

Narrow to the current project:

```bash
grep -ril "scaling laws" "$HOME/.omp/agent/sessions/$(pwd | tr '/' '-')/"
```

To read a specific match in full, `read` the JSONL file directly, or use `omp --export <path>` to render it to HTML for easier review (see `omp --help`).
