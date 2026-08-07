---
gsd_state_version: '1.0'
feature: ai-router-hub
ticket: ""
branch: ""
status: blocked
active_phase: "M1"
next_action: unblock-M1-credentials
next_phases: ["M1"]
progress:
  total_phases: 6
  completed_phases: 1
  percent: 17
last_updated: "2026-08-07"
---

# State — AI Router Hub

## Project Reference

See: su-code/planning/ai-router-hub/PROJECT.md · ROADMAP: su-code/planning/ai-router-hub/ROADMAP.md
**Core value:** 1 điểm router AI hosted + admin OAuth thật — cấp quyền omp cho member team, token provider server-side.
**Current focus:** M0 ✅ SHIPPED (Go-level verified). M1 blocked trên B3 credentials (data-plane cần provider account + Postgres).

## Current Position

Phase: M0 of 6 (Foundation) — ✅ **DONE** (Go-level) · M1 = **BLOCKED**
Plan: M0-01-PLAN — 5/5 task qua engine verify-gate.
Status: M0 shipped; encore-native gate NEEDS-CONFIRM (Docker box). M1 chờ credentials.
Vì sao dừng: M1 (data-plane bring-up) cần CLIProxyAPI hosted + Postgres + ≥1 provider account = B3 credentials (hard blocker, không tự cấp được) → /auto dừng đúng luật ("stop chỉ khi true blocker").
Last activity: 2026-08-07 — author + Go-verify (vet/build/test) + CI gate scaffold M0 trong podman; review pass; engine 5/5.

## Accumulated Context

### Decisions (append, 3-5 gần nhất; full ở PROJECT.md)
- [M0]: chạy CLIProxyAPI nguyên làm data plane, KHÔNG rewrite trong Encore (nó đã có OAuth+refresh+translate; rewrite giòn).
- [M0]: control-plane Encore = source-of-truth RBAC; thu quyền = disable key ở Encore.
- [M0]: admin UI = Next.js/Vercel + BFF (không native Encore HTML).
- [M0]: code target mặc định = app mới trong monorepo 8sync-startup (chờ user confirm).

### Contract — phase sau CẦN BIẾT (append mỗi khi phase xong)
- **[M0 export]** Encore Go app `deploy/ai-router-hub/backend-go/` (module `ai-router-hub`, pkg `gate/`): authHandler Bearer → `AuthData{MemberID,Role}` (`errs.Unauthenticated` khi thiếu/sai); envelope `Response{success,message,result}` + `ok()/fail()`; `GET /health` (public), `GET /whoami` (auth); CI `scripts/verify-public-endpoints.sh` (allowlist public). Verify Go-level: `podman golang:1.24` vet/build/test PASS. ⚠ encore-native (run/test) NEEDS-CONFIRM — xem M0-VERIFICATION Risk #1 (`Result interface{}`).

### Files touched (per phase)
- [M0]: code `deploy/ai-router-hub/backend-go/{encore.app,go.mod,go.sum,.gitignore,README.md,gate/*.go,scripts/verify-public-endpoints.sh,.github/workflows/backend.yml}` — **on-disk + Go-verified nhưng CHƯA git-commit** (monorepo `.gitignore` `/deploy/*` chỉ track submodule; commit khi promote submodule + có GitHub repo). Planning `su-code/planning/ai-router-hub/*.md` + KNOWLEDGE/PLAYBOOKS = commit local trong repo su-code.

### Blockers/Concerns  (⚠ = NEEDS-CONFIRM, chặn `/feature go` M0+)
- ✅ **B1 RESOLVED:** code target = `~/Projects/startup/8sync-startup/deploy/ai-router-hub/backend-go/` (dir thường; promote submodule sau). Submodule mind0/zus trong monorepo đang empty → scaffold sạch từ template Encore Go, chỉ áp convention (không mirror source).
- ✅ **B2 RESOLVED (chọn podman) + probe:** Encore CLI chạy trong podman OK (`v1.57.13`); NHƯNG `encore build/run/test` cần Docker → chỉ verify được **Go-level** (`go vet/build`) trong podman rootless. Encore-native gate (`encore run/test`) chạy ở deploy box có Docker của user. ⇒ M0 go: author scaffold + Go-level verify ở đây; encore-native verify = NEEDS-CONFIRM (user deploy).
- ⚠ **B3 credentials (hard blocker, sẽ SKIP+NEEDS-CONFIRM khi tới):** Encore Cloud account (M0 deploy/M5); ≥1 account provider để onboard (M1); Postgres cho CLIProxyAPI store (M1); Google Workspace SSO cho admin login (M4/M5); domain (M5).
- Note: field cấu hình chính xác của omp để trỏ custom provider base_url+key cần confirm trong omp provider config (`~/.omp/agent/models.yml`) khi tới M2.
- ⛔ **M1 BLOCKED (true blocker, /auto stop):** M1 = host CLIProxyAPI thật → cần (a) Postgres, (b) ≥1 account provider (Claude/Gemini/Codex subscription) để onboard, (c) chỗ chạy CLIProxyAPI binary. Đều là credentials/data ngoài tầm agent. Cấp B3 → resume `/feature plan` M1.

## Session Continuity

Stopped at: M0 SHIPPED — scaffold `backend-go` authored, Go-level verified (podman vet/build/test), CI gate PASS, engine 5/5, review READY, M0-VERIFICATION ghi. Commit local (chưa push).
Next: gỡ B3 (Postgres + provider account + host cho CLIProxyAPI) → `/feature plan` M1 (data-plane bring-up). Trên Docker box: chạy `encore run` + kiểm Risk #1 (`Result interface{}`) trước.
