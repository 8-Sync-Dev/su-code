# 00 — Force Load Skills (managed by `8sync skill sync`)

## MANDATORY RULE

Before starting **any** non-trivial task you must:

1. Read `~/.forge/skills/karpathy-guidelines/SKILL.md` — no exceptions.
2. Read `~/.forge/skills/image-routing/SKILL.md` to pick image vs text reads.
3. Read `~/.forge/skills/8sync-conventions/SKILL.md` if a project has `AGENTS.md` referencing 8sync.
4. Never dump huge tool output into context; summarize first, then read narrow slices.

## Skill selection guide

| Task type            | Skills to read in order                                          |
|----------------------|------------------------------------------------------------------|
| Any coding task      | **karpathy-guidelines** (always first, mandatory)                |
| Review UI / PDF / diff | **image-routing** (always check before fetching content)       |
| Inside 8sync project | **karpathy** + **image-routing** + **8sync-conventions**         |

## Never skip karpathy

If unsure which skills apply, read karpathy first.
Karpathy overrides any urge to jump straight to implementation.
