# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**v0.46.2 shipped (this session)** — 3 patch releases off the `agents/`→`su-code/` rename (v0.46.0). All committed + pushed + gh-released; `Cargo.toml` = latest tag = **v0.46.2**.
- **v0.46.0** — renamed agent-memory folder `agents/` → **`su-code/`** (distinctive project marker). `is_omp_project`/root-detection key on `su-code/`; `memory::migrate_legacy_layout` auto-migrates (guarded on real memory files; source pkg `agents/`, `.agents/`, `subagents/` untouched; idempotent). Recall hook keeps `agents/STATE.md` fallback for un-migrated repos. Swept all 16 `~/Projects` repos → 0 legacy left.
- **v0.46.1** — `--sweep` migrated the memory folder but left the project-level `.omp/commands/auto.md` (+ `8sync-engine.ts`) untouched, so `/auto` in a swept repo kept reading `agents/STATE.md` from a stale copy (**project commands outrank the global one in omp**). Fix: `stamp_project` now calls `deploy::ensure_engine` → every swept project's `/auto` refreshed to `su-code/`. Verified 0 stale.
- **v0.46.2** — `8sync-harness-up.service` (from `harness up --timer`) had **no cgroup limits** → per-tick `codegraph index` hit ~5.3 GB RSS → kernel **OOM-killed the machine** every 10 min (`Result: oom-kill, Mem peak 5.3G`). Fix: generated unit is now bounded + low-priority — `MemoryHigh=2G`/`MemoryMax=4G`/`MemorySwapMax=512M`/`OOMPolicy=stop`/`Nice=15`/`CPUWeight=10`/`IOWeight=10`/`TimeoutStartSec=900`. Verified: codegraph held ~2 GB by reclaim pressure (was 5.3 GB). Re-run `harness up --timer <dur>` overwrites already-installed unbounded units.

## Next (chưa làm)
- [ ] **macOS + Windows build** (user asked for a plan — plan delivered, awaiting direction on scope). Findings: code compiles cross-platform today (0 `std::os::unix`, 0 cfg-gating; `cargo check --target x86_64-pc-windows-gnu` passes all Rust code + pure-Rust deps). Blockers to resolve: (1) `.cargo/config.toml target-cpu=native` makes prebuilts non-portable (SIGILL on older CPUs — affects the current Linux prebuilt too) → drop for release; (2) C-FFI deps `libsqlite3-sys` (rusqlite bundled, toolstats) + `zstd-sys` (include-flate) need a C toolchain → **native CI runners** (macos-14/windows-latest) build cleanly, cross-from-Linux needs mingw/osxcross; (3) cfg-gate 4 Linux-only verbs (`setup` pacman · `sec` warp/ufw · `bt` bluetooth · `clean` pacman/nvidia) + port `up --timer` (systemd→launchd/schtasks) & `.` (abduco). Phases: (1) portability guardrails [buildable+verifiable from Linux], (2) `.github/workflows/release.yml` 3-OS matrix + `install.ps1`, (3) docs.
- [ ] Phase 3b — gstack host `omp` (DEFERRED; xem archive + `reference/gstack` docs/ADDING_A_HOST.md).
- [ ] (tùy) `8sync harness eval --baseline` định kỳ · loại `reference/` khỏi codegraph (deinit).

## Open questions / blockers
- mac/win build **scope + effort branch** — awaiting user's pick: (A) MVP = AI-harness core cross-platform + gate Linux-only verbs, vs (B) full parity with platform equivalents (brew/winget setup, launchd/schtasks timer). Need macOS + Windows CI runners (or the user's own machines) for real *runtime* verification — cannot test mac/win runtime from this Linux box.

## Assumptions (auto-decided — user can correct)
- Default autonomy = L2 (assisted); L3 bật bằng `/auto` + `8sync harness up --timer`.
- Reference submodules để deinit mặc định (token-lean hơn luôn-có-sẵn).
- Spine advisory threshold = spine >50% upfront (relative, không absolute floor).

## Handoff (đổi máy — làm theo thứ tự)
1. `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code`
2. `bash scripts/bootstrap.sh` (hoặc `8sync up`) → build + cài `8sync`
3. `8sync harness` — auto-setup (MCP + skills + memory + inject + index)
4. `gh auth login` (cho `8sync ship` / release)
5. Mở omp → `/auto <mục tiêu>` để chạy engine tự động.
- Lịch sử quyết định + bài học: `su-code/KNOWLEDGE.md` (+ `su-code/archive/`).
