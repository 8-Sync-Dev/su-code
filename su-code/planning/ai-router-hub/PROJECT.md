# PROJECT — AI Router Hub (hosted OAuth + team gateway)

> Vision + ràng buộc + decisions. Gần như bất biến. "Tại sao + luật chơi" của feature.

## What This Is

Một **server hosted** đóng vai trò 1-điểm-vào cho AI: giữ các tài khoản OAuth provider (Claude/Gemini/Codex subscription) ở server, expose 1 endpoint OpenAI-/Anthropic-compatible, và **cấp quyền cho từng member team** — ai được cấp thì omp của họ chỉ cần trỏ `base_url + key` là dùng được, **hết cảnh mỗi người OAuth local**. Kèm admin UI để xem "bao nhiêu account đang xài", cấp/thu quyền, và theo dõi usage/cost per-member.

Done toàn bộ khi: (1) CLIProxyAPI chạy hosted giữ token provider server-side; (2) Encore control-plane cấp/thu key per-member + forward request có kiểm quyền/quota; (3) admin UI Next.js quản member + usage; (4) deploy thật (Encore Cloud + Vercel + domain) và omp 1 member chạy qua được đầu-cuối.

## Core Value

1 điểm router AI hosted + admin OAuth thật: cấp quyền omp cho member team, token provider nằm server-side (không plaintext trên desktop từng người).

## Cắm vào codebase (brownfield)

> Đây là **product mới**, KHÔNG nằm trong repo `su-code` (Rust CLI harness). su-code chỉ là nơi chứa planning + là harness omp mà team dùng.

- **Dùng lại (chạy nguyên, KHÔNG rewrite):** `router-for-me/CLIProxyAPI` (Go, Gin) — router + OAuth upstream + refresh token + endpoint OpenAI/Anthropic-compat + Management API. Dùng Postgres store (`internal/store/postgresstore.go`) + Home/cluster mode để hostable stateless.
- **Tham khảo template (monorepo mẫu):** `~/Projects/startup/8sync-startup/` — `deploy/mind0/backend-go` (scaffold Encore Go sạch: authHandler Bearer JWT→`{userID,tenantId,role}`, envelope `{success,message,result}`, validator, folder-per-service); `zus-backend` GATE+CORE (IAM key `ena_...`, admin, per-IP quota, key-pool); `zus-admin` (Next.js + BFF proxy — pattern admin UI); `scripts/verify-public-endpoints.sh` (security gate).
- **Module/thành phần mới:** control-plane Encore Go app (identity/RBAC/key/quota/usage/gateway) + admin UI Next.js.
- **KHÔNG đụng:** không sửa `omp` (member chỉ set provider base_url+key); không reimplement luồng OAuth provider của CLIProxyAPI; không đụng repo `su-code` source (ngoài `su-code/planning/`).

## Ràng buộc / Architecture Decisions

- **Kiến trúc 2 mặt phẳng:** data plane = CLIProxyAPI (giữ account provider); control plane = Encore Go (RBAC/key/quota/usage) đứng TRƯỚC làm gateway. Member không thấy account upstream.
- **2 OAuth tách biệt:** (a) provider upstream = việc CLIProxyAPI (admin onboard 1 lần); (b) member login vào admin = việc Encore (Google Workspace SSO/email).
- **RBAC source-of-truth = control-plane Encore** (đã duyệt); CLIProxyAPI chỉ là pool account vô danh phía sau.
- Data: Postgres (token store cho CLIProxyAPI Home mode; members/grants/keys/usage/quota cho control plane).
- API: Encore authHandler Bearer key `ena_...`; forward sang CLIProxyAPI `/v1/chat/completions` bằng internal service key.
- UI: **Next.js on Vercel + BFF proxy** (KHÔNG raw-HTML native Encore — ngược pattern & ngược mục tiêu UI đẹp). Có thể nhúng CPAMC (`Cli-Proxy-API-Management-Center`) cho mặt onboarding account provider.
- Convention: theo `AGENTS.md` monorepo 8sync-startup + envelope `{success,message,result}` + `verify-public-endpoints.sh` gate.

## Key Decisions (table — append khi chốt)

| Ngày | Phase | Quyết định | Lý do |
|------|-------|-----------|-------|
| 2026-08-07 | M0 | Chạy CLIProxyAPI nguyên làm data plane, KHÔNG rewrite trong Encore | Nó đã làm OAuth+refresh+translate; rewrite là công lớn & giòn (endpoint upstream đổi liên tục) |
| 2026-08-07 | M0 | Control-plane Encore là source-of-truth RBAC | User duyệt; thu quyền = disable key ở Encore, không đụng CLIProxyAPI |
| 2026-08-07 | M0 | Admin UI = Next.js/Vercel + BFF, không native Encore HTML | Theo pattern zus-admin; mục tiêu "UI/UX đẹp chuẩn" |
| 2026-08-07 | M0 | Code target = `deploy/ai-router-hub/backend-go/` trong monorepo `8sync-startup` (CONFIRMED) | User duyệt B1; đúng nơi product server sống; promote thành submodule khi có GitHub repo; su-code chỉ giữ planning |

## Requirements

### Validated (đã ship, xác nhận giá trị)
- (chưa có)

### Active (đang làm — chi tiết ở REQUIREMENTS.md)
- [ ] UC1 Encore control-plane scaffold deploy được
- [ ] UC2 CLIProxyAPI hosted (Postgres store) + 1 provider onboarded, request thật trả completion
- [ ] UC3/UC4 Identity + per-member key + gateway forward có kiểm quyền
- [ ] UC5 Usage + quota per-member
- [ ] UC6 Admin UI Next.js
- [ ] UC7 Hardening + deploy thật
