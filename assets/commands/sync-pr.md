---
name: sx-sync-pr
argument-hint: '[<branch>] [--push] [--main <name>]'
description: Multi-branch audit, deep-preview, safe merge to main, and zero-conflict sync of every active branch to latest main. Prefers the branch-sync skill's script when installed; falls back to native git so it never hard-depends on a vendored file. Local-only by default (--push to also push) — honours the no-unprompted-push guardrail the other sx- commands keep.
---

# /sx-sync-pr — audit · preview · merge · zero-conflict sync

`$ARGUMENTS` = optional branch to deep-preview and merge into `main` before syncing the rest.
- `--push` — also push (merged `main` and synced branches) to `origin`. **Default: local-only** —
  this command merges/syncs locally and reports the exact push commands; it does not push unless you
  pass `--push` or ask. This matches the guardrail every other sx- command keeps.
- `--main <name>` — override the integration branch (default `main`; auto-detects `master`).

This command **extends omp canonically**: it composes **git** (the source of truth for branch state)
+ the **`branch-sync`** skill's script when present, with a native-git fallback so an omp/skill
upgrade or a missing skill never breaks it. It reinvents nothing — the fallback is the literal git
recipe the skill's own `SKILL.md` documents.

## 0. Ground (fast, safe — do this every time)
1. **Remember the start branch** — `START=$(git rev-parse --abbrev-ref HEAD)`. You MUST return here
   at the end, no matter what fails.
2. **Dirty tree?** `git status --porcelain`. If non-empty: **STOP** — show the user and do not
   stash-or-clobber. (The skill's script *exits* on a dirty tree at merge time; matching that reality
   here avoids the "auto-stash" claim the old doc made but the script never implemented.) The user
   stashes/commits, then re-runs.
3. `git fetch --all --prune` — establish the real remote state before any ahead/behind claim.

## 1. Choose the engine (existence guard)
```bash
SCRIPT=~/.omp/skills/branch-sync/scripts/branch_sync.py
[ -f "$SCRIPT" ] && echo "script-present" || echo "fallback-native"
```
- **script-present** → use it for the fast path (steps 3–5 invoke `python3 $SCRIPT --action …`).
- **fallback-native** → use the native-git recipe in each step (the `<fallback>` block). This is the
  skill's own documented recipe, so behaviour matches; you just lose the pretty summary formatting.

Either way the **gates in steps 2, 4.0, and 6 apply to both paths** — they are command-layer, not
script internals.

## 2. main-up-to-date gate (NEW — the silent-stale-main trap)
The script runs `git pull --ff-only origin main` with errors silenced (`check=False`); if `main`
diverged, it **proceeds with stale main and merges behind**. Assert instead:
```bash
git checkout "$MAIN" || { echo "no $MAIN branch"; exit 1; }
git pull --ff-only origin "$MAIN"      # NOT silenced — surface divergence
# Compare SHAs directly (unambiguous — no rev-list direction to get backwards):
[ "$(git rev-parse "$MAIN")" = "$(git rev-parse "origin/$MAIN")" ] \
  || { echo "ERROR: $MAIN != origin/$MAIN after ff-pull — diverged. STOP."; exit 1; }
```
On divergence: STOP, report, and tell the user the rebase/merge is a separate decision. Never sync
branches onto a stale `main`.

## 3. Audit & inventory
```bash
python3 "$SCRIPT" --action audit --main "$MAIN"
# <fallback>: git for-each-ref --format='%(refname:short) | ahead-behind vs $MAIN: %(ahead-behind:$MAIN) | %(subject) (%(authordate:relative))' refs/heads/
```
Read it: which branches are ahead of `main` (unmerged work), behind (stale), or clean. This drives
step 5.

## 4. Branch preview + merge (only if `$ARGUMENTS` names a branch)
**4.0 Validate the argument (NEW — closes the injection gap).** The skill's script builds git
commands with `shell=True` + f-string interpolation of the branch name, and the old command passed
`$ARGUMENTS` raw as `--branch`. A value with shell metacharacters or a leading `-` is an injection.
Gate it at the command layer:
```bash
B="$1"   # the branch token from $ARGUMENTS (strip any --flags first)
case "$B" in ''|-*) echo "no branch given (or it started with '-'): $B"; exit 1;; esac
git rev-parse --verify "refs/heads/$B" >/dev/null 2>&1 || { echo "not a local branch: $B"; exit 1; }
[ -z "$(printf '%s' "$B" | tr -d 'A-Za-z0-9._/-')" ] || { echo "branch name has shell-unsafe chars: $B"; exit 1; }
```
Only a real local branch passes. Then:

