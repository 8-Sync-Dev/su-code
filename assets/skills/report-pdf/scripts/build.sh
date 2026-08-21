#!/usr/bin/env bash
# Render a report-pdf HTML -> PDF via WeasyPrint (Liberation fonts via fontconfig aliases).
# Usage: build.sh <input.html> [output.pdf]
#   default output = <input basename without .html>.pdf, next to the HTML.
set -euo pipefail
in="${1:?usage: build.sh <input.html> [output.pdf]}"
[ -f "$in" ] || { echo "no such file: $in" >&2; exit 1; }
out="${2:-${in%.html}.pdf}"
uv run --with weasyprint python -m weasyprint "$in" "$out"
echo "rendered -> $out"
# Page count (defensive: a report should not silently balloon).
uv run --with pypdf python -c "from pypdf import PdfReader as R;print('pages:', len(R('$out').pages))"
