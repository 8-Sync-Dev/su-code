# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)

## Goal
Biến 8sync/omp thành **super agent-team** token-optimal: omp = core, su-code = tools. Automation = **`/auto`** (`8sync-engine`: slice/task state machine · code-enforced verify-retry · worktree); model **adaptive per-prompt**; context **always-read**; terminal + web **glass**.

## Current step
**v0.47.0 — cross-platform (macOS + Windows), option B "full parity" (this session)**. Was Linux/Arch-only. Code + CI + installers done; `Cargo.toml` = **v0.47.0** (release/tag pending commit+push).
- **New `platform` module** (`crates/cli/src/platform.rs`) — the OS seam: `os()` (compile-time const), `require_linux()` guard, `pkg_manager()` (pacman/brew/winget) + `install_core_pkg()`, and a cross-platform periodic timer `install_timer`/`remove_timer` → **systemd (Linux) / launchd LaunchAgent (macOS) / schtasks (Windows)**. Only cross-platform std/crate APIs (no `std::os::unix`).
- **Portability:** dropped `target-cpu=native` (`.cargo/config.toml`) → prebuilts run on any CPU of the arch (was SIGILL-prone). `harness up --timer` + `clean --timer` route through `platform::*` (Linux keeps the 0.46.2 cgroup bounds). `setup` Stage A cross-platform (`gh` via native pkg mgr; `paru`/AUR + Arch Stage B skipped off-Linux). `sec`/`bt`/`clean` → `require_linux` clean no-op off-Linux.
- **CI + dist:** `.github/workflows/release.yml` — on `v*` tag, matrix builds native binaries (musl-static linux x86_64/aarch64 [aarch64 via `cross`], macOS x86_64/arm64, Windows MSVC) → GitHub Release, `8sync-<tag>-<os>-<arch>` scheme. `install.sh` full os×arch matrix + new `install.ps1` (Windows). Orphan `agents/` skill mirror cleaned.
- Verified: Linux release build clean (0 warn, 6.16 MB) + runtime smoke (sec/clean/help) unaffected. **mac/Windows binaries build + verify on CI native runners** — a Linux host can't build MSVC/Apple-SDK targets or the C deps (`rusqlite`/`zstd-sys`) without each platform's toolchain (no passwordless sudo for mingw here; `x86_64-pc-windows-gnu` check blocks on `libsqlite3-sys` C build as expected).

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

## Handoff (đổi máy — làm theo thứ tự)
1. `git clone https://github.com/8-Sync-Dev/su-code.git && cd su-code`
2. `bash scripts/bootstrap.sh` (hoặc `8sync up`) → build + cài `8sync`
3. `8sync harness` — auto-setup (MCP + skills + memory + inject + index)
4. `gh auth login` (cho `8sync ship` / release)
5. Mở omp → `/auto <mục tiêu>` để chạy engine tự động.
- Lịch sử quyết định + bài học: `su-code/KNOWLEDGE.md` (+ `su-code/archive/`).
