# M2 — Verification

Phase: M2 Eliminate, don't just gate · 2026-08-02

## Result

| build | M1 | M2 | delta |
|---|---:|---:|---:|
| default (`web`) | 6 407 848 | **4 859 696** | **−1 548 152 B (−24.2 %)** |
| minimal (`--no-default-features`) | 3 081 416 | 3 109 496 | +28 080 B |
| vs 4 MiB budget | +2 213 544 | **+665 392** | closed 70 % of the overshoot |

`scripts/size-report.sh` (explicit `--target`): full **4 859 632** (+15.86 % vs
budget) · minimal **3 109 496** (−25.86 %) · `web` gate **1 750 136 B**.

The minimal build grew 28 080 B because `toolstats` is no longer gated out of it
— it is now dependency-free and always present. That is the intended trade: a
lean build gains a feature and still sits 25.86 % under budget.

## Target 1 — `rusqlite` deleted (−1 035 384 B)

Replaced with an in-memory fold. The five queries became one pass building four
`HashMap`s. `SELECT … ORDER BY 2 DESC` tie order was reproduced by tie-breaking
on first appearance (`ranked()` in `toolstats.rs`) — SQLite left ties in
table-scan order, which is why the old output printed `write×2, generate_image×2`
rather than alphabetically.

**AC-02 proven under controlled input**, not by eyeballing two live runs: the M1
binary was rebuilt in a detached `git worktree`, the session JSONL frozen into
`/tmp/th/.omp/agent/sessions/…`, and both binaries run against it with `HOME`
pointed at the frozen tree. `diff` output:

```
-✓ tracked 2611 call(s) from 6 session(s) → …/.cache/8sync/toolstats.db
+✓ tracked 2611 call(s) from 6 session(s) ← /tmp/th/.omp/agent/sessions/…
```

That single line is the intended provenance change. **Every other line — counts,
percentages, per-detail rows, and the tie-broken failing-calls ordering — is
byte-identical.**

## Target 2 — `elkjs` → `@dagrejs/dagre` (−512 768 B)

`elk.bundled.js` was 1 606 238 B of the 1 891 858 B dashboard bundle: **85 % of
the frontend was a GWT-compiled Java layout engine** serving two `layered`
calls. A dynamic import was measured first and rejected — `rust-embed` embeds
the whole `web/dist` tree, so splitting the chunk saves zero binary bytes.

New `web/src/layout.ts` exposes `layered(nodes, edges, dir, nodeSep)`. dagre
reports node **centres** where elk reported top-left, so each result is shifted
by half the node box; edges with an endpoint outside the node set are dropped,
because dagre would otherwise invent a phantom node that elk silently ignored.

Bundle: **1 891 858 → 478 704 B (−75 %)**.

### Browser proof (D-M2-4 — required before this could ship)

Dashboard served on `:8794`, driven headless:

| check | result |
|---|---|
| Codegraph package graph | 15 nodes, 10 edges; x ∈ {0, 210, 420, 630} — four clean LR ranks; 7 distinct y; nothing collapsed to origin |
| Workflow auto-layout, connected 3-node chain | y = 0 / 134 / 268, single x column — correct top-down layering |
| Workflow auto-layout, 8 disconnected nodes | one rank, spread on x — correct for an edgeless graph |
| `pageerror` listener | **0 errors** across every interaction |
| Screenshot | layered graph + Leiden cluster panel render correctly |

## AC matrix

| AC | Criterion | Result |
|---|---|---|
| AC-01 | `rusqlite` gone from the tree | **PASS** — `cargo tree -i rusqlite` → "did not match any packages"; 0 `libsqlite3-sys` in `Cargo.lock` |
| AC-02 | `toolstats` output byte-identical | **PASS** — frozen-input diff, provenance line only |
| AC-03 | Default build ≳ 1 000 000 B smaller | **PASS** — −1 548 152 B (SQLite −1 035 384, layout engine −512 768) |
| AC-04 | `toolstats` flag gone; lean build green | **PASS** — `features` is just `web`; lean compiles warning-clean, runs, and now *has* `toolstats` |
| AC-05 | `size-report.sh` updated and green | **PASS** — two combinations, runs clean |
| AC-06 | ELK decision measured + recorded | **PASS** — swapped, with the dynamic-import alternative measured and rejected, plus browser proof |
| AC-07 | Docs corrected | **PASS** — `AGENTS.md:132,272,365`, `README.md:160,231,281` |
| AC-08 | Dashboard + APIs still work | **PASS** — `/` `200`, `/api/bench` `200`, `/api/marketplace` `200`, plus the interactive checks above |

## Honest position on the budget

The full build is **4 859 696 B against a 4 194 304 B budget — still 665 392 B
over**. What remains is not fat with an easy owner:

- `web` gate 1 750 136 B — axum/tokio/scraper plus a 478 KB FE. Hand-rolling
  HTTP to save ~400 KB is a footgun, not an optimisation.
- The rest is `assets/` (3.0 MB raw, `impeccable` 2.1 MB of it), which is the
  product: bundled always-on skills.

So M3 chooses between trimming `impeccable`'s 1.6 MB of `scripts/` (needs a
network fallback — deferred to v2 in REQUIREMENTS) and amending §8 to a number
this measurement can defend. **The budget line has already been updated to state
the measured reality rather than an aspiration** (`AGENTS.md:365`).

## Defect found during M2 (in M0's own work) — fixed here

`git add -A` for the closing docs commit was blocked by the gitleaks pre-commit
hook, on regex literals inside the `senior-security` skill. Those files should
never have been in the repo: a blank memory tree (`STATE.md`, `KNOWLEDGE.md`,
`PLAYBOOKS.md`, …) plus a **74-entry `skills/` tree had been stamped into the
repo root**.

Cause: `discover::detect_current_project_root` and `global::is_omp_project`
accepted any directory *named* `su-code` as proof of an omp project. This
checkout is `~/Projects/tools/su-code`, so an auto-stamp (added by M0's
`3c8c008`) run with cwd `~/Projects/tools` resolved the project root to
`tools/`, and wrote its memory layer into `tools/su-code/` — this repo's root.
Reproduced deterministically with a bare `/tmp/parent/su-code` directory.

Fixed in `b331832`: `is_memory_dir()` requires real memory content, and
`is_omp_project` moved into `discover` so both paths share one rule.

A first attempt used `brand::NS` for the directory name. It compiled and still
passed the negative test — but `NS` is `"8sync"` (the config/artifact
namespace), not the memory dir, so detection silently broke for *every real
project*. Caught because the positive cases were tested too. Now
`discover::MEMORY_DIR`.

| scenario | expected | result |
|---|---|---|
| bare `su-code/` dir | untouched | **PASS** |
| repo with `AGENTS.md` | stamped | **PASS** |
| memory tree, no `AGENTS.md` | stamped | **PASS** |
| this repo | stamps `su-code/`, root clean | **PASS** |

## Verdict

**M2 PASS** — 8/8 AC. Proceed to M3.
