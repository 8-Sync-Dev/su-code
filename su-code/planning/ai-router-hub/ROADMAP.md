# ROADMAP — AI Router Hub

> Bản đồ phase + dependency. Thưa — chỉ tick khi phase bắt đầu/xong.
> Task chi tiết KHÔNG ở đây (ở M<x>-NN-PLAN.md). Tiến độ task ở STATE.md.

**Created:** 2026-08-07

## Phases (theo dependency)

- [x] **M0 Foundation** — Encore Go control-plane scaffold (authHandler Bearer, envelope, health, folder-per-service) + chốt repo location + CI/deploy skeleton · UC: UC1 · ✅ Go-level (encore-native NEEDS-CONFIRM)
- [ ] **M1 Data-plane bring-up** — CLIProxyAPI hosted (Postgres store + Home mode), onboard 1 provider, verify completion thô · cần: M0 · UC: UC2
- [ ] **M2 Gateway + identity** — members/roles/grants, key per-member `ena_...`, authHandler→forward, revoke · cần: M0,M1 · UC: UC3,UC4
- [ ] **M3 Usage + quota** — ghi usage per-member, enforce quota, usage API · cần: M2 · UC: UC5
- [ ] **M4 Admin UI** — Next.js (login, member list+count, grant/revoke, usage, provider onboarding) · cần: M2,M3 · UC: UC6
- [ ] **M5 Hardening + deploy** — SSO, secrets, rate-limit, deploy Encore Cloud+Vercel, domain, endpoint gate, e2e 1 member · cần: M0-M4 · UC: UC7

Status: `[ ]` chưa · `[~]` đang làm · `[x]` xong

## Dependency graph

```
M0 ──┬─→ M1 ──→ M2 ──→ M3 ──┐
     │                       ├─→ M4 ──→ M5
     └───────────────────────┘
```

## Integration Contracts (khớp giữa phase — chống lệch)

- **M0 export:** Encore app skeleton + authHandler resolve `{memberId, role}` + envelope `{success,message,result}` + CI gate `verify-public-endpoints` → M1..M5 dùng.
- **M1 export:** CLIProxyAPI `base_url` + internal service key + Postgres store schema (auth/cooldown) + Management API base → M2 forward, M4 onboarding.
- **M2 export:** format key `ena_<member>`; endpoint `/gate/keys` (issue/revoke), `/gate/members`; forward path `POST /v1/chat/completions` (proxy) → M3 gắn usage, M4 gọi.
- **M3 export:** bảng `usage` (member_id, model, tokens_in/out, cost, ts) + `/usage?member=` API + quota check middleware → M4 render.
- **M4 export:** admin UI + BFF proxy path `/api/backend/*` → M5 deploy + SSO.

## Phase log (append khi ship)

- **M0** (2026-08-07): scaffold `deploy/ai-router-hub/backend-go/` — gate service (health public + whoami auth + authHandler Bearer + envelope) + CI allowlist gate. Go-level verified (podman vet/build/test PASS, 5/5 engine task). encore-native run/test defer Docker box. M1 blocked trên B3 credentials.
