#!/usr/bin/env bash
# 8sync size-gate — fail a release asset that exceeds the declared ceiling.
#
#   bash scripts/size-gate.sh <path-to-binary>
#   CEILING=4194304 bash scripts/size-gate.sh <path>   # override for a check
#
# Why this exists: `AGENTS.md` §8 carried a "< 4 MB stripped" budget that nothing
# enforced, and the binary quietly drifted to 6 407 848 B — 52 % over — before an
# audit noticed. A comment is not a budget; this is.
#
# The ceiling sits ABOVE today's size on purpose. A gate that is already red gets
# ignored; this one only fires on NEW growth. Lower it whenever `size-report.sh`
# shows real headroom — that is the ratchet.

set -euo pipefail
export LC_ALL=C

CEILING="${CEILING:-5242880}"   # 5 MiB. Goal is 4 MiB (4194304) — see AGENTS.md §8.
GOAL=4194304

asset="${1:-}"
[ -n "$asset" ] || { echo "usage: size-gate.sh <binary>" >&2; exit 2; }
[ -f "$asset" ] || { echo "size-gate: no such file: $asset" >&2; exit 2; }

# stat(1) differs between GNU and BSD/macOS; Windows runners use the bash shim.
size=$(stat -c%s "$asset" 2>/dev/null || stat -f%z "$asset")

pct() { awk -v a="$1" -v b="$2" 'BEGIN{printf "%+.2f%%", (a-b)*100/b}'; }

printf 'asset   %s\n' "$asset"
printf 'size    %d bytes\n' "$size"
printf 'ceiling %d bytes (%s)\n' "$CEILING" "$(pct "$size" "$CEILING")"
printf 'goal    %d bytes (%s)\n' "$GOAL" "$(pct "$size" "$GOAL")"

if [ "$size" -gt "$CEILING" ]; then
  echo "::error::size gate FAILED — $asset is $size B, over the $CEILING B ceiling by $((size - CEILING)) B."
  echo "Attribute it with \`bash scripts/size-report.sh\` before raising the ceiling."
  exit 1
fi

if [ "$size" -gt "$GOAL" ]; then
  echo "::warning::over the $GOAL B goal by $((size - GOAL)) B (under the ceiling, so not fatal)."
fi

echo "size gate OK"
