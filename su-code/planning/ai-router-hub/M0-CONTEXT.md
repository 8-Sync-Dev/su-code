# M0-CONTEXT — Foundation (Encore control-plane scaffold)

> Hợp đồng "tại sao + nghiệm thu" của phase M0. `/feature go` + `/feature ship` đọc file này làm chuẩn.

## 📌 Requirement scope (UC từ REQUIREMENTS.md)

| UC | Mô tả (literal) | Trong phase này làm gì | Không làm ở phase này |
|----|-----------------|------------------------|-----------------------|
| UC1 | Encore Go control-plane app deploy được: health, authHandler (Bearer), envelope, folder-per-service skeleton. | Scaffold app Encore Go (mirror `deploy/mind0/backend-go`): 1 service `gate` với health + authHandler stub resolve `{memberId,role}` từ Bearer + envelope helper; chốt repo location; skeleton CI (`verify-public-endpoints`) + deploy config. | Chưa nối CLIProxyAPI (M1); chưa có DB members/keys thật (M2); chưa UI (M4); chưa deploy production (M5). |

## 🎯 Goal

Có 1 Encore Go app biên dịch + chạy local được, expose `GET /health` trả envelope `{success:true,...}` và authHandler từ chối Bearer thiếu/sai bằng 401-envelope, cấu trúc folder-per-service + CI gate sẵn sàng để M1..M5 cắm vào — **CHƯA** nối data plane, **CHƯA** DB thật, **CHƯA** deploy production.

## ✅ Acceptance Criteria (UAT)

> Mỗi UC ⇒ ≥1 AC. AC đo được. Cột "Cách verify" = `verify` của engine task ở `/feature go`.

| AC | UC | GIVEN / WHEN / THEN (đo được) | Cách verify | Tier | Task nguồn |
|----|----|-------------------------------|-------------|------|------------|
| AC-01 | UC1 | GIVEN repo control-plane WHEN build app THEN compile sạch, 0 lỗi | `encore build` (hoặc `go build ./...` trong container nếu chưa có Encore CLI) | must-test | T1 |
| AC-02 | UC1 | GIVEN app chạy WHEN `GET /health` THEN 200 + body `{success:true,message,result}` | `encore test` / curl endpoint · unit test envelope | must-test | T2 |
| AC-03 | UC1 | GIVEN authHandler WHEN request thiếu/sai Bearer THEN 401 envelope `{success:false}`; WHEN Bearer test-token hợp lệ THEN resolve `{memberId,role}` | encore api test / unit test authHandler | must-test | T3 |
| AC-04 | UC1 | GIVEN repo WHEN kiểm tra layout THEN có folder-per-service + envelope helper + `verify-public-endpoints`-style gate script chạy pass trên public endpoints | script gate chạy exit 0 · file tồn tại | verify-only | T4 |
| AC-05 | UC1 | GIVEN quyết định location WHEN đọc PROJECT/STATE THEN repo target được ghi rõ + đã checkout/khởi tạo (không còn ⚠B1) | STATE.B1 gỡ + repo path tồn tại | verify-only | T5 |

## Decisions (D1, D2… — quyết định riêng phase, append khi chốt)

- D1: scaffold **sạch từ template Encore Go chính thức** (Encore Go), KHÔNG Encore.ts. Bản mind0/zus trong `8sync-startup` đang **empty (chưa checkout)** nên KHÔNG mirror được source — chỉ áp *convention* đã trích từ research (envelope `{success,message,result}`, folder-per-service, authHandler Bearer). Muốn mirror thật → cần checkout submodule (network/creds). (nguồn: research + disk state 2026-08-07)
- plan-review SKIP (config `complex`): M0 = scaffold 1-domain, không DB, ambiguity thấp — dưới ngưỡng "complex".
- **Probe verify-boundary (2026-08-07):** Encore CLI cài + chạy trong podman (`encore version v1.57.13`) OK, nhưng `encore build` chỉ có subcommand `docker` và `encore run/test` cần Docker daemon → KHÔNG chạy được trong podman rootless. ⇒ AC-01/02/03 verify được ở **Go-level** (`go vet ./...`, `go build ./...`) trong podman; **encore-native gate** (`encore run`/`encore test` — chứng minh `//encore:api` + authHandler wiring) phải chạy ở **môi trường có Docker** (deploy box của user). Ghi NEEDS-CONFIRM, không claim full-pass khi mới Go-level.

## Plan-review notes (điền sau Step 3.5 nếu có chạy)

- (chưa có)

---

**Phase DONE khi mọi AC PASS (ghi ở M0-VERIFICATION.md). AC FAIL → không ship.**
**⚠ Chặn go:** AC-01..03 cần Encore CLI/Go (B2) + repo location (B1, AC-05). Chưa gỡ → `/feature go` M0 dừng ở build, ghi NEEDS-CONFIRM.**
