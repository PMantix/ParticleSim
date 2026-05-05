#!/usr/bin/env bash
# run_job.sh — run an eis_quick_sweep-style job in an isolated per-job
# workdir so multiple jobs can run concurrently without clobbering each
# other's eis_timeseries/ directory (which the binary writes to a
# hardcoded, CWD-relative path).
#
# Usage:
#   scripts/run_job.sh --id <job-id> --log <log-path> \
#       [--binary <name>] -- <binary args...>
#
# Behavior:
#   - Creates runs/<job-id>/ (gitignored) and runs the binary from there.
#   - Resolves the binary at <repo>/target/release/<binary>[.exe].
#   - Absolutizes --scenario (relative paths resolved against the repo
#     root). If --scenario is absent, inserts the default scenario
#     measurement_configs/eis_validation_flat_symmetric.toml.
#   - Redirects stdout+stderr to <log-path> (absolutized against repo root).
#   - Exits with the binary's exit code.
#
# After the run, time-series CSVs land in runs/<job-id>/eis_timeseries/.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

ID=""
LOG=""
BINARY="eis_quick_sweep"
PASSTHRU=()

SAW_DASHDASH=0
while [[ $# -gt 0 ]]; do
  if [[ $SAW_DASHDASH -eq 1 ]]; then
    PASSTHRU+=("$1"); shift; continue
  fi
  case "$1" in
    --id)     ID="$2"; shift 2;;
    --log)    LOG="$2"; shift 2;;
    --binary) BINARY="$2"; shift 2;;
    --)       SAW_DASHDASH=1; shift;;
    -h|--help)
      sed -n '2,20p' "$0"; exit 0;;
    *)
      echo "run_job.sh: unknown wrapper arg: $1" >&2; exit 2;;
  esac
done

if [[ -z "$ID" ]]; then echo "run_job.sh: --id required" >&2; exit 2; fi
if [[ -z "$LOG" ]]; then echo "run_job.sh: --log required" >&2; exit 2; fi

absolutize() {
  # Echoes $1 if already absolute (POSIX /... or Windows drive C:...);
  # otherwise prepends $REPO_ROOT/.
  case "$1" in
    /*|[A-Za-z]:*) printf '%s' "$1";;
    *)             printf '%s/%s' "$REPO_ROOT" "$1";;
  esac
}

LOG="$(absolutize "$LOG")"

EXE="$REPO_ROOT/target/release/${BINARY}.exe"
[[ -x "$EXE" ]] || EXE="$REPO_ROOT/target/release/${BINARY}"
if [[ ! -x "$EXE" ]]; then
  echo "run_job.sh: binary not found at target/release/${BINARY}[.exe]; attempting auto-build..." >&2
  mkdir -p "$(dirname "${LOG}")"
  (cd "$REPO_ROOT" && cargo build --release --bin "$BINARY") >>"$LOG" 2>&1
  [[ -x "$EXE" ]] || EXE="$REPO_ROOT/target/release/${BINARY}.exe"
  if [[ ! -x "$EXE" ]]; then
    echo "run_job.sh: auto-build failed for $BINARY" >&2
    exit 3
  fi
fi

SCENARIO_DEFAULT="$REPO_ROOT/measurement_configs/eis_validation_flat_symmetric.toml"
ABS_ARGS=()
HAS_SCENARIO=0
i=0
while [[ $i -lt ${#PASSTHRU[@]} ]]; do
  arg="${PASSTHRU[$i]}"
  if [[ "$arg" == "--scenario" ]]; then
    HAS_SCENARIO=1
    if [[ $((i+1)) -ge ${#PASSTHRU[@]} ]]; then
      echo "run_job.sh: --scenario requires a value" >&2; exit 2
    fi
    val="${PASSTHRU[$((i+1))]}"
    ABS_ARGS+=("--scenario" "$(absolutize "$val")")
    i=$((i+2))
  else
    ABS_ARGS+=("$arg")
    i=$((i+1))
  fi
done
if [[ $HAS_SCENARIO -eq 0 ]]; then
  ABS_ARGS+=("--scenario" "$SCENARIO_DEFAULT")
fi

WORKDIR="$REPO_ROOT/runs/$ID"
mkdir -p "$WORKDIR" "$(dirname "$LOG")"

{
  echo "[run_job] id=$ID"
  echo "[run_job] workdir=$WORKDIR"
  echo "[run_job] exe=$EXE"
  echo "[run_job] log=$LOG"
  echo "[run_job] args: ${ABS_ARGS[*]}"
  echo "[run_job] start=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} | tee -a "$LOG"

cd "$WORKDIR"
"$EXE" "${ABS_ARGS[@]}" >>"$LOG" 2>&1
RC=$?
echo "[run_job] end=$(date -u +%Y-%m-%dT%H:%M:%SZ) rc=$RC" | tee -a "$LOG"
exit $RC
