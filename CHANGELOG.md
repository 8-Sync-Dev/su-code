# Changelog

Mọi thay đổi đáng kể của `8sync` ghi vào đây. Format theo [Keep a Changelog](https://keepachangelog.com),
versioning theo [SemVer](https://semver.org). **8sync rule:** mỗi PR cập nhật mục `Unreleased`.

## [Unreleased]

## [0.15.0] — 2026-06-16

### Added

- **`8sync harness` verb** — one command to stand up the full agent harness.
  - `harness init`: deploy mọi bundled skill + codegraph binary + external skill
    packs (best-effort clone), mirror vào `agents/skills/`, `codegraph init`,
    seed `agents/*` memory + `CHANGELOG.md`, inject force-load vào AGENTS.md/CLAUDE.md
    + một index gọn vào **mọi sub-folder code** (progressive disclosure). Có progress
    UI `[i/N]` + thời gian.
  - `harness up`: refresh theo state hiện tại (re-inject + refresh `agents/KNOWLEDGE.md`
    breadcrumb + `codegraph index`). `--loop <dur>` chạy foreground; `--timer <dur>|off`
    cài/gỡ systemd **user timer** (đúng cách cho chạy nền, mirror `8sync clean --timer`).
- **6 bundled skill mới**: `ponytail` (always-on, lazy-senior YAGNI), `code-review-and-quality`,
  `senior-security`, `senior-frontend`, `full-flow`, `encore-deploy` (on-demand). Trước đó
  (0.14.x → nội bộ) đã thêm `assp-skill`, `impeccable`, `taste-skill`. Tổng **15 bundled**.
- **Always-on order** (đọc top-down, ưu tiên): codegraph → karpathy → ponytail → assp →
  impeccable → taste → 8sync-cli → image-routing. Inject block dạy rõ *cách tận dụng* từng skill.
- **Tech-gated skills**: `encore-deploy` chỉ hiện trong force-load block khi project dùng
  Encore (`encore.app` / `encore.dev`).
- **Opt-in skills**: `social-growth` (chiến dịch social/branding/lead-gen cho FB/YouTube/TikTok,
  page setup, insight, monthly plan + target) — KHÔNG auto-bật; bật bằng
  `8sync skill add builtin:social-growth`.
- **`8sync skill add` collection-aware**: clone repo rồi cài mọi `skills/<name>/SKILL.md`
  (vd `addyosmani/agent-skills` 24 skill, `ponytail` full); `builtin:<name>` deploy
  bundled skill từ embedded assets.
- **Sub-folder `AGENTS.md` index** + **`agents/KNOWLEDGE.md` breadcrumb** + **`CHANGELOG.md`**
  seeding tự động, để agent không bỏ sót rule và tự học theo state dự án.

### Changed

- **`8sync skill sync` → `8sync harness init`** (clean cutover, không giữ alias). `skill sync`
  in cảnh báo trỏ sang lệnh mới.
- `crates/cli/src/verbs/skill.rs` (~1340 dòng) tách thành module tree `verbs/skill/`
  (`mod` · `meta` · `discover` · `list` · `spec` · `add` · `gen` · `deploy` · `inject` · `index`),
  mỗi file < 500 dòng. Harness logic ở `verbs/harness/` (`mod` · `init` · `up` · `memory` · `external`).
- `8sync .` giờ cũng inject sub-folder index (nearest-AGENTS.md wins).
- Binary size target: < 4 MB (binary ~3.8 MB stripped, gồm 15 bundled skill).

## [0.14.2] — 2026-06-02

- fix(bt): Bluetooth vanishing after cold boot (USB autosuspend).

## [0.14.1] — 2026-05-31

- clean is project-safe: never touches models / Playwright / download caches.

## [0.14.0] — 2026-05-31

- `8sync clean`: disk/RAM reclaim + CPU/GPU report + periodic timer.

## [0.13.0] — 2026-05-31

- `8sync bt` bluetooth verb; Caelestia desktop install removed.

## [0.12.1] — 2026-05-30

- two-tier skill injection (always-on vs on-demand).
