# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)
> **Active feature:** none open. `lean-binary` shipped · omp-17 ext fix shipped · STEP-0 enforcement shipped **and now actually working** (v1 was broken — see below).

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — 2026-08-06 (cold resume from here)

**Repo state:** branch `main`. `origin/main` = `50462db`. Local is **ahead by 3 commits, NOT pushed**: `91df6f7` (STEP-0 repair) · `325e7d6` (harness artifacts) · this slice's commit. Tag **v0.52.0** (Cargo.toml unchanged — WIP, not a release).

**What actually happened this session: STEP-0 v1 (`98ac9a4`) shipped BROKEN in both halves, and is now repaired + proven.**

- **`8sync ai` was dead on arrival.** `--tools` listed `python`/`notebook`, copied from omp's `--help` "Available Tools" section, which is **stale**. omp 17.2.9's validator rejects them and EXITS → every `8sync ai` / `8sync .` failed to launch. v1 claimed "verified: allowlist embedded in binary ✓" — that verified the *string was in the binary*, never that omp accepts it.
- **`--tools` is an ALLOWLIST and omp has no deny-list** (`tools.blocked` is a telemetry counter). v1 also omitted **17 real tools** including `recall`/`retain`/`reflect`/`memory_edit` — the whole mnemopi memory stack. Fixing only the two invalid names would have silently killed memory. Correct list = omp's validator list − `grep` − `glob` − `computer`.
- **bashInterceptor blocked nothing.** Rules were written `{pattern, reason}`; omp's shape is `{pattern, tool, message}` and its matcher does `if (!toolNames.includes(p.tool)) continue` — a rule with no `tool` is skipped unconditionally. Worse, the obvious repair is *also* wrong: omp's stock `grep|rg` rule carries `tool:"grep"`, which STEP-0 removes, so it disables itself exactly when needed. Every rule now points at `lsp` (always present).
- **Migration never fired** because omp rewrites `config.yml` in its own style, so byte-exact matching missed; it is now signature-based (`STEP-0`).

**Done ✓ (each with live evidence, not claims)**
- [x] STEP-0 repaired: provider request carries 18 tools — `grep`/`glob` **absent**, `recall`/`retain`/`reflect`/`memory_edit`/`hub`/`eval`/`ast_edit`/`lsp` **present**. `8sync ai` returns normally.
- [x] Interceptor blocks for real: `rg main main.rs` → `Blocked: STEP-0: …`; `grep -r main .` → blocked; plain `grep main main.rs` **still runs** (no over-block).
- [x] Acceptance: asked "where is X defined and who calls it" → agent used `codegraph query` then `mcp__serena_find_referencing_symbols`, `read` only to confirm a line. This is the behaviour change prose failed 3× to produce.
- [x] **Drift guard** (`8sync doctor`): probes omp's live validator list and diffs `STEP0_TOOLS`. Proven by injecting drift (`bogus_tool` → `REJECTED by omp … will fail to launch`), then reverting (`✓ matches`). Free + offline.
- [x] `--continue` history loss **not reproducible** — `BANANA47` survives both `omp --continue` and the `8sync ai` resume path. Symptom was almost certainly v1's launch failure making users fall back to a bare `omp`.
- [x] su-code indexed into cbm (5,843 nodes / 16,775 edges) — STEP-0 told the agent to use cbm while this repo was not in it.
- [x] Caller queries routed to serena in `APPEND_SYSTEM.md` (see blocker below).

**Next / TODO ▸**
- [ ] **Push the 3 local commits** — `/auto` guardrail forbids push, so this waits for `/push-now` or an explicit go.
- [ ] **Tag a release.** Until then `8sync up` reinstalls v0.52.0 and reverts every fix here (see blocker).
- [ ] **Measure the payoff:** after a few NEW sessions under enforcement, `8sync harness toolstats` should show `grep`→0 and `cbm`/`serena`>0. Baseline was 66.7 % optimizer (codegraph 6, cbm/serena 0, grep 3). Cannot be measured in the session that built the enforcement.
- [ ] Ratchet the size ceiling as headroom appears (`scripts/size-report.sh` → `scripts/size-gate.sh`).
- [ ] REQUIREMENTS v2: un-embed `impeccable/scripts` (1.6 MB) behind a lazy fetch — last big chunk of the 665 392 B over goal.
- [ ] Real mac/Windows **runtime** verify needs the actual OSes.

