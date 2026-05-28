#!/usr/bin/env sh
# index-fresh.sh — initialize codegraph in the current directory and run a full
# index. Idempotent: re-running on an already-indexed project just rebuilds the
# index. Safe to invoke from AI tools / hooks / session start.
set -eu
if [ ! -d .codegraph ]; then
  codegraph init .
fi
codegraph index .
