# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — sang máy khác làm tiếp GẤP (2026-07-15)
**Repo state:** branch `main`, tag mới nhất `v0.52.0` (Cargo.toml vẫn 0.52.0 — commit này là WIP checkpoint, KHÔNG release). HEAD trước session = `c402209` (`/pull-now` command). Session này thêm 1 commit (STEP-0 MCP activation fix) → sau push cây SẠCH.

**Đã làm session này (2026-07-15) — fix "MCP connected nhưng không bao giờ được gọi":**
1. **Root cause (đo từ 29 sessions / 13.854 tool calls: serena 0 · headroom 0 · cbm 10 · zai 3):** (a) omp `tools.discoveryMode: auto` ẨN toàn bộ MCP tools sau `search_tool_bm25` khi registry >40 tools (stack này = 48); (b) mọi instruction surface dạy tên BASE (`search_graph`, `find_symbol`) — không gọi được; tên đăng ký thật = `mcp__codebase_memory_mcp_search_graph`, `mcp__serena_find_symbol`, … (ngoại lệ `mcp__headroom_compress`).
2. **`crates/cli/src/verbs/skill/deploy.rs`** — `ensure_mcp_tools_visible` (thay `ensure_tools_essential_default` trong plan): ghi `mcp.discoveryDefaultServers: [codebase-memory-mcp, headroom, serena, zai-vision]` vào `~/.omp/agent/config.yml`. **QUAN TRỌNG:** `tools.essentialOverride` KHÔNG dùng được cho MCP — omp 16.4.8 lọc entries chỉ nhận BUILT-IN names (đã extract logic từ binary); block pin cũ (inert) được auto-migrate xóa (byte-exact). Idempotent, không đè key user. + header catalog `## Registered MCP servers` dạy tên `mcp__…`.
3. **Instruction surfaces đồng bộ tên đăng ký:** `assets/configs/omp/APPEND_SYSTEM.md` (RULE #0 viết lại; headroom mandate đổi thành "nén những gì BẠN phát lại"), `assets/skills/00-force-load.md`, `crates/cli/src/verbs/skill/inject.rs` (AGENTS sentinel template), `assets/skills/feature/*` + mirror `su-code/skills/feature/*` (R10 literals), `AGENTS.md`/`CLAUDE.md` (harness regenerate). Xóa tên rác: `semantic_query` (cbm không có), `codegraph search/deps/defs` (verbs thật 1.1.2 = `query/explore/node/callers/callees/impact`).
4. **`crates/cli/src/verbs/doctor.rs`** — warn khi MCP tools bị ẩn (`discoveryDefaultServers` missing) + check serena registered/runnable (mcp.json + uvx). `toolstats.rs` matcher bỏ tên không tồn tại.
5. **Verified live:** `omp -p` gọi thẳng `mcp__codebase_memory_mcp_search_graph` + `mcp__serena_find_symbol` → OK (trước fix: MISSING); toolstats lần đầu ghi nhận serena/cbm optimizer calls; harness global re-run idempotent (skip); doctor ✓.

**Next ▸ (cụ thể):**
- [ ] Máy mới: chạy `8sync harness` (bắt buộc — để ghi `mcp.discoveryDefaultServers` vào `~/.omp/agent/config.yml` máy đó; fix là per-machine config + code).
- [ ] Theo dõi adoption: `8sync harness toolstats` sau vài session — kỳ vọng optimizer % tăng từ 24%.
- [ ] (tùy, ngoài scope) Friction còn lại: serena cần `activate_project` mỗi session; cbm cần đúng project slug (`list_projects` trước). Nếu muốn 0-friction: cân nhắc auto-activate trong recall hook (`~/.omp/hooks/pre/8sync-recall.ts`).

**⚠ Per-machine gotchas (KHÔNG theo git):**
- `~/.omp/agent/config.yml` là per-machine — fix MCP visibility chỉ có hiệu lực sau khi chạy `8sync harness` trên máy đó.
- npm/pnpm shim hỏng → feynman crash `MODULE_NOT_FOUND …/npm-cli.js`: thay symlink `~/.local/bin/{npm,npx}` bằng wrapper `exec ~/.local/share/pnpm/{npm,npx} "$@"` (chi tiết: `su-code/KNOWLEDGE.md`, entry feynman).
- zai-vision key nằm trong `~/.omp/agent/mcp.json` (per-machine, không theo git).

**Trên máy mới — runbook (theo thứ tự):**
1. `git pull` (hoặc clone `https://github.com/8-Sync-Dev/su-code.git`).
2. `bash scripts/bootstrap.sh` (build+install) **hoặc** `curl -fsSL https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/install.sh | sh`.
3. `8sync setup` (omp + codegraph + MCP/skills + gh) → cấu hình omp API key.
4. `8sync harness` → deploy skills + AGENTS.md + codegraph index + commands + gitleaks hook + **MCP always-visible config**.
5. `8sync doctor` → phải thấy `✓ STEP-0 MCP servers always visible (mcp.discoveryDefaultServers)`.
6. Per-máy nếu cần: npm fix ở trên · `8sync feynman auth-omp` · `8sync harness browser` · `8sync vpn install`.

## Current step
**STEP-0 MCP activation fix (WIP checkpoint trên v0.52.0)** — done + verified live trên máy này; commit này chính là nó. Plan gốc: `local://mcp-step0-activation-plan.md` (12/12 tasks done; cơ chế đổi essentialOverride → `mcp.discoveryDefaultServers` theo contingency B, có evidence trong KNOWLEDGE).
- **Prior shipped**: `/push-now` + `/pull-now` commands (c402209, 6bb38ae) · v0.52.0 (`8sync vpn`) · v0.51.0 (`feynman auth-omp`) · v0.50.0 (omp `/new` fix + `harness browser`) · v0.49.x (`add-model`) · v0.48.0 (`/feature` GSD) · v0.47.0 cross-platform.

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
