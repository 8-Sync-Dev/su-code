#!/usr/bin/env python3
"""
Branch Sync Helper Script (`branch_sync.py`)
Performs multi-branch audit, deep diff preview, safe merge to main, and zero-conflict sync across all branches.
"""

import sys
import os
import subprocess
import argparse

def run_cmd(cmd, check=True, capture=True):
    res = subprocess.run(cmd, shell=True, text=True, capture_output=capture)
    if check and res.returncode != 0:
        print(f"Error running command: {cmd}\nStderr: {res.stderr.strip()}", file=sys.stderr)
    return res

def get_current_branch():
    res = run_cmd("git rev-parse --abbrev-ref HEAD")
    return res.stdout.strip()

def list_branches():
    run_cmd("git fetch --all --prune", check=False)
    res = run_cmd("git branch --format='%(refname:short)'")
    branches = [b.strip() for b in res.stdout.strip().split("\n") if b.strip()]
    return branches

def audit_branches(main_branch="main"):
    print("=== GIT BRANCH AUDIT ===")
    current = get_current_branch()
    branches = list_branches()
    print(f"Current branch: {current}")
    print(f"Main branch:    {main_branch}\n")
    
    for b in branches:
        if b == main_branch:
            continue
        ahead_behind = run_cmd(f"git rev-list --left-right --count {main_branch}...{b}", check=False).stdout.strip()
        parts = ahead_behind.split()
        behind, ahead = (parts[0], parts[1]) if len(parts) >= 2 else ("?", "?")
        last_commit = run_cmd(f"git log -1 --format='%s (%cr)' {b}", check=False).stdout.strip()
        marker = "* " if b == current else "  "
        print(f"{marker}{b:<24} | Ahead: {ahead:<3} | Behind: {behind:<3} | Last: {last_commit}")

def preview_branch(branch, main_branch="main"):
    print(f"=== DEEP PREVIEW: {branch} vs {main_branch} ===")
    # Commits
    commits = run_cmd(f"git log {main_branch}..{branch} --oneline", check=False).stdout.strip()
    print(f"\n--- Commits on {branch} not in {main_branch} ---")
    print(commits if commits else "(None)")
    
    # Stat
    stat = run_cmd(f"git diff {main_branch}...{branch} --stat", check=False).stdout.strip()
    print(f"\n--- Changed Files & Stats ---")
    print(stat if stat else "(No changes)")
    
    # Conflict check
    base = run_cmd(f"git merge-base {main_branch} {branch}", check=False).stdout.strip()
    if base:
        mt = run_cmd(f"git merge-tree {base} {main_branch} {branch}", check=False).stdout
        if "<<<<<<<" in mt:
            print("\n⚠️ WARNING: Conflicts detected if merged to main!")
        else:
            print("\n✅ Clean merge expected (no conflicts detected).")

def merge_to_main(branch, main_branch="main"):
    print(f"=== SAFE MERGE: {branch} -> {main_branch} ===")
    curr = get_current_branch()
    
    # Check dirty
    status = run_cmd("git status --porcelain").stdout.strip()
    if status:
        print("Error: Working directory has uncommitted changes. Stash or commit first.", file=sys.stderr)
        sys.exit(1)
        
    print(f"Switching to {main_branch}...")
    run_cmd(f"git checkout {main_branch}")
    run_cmd(f"git pull --ff-only origin {main_branch}", check=False)
    
    print(f"Merging {branch} into {main_branch}...")
    res = run_cmd(f"git merge --no-ff {branch} -m 'feat: merge {branch} into {main_branch}'", check=False)
    if res.returncode == 0:
        print(f"✅ Successfully merged {branch} into {main_branch}.")
    else:
        print(f"❌ Merge failed. Conflict detected! Aborting merge...", file=sys.stderr)
        run_cmd("git merge --abort", check=False)
        run_cmd(f"git checkout {curr}", check=False)
        sys.exit(1)

def sync_all(main_branch="main"):
    print(f"=== ZERO-CONFLICT SYNC ALL BRANCHES -> LATEST {main_branch} ===")
    curr = get_current_branch()
    branches = list_branches()
    
    # Ensure main is fresh
    run_cmd(f"git checkout {main_branch}")
    run_cmd(f"git pull --ff-only origin {main_branch}", check=False)
    
    successes = []
    skipped = []
    conflicts = []
    
    for b in branches:
        if b in [main_branch, "master"]:
            continue
            
        # Check if behind main
        behind = run_cmd(f"git rev-list --count {b}..{main_branch}", check=False).stdout.strip()
        if behind == "0":
            skipped.append((b, "Already up to date"))
            continue
            
        # Check conflict potential
        base = run_cmd(f"git merge-base {main_branch} {b}", check=False).stdout.strip()
        if base:
            mt = run_cmd(f"git merge-tree {base} {b} {main_branch}", check=False).stdout
            if "<<<<<<<" in mt:
                conflicts.append((b, f"Conflict detected when merging {main_branch} into {b}"))
                continue
                
        # Attempt checkout & merge
        run_cmd(f"git checkout {b}", check=False)
        mres = run_cmd(f"git merge {main_branch} -m 'sync: update {b} with latest {main_branch}'", check=False)
        if mres.returncode == 0:
            successes.append(b)
            # Try push if remote branch exists
            run_cmd(f"git push origin {b}", check=False)
        else:
            run_cmd("git merge --abort", check=False)
            conflicts.append((b, "Merge failed during execution"))
            
    # Return to original
    run_cmd(f"git checkout {curr}", check=False)
    
    print("\n=== SYNC SUMMARY ===")
    print(f"Updated:   {len(successes)} branches ({', '.join(successes) if successes else 'None'})")
    print(f"Skipped:   {len(skipped)} branches")
    if conflicts:
        print(f"⚠️ Conflicts: {len(conflicts)} branches (skipped to prevent corruption):")
        for cb, reason in conflicts:
            print(f"   - {cb}: {reason}")

def main():
    parser = argparse.ArgumentParser(description="Multi-branch management & sync tool")
    parser.add_argument("--action", choices=["audit", "preview", "merge", "sync-all"], default="audit")
    parser.add_argument("--branch", help="Target branch for preview or merge")
    parser.add_argument("--main", default="main", help="Main branch name (default: main)")
    
    args = parser.parse_args()
    
    if args.action == "audit":
        audit_branches(args.main)
    elif args.action == "preview":
        if not args.branch:
            print("Error: --branch required for preview", file=sys.stderr)
            sys.exit(1)
        preview_branch(args.branch, args.main)
    elif args.action == "merge":
        if not args.branch:
            print("Error: --branch required for merge", file=sys.stderr)
            sys.exit(1)
        merge_to_main(args.branch, args.main)
    elif args.action == "sync-all":
        sync_all(args.main)

if __name__ == "__main__":
    main()
