#!/usr/bin/env bash
# north_p2p.sh — thin wrapper around south's LAN messaging server.
#
# Token loaded from $REPO_ROOT/.doe_token (gitignored).
# Server URL hardcoded; override via DOE_URL env var if it changes.
#
# Usage:
#   ./scripts/north_p2p.sh health
#   ./scripts/north_p2p.sh index
#   ./scripts/north_p2p.sh tail south_status.jsonl       # full file
#   ./scripts/north_p2p.sh tail south_to_north.jsonl 14  # tail past line 14
#   ./scripts/north_p2p.sh post north_to_south.jsonl '{"ts":"2026-05-02T18:30:00Z","from":"north","subject":"...","body":"..."}'

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
URL="${DOE_URL:-http://192.168.1.184:8765}"

if [[ ! -f "$REPO_ROOT/.doe_token" ]]; then
  echo "no token at $REPO_ROOT/.doe_token" >&2
  exit 2
fi
TOKEN="$(cat "$REPO_ROOT/.doe_token")"

cmd="${1:-}"
case "$cmd" in
  health)
    curl -s --max-time 5 -H "Authorization: Bearer $TOKEN" "$URL/health"
    echo
    ;;
  index)
    curl -s --max-time 5 -H "Authorization: Bearer $TOKEN" "$URL/index"
    echo
    ;;
  tail)
    file="${2:?file required}"
    after="${3:-}"
    if [[ -n "$after" ]]; then
      curl -s --max-time 5 -H "Authorization: Bearer $TOKEN" \
        "$URL/files/$file?after=$after"
    else
      curl -s --max-time 5 -H "Authorization: Bearer $TOKEN" \
        "$URL/files/$file"
    fi
    ;;
  post)
    file="${2:?file required (north_to_south.jsonl or north_jobs.jsonl)}"
    body="${3:?json line body required}"
    curl -s --max-time 5 -X POST \
      -H "Authorization: Bearer $TOKEN" \
      -H "Content-Type: application/x-ndjson" \
      --data "$body" \
      "$URL/files/$file"
    echo
    ;;
  *)
    echo "Usage: $0 {health|index|tail <file> [after_line]|post <file> <json_line>}" >&2
    exit 2
    ;;
esac