**Blockers ⚠ (per-machine — NOT in git)**
- **`8sync up` reverts everything** until a new tag is cut: it installs the latest *release* (v0.52.0), which lacks the ext fix, the STEP-0 repair, and the drift guard. Fixed binary = local `cargo build --release` → `install -m755 target/release/8sync ~/.local/bin/8sync`.
- **bashInterceptor is per-machine** (`~/.omp/agent/config.yml`): each machine needs `8sync harness global`. omp's own `omp update` rewrites config.yml and can drop it — re-run.
- **`codegraph callers` returns FALSE NEGATIVES** (reproduced on a clean full index: 2 real callers reported as none; misses cluster by *caller* — nothing inside `global_pass` resolves). codegraph is a prebuilt external binary with no local source, so this is upstream. Use `mcp__serena_find_referencing_symbols` for "who calls X". **Never `rm -rf .codegraph`** — that deletes the exclusion config and the next index walks 16k files; use `codegraph index --force` or `8sync harness`.

**New-machine runbook (ordered):**
1. `git pull`
2. `bash scripts/bootstrap.sh` → builds from source (gets every fix above) + installs `~/.local/bin/8sync`. **Do NOT** run `8sync up` afterwards.
3. `8sync setup` (omp + codegraph + MCP/skills + gh + PATH)
4. `8sync harness` → redeploys the fixed extension + bashInterceptor + skills + memory + inject + codegraph index
5. `8sync doctor` — must show `✓ STEP-0 allowlist matches omp's tool list`
- Decisions + lessons: `su-code/KNOWLEDGE.md` (+ `su-code/archive/`).

## ✅ SHIPPED — `lean-binary` feature (2026-08-02)
1. **M0** — landed 5 pending deliverables (`8sync omp update` verb · `branch-sync` skill + `/sync-pr` · `harness global` auto-stamp · `deep-research` §5 + binary brief).
2. **M1** — first `[features]` table; A/B'd every gate with `scripts/size-report.sh`. `cargo bloat` under-attributed SQLite **~26×**.
3. **M2** — deleted what the data pointed at: `rusqlite` (**−1 035 384 B**; the DB stored nothing — ingest opened with `DELETE FROM calls`) and `elkjs` → dagre (**−512 768 B**). Output byte-identical under frozen input. **Bug fixed mid-flight (`b331832`):** a directory merely *named* `su-code` made its parent look like a project → blank memory + 74 skills in the repo root. `discover::MEMORY_DIR` = `"su-code"`, **not** `brand::NS` (= `"8sync"`).
4. **M3** — `cross` → `cargo-zigbuild` for aarch64 (the Docker leg had no JS toolchain → embedded the **stub dashboard**), plus `scripts/size-gate.sh` (5 MiB ceiling / 4 MiB goal). `universal2` rejected.

**Size table:** x86_64 default **4 859 696** (+15.87 % vs goal) · aarch64-musl **4 151 328** (−1.02 %) · `--no-default-features` **3 109 496** (−25.86 %).

**Prior shipped:** omp-17 MCP fix + Lark (`589807e`) · STEP-0 MCP fix omp-16 (`64bd650`) · `/push-now`+`/pull-now` (`c402209`, `6bb38ae`) · v0.52.0 (`8sync vpn`).

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 bật bằng `/auto` + `8sync harness up --timer`.
- **prompt-optimizer (linshenkx) evaluated and NOT integrated** — it optimizes human-authored prose, which is the layer that already failed 3× here; it is AGPL-3.0 vs su-code MIT (no vendoring), and a 5th MCP server adds catalog pressure against omp's ~40-tool discovery cutoff. Use the hosted web app for authoring skill/system-prompt text if wanted.
- **eino (CloudWeGo) evaluated and NOT adopted** — omp is already the agent runtime; replacing it forfeits MCP xd:// devices, skills, sessions, extensions. eino would only make sense for a future Go sidecar exposing a deterministic pipeline as an MCP tool, a niche codegraph + cbm already fill.
- Division of labour that the evidence supports: **LLM does judgement, Rust does enforcement** (allowlist, interceptor, drift guard) — deterministic levers the model cannot talk its way around.
