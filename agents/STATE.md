# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation mạnh kiểu gsd-pi = **`/auto`** (`8sync-engine`, 100% trên omp core: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read** (APPEND_SYSTEM + serena); terminal + web **glass**. (`/gs` = skill-backed lead, không còn là engine.)

## Definition of Done
- [x] Loop-engineering v2 (Phase A–E) shipped + đo bằng `8sync harness bench`
- [x] `/gs` — một lệnh chạy team tự động; `auto` không dừng; hint `[auto|<goal>|status|next|stop]`; QA+Closeout gate
- [x] Bare `8sync harness` = auto-setup đầy đủ (MCP + skills + /gs + memory + inject + index)
- [x] **Doc-hygiene code-backed** (`8sync harness audit`: stale paths / oversized / churn) + wired vào `doctor` + `/gs` (v0.22.0)
- [x] **AI-engine health check** trong `doctor` — codegraph/cbm/headroom present + registered ("luôn xài") (v0.22.0)
- [x] **Loop correctness** — codegraph verbs đúng (query/callers/callees/impact) · force-load dedup theo frontmatter name · impeccable `.agents`→`agents` path fix (v0.22.0)
- [x] **Loop quality probe** `8sync harness eval` (omp task-suite + verify.sh + baseline diff) — verified 3/3 (v0.23.0)
- [x] **/gs L3 worktree isolation** cụ thể hoá: `git worktree add .gs/wt/<slug> -b gs/<slug>` (v0.23.0)
- [x] **Dashboard redesign + Models/Projects** (v0.29.0): `8sync harness web` full impeccable redesign · Models page (view/edit routing live) · project switcher (status dots) · markdown render · serena-off fix · context honesty · workflow canvas — 14 pages, 0 console errors.

## Current step
**/auto (Unreleased): local GGUF models for omp + LocateAnything-3B vision.**
Runner decision (per user directive "tận dụng rust mạnh mem leak tốt để load gguf"):
**mistral.rs** — pure-Rust, memory-safe GGUF loader with an OpenAI `/v1` server;
its `install.sh` ships a prebuilt CUDA/CPU binary (no nvcc — was missing here).
- [x] **Slice 1 — `8sync harness add-local-model <path> [name]`** (`harness/local_model.rs`):
  classify path (local .gguf | HF repo | URL, GGUF magic-checked) → ensure mistralrs →
  systemd user service `8sync-llm-<name>` → register omp provider `local/<name>` in
  models.yml (sentinel block; TSV registry source-of-truth) → verify `/v1`. `list`/`rm`.
- [x] **Slice 2 — coexist+wire**: `gateway apply` preserves the local block; `doctor` +
  `capabilities.md` report local models; help/README/CHANGELOG/KNOWLEDGE.
- [x] **Slice 3 — LocateAnything-3B**: `8sync locate <img> <prompt>` via mudler/
  locate-anything.cpp (MIT ggml port, prebuilt q8_0 GGUF on HF) → boxes JSON for GUI
  grounding/OCR-localization; `--setup` builds the CLI. Skill + NVIDIA non-commercial caveat.
- [x] **Slice 4 — closeout**: real E2E proven on this box — mistral.rs 0.8.23 (auto
  `cuda131-sm120` RTX 5080) serves SmolLM2-135M GGUF; `add-local-model` → systemd unit →
  `/v1/chat/completions` real text → `rm` clean. Caught+fixed 2 bugs: serve needs
  `-m <dir> -f <file> --format gguf` (not `-m <file>`); omp `id: default` (mistral.rs
  serves only `default`+dir-path), `local/<name>` selector via `name:` field.
**8/8 tasks done (`engine_status`).** No push until asked — commit a9140c2 local.

