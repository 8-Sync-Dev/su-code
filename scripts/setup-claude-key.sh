#!/usr/bin/env bash
# Auto-setup Claude Code settings & omp Anthropic models from JSON.
set -euo pipefail

INPUT="${1:-}"
if [ -z "$INPUT" ]; then
    if [ ! -t 0 ]; then
        INPUT=$(cat)
    else
        echo "Please paste your Claude Code settings JSON (Ctrl+D to finish):"
        INPUT=$(cat)
    fi
fi

if [ -f "$INPUT" ]; then
    INPUT=$(cat "$INPUT")
fi

if command -v 8sync >/dev/null 2>&1; then
    8sync harness claude-code "$INPUT"
else
    # Fallback to python / jq to write ~/.claude/settings.json
    CLAUDE_DIR="$HOME/.claude"
    mkdir -p "$CLAUDE_DIR"
    echo "$INPUT" > "$CLAUDE_DIR/settings.json"
    echo "✓ Wrote $CLAUDE_DIR/settings.json"
fi
