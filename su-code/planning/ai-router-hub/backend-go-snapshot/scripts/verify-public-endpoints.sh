#!/usr/bin/env bash
# verify-public-endpoints.sh — CI gate: liệt kê mọi endpoint `//encore:api public`
# và soát với allowlist. Endpoint public nằm ngoài allowlist -> FAIL (chống lộ API
# không chủ đích). Grep-based nên chạy được ở mọi CI (không cần Encore daemon/Docker).
#
# Promote sang `encore ... meta` (chính xác hơn) khi build env có Encore CLI + Docker.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Allowlist: đường path public được phép (1 dòng 1 path).
ALLOWLIST=(
	"/health"
)

# Trích path từ mọi directive public trong SOURCE Go (chỉ *.go — tránh match
# comment/README/workflow). Chỉ endpoint `public`; `auth`/`private` bị loại.
mapfile -t FOUND < <(grep -rhoE --include='*.go' '//encore:api[[:space:]]+public[^\n]*path=[^[:space:]]+' "$ROOT" \
	| grep -oE 'path=[^[:space:]]+' | sed 's|path=||' | sort -u)

echo "Public endpoints tìm thấy: ${FOUND[*]:-<none>}"

rc=0
for ep in "${FOUND[@]:-}"; do
	[ -z "$ep" ] && continue
	ok=0
	for allow in "${ALLOWLIST[@]}"; do
		[ "$ep" = "$allow" ] && ok=1 && break
	done
	if [ "$ok" -ne 1 ]; then
		echo "FAIL: endpoint public '$ep' KHÔNG có trong allowlist" >&2
		rc=1
	fi
done

if [ "$rc" -eq 0 ]; then
	echo "OK: mọi endpoint public đều trong allowlist"
fi
exit "$rc"
