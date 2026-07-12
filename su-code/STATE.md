# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — sang máy khác làm tiếp (2026-07-09)
**Repo state:** `main`, latest tag **v0.51.0** (CI publishes 5 assets/tag). Cây làm việc SẠCH sau mỗi ship. Không còn gì để commit về code.

**Trên máy mới — runbook (theo thứ tự):**
1. `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code` (hoặc `git pull` nếu đã có).
2. `bash scripts/bootstrap.sh` (build từ source) **hoặc** cài binary prebuilt: `curl -fsSL https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/install.sh | sh` (v0.50.0 đã có asset).
3. `8sync setup` → cài AI core (omp + codegraph + MCP/skills + gh). Cấu hình omp API key.
4. `8sync harness init` → deploy skills + AGENTS.md + codegraph index + gitleaks hook.
5. **Config KHÔNG theo repo (phải làm lại per-máy, nằm trong `~`, không phải trong git):**
   - `8sync harness browser` → ghim omp browser vào system Chromium (cài `ungoogled-chromium-bin` + export env vào rc). Rồi **mở shell mới**. Đây là fix #2 của 0.50.0 — code đi theo repo nhưng việc *áp dụng* lên máy thì phải chạy lệnh này lại.
   - Nếu cần custom model: `8sync harness add-model <provider/model> --url <baseUrl> [--key|--think ...]` (models.yml live local, không commit).
   - `8sync feynman auth-omp` → nếu dùng Feynman (Pi research agent): sau khi omp đã auth (Claude OAuth/keys), lệnh này bắc cầu creds omp → `~/.feynman/agent/auth.json` (per-máy, không theo repo). `feynman model list` sẽ hiện cùng model omp.

**Đã xong (0.50.0, không cần làm lại):** code cả 2 fix (`/new` `--cwd` root pin + `harness browser`), CHANGELOG, KNOWLEDGE (2 learnings), README row, help/examples. Tag + CI + publish xong.

**Việc còn lại / cần quyết:**
- [ ] **grok-4.5 loose end** (chỉ trên MÁY NÀY, trong `~/.omp/agent/models.yml`): entry `xai/grok-4.5` đang có **placeholder key**. Hoặc `export XAI_API_KEY=... && 8sync harness add-model xai/grok-4.5 --url https://api.x.ai/v1 --ctx 500000 --think` (dùng API key thật), hoặc `8sync harness add-model rm xai/grok-4.5` (bạn vốn dùng grok qua OAuth `xai-oauth`). KHÔNG theo repo — chỉ ảnh hưởng máy này.
- [ ] (tùy) Máy mới: `8sync harness browser status` để confirm wiring sau khi mở shell mới.

## Current step
**v0.51.0 — `8sync feynman auth-omp` (bridge Feynman to omp's auth)**. `Cargo.toml` = **v0.51.0**.
- **New top-level verb** `feynman [auth-omp|status|off]` (`crates/cli/src/verbs/feynman.rs`). Feynman (companion-inc/feynman) + omp are both Pi (earendil-works/pi) → same `<home>/agent/auth.json` schema. `auth-omp` mirrors omp's live creds into `~/.feynman/agent/auth.json`: oauth (Claude Pro/Max) = `{type:oauth, access:<omp token>}` **no refresh** (omp sole refresher, no dueling rotation; re-run on expiry); api_key = `{type:api_key, key:"!omp token <p> --raw"}` (live, no secret copied). Sidecar `.8sync-omp.json` → `off` removes only ours. Reads omp SQLite `auth_credentials` for provider+type (via `sqlite3`, never the `data` blob); tokens via `omp token <p> --raw`.
- **Verified live** (feynman 0.3.5 + omp 16.4.6): `auth-omp` → `feynman model list` shows `anthropic/claude-opus-4-8` + `zai/*` (31 authed) reusing omp Claude OAuth; `status` lists managed; `off` → 0 authed, auth.json `{}`, sidecar gone; re-apply idempotent. (feynman `chat` needs `feynman setup` for Pi npm runtime — auth resolution verified independently.)
- **Prior shipped**: v0.50.0 (omp `/new` root fix + `harness browser`) · v0.49.1 (`add-model --think`) · v0.49.0 (`harness add-model`) · v0.48.0 (`/feature` GSD + `brand.rs` + dashboard) · v0.47.0 cross-platform.

## Next (chưa làm)
- [ ] **Push tag v0.51.0** → CI release matrix produces the 5 assets.
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
