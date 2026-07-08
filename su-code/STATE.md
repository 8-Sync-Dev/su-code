# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**v0.48.0 (this machine) — `/feature` GSD framework + single-source CLI name (`brand.rs`) + dashboard Knowledge/Create-Project + `harness model` combo, REBASED onto the other machine's v0.47.0 cross-platform release. Integrated + built; pushing + releasing v0.48.0.**
  - **This machine's features (now v0.48.0)**: `feature` GSD framework (bundled skill + `/feature` command + `8sync feature new/switch/status/list` verb; planning tree `su-code/planning/<slug>/`; `ACTIVE` cross-feature switch; `/feature go` → `engine_*` verify-gate loop) · single-source CLI name `brand.rs` (`CMD`+`NS`, default `8sync` byte-identical, `SC_CMD`/`SC_NS` rebrands everything + migration shim) · dashboard Knowledge browser + Create-Project · `harness model <strong>+<cheap>` combo.
  - **Other machine's v0.47.0 (rebased under mine)**: cross-platform macOS/Windows — `platform.rs` OS seam, `require_linux` guards, cross-platform timer, `.github/workflows/release.yml` matrix CI, `install.ps1`, dropped `target-cpu=native`.
  - **Merge resolution**: `up.rs`/`setup.rs` took the cross-platform `platform::*` version (my systemd-specific `ns_file` timer edit was superseded); `main.rs` auto-merged (`mod platform` + `mod brand` + feature verb); CHANGELOG = `[0.48.0]` (mine) over `[0.47.0]` (cross-platform); KNOWLEDGE keeps both sessions' learnings.
  - `Cargo.toml` = **0.48.0**. Pushing `main` + tagging `v0.48.0` → `release.yml` matrix CI builds the multi-platform GitHub Release.

## Next (chưa làm)
- [ ] **Push tag v0.47.0** → CI produces the 5 assets → first mac/Windows prebuilts land on Releases. Then a real *runtime* smoke on a mac + a Windows box (only venue left; can't be done from this Linux host).
- [ ] Phase 3b — gstack host `omp` (DEFERRED; xem archive + `reference/gstack` docs/ADDING_A_HOST.md).
- [ ] (tùy) `8sync harness eval --baseline` định kỳ · loại `reference/` khỏi codegraph (deinit).

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
