# ai-router-hub — control plane (Encore Go)

Control plane cho AI Router Hub: RBAC / per-member key / quota / usage.
Data plane (OAuth provider accounts, translate, round-robin) = **CLIProxyAPI** chạy riêng (M1).

## Trạng thái: M0 Foundation (scaffold)

- `gate/` — service duy nhất hiện tại: `GET /health` (public), `GET /whoami` (auth), authHandler Bearer, envelope `{success,message,result}`.
- `scripts/verify-public-endpoints.sh` — CI gate: liệt kê endpoint `public` và soát allowlist.

## Chạy (cần Docker — máy deploy)

```sh
encore run          # local dev, http://localhost:4000
encore test ./...   # test-suite (compile + wiring check)
```

Ở máy chỉ có podman rootless: verify Go-level bằng `go vet ./... && go build ./...` (không chứng minh Encore wiring — đó là `encore run/test`).

## Convention (từ mind0/zus)

- Envelope thống nhất `{success:bool, message:string, result:any}`.
- Folder-per-service (mỗi service = 1 package dưới root).
- authHandler Bearer → `AuthData{MemberID, Role}`.
