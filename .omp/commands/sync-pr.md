---
name: sync-pr
argument-hint: '[<branch>]'
description: Sync all branches across any project — audit local/remote branches, deep-preview PR/feature branch changes, safely merge verified branches into main, and update all active branches to latest main with zero conflicts. Powered by the branch-sync skill.
---

# /sync-pr — Multi-branch audit, preview, merge & zero-conflict sync

`$ARGUMENTS` = optional branch name to deep-preview and merge into `main` before syncing all branches.

Purpose: Automatically invoke the **`branch-sync`** skill (`~/.omp/skills/branch-sync/SKILL.md`) to inspect all local and remote git branches, preview changes, merge verified features to `main`, and synchronize every active branch to match latest `main` cleanly without causing conflicts.

---

## Workflow

### 1. Load Skill & Ground State
- Load `~/.omp/skills/branch-sync/SKILL.md` (or `<project>/su-code/skills/branch-sync/SKILL.md`).
- Run `git status --porcelain` to check for uncommitted changes. If dirty, auto-stash or alert before proceeding.
- Fetch all remotes: `git fetch --all --prune`.

### 2. Audit & Inventory
Run branch audit:
```bash
python3 ~/.omp/skills/branch-sync/scripts/branch_sync.py --action audit
```
Lists current branch, main branch, tracking state, ahead/behind commit counts, and last commit dates for all branches.

### 3. Optional Branch Preview & Merge
If `$ARGUMENTS` contains a branch name (e.g. `feature/my-feature`):
1. **Deep Preview:**
   ```bash
   python3 ~/.omp/skills/branch-sync/scripts/branch_sync.py --action preview --branch $ARGUMENTS
   ```
   Shows commits ahead of `main`, file stats, and runs a `git merge-tree` dry-run conflict check.
2. **Safe Merge to Main:**
   If no conflicts:
   ```bash
   python3 ~/.omp/skills/branch-sync/scripts/branch_sync.py --action merge --branch $ARGUMENTS
   ```
   Merges `<branch>` into `main` cleanly and verifies build/tests.

### 4. Zero-Conflict Multi-Branch Sync
Synchronize all remaining active branches with the latest `main`:
```bash
python3 ~/.omp/skills/branch-sync/scripts/branch_sync.py --action sync-all
```
- Checks each branch against latest `main`.
- Dry-runs conflict check via `git merge-tree`.
- Merges `main` into branches that are clean.
- Automatically skips any branch that would produce a merge conflict, reporting the conflict cleanly without corrupting the working tree or losing code.
- Restores original branch upon completion.

### 5. Report
Print a concise summary:
- **Main HEAD:** current `main` short-sha and subject.
- **Merged Branch:** `$ARGUMENTS` (if merged).
- **Updated Branches:** list of updated branches synced to latest `main`.
- **Skipped / Conflicting Branches:** any branch skipped due to conflict risk.

---

## Guardrails
- **Zero Conflict Corruption:** Never force-merge or leave conflict markers on any branch.
- **Uncommitted Work Safety:** Working tree is stashed before sync and restored cleanly.
- **Current Branch Preservation:** Always returns the user to their starting branch after execution.

Begin: audit branches, process `$ARGUMENTS` if provided, sync all branches to latest main, and report results.
