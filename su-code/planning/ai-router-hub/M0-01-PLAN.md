# M0-01-PLAN — Foundation (Encore Go control-plane scaffold)

> Chuẩn: `M0-CONTEXT.md` (Goal + AC-01..05). Mỗi task ↔ ≥1 AC ↔ ≥1 UC. Engine mapping: task → engine task; `verify` = cột "Cách verify" của AC.
> Location (B1 chốt): `~/Projects/startup/8sync-startup/deploy/ai-router-hub/backend-go/` (dir thường trong monorepo; promote thành submodule khi có GitHub repo). Build env (B2 chốt): podman container (Encore CLI + Go trong container), deploy user tự chạy.

## Wave 1 (nền — phải xong trước)

- [ ] **T1**: Tạo skeleton app Encore Go — dir + `encore.app` + `go.mod` (module `ai-router-hub`) + `.gitignore`. `[file: deploy/ai-router-hub/backend-go/{encore.app,go.mod,.gitignore}]` `[skill: api-and-interface-design]` `[tier: must-test]` `[UC: UC1]` `[AC: AC-01, AC-05]`

## Wave 2 (cần T1 — service files, khác file nhau)

- [ ] **T2**: Service `gate` — `GET /health` (`//encore:api public`) trả envelope `{success:true,message,result}` + helper envelope (`ok()/fail()`). `[file: .../gate/health.go, .../gate/envelope.go]` `[skill: api-and-interface-design]` `[tier: must-test]` `[UC: UC1]` `[AC: AC-02]`
- [ ] **T3**: authHandler (`encore.dev/beta/auth`) — parse Bearer, thiếu/sai → `errs.Unauthenticated` (401 envelope); token test hợp lệ → resolve `AuthData{MemberID, Role}`. `[file: .../gate/auth.go]` `[skill: api-and-interface-design]` `[tier: must-test]` `[UC: UC1]` `[AC: AC-03]`

## Wave 3 (cần T1 — CI/gate)

- [ ] **T4**: Skeleton CI — script kiểu `verify-public-endpoints` (đọc `encore ... meta` liệt kê public endpoint, allowlist) + `.github/workflows` build gate. `[file: .../scripts/verify-public-endpoints.sh, .github/workflows/backend.yml]` `[skill: ci-cd-and-automation]` `[tier: verify-only]` `[UC: UC1]` `[AC: AC-04]`
- [ ] **T5**: Chốt location — tạo dir trong monorepo, ghi path thật vào PROJECT/STATE, gỡ ⚠B1. `[file: su-code/planning/ai-router-hub/{PROJECT,STATE}.md]` `[skill: —]` `[tier: verify-only]` `[UC: UC1]` `[AC: AC-05]`

## Checkpoints / Gates

- **review dimensions** (từ config): security · correctness · convention.
- **plan-review**: BỎ — M0 là scaffold 1-domain, không DB, ambiguity thấp (không đạt ngưỡng "complex": chỉ 1 app mới, wave phụ thuộc tuyến tính đơn giản). Ghi lý do ở CONTEXT Plan-review notes.
- **Acceptance**: phase done ⇔ AC-01..05 PASS (verify ở `/feature ship` → M0-VERIFICATION.md).
- **engine mapping**: T1→AC01/05, T2→AC02, T3→AC03, T4→AC04, T5→AC05. `verify`:
  - AC-01/02/03: `go vet ./...` + `go build ./...` (Go-level, chạy trong podman); FULL Encore validation = `encore build`/`encore test` trong podman (nếu Encore CLI chạy được trong container — xác định bằng probe M0-go).
  - AC-04: script gate exit 0 trên public endpoints.
  - AC-05: dir tồn tại + STATE.B1 gỡ.
- **⚠ verify boundary**: nếu Encore CLI KHÔNG chạy trong podman → AC-01/02/03 chỉ verify được ở Go-level (`go build/vet`); `encore build/test` full deferred sang môi trường deploy của user → ghi NEEDS-CONFIRM, không claim full-pass.
