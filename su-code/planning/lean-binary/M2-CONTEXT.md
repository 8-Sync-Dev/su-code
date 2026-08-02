# M2 — Eliminate, don't just gate

## 📌 Requirement scope

UC-4 — the **released** binary gets smaller, because dead weight is deleted
rather than merely made optional.

## 🎯 Goal

Remove weight from the default build without removing a feature. M1's A/B picked
the targets; this phase spends them.

## Target 1 — `rusqlite` (measured 1 060 840 B)

The decisive finding is not "SQLite is big", it is that **the database does
nothing**. `toolstats::ingest` opens with `DELETE FROM calls` (`toolstats.rs:77`)
and re-parses every session JSONL on every run, so nothing is ever carried
across invocations. The module's own doc comment ("Idempotent: re-ingest is keyed
on (session, seq), so re-running only adds new calls") is **false** — the `DELETE`
makes the `INSERT OR IGNORE` unreachable as a dedupe path.

Every query is a fold over rows the same process just built in memory:

| SQL | replacement |
|---|---|
| `SELECT COUNT(*) FROM calls` | `Vec::len` |
| `COUNT(*), SUM(1-ok) … WHERE category=?` | one pass, `HashMap<&str,(u32,u32)>` |
| `COUNT(*) … WHERE detail=?` | `HashMap<String,u32>` lookup |
| `GROUP BY detail WHERE ok=0 ORDER BY 2 DESC LIMIT 8` | sort a small `Vec` |
| `GROUP BY detail WHERE category=? ORDER BY 2 DESC` | same |

So this is not a rewrite, it is deleting a round-trip through 1 MB of embedded C
for data that never leaves the process. **No capability is lost** — there was no
persistence to lose.

## Target 2 — `elkjs` in the dashboard bundle

`web/dist` is one 1 891 858 B JS chunk. `web/src/App.tsx:25` statically imports
`elkjs/lib/elk.bundled.js` — a GWT-compiled Java layout engine, 7.7 MB installed —
for exactly two `elk.layout()` calls (`App.tsx:786`, `:1724`).

A dynamic import would split the chunk but **save nothing**: `rust-embed` embeds
the whole `web/dist` tree either way. The only real saving is a smaller layout
engine. Measure first, swap only if the number justifies the regression risk.

## Decisions

- **D-M2-1** — replace `rusqlite` with in-memory aggregation; drop the
  `.cache/8sync/toolstats.db` artifact entirely. It was a cache of a
  from-scratch rescan, not a store.
- **D-M2-2** — with `rusqlite` gone the `toolstats` feature guards nothing, so
  **delete the flag** and un-cfg the module. A gate whose contents are free is
  cfg noise. `features` reduces to `web`.
- **D-M2-3** — docs that describe a "SQLite tracker" (`AGENTS.md` §5,
  `README.md`) become wrong the moment this lands, so they are corrected here,
  not deferred to M3.
- **D-M2-4** — the ELK swap ships only if measured *and* the codegraph view is
  verified in a real browser. A layout regression in the dashboard is a feature
  regression; PROJECT forbids trading features for bytes.

## ✅ Acceptance Criteria

| AC | Criterion | How verified |
|---|---|---|
| AC-01 | `rusqlite` is absent from the dependency tree | `cargo tree -i rusqlite` finds nothing; `Cargo.lock` has no `libsqlite3-sys` |
| AC-02 | `8sync harness toolstats` output is **byte-identical** to the SQLite version for the same sessions | capture before/after, `diff` |
| AC-03 | Default build shrinks by ≳ 1 000 000 B vs the M1 baseline of 6 407 848 B | `stat -c%s` |
| AC-04 | `toolstats` feature flag is gone; `--no-default-features` still builds, runs and is warning-clean | `cargo build` + smoke |
| AC-05 | `scripts/size-report.sh` reflects the new combination set and runs green | run it |
| AC-06 | ELK: measured decision recorded — swapped with browser proof, or rejected with the number that rejected it | `M2-VERIFICATION.md` |
| AC-07 | Docs no longer claim a SQLite tracker or a `.db` path | grep `AGENTS.md`, `README.md` |
| AC-08 | Dashboard still serves and `/api/*` still answers | `curl` |
