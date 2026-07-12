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
   - `8sync vpn install` + `8sync vpn on [CC]` → nếu muốn tunnel qua VPN Gate (SoftEther): cài engine + Wine GUI + dhcpcd, connect + route (per-máy, cài package + đổi route/DNS máy — không theo repo).

**Đã xong (0.50.0, không cần làm lại):** code cả 2 fix (`/new` `--cwd` root pin + `harness browser`), CHANGELOG, KNOWLEDGE (2 learnings), README row, help/examples. Tag + CI + publish xong.

**Việc còn lại / cần quyết:**
- [ ] **grok-4.5 loose end** (chỉ trên MÁY NÀY, trong `~/.omp/agent/models.yml`): entry `xai/grok-4.5` đang có **placeholder key**. Hoặc `export XAI_API_KEY=... && 8sync harness add-model xai/grok-4.5 --url https://api.x.ai/v1 --ctx 500000 --think` (dùng API key thật), hoặc `8sync harness add-model rm xai/grok-4.5` (bạn vốn dùng grok qua OAuth `xai-oauth`). KHÔNG theo repo — chỉ ảnh hưởng máy này.
- [ ] (tùy) Máy mới: `8sync harness browser status` để confirm wiring sau khi mở shell mới.

## Current step
**v0.52.0 — `8sync vpn` (SoftEther client + VPN Gate, study-through-region)**. `Cargo.toml` = **v0.52.0**.
- **New top-level verb** `vpn [install|gui|list|on|off|status]` (`crates/cli/src/verbs/vpn.rs`). Connect through VPN Gate (U. Tsukuba academic public relays) like the Windows client. `install` = native Linux engine `softethervpn` (RTM 4.44, **not** `-git` 5.x) + **Windows VPN Client Manager GUI via Wine** (`softethervpn-client-manager`, region-switch plugin lives there; `--no-gui` skips) + `dhcpcd` + enable client service. `gui` opens the manager.
- **Grounded (SoftEther docs):** Linux client has **no native GUI** + **can't auto-rewrite the routing table** → the reliable region-switch is the CLI. `list [CC]` ranks the VPN Gate CSV API; `on [CC|ip]` picks best relay, `vpncmd` connect (HUB VPNGATE, user/pass `vpn`), **pins relay route via physical uplink**, DHCPs tap, full-tunnels default, DNS→1.1.1.1, **auto-rollback if egress unchanged** (egress via Cloudflare IP-trace, DNS-swap-safe); `off` restores.
- **Verified**: build clean; `vpn -h`/status(not-installed)/`list`/`list JP`(live VPN Gate fetch)/`gui`(not-installed)/`off`(idle guard) all OK. NOT run live in-session: `install` (AUR softethervpn + Wine GUI, sudo) + `on` (reroutes the operating shell) — user runs on their box.
- **Prior shipped**: v0.51.0 (`feynman auth-omp`) · v0.50.0 (omp `/new` fix + `harness browser`) · v0.49.1 (`add-model --think`) · v0.49.0 (`harness add-model`) · v0.48.0 (`/feature` GSD + `brand.rs`) · v0.47.0 cross-platform.

## Next (chưa làm)
- [ ] **Push tag v0.52.0** → CI release matrix produces the 5 assets.
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
