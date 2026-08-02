# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)
> **Active feature:** `su-code/planning/lean-binary/STATE.md` — `8sync feature status`

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## ✅ SHIPPED — `lean-binary` feature complete (2026-08-02)
**Repo state:** branch `main`, base `589807e` → HEAD `60561f0`, **17 commits, tracked tree clean, nothing pushed**. `Cargo.toml` still 0.52.0 (WIP checkpoint, not a release).

**Result — binary 24 % smaller, zero features removed:**

| build | before | after |
|---|---:|---:|
| x86_64 default | 6 407 848 | **4 859 696** (−24.2 %) |
| aarch64-musl | shipped a STUB dashboard | **4 151 328** (under the 4 MiB goal) |
| `--no-default-features` | did not exist | **3 109 496** |

**How, in order (details: `su-code/planning/lean-binary/M*-VERIFICATION.md`):**
1. **M0** — landed 5 pending deliverables (`8sync omp update` verb · `branch-sync` skill + `/sync-pr` · `harness global` auto-stamp · `deep-research` §5 native-audit protocol + the binary brief).
2. **M1** — first-ever `[features]` table; A/B'd every gate with `scripts/size-report.sh`. Gating shipped **0** user bytes by design — its job was to locate the fat. `cargo bloat` under-attributed SQLite **~26×**.
3. **M2** — deleted what the data pointed at: `rusqlite` (**−1 035 384 B**; the DB stored nothing — ingest opened with `DELETE FROM calls`) and `elkjs` → dagre (**−512 768 B**; 85 % of the FE chunk was a GWT-compiled Java layout engine). Output byte-identical under frozen input; dashboard verified headless.
4. **M3** — `cross` → `cargo-zigbuild` for aarch64 (the Docker leg had no JS toolchain, so it embedded the **stub dashboard**), plus `scripts/size-gate.sh` enforcing a 5 MiB ceiling / 4 MiB goal per asset. `universal2` rejected — it would double every Mac download.

**Bug found and fixed mid-flight (`b331832`):** a directory merely *named* `su-code` made its parent look like an omp project. Since this checkout is `~/Projects/tools/su-code`, an auto-stamp from `~/Projects/tools` wrote a blank memory tree **plus 74 skills into the repo root**. Now `discover::is_omp_project` requires real memory content. Note `discover::MEMORY_DIR` = `"su-code"`, **not** `brand::NS` (= `"8sync"`).

## Next
- [ ] Ratchet the size ceiling down as headroom appears (`bash scripts/size-report.sh` → `scripts/size-gate.sh`).
- [ ] REQUIREMENTS v2: un-embed `impeccable/scripts` (1.6 MB) behind a lazy fetch — the last big chunk of the 665 392 B still over goal.
- [ ] Push / release when wanted: `/push-now` or bump `Cargo.toml` + tag.
- [ ] (tùy) Hardening: `8sync feynman auth-omp`/`doctor` detect `npm` hỏng và warn (feynman phụ thuộc npm runtime). Chưa làm — ngoài phạm vi yêu cầu.
- [ ] Phase 3b — gstack host `omp` (DEFERRED; xem archive + `reference/gstack` docs/ADDING_A_HOST.md).

**Prior shipped:** omp-17 MCP fix + Lark (589807e) · STEP-0 MCP fix omp-16 (64bd650) · `/push-now`+`/pull-now` (c402209, 6bb38ae) · v0.52.0 (`8sync vpn`).

## Open questions / blockers
- Real mac/Windows **runtime** verification needs the actual OSes (or the pushed-tag CI artifacts) — the code path (launchd/schtasks/brew/winget) is written + compiles cross-platform but hasn't executed on a live mac/Win yet.

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 bật bằng `/auto` + `8sync harness up --timer`.
- Reference submodules để deinit mặc định (token-lean hơn luôn-có-sẵn).
- Spine advisory threshold = spine >50% upfront (relative, không absolute floor).
- **Knowledge feature (this session):** source = `curl` raw `sindresorhus/awesome` README (`raw.githubusercontent.com/.../main/readme.md`; lighter than git-clone, it's one README), cached `.cache/8sync/knowledge/` 6h TTL. Parse `##`/`###` headings → `- [name](url) - desc` entries (skip TOC `#` anchors). Apply target = `<proj>/su-code/REFERENCES.md` (new curated-links file; KNOWLEDGE.md stays append-only learnings). Reuse `marketplace.rs` curl+cache pattern.
- **Create-project feature (this session):** `POST /api/projects/create` {name|path, skills[], mcp[], knowledge[]} → mkdir (default parent `~/Projects/<name>`, refuse if exists = reversible) + `git init` + full 8sync stamp (AGENTS.md + su-code memory + skills mirror + inject) + `8sync skill add` per extra skill + selected MCP → `<proj>/.omp/mcp.json` (project-scoped) + knowledge → REFERENCES.md + activate. Reuse `here::seed_project_context` + `skill_cmd`.

## Handoff (đổi máy — làm theo thứ tự)
1. `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code`
2. `bash scripts/bootstrap.sh` (hoặc `8sync up`) → build + cài `8sync`
3. `8sync harness` — auto-setup (MCP + skills + memory + inject + index)
4. `gh auth login` (cho `8sync ship` / release)
5. Mở omp → `/auto <mục tiêu>` để chạy engine tự động.
- Lịch sử quyết định + bài học: `su-code/KNOWLEDGE.md` (+ `su-code/archive/`).
