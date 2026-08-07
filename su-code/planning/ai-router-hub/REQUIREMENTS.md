# REQUIREMENTS — AI Router Hub

> Ý ĐỊNH (làm gì + ranh giới), KHÔNG phải bản ghi đã-làm.
> UC-ID là khóa traceability: ROADMAP phase → M<x>-CONTEXT Requirement scope → AC → PLAN task → VERIFICATION.

## v1 (làm ngay)

| UC | Mô tả | Phase |
|----|-------|-------|
| UC1 | Encore Go control-plane app deploy được: health endpoint, authHandler (Bearer), envelope `{success,message,result}`, folder-per-service skeleton. | M0 |
| UC2 | CLIProxyAPI chạy hosted làm data plane với **Postgres token store + Home mode** (bền qua redeploy); onboard ≥1 account provider (vd Claude) qua Management API; 1 request thô tới endpoint OpenAI-compat trả về completion hợp lệ. | M1 |
| UC3 | Identity + RBAC: tạo member, role/grant, phát **key per-member** (`ena_...`), enable/disable (thu quyền). Đây là "đếm được bao nhiêu account, ai được cấp". | M2 |
| UC4 | Gateway: omp trỏ `base_url = Encore gateway` + member key → Encore authHandler verify key + check grant → forward sang CLIProxyAPI bằng internal key → trả completion. Key bị disable → 401/403. | M2 |
| UC5 | Usage + quota: ghi tokens/cost/requests per-member cho mỗi request; enforce quota per-member; API query usage. | M3 |
| UC6 | Admin UI (Next.js): member login (SSO/email), danh sách member + tổng số, cấp/thu quyền, xem usage/cost per-member, onboarding account provider (drive Management API của CLIProxyAPI hoặc nhúng CPAMC). | M4 |
| UC7 | Hardening + deploy: admin SSO thật, secrets management, rate-limit, deploy Encore Cloud + Vercel, gắn domain, chạy `verify-public-endpoints` gate; 1 member chạy omp qua đầu-cuối trên môi trường thật. | M5 |

## v2 (sau — không làm milestone này)

- UC-v2a: Per-key model allowlist / routing pin (giới hạn member dùng model nào).
- UC-v2b: Cost chargeback / billing per team hoặc per member.
- UC-v2c: Multi-tenant orgs (nhiều team tách biệt trên 1 instance).
- UC-v2d: Audit log export + alerting.
- UC-v2e: Dashboard analytics giàu (giống dashboard omp) tổng hợp cross-member.

## Out-of-scope (KHÔNG làm — ranh giới cứng)

> Agent KHÔNG tự ý làm các mục dưới. Vượt ranh giới = dừng, hỏi user.

- Reimplement luồng OAuth provider (Claude/Gemini/Codex PKCE + refresh) native trong Encore — dùng CLIProxyAPI as-is.
- Sửa source `omp` hoặc source `su-code` (ngoài `su-code/planning/`).
- Bán/mở API ra ngoài team (chỉ nội bộ member được cấp).
- Fork/modify CLIProxyAPI (chỉ config + gọi Management API; nếu buộc phải patch → dừng, hỏi user).