**4.1 Deep preview** (commits, file stats, conflict pre-check via `git merge-tree`):
```bash
python3 "$SCRIPT" --action preview --branch "$B" --main "$MAIN"
# <fallback>: git log "$MAIN..$B" --oneline; git diff "$MAIN...$B" --stat
#             base=$(git merge-base "$MAIN" "$B"); git merge-tree "$base" "$MAIN" "$B" | grep -q '<<<<<<<' && echo CONFLICT || echo CLEAN
```

**4.2 Safe merge to main** (only if 4.1 reported CLEAN):
```bash
python3 "$SCRIPT" --action merge --branch "$B" --main "$MAIN"
# <fallback>: git checkout "$MAIN"; git merge --no-ff "$B" -m "feat: merge $B into $MAIN"
#             # on conflict: git merge --abort; git checkout "$START"; exit 1
```
**Note the script's merge is LOCAL-ONLY** — it does not `git push origin main` (the skill's `SKILL.md`
step 4 says to push, but the script doesn't). So neither does this command, unless `--push`:
```bash
[ "$PUSH" = "1" ] && git push origin "$MAIN" || echo "main merged locally; push with: git push origin $MAIN"
```

## 5. Zero-conflict multi-branch sync → latest main
```bash
python3 "$SCRIPT" --action sync-all --main "$MAIN"
```
**⚠ Known script behaviour — `sync-all` silently pushes every synced branch** (`git push origin <b>`
inside the loop, `check=False`). If you want the no-push guarantee, use the fallback instead:
```bash
# <fallback, NO silent push>:
git checkout "$MAIN"
for b in $(git for-each-ref --format='%(refname:short)' refs/heads/); do
  case "$b" in "$MAIN"|master) continue;; esac
  git rev-list --count "$b..$MAIN" | grep -q '^0$' && continue   # already up to date
  base=$(git merge-base "$MAIN" "$b")
  if git merge-tree "$base" "$b" "$MAIN" | grep -q '<<<<<<<'; then
    echo "SKIP $b (conflict)"; continue
  fi
  git checkout "$b" && git merge "$MAIN" -m "sync: update $b with latest $MAIN" \
    || { git merge --abort; echo "SKIP $b (merge failed)"; }
  [ "$PUSH" = "1" ] && git push origin "$b"
done
```
Either path: any branch that would conflict is **skipped and reported**, never force-merged; the
working tree is never left with conflict markers.

## 6. Acceptance check (NEW — a sync that "ran" is not a sync that "worked")
```bash
git checkout "$START"                                 # MUST return to the start branch
git status --porcelain | grep -q . && echo "FAIL: tree dirty" || echo "OK: tree clean"
# Branches still behind $MAIN (should be only the conflict-skipped ones).
# %(ahead-behind) prints "ahead N, behind M" — match "behind 0" with NO colon:
git for-each-ref --format='%(refname:short) %(ahead-behind:'"$MAIN"')' refs/heads/ \
  | grep -v 'behind 0' || echo "OK: no branch behind $MAIN"
grep -RIn '<<<<<<<' . 2>/dev/null | grep -v '.git/' | head || echo "OK: no conflict markers"
```
Report: start branch restored · tree clean · branches still behind `main` (should be only the
conflict-skipped ones) · no stray conflict markers · per-branch merged/skipped/conflicted counts.

## Guardrails
Never force-push (`--force`/`-f`) or force-merge; never leave conflict markers (on conflict: `--abort`
+ restore `$START` + report). Always return to `$START`. **No push unless `--push` given or the user
asked** — this is the one guardrail the old command silently broke (script `sync-all` pushes). Scope =
branches only; do not switch remotes, delete branches, or touch `main` beyond the explicit merge. If
`$ARGUMENTS` is not a real local branch, refuse rather than pass it through.

## Note on a deeper fix (out of scope here)
The skill's `branch_sync.py` itself uses `subprocess.run(shell=True)` with f-string branch
interpolation and silences several `git` failures (`check=False`). The command-layer gates above
(validate branch, assert main current, no-push default, acceptance check) neutralise the
command-facing risk. Hardening the script itself (switch to argument lists, propagate failures) is a
separate change to the `branch-sync` skill — flag it, do not mix it into a command edit.

Begin: ground + remember `$START`, pick the engine, gate `main`, audit, then act on `$ARGUMENTS` and
sync — local-only unless `--push`.
