# M0-01 — Plan: land pending WIP

Wave 1 is strictly sequential (each task is a commit on the same branch), so
there is no fan-out here — parallel agents on one index would serialise anyway.

| # | Task | AC | UC | skill | verify |
|---|---|---|---|---|---|
| T1 | Commit G1 — `8sync omp update` verb (`verbs/omp.rs` + `main.rs` + `verbs/mod.rs` + `verbs/up.rs`) | AC-02, AC-05 | UC-1 | 8sync-cli | `cargo build --release` |
| T2 | Commit G2 — `branch-sync` skill + `/sync-pr` command + `deploy.rs` wiring | AC-02, AC-05 | UC-1 | branch-sync | `cargo build --release` |
| T3 | Commit G3 — `harness global` `su-code/` auto-stamp | AC-02, AC-05 | UC-1 | 8sync-cli | `cargo build --release` |
| T4 | Commit G4 — `deep-research` §5 protocol + the binary-weight brief in `outputs/` | AC-02, AC-05 | UC-1 | deep-research | `cargo build --release` |
| T5 | Commit G5 — CHANGELOG + KNOWLEDGE + STATE + regenerated AGENTS/CLAUDE + this planning tree | AC-01, AC-05 | UC-1 | — (docs) | `test -z "$(git status --porcelain)"` |
| T6 | Smoke + baseline: run the AC-03 command set, `8sync doctor`, record `stat -c%s` into `M0-VERIFICATION.md` | AC-03, AC-04, AC-06 | UC-1 | — | smoke script exits 0 |

## Notes

- T5 is the only task allowed to touch `su-code/planning/**` — the feature's own
  tree lands with the docs commit.
- `.omp/commands/sync-pr.md` is a *deployed* artifact of `assets/commands/sync-pr.md`.
  Check `git check-ignore` before committing; if `.omp/` is ignored, skip it.
