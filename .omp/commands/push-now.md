---
name: push-now
argument-hint: '[<commit message>]'
description: Urgent cross-machine handoff — rewrite su-code/STATE.md with a cold-resume handoff (branch/HEAD, what changed this session, done, next, blockers, new-machine runbook), update CHANGELOG/KNOWLEDGE if code changed, then git add -A + commit (gitleaks-clean) + push origin current branch. NO PR. For "I'm switching machines right now, capture everything and push."
---

# /push-now — capture state + commit + push (switching machines)

`$ARGUMENTS` = optional commit message (a good default is derived from the diff if empty).

Purpose: I am **about to move to another machine and continue urgently**. Leave the repo so the next machine resumes **cold** — no memory of this session needed. Obey `~/.omp/agent/APPEND_SYSTEM.md` (code-intel first; always-on skills).

## 1. Ground (fast, token-lean)
- `git status --porcelain`, `git branch --show-current`, `git log --oneline -3`, `git diff --stat` (+ `--cached`). `headroom_compress` if >50 lines.
- Read `su-code/STATE.md` (current spine) so the rewrite is a delta, not a reset.

## 2. Rewrite `su-code/STATE.md` for a COLD resume (the important part — be detailed)
Rewrite the `## 🚚 HANDOFF` block (create it right under `## Goal` if absent) with **today's date** and, concretely:
- **Repo state:** branch, HEAD short-sha + subject, latest tag, whether the tree is now clean.
- **What changed THIS session:** the actual files touched (from `git diff --stat`) and WHY — one line each. Name symbols/paths, not vibes.
- **Done ✓** vs **Next / TODO ▸** vs **Blockers ⚠** — explicit checkboxes. A TODO with no owner-obvious next action is useless; write the exact command or file to touch.
- **Per-machine (NOT in git)** gotchas that bit this session, so the next box doesn't rediscover them (e.g. broken `npm`/pnpm shim → feynman won't launch; `8sync harness browser`; custom models in `~/.omp/agent/models.yml`; `8sync feynman auth-omp`). Cross-link `su-code/KNOWLEDGE.md` entries.
- **New-machine runbook** (ordered): `git pull` → `bash scripts/bootstrap.sh` (or install.sh) → `8sync setup` → `8sync harness` → any per-machine re-apply.

## 3. Repo hygiene (only if code changed)
- Code touched → add a `## [Unreleased]`/version bullet to `CHANGELOG.md` (repo convention).
- Learned something durable → append a `validated:`/`failure:` line to `su-code/KNOWLEDGE.md` (append-only; never edit the managed block).
- Do NOT bump the version tag here — `/push-now` is a work-in-progress checkpoint, not a release. (Release = separate, with a tag push.)

## 4. Commit + push (this is the whole point — pushing IS authorized by invoking me)
- `git add -A`.
- Commit. Message = `$ARGUMENTS` if given, else a concise, honest summary of the diff (e.g. `wip(handoff): <area> — <what changed>`). Multi-line body listing the key files is welcome for a cold reader.
- **gitleaks gate:** the repo's pre-commit hook scans staged content; if it blocks on a secret, STOP, unstage that file, tell the user — never `--no-verify` past a real secret.
- `git push` to `origin` current branch. If the branch has no upstream: `git push -u origin <branch>`. If the remote rejected (non-fast-forward), `git pull --rebase` then push; on conflict, stop and report (don't force-push).

## 5. Report (terse)
Print: pushed **`<sha>` `<subject>`** → `origin/<branch>`, files in the commit, and the exact one-liner for the other machine:
`git pull && bash scripts/bootstrap.sh && 8sync harness` (+ any per-machine step from §2).

## Guardrails
Current branch only — never switch/create branches or open a PR (that's `8sync ship`). Never force-push. Never bump a release tag. Scope = the working tree + `su-code/` memory. If the tree is already clean and STATE is current, say so and do nothing rather than an empty commit.

Begin: ground, rewrite STATE for a cold resume, then commit + push `$ARGUMENTS`.
