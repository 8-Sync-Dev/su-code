# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## 🚚 HANDOFF — sang máy khác làm tiếp GẤP (2026-07-22)
**Repo state:** branch `main`, HEAD trước session = `64bd650` (STEP-0 MCP fix, omp-16-era), tag mới nhất `v0.52.0` (Cargo.toml = 0.52.0 — commit này là WIP checkpoint, KHÔNG release). Session này thêm 1 commit → sau push cây **tracked SẠCH** (còn stray untracked local-only: root `STATE.md` foreign + `outputs/*` research — KHÔNG push, chỉ có trên máy này).

**Đã làm session này (2026-07-22) — 2 việc:**

**A. Fix omp-17 MCP "HIDDEN" phantom (bug thật, sửa tận gốc):**
- **Phát hiện:** omp 17.x ĐÃ BỎ hẳn cơ chế bm25 discovery — không còn `search_tool_bm25`, không còn `mcp.discoveryDefaultServers` (biến mất khỏi settings schema). Thay bằng `tools.xdev` (default ON): MCP tools mount thành `xd://mcp__…` device URLs, callable qua read/write, không ship schema mỗi request. → Fix `64bd650` (ghi `discoveryDefaultServers`) là **NO-OP trên omp 17** và là **nguồn churn**: omp self-upgrade reset `~/.omp/agent/config.yml` → doctor check `cfg.contains("discoveryDefaultServers")` la làng "HIDDEN" dù tools vẫn gọi được. "MCP cứ regress sau mỗi omp upgrade" = PHANTOM.
- `crates/cli/src/env_detect.rs` — thêm `omp_major()` parse `omp/17.0.6` → 17.
- `crates/cli/src/verbs/skill/deploy.rs` — `ensure_mcp_tools_visible`: omp ≥17 → early-return, skip ghi key chết (báo xd:// mount); <17 giữ logic cũ.
- `crates/cli/src/verbs/doctor.rs` — MCP check omp-version-aware: omp ≥17 báo `✓ xd:// devices callable`, hết "HIDDEN" giả.
- `crates/cli/src/verbs/harness/global.rs` — summary bullet khớp cơ chế mới.
- **Verified:** build xanh; `8sync doctor` → `✓ STEP-0 MCP tools mounted as xd:// devices (omp ≥17 tools.xdev)`; harness không ghi key chết; MCP callable live cả session qua `xd://mcp__…`.

**B. Lark → profile auto-download (KHÔNG nhúng binary vào git):**
- `assets/profiles/apps-personal.toml` — thêm `aur = ["larksuite-bin"]` (v7.66.11, larksuite.com) cạnh Bitwarden + `requires.aur_helper = true`. AUR bin tự tải .deb official lúc build, pacman-tracked, KHÔNG commit 437MB binary. Bản TQ (Feishu) = `feishu-bin`. Opt-in; bundle `alexdev` cố ý KHÔNG include apps-personal.
- **Verified:** rebuild + `8sync setup profile show apps-personal` liệt kê `larksuite-bin`; dry-run `would paru install: larksuite-bin`.

**Docs:** CHANGELOG (2 entries Unreleased: omp-17 fix + Lark), KNOWLEDGE (validated: omp-17 dropped bm25 — SUPERSEDES entry `discoveryDefaultServers` cũ + gotcha omp auto-upgrade reset config.yml). AGENTS.md/CLAUDE.md = harness regen churn.

**Next ▸ (cụ thể):**
- [ ] Máy mới: `8sync harness`. Trên omp ≥17 KHÔNG cần config MCP nữa (tools.xdev tự mount) — doctor confirm.
- [ ] (tùy) Cài Lark: `8sync setup --profile apps-personal` (cần paru/yay).
- [ ] (tùy) Dọn bản Lark cài tay session trước: `~/.local/opt/lark` + `~/.local/bin/bytedance-lark-stable` + `~/.local/share/applications/bytedance-lark.desktop` nếu chuyển sang bản AUR pacman-tracked.
- [ ] (tùy) `8sync harness toolstats` sau vài session — kỳ vọng optimizer % tăng (tools callable trực tiếp).

**⚠ Per-machine gotchas (KHÔNG theo git):**
- omp AUTO-UPGRADE mạnh (16.5.2→17.0.6 qua ~3 session), MỖI upgrade rewrite `~/.omp/agent/config.yml` về default tối thiểu (mất mnemopi/compaction/modelRoles của 8sync). Fix: chạy lại `8sync harness global` (idempotent re-apply). Chỉ config.yml bị ảnh hưởng; mcp.json/skills/hooks/APPEND_SYSTEM sống sót.
- omp ≥17: MCP tools = `xd://mcp__…` devices (tools.xdev default), gọi trực tiếp. Không còn `discoveryDefaultServers`.
- npm/pnpm shim hỏng → feynman crash `MODULE_NOT_FOUND …/npm-cli.js`: thay symlink `~/.local/bin/{npm,npx}` bằng wrapper `exec ~/.local/share/pnpm/{npm,npx} "$@"` (chi tiết KNOWLEDGE).
- zai-vision key trong `~/.omp/agent/mcp.json` (per-machine).

**Trên máy mới — runbook (theo thứ tự):**
1. `git pull` (hoặc clone `https://github.com/8-Sync-Dev/su-code.git`).
2. `bash scripts/bootstrap.sh` (build+install) hoặc `curl -fsSL .../install.sh | sh`.
3. `8sync setup` → cấu hình omp API key.
4. `8sync harness` → skills + AGENTS + codegraph index + commands + gitleaks hook (+ config MCP nếu omp <17).
5. `8sync doctor` → omp ≥17 phải thấy `✓ STEP-0 MCP tools mounted as xd:// devices (omp ≥17 tools.xdev)`.
6. Per-máy nếu cần: npm fix · `8sync feynman auth-omp` · `8sync harness browser` · `8sync setup --profile apps-personal` (Lark).

## Current step
**omp-17 MCP fix + Lark profile (WIP checkpoint trên v0.52.0)** — done + verified máy này; commit này là nó. KHÔNG release (không bump tag).
- **Prior shipped**: STEP-0 MCP fix omp-16 (64bd650, nay được omp-17-aware hóa) · `/push-now`+`/pull-now` (c402209, 6bb38ae) · v0.52.0 (`8sync vpn`) · v0.51.0 (`feynman auth-omp`) · v0.50.0 · v0.48.0 (`/feature` GSD) · v0.47.0 cross-platform.

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
