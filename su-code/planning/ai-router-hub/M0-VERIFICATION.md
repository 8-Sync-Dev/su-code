# M0-VERIFICATION — Foundation

> Nghiệm thu AC-01..05 (M0-CONTEXT). Verify chạy trong podman `golang:1.24` (Go-level); encore-native (`encore run/test`) defer sang deploy box có Docker — ghi NEEDS-CONFIRM, KHÔNG claim full-pass.

**Ngày:** 2026-08-07 · **Engine:** 5/5 task done qua verify-gate.

## AC matrix

| AC | UC | Cách verify | Kết quả | Bằng chứng |
|----|----|-------------|---------|------------|
| AC-01 | UC1 | `go vet ./...` + `go build ./...` | ✅ PASS (Go-level) | podman golang:1.24 → `VET_OK` `BUILD_OK`; `encore.dev v1.57.13` resolve OK |
| AC-02 | UC1 | `/health` public trả envelope `{success:true,...}` | ✅ PASS (Go-level) | compiles + `go test` `TestOk`/`TestFail` PASS. Wiring HTTP thật = encore-native (dưới) |
| AC-03 | UC1 | authHandler: thiếu/sai Bearer → `errs.Unauthenticated`; token đúng → AuthData | ✅ PASS (Go-level) | `go test` `TestAuthHandler` 3 case (empty/invalid/valid) PASS |
| AC-04 | UC1 | `verify-public-endpoints.sh` exit 0, chỉ endpoint allowlist là public | ✅ PASS | script exit 0; chỉ `/health` public (`/whoami` auth bị loại đúng) |
| AC-05 | UC1 | dir tồn tại + path thật vào PROJECT/STATE, gỡ B1 | ✅ PASS | `deploy/ai-router-hub/backend-go/` tồn tại; PROJECT Key-Decision + STATE cập nhật CONFIRMED |

**Kết luận Go-level: 5/5 AC PASS.**

## ⚠ NEEDS-CONFIRM — encore-native gate (chạy ở máy có Docker của user)

Podman rootless ở máy này KHÔNG chạy được `encore run/build/test` (cần Docker daemon). Các mục sau CHƯA chứng minh, phải xác nhận khi deploy:

1. `encore run` khởi động sạch + `//encore:api` + `//encore:authhandler` register đúng (health reachable, whoami đòi Bearer).
2. `encore test ./...` (test-harness có runtime — khác `go test` thuần đã pass ở đây).
3. **Risk #1:** `Response.Result interface{}` (envelope.go) — Encore schema parser CÓ THỂ từ chối bare `interface{}` trong API type (lỗi `go build` không bắt được, `encore run` mới bắt). Nếu vậy: đổi sang type cụ thể / `json.RawMessage` (fix 1 dòng). Kiểm TRƯỚC TIÊN khi `encore run`.
4. Deploy Encore Cloud (B3 credentials) — hard blocker, ngoài M0.

## Review (independent pass 2026-08-07)

READY (Go-level). Không defect thật ngoài scope M0. `test-token` cứng trong auth.go = placeholder M0 có ghi chú (key lookup thật = M2), không tính defect. Risk #1 ở trên là mục theo dõi cho gate encore-native.