## Previous (Unreleased, shipped earlier this session)
Shipped 2 fixes (Unreleased): **(1) zai-vision MCP + skill** — `8sync harness`
auto-installs `@z_ai/mcp-server`, registers `zai-vision` MCP with
`Z_AI_VISION_MODEL=glm-4.6v-flash` (only free/working vision model on a stock
key — verified end-to-end via real stdio JSON-RPC tool calls), key from
`omp token zai`. New skill `assets/skills/zai-vision/SKILL.md` (auto-deployed)
documents the full combination matrix + real verified examples.
`~/.omp/capabilities.md` snapshot added, surfaced by `doctor`.
**(3) exact MCP tool catalogs** — `capabilities.md` now embeds the FULL exact
tool list (name + 1-line use) for every registered MCP server
(codebase-memory-mcp/headroom/serena/zai-vision), omp's own built-in tools
(parsed live from `omp --help`), and Mnemopi memory tools — no more guessing
tool names (the failure mode that caused the earlier codegraph-verb bug).
`APPEND_SYSTEM.md` RULE #0 + the recall hook both point at it as ground truth.
**(2) kitty window decorations** — `render_kitty_conf` hardcoded
`hide_window_decorations yes`, breaking title bar/min-max-close/resize on
stacking WMs (reproduced live on this machine's KDE/kwin/Wayland). New
`env_detect::is_tiling_wm()` gates it to Hyprland/HyDE/sway/etc only.
Re-ran `8sync setup --profile terminal` — `~/.config/kitty/8sync.conf` fixed;
user must close/reopen the kitty window (decorations negotiate at
window-creation, not live-reloadable). See KNOWLEDGE.md top 2 entries +
CHANGELOG Unreleased for full detail.
**(4) kitty zoom binding** — `ctrl+shift+minus` (kitty's default zoom-out)
was silently overridden by the 3-pane vsplit map; moved vsplit to
`ctrl+shift+backslash`. Live config regenerated on this machine.
**(5) 18 feynman research skills ported to omp-native** — 20 skills in
`agents/skills.toml` pointed at `companion-inc/feynman`; audited via a
temp submodule (removed after) and found 14 were dead stubs pointing at
feynman's own slash-commands (`/deepresearch`, `/lit`, …, not usable in
omp) + 4 more with only cosmetic feynman naming. Ported all 18 into
`assets/skills/<name>/SKILL.md` (builtin, omp-native tools: `task` /
`web_search` / `read` / `ask`). `alpha-research` kept pointed at feynman
(real CLI dependency, `ensure_feynman_cli()`). Also fixed a real bug in
`update_skills`: registering one skill from a collection repo silently
reinstalled EVERY sub-skill on every bulk run, un-droppable via the
manifest — fixed the filter logic. `peer-review` renamed `research-review`
(feynman's actual current name; the old entry never resolved).

## Next (chưa làm — tùy chọn)
- [x] **DONE: `outputs/one-auto-unification-plan.md` P1–P6** — `/auto` là lệnh tự động DUY NHẤT (v0.28.0).
- [ ] **Phase 3b — gstack host `omp`** (DEFERRED, không regression): role `/qa`,`/ship` đã fallback bundled; host nằm TRONG submodule gstack (foreign repo, pinned SHA) — KHÔNG thuộc binary su-code. Chỉ làm khi muốn role tool-backed chạy thật qua gstack: `git submodule update --init reference/gstack` → đọc `docs/ADDING_A_HOST.md` → implement → `./setup --host omp` → deinit lại.
- [ ] (tùy) Chạy `8sync harness eval --baseline` định kỳ để theo dõi chất lượng loop qua thời gian (kết quả ở `.cache/8sync/eval/`, gitignored).
- [x] **(P2 — DONE) Mnemopi memory** wired vào `8sync harness`+`init` (`deploy.rs::ensure_mnemopi_memory`, idempotent sentinel-block, không clobber) + `doctor` báo ON/OFF; bật máy này (`~/.omp/agent/config.yml`, API-only `llmMode:smol`+`noEmbeddings`). omp 16.1.20 load OK.
- [ ] (tùy) Loại `reference/` khỏi codegraph (không honor exclude — xem failure trong KNOWLEDGE); tạm deinit.

## Open questions / blockers
_none._

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 treo bật bằng `/gs auto` + `8sync harness up --timer`.
- Reference submodules để deinit mặc định (ưu tiên token-lean hơn là luôn-có-sẵn nội dung).

## Handoff (đổi máy — làm theo thứ tự)
1. `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code`
2. `bash scripts/bootstrap.sh` (hoặc đã có 8sync thì `8sync up`) → build + cài `8sync` ≥ 0.23.0
3. `8sync harness` — auto-setup hết (MCP + skills + `/gs` + memory + index)
4. `gh auth login` (để `8sync ship` / release hoạt động)
5. Muốn đọc repo tham khảo: `git submodule update --init reference/gstack reference/gsd-pi` (xong thì `git submodule deinit -f reference/<name>` cho index gọn)
6. Mở omp → `/gs <mục tiêu>` để giao việc, `/gs auto` để chạy không dừng.
- Toàn bộ lịch sử quyết định + bài học: đọc `agents/KNOWLEDGE.md` (mục Learnings, đọc các entry `validated:`/`failure:` gần nhất trước).
- Kế hoạch gốc đầy đủ: `outputs/harness-loop-engineering-v2-plan.md`.
