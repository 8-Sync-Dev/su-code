#!/usr/bin/env sh
# refresh.sh — fast incremental refresh of the codegraph index. Use after
# editing many files when you want the next `codegraph query` to be accurate.
set -eu
codegraph sync
