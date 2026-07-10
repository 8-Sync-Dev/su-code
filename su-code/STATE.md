# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**v0.50.0 — omp `/new` root fix + `8sync harness browser` (browser reaches internet)**. `Cargo.toml` = **v0.50.0**.
- **`/new` wrong-root fix**: omp's `/new` = `newSession({parentSession})` — inherits the LAUNCH root, does NOT re-detect cwd. So a drifting cwd made `/new` land in the wrong project. `8sync .` + `8sync ai` now pin omp's `--cwd <detected-root>` (+ `current_dir`); `ai.rs` previously launched omp in ambient cwd unpinned. (`crate::verbs::here::detect_project_root` reused.)
- **`harness browser [fix|status|off]`** (`crates/cli/src/verbs/harness/browser.rs`): omp Puppeteer browser rendered but couldn't reach the internet on bundled `chrome-headless-shell`. Ensures `ungoogled-chromium-bin` (`/usr/bin/chromium`), exports `PUPPETEER_EXECUTABLE_PATH`+`BUN_CHROME_PATH` (omp/Bun honor, +`--no-sandbox`) in zsh/bash/fish (sentinel rc block, idempotent). Verified: chromium fetches headless; interactive bash+zsh resolve the path; `off` reverts.
- **Prior shipped**: v0.49.1 (`add-model --think` full reasoning range + mode-by-api) · v0.49.0 (`harness add-model` remote custom models) · v0.48.0 bundle (`/feature` GSD + `brand.rs` + dashboard + `harness model` combo) · v0.47.0 cross-platform (mac/Win + release CI).

## Next (chưa làm)
- [ ] **Push tag v0.50.0** → CI release matrix produces the 5 assets. (Real mac/Win *runtime* smoke still unverified — can't from this Linux host.)
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
