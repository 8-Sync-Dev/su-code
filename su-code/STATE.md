# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — sang máy khác làm tiếp GẤP (2026-07-13)
**Repo state:** branch `main`. HEAD trước session = `52e0b25` (v0.52.0, đã tag + CI publish). Session này **THÊM 1 commit** (chưa tag — WIP checkpoint): omp command `/push-now` + fix môi trường feynman. Sau khi push, cây SẠCH.

**Đã làm session này (2026-07-13):**
1. **Fix feynman không mở được** (env, KHÔNG phải repo code): `8sync feynman auth-omp` chạy đúng (bridge OK — `feynman model list`/`doctor` thấy anthropic+zai). Crash thật khi `feynman chat`: feynman gọi `npm install @companion-ai/alpha-hub …` lúc khởi động, mà `npm` trên PATH hỏng. Root cause: `~/.local/bin/{npm,npx}` là **symlink** → pnpm shim `~/.local/share/pnpm/{npm,npx}`; shim tính `basedir=$(dirname "$0")` = `~/.local/bin` → tìm `~/.local/bin/global/5/.pnpm/npm@…/npm-cli.js` (không có; thật ra ở `~/.local/share/pnpm/global/…`) → `MODULE_NOT_FOUND`. **Fix**: thay 2 symlink bằng wrapper `#!/bin/sh` + `exec /home/<u>/.local/share/pnpm/{npm,npx} "$@"`. → feynman mở sạch, alpha-hub cài xong. Chi tiết: `su-code/KNOWLEDGE.md` (validated, cuối file).
2. **Thêm omp command `/push-now`** (repo code, đi theo git): asset `assets/commands/push-now.md` + wire vào `ensure_engine` (`crates/cli/src/verbs/skill/deploy.rs`, cạnh `/auto` `/feature`). `8sync harness` deploy → `~/.omp/agent/commands/push-now.md` (global) + `.omp/commands/push-now.md` (project). Đã build release clean + harness deploy live trên máy này. `/push-now [msg]` = viết handoff vào STATE + commit + push (no PR/tag/force). CHANGELOG updated.

**⚠ MÁY KHÁC RẤT CÓ THỂ DÍNH CÙNG LỖI npm** (nếu cũng dùng pnpm global): nếu `feynman chat` crash `MODULE_NOT_FOUND …/npm-cli.js`, hoặc `npm --version` lỗi → chạy đúng fix #1 ở trên (thay symlink `~/.local/bin/{npm,npx}` bằng wrapper trỏ đường thật của pnpm). Kiểm tra nhanh: `npm --version` phải in số (12.0.1), không phải stacktrace.

**Trên máy mới — runbook (theo thứ tự):**
1. `git pull` (hoặc clone `https://github.com/8-Sync-Dev/su-code.git`).
2. `bash scripts/bootstrap.sh` (build+install) **hoặc** `curl -fsSL https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/install.sh | sh`.
3. `8sync setup` (omp + codegraph + MCP/skills + gh) → cấu hình omp API key.
4. `8sync harness` → deploy skills + AGENTS.md + codegraph index + **`/push-now` `/auto` `/feature` commands** + gitleaks hook.
5. **Config per-máy (KHÔNG theo git, nằm trong `~`):**
   - `npm` fix ở trên (nếu feynman crash).
   - `8sync feynman auth-omp` → nếu dùng Feynman (sau khi omp auth): bắc cầu creds → `~/.feynman/agent/auth.json`.
   - `8sync harness browser` → ghim omp browser vào system Chromium, rồi mở shell mới.
   - custom model: `8sync harness add-model …` (models.yml live local).
   - `8sync vpn install` + `8sync vpn on [CC]` → nếu cần tunnel VPN Gate.

## Current step
**omp handoff-pair commands `/push-now` + `/pull-now` (WIP checkpoint trên v0.52.0)**. `Cargo.toml` vẫn = **v0.52.0** (chưa bump — checkpoint, không phải release).
- Assets `assets/commands/{push-now,pull-now}.md` + wire `ensure_engine` (`crates/cli/src/verbs/skill/deploy.rs`) → deploy cạnh `/auto` `/feature`. Build release clean + harness deploy live OK (global + project).
- `/push-now [msg]` = rời máy: rewrite HANDOFF cold-resume → CHANGELOG/KNOWLEDGE → `git add -A` + commit (gitleaks-gate) + push. **`/pull-now [go]`** = tới máy: `git pull` (ff-only/rebase, stop nếu bẩn/conflict) → đọc STATE HANDOFF + KNOWLEDGE + CHANGELOG/log → prepare (rebuild+harness nếu `crates/`/`assets/` đổi, verify per-machine gotchas, `8sync doctor`) → report state + next action; `go` = làm luôn, rỗng = dừng chờ human. Cả hai: NO PR/tag/branch-switch/force.
- **Prior shipped**: v0.52.0 (`8sync vpn`) · v0.51.0 (`feynman auth-omp`) · v0.50.0 (omp `/new` fix + `harness browser`) · v0.49.x (`add-model`) · v0.48.0 (`/feature` GSD) · v0.47.0 cross-platform.

## Next (chưa làm)
- [ ] (tùy) Nếu muốn `/push-now` thành release: bump `Cargo.toml` + CHANGELOG version + push tag → CI 5 assets.
- [ ] (tùy) Hardening: `8sync feynman auth-omp`/`doctor` detect `npm` hỏng và warn (feynman phụ thuộc npm runtime). Chưa làm — ngoài phạm vi yêu cầu.
- [ ] Phase 3b — gstack host `omp` (DEFERRED; xem archive + `reference/gstack` docs/ADDING_A_HOST.md).

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
