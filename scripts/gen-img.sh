#!/usr/bin/env bash
# Generate an AI image from a text prompt via gpt-image-2 (OpenAI / api.apikey.fun)
set -euo pipefail

PROMPT="${1:-}"
OUTPUT="${2:-./generated-image.png}"
MODEL="${MODEL:-gpt-image-2}"
SIZE="${SIZE:-1024x1024}"
ENDPOINT="${OPENAI_BASE_URL:-https://api.apikey.fun/v1}/images/generations"

if [ -z "$PROMPT" ]; then
  echo "Usage: $0 \"<prompt>\" [output.png]"
  exit 1
fi

KEY="${OPENAI_API_KEY:-}"
if [ -z "$KEY" ] && [ -f "$HOME/.omp/agent/models.yml" ]; then
  KEY=$(awk '/codex:|openai:/{flag=1; next} flag && /apiKey:/{gsub(/["'\'' ]/, "", $2); print $2; exit} flag && /^[^ ]/{flag=0}' "$HOME/.omp/agent/models.yml")
fi

if [ -z "$KEY" ]; then
  echo "Error: No API key found. Set OPENAI_API_KEY." >&2
  exit 1
fi

echo "-> Generating image with $MODEL (size: $SIZE)..."
echo "   Prompt: \"$PROMPT\""

RESP=$(curl -s -X POST "$ENDPOINT" \
  -H "Authorization: Bearer $KEY" \
  -H "x-openai-actor-authorization: apikey.fun" \
  -H "Content-Type: application/json" \
  -d "{\"model\": \"$MODEL\", \"prompt\": $(jq -n --arg p "$PROMPT" '$p'), \"size\": \"$SIZE\", \"n\": 1}")

B64=$(echo "$RESP" | jq -r '.data[0].b64_json // empty')
URL=$(echo "$RESP" | jq -r '.data[0].url // empty')

mkdir -p "$(dirname "$OUTPUT")"

if [ -n "$B64" ]; then
  echo "$B64" | base64 -d > "$OUTPUT"
  echo "✓ Saved image to: $OUTPUT"
elif [ -n "$URL" ]; then
  curl -s -o "$OUTPUT" "$URL"
  echo "✓ Downloaded image to: $OUTPUT"
else
  ERR=$(echo "$RESP" | jq -r '.error.message // "Unknown error"')
  echo "Error: $ERR" >&2
  exit 1
fi
