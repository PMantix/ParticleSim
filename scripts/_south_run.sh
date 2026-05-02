#!/usr/bin/env bash
# _south_run.sh — internal helper used by scripts/south_controller.py.
#
# Runs scripts/run_job.sh and, regardless of outcome, writes
# doe_results/eis_doe_lf/<id>.meta with EXIT/WALL/START/END so the
# controller can detect completion on its next poll. This must be done by
# the subprocess (not the controller) so completions are recorded even if
# the controller has been restarted.
#
# Usage:
#   bash scripts/_south_run.sh <id> <log_path> <binary> -- <binary args...>
set -uo pipefail

ID="$1"
LOG="$2"
BIN="$3"
shift 3
if [[ "${1:-}" == "--" ]]; then shift; fi

START=$(date +%s)
bash scripts/run_job.sh --id "$ID" --log "$LOG" --binary "$BIN" -- "$@"
EXIT=$?
END=$(date +%s)
WALL=$((END-START))

mkdir -p doe_results/eis_doe_lf
printf "EXIT=%d\nWALL=%d\nSTART=%d\nEND=%d\n" "$EXIT" "$WALL" "$START" "$END" \
  > "doe_results/eis_doe_lf/${ID}.meta"
