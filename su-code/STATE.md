# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**v0.49.0 — `8sync harness add-model` (remote custom models)**, rebased onto this repo's v0.48.0 bundle. Register a model omp's fetched catalog lacks — or lists with null metadata (e.g. new `xai-oauth/grok-4.5`: `context -`, `max-out -`) — as a full custom provider in `~/.omp/agent/models.yml`, so it shows in `/model` + routes. `Cargo.toml` = **v0.49.0**.
- **New module** `crates/cli/src/verbs/harness/custom_model.rs` (cloud sibling of `add-local-model`, no mistral.rs/systemd). `harness add-model <provider/model> --url <baseUrl> [--key|--api|--ctx|--max|--vision|--think]` · `list` · `rm`. `add-model` was an undocumented alias of `add-local-model` → repurposed; GGUF stays on `add-local-model`. Adopts the new `brand::NS` path namespace + `brand::render` help convention from v0.48.0.
- **Grounded on omp 16.3.12:** metadata-only merge is REJECTED (`"baseUrl" is required when defining custom models`) → `--url` mandatory; selector = `<providerKey>/<modelId>`. TSV registry `~/.config/<NS>/custom-models.tsv`; sentinel block **coexists** with local-models + 9router gateway (strip-only-own-block; `gateway apply` re-attaches both). Post-write `omp models --json` re-validation warns on a bad `--think`/`--api` combo.
- Verified live: add fills grok-4.5 ctx/max (was null) · grouping (2 models/provider) · `--vision`/`--api anthropic`/`--think` valid · `rm` keeps siblings · 3-block coexistence (gateway+local+custom) all load. Config restored to pristine `providers: {}` after test.
- **Rebased under v0.49.0**: v0.48.0 bundle (`/feature` GSD framework + `8sync feature` verb · single-source CLI name `brand.rs` [`CMD`/`NS`, default `8sync` byte-identical] · dashboard Knowledge/Create-Project · `harness model <strong>+<cheap>` combo) and v0.47.0 cross-platform (macOS/Windows `platform.rs`, release.yml matrix CI, `install.ps1`).

## Next (chưa làm)
- [ ] **Push tag v0.49.0** → CI release matrix produces the 5 assets. (v0.47.0/v0.48.0 already shipped; a real *runtime* smoke on a mac + Windows box is still the only unverified venue — can't be done from this Linux host.)
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
