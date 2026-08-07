# backend-go-snapshot — machine-transfer copy (KHÔNG phải canonical)

## Đây là gì
Bản sao **byte-for-byte** của scaffold M0 `ai-router-hub` (Encore Go control-plane),
verified Go-level (podman `golang:1.24`: `go vet`/`build`/`test` PASS).

**Canonical location** (nơi thật sự dùng để build/deploy):
`~/Projects/startup/8sync-startup/deploy/ai-router-hub/backend-go/`

## Vì sao có snapshot này
Monorepo `8sync-startup/.gitignore` dòng 57 = `/deploy/*` → chỉ track submodule.
Nên `deploy/ai-router-hub/backend-go/` **không transfer qua `git pull`**. Snapshot này
nằm trong repo `su-code` (được track) để code sống sót khi đổi máy.

## Restore trên máy mới
```sh
# từ repo su-code (sau git pull)
SNAP=su-code/planning/ai-router-hub/backend-go-snapshot
DST=~/Projects/startup/8sync-startup/deploy/ai-router-hub/backend-go
mkdir -p "$DST"
cp -r "$SNAP"/. "$DST"/
rm "$DST/_RESTORE.md"   # note này không thuộc scaffold

# verify lại Go-level (cần podman):
podman run --rm -v "$DST":/app:Z -v ai-router-gocache:/go -w /app \
  docker.io/library/golang:1.24 sh -c 'go mod tidy && go vet ./... && go build ./... && go test ./...'
```

## Tiếp theo sau restore
Xem `../STATE.md` (feature state) + `../M0-01-PLAN.md` + `../M0-VERIFICATION.md`.
- M0 = DONE (Go-level). M1 = BLOCKED trên B3 credentials (Postgres + provider account + CLIProxyAPI host).
- ⚠ Trên máy có Docker: chạy `encore run`/`encore test` để confirm encore-native, và kiểm **Risk #1** = `Response.Result interface{}` (Encore schema parser có thể reject — fix 1 dòng, xem M0-VERIFICATION).
- Promote thành GitHub repo/submonorepo → mới commit được vào monorepo (bỏ snapshot này).
