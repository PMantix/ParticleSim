# Two-machine Claude coordination — DOE protocol

Two Claude Code sessions are collaborating on EIS DOE work for ParticleSim:

- **North** — running on `phillipaquino`'s Mac (`192.168.1.X`). Owns analysis,
  job design, and result interpretation.
- **South** — running on the user's Windows machine (`192.168.1.184`).
  Owns running cargo binaries and pushing back results.

There is no direct IPC between the two sessions. They coordinate via this
git branch (`feature/eis-amplitude-study` on `origin`). Each side polls
periodically; the user is the wakeup signal.

## Files and ownership

| File | Owner (writes) | Reader |
|---|---|---|
| `coordination/north_jobs.jsonl` | **North** appends | South reads |
| `coordination/south_status.jsonl` | **South** appends | North reads |
| `doe_results/eis_doe_lf/<job_id>.log` | **South** writes | North reads |
| `coordination/PROTOCOL.md` (this file) | Either edits as needed | Both read |

Each side **only writes its own file**. This avoids merge conflicts —
both sides can `git pull --rebase` without the other's commits ever
touching their own state file.

## File formats

Both `.jsonl` files are append-only JSON Lines (one JSON object per line).

### `coordination/north_jobs.jsonl`

```json
{"id": "doe-001", "ts": "2026-05-02T08:30:00Z", "binary": "eis_quick_sweep", "args": ["--amplitude", "0.6", "--f-min", "5e-7", "--f-max", "5e-6"], "expected_min": 1000, "log": "doe_results/eis_doe_lf/doe-001.log", "note": "ultra-LF at locked amplitude"}
```

Fields:
- `id` — unique job identifier (string, used as filename stem). Treat
  as opaque; never re-use.
- `ts` — ISO-8601 UTC time North enqueued the job.
- `binary` — cargo binary name (e.g. `eis_quick_sweep`,
  `verify_galvanostatic_amplitude`).
- `args` — array of CLI args passed to the binary verbatim.
- `expected_min` — North's wall-time estimate in minutes (informational only).
- `log` — relative path South should write its log to.
- `note` — free-text rationale (informational, no automation depends on it).

Special control records:
- `{"id": "STOP", "ts": "...", "note": "DOE complete"}` — terminator.
  South records a final status line, stops polling, and reports back.

### `coordination/south_status.jsonl`

South appends one line when a job starts, and one line when it ends:

```json
{"id": "doe-001", "ts": "2026-05-02T08:31:00Z", "status": "running", "host": "DESKTOP-F1RA637"}
{"id": "doe-001", "ts": "2026-05-02T19:45:00Z", "status": "done", "exit_code": 0, "log": "doe_results/eis_doe_lf/doe-001.log", "wall_seconds": 40460}
```

`status` ∈ `{queued, running, done, failed, skipped}`. `failed` rows
include a short `error` string. `skipped` is for jobs South can't run
(e.g., binary not built, scenario file missing) — include `reason`.

## North's loop (analysis side)

```bash
# Pull latest
git pull --rebase

# Append a new job
cat >> coordination/north_jobs.jsonl <<'EOF'
{"id": "doe-001", ...}
EOF

# Commit and push
git add coordination/north_jobs.jsonl
git commit -m "[DOE] queue doe-001"
git push origin feature/eis-amplitude-study

# Later, check for results
git pull --rebase
tail -20 coordination/south_status.jsonl
# Read any new doe_results/eis_doe_lf/*.log that landed
# Analyze, plot, decide next jobs
```

North polls roughly **every 10–30 minutes** (or whenever the user prompts).

## South's loop (execution side)

This is the prescribed work loop. Run it as a single bash function or
follow the steps interactively.

```bash
# 1. Pull the latest queue
git pull --rebase origin feature/eis-amplitude-study

# 2. Find the next unprocessed job:
#    - Iterate north_jobs.jsonl in order
#    - For each entry, check if its `id` already appears in south_status.jsonl
#      with status ∈ {done, failed, skipped} OR the most recent status row
#      for that id is `running` (someone — probably you — is already on it)
#    - The first id with no terminal status is the next job
#    - If `id == "STOP"`, append a final status line and exit the loop

# 3. Mark started
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo '{"id":"<id>","ts":"'$NOW'","status":"running","host":"'$(hostname)'"}' \
  >> coordination/south_status.jsonl
git add coordination/south_status.jsonl
git commit -m "[DOE] start <id>"
git push

# 4. Build (skip if already built recently) and run
cargo build --release --bin <binary>
START=$(date +%s)
cargo run --release --bin <binary> -- <args...> > <log_path> 2>&1
EXIT=$?
END=$(date +%s)
WALL=$((END-START))

# 5. Record result + push
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo '{"id":"<id>","ts":"'$NOW'","status":"done","exit_code":'$EXIT',"log":"<log_path>","wall_seconds":'$WALL'}' \
  >> coordination/south_status.jsonl
git add coordination/south_status.jsonl <log_path>
git commit -m "[DOE] finish <id> (exit=<EXIT>, wall=${WALL}s)"
git push

# 6. Loop back to step 1
```

If a `git push` rejects with non-fast-forward (North pushed in the meantime):

```bash
git pull --rebase origin feature/eis-amplitude-study
git push
```

The append-only design means rebases never produce conflicts — each
file's commits only ever come from one side.

South polls roughly **every 5 minutes** when idle (no jobs queued).
While jobs are running, no polling needed — finish them, then poll once.

## Fan-out (running multiple jobs in parallel)

South's hardware (Ryzen 9 7950X3D, 16C/32T, 128 GB RAM) can run several
cargo binaries concurrently. The validation scenario has ~3500 bodies;
Rayon doesn't scale linearly past ~4 cores at that size, so the sweet
spot is **~4 concurrent jobs × ~4 Rayon threads each** (≈ 16 cores
utilized). Adjust based on observed CPU saturation.

### Required setup for parallel jobs

Each cargo process **must** be given a unique `EIS_TS_DIR` env var,
otherwise their `eis_timeseries/eis_ts_*.csv` outputs collide and
overwrite each other. The binary reads `EIS_TS_DIR` (added in commit
adding the fan-out support) and falls back to `eis_timeseries/` if
unset. Recommended pattern:

```bash
# in the wrapper, per parallel job:
JOB_ID=doe-002
export EIS_TS_DIR="eis_timeseries_${JOB_ID}"
export RAYON_NUM_THREADS=4   # cap per-job parallelism
cargo run --release --bin eis_quick_sweep -- <args> > <log_path> 2>&1
```

After the run, the per-job timeseries CSVs in
`eis_timeseries_doe-002/` are job-specific. They can be committed
along with the run log if North wants them, or deleted to save space.

### Parallel claim semantics

To avoid two parallel processes claiming the same job, claims must be
serialized via a single point of commit. Recommended structure:

1. **Single controller process** on South pulls the queue.
2. Controller selects the next K un-claimed jobs (where K is the desired
   fan-out, e.g. 4).
3. Controller appends K `running` records to `south_status.jsonl` in a
   single commit, pushes immediately.
4. Controller spawns K cargo subprocesses with unique `EIS_TS_DIR`s.
5. As each subprocess finishes, controller appends its `done`/`failed`
   record to `south_status.jsonl` (single-writer, so file-append races
   are not an issue) and `git add`s the corresponding log file.
6. When all K finish (or after a debounce timer to batch commits),
   controller commits + pushes.
7. Controller pulls the queue and starts the next batch of K.

Don't have multiple parallel jobs each call `git commit` — only the
controller commits.

### Recommended fan-out by job size

For mixed-size queues, prefer running same-cost jobs together so the
batch finishes around the same wall-time:

| Job decade (f_min) | Wall (per job, ~80 steps/s) | Suggested concurrency |
|---|---|---|
| 5e-4 to 5e-3 | ~2 min | 8 (small jobs, low Rayon needs) |
| 5e-5 to 5e-4 | ~12 min | 4 |
| 5e-6 to 5e-5 | ~100 min | 4 |
| 5e-7 to 5e-6 | ~16 hr | 2-4 (long jobs, high Rayon benefit) |

These numbers are from Mac perf; expect Windows Ryzen to be similar or
slightly faster per-step. Update with empirical data as it lands.

## Parallel jobs (South wrapper)

`eis_quick_sweep` writes time-series CSVs to a hardcoded, CWD-relative
path (`eis_timeseries/`). Two concurrent runs in the same CWD would
overwrite each other's files. To run jobs in parallel, South uses
`scripts/run_job.sh`, which gives each job an isolated workdir at
`runs/<job-id>/` (gitignored).

Wrapper usage (replaces the `cargo run` step in South's loop):

```bash
scripts/run_job.sh \
  --id <job-id> \
  --log <log-path> \
  --binary eis_quick_sweep \
  -- <binary args verbatim from north_jobs.jsonl>
```

The wrapper:
- Resolves the prebuilt binary at `target/release/<binary>[.exe]` (run
  `cargo build --release --bin <binary>` first).
- Absolutizes `--scenario` against the repo root, inserting the default
  `measurement_configs/eis_validation_flat_symmetric.toml` if absent.
- `cd`s into `runs/<job-id>/` before exec, so `eis_timeseries/` lands
  inside the workdir.
- Tees a small `[run_job]` header (id, paths, start/end timestamps,
  rc) into the log.

With this in place, North can queue jobs that are safe to run
concurrently. Suggested ceiling on the current Windows host (Ryzen 9
7950X3D, 16C/32T, 128 GB, NVMe): **2–4 concurrent jobs**, possibly more
with diminishing returns since the sim is multi-threaded internally.
South decides serial vs. fan-out based on queue depth and the
concurrency ceiling, and reports concurrency choices via the `note`
field on the relevant `south_status.jsonl` rows.

## Pre-flight checks (one-time, both sides)

- `git config user.email` returns something. If not, set it.
- `cargo build --release` succeeds.
- `git pull --rebase origin feature/eis-amplitude-study` succeeds clean.

## Job design

North enqueues jobs based on what's been measured and what gaps remain.
For Phase 1.2 / 1.3 EIS work, the dimensions of interest:

- **Amplitude**: `0.1, 0.3, 0.6, 1.0, 3.0` e/fs (Galvanostatic).
- **Frequency band**: spec range is `[5e-7, 5e-3]` /fs. Each decade
  costs ~10× the wall time of the next-higher decade, so prefer
  one-decade-at-a-time jobs.
- **Periods per freq**: usually 4 (default). Setting >4 has been shown
  to *not* improve R²(V) in this scenario (cycle 3c, 3e).
- **Settle periods**: 4 (default).

Output goes to `doe_results/eis_doe_lf/<job_id>.log`. North harvests
the EisPoints table from the log via `python3 scripts/plot_eis_summary.py
<log_path>`.

## Termination

When North is satisfied with coverage, it appends a STOP record. South
records the stop and exits its loop. The user can squash/clean
the coordination commits later before merging the feature branch.

## Etiquette

- **No force-pushes** on this branch from either side — would clobber
  the other's commits.
- **Commits should be small and frequent** so the other side's polls
  surface progress quickly.
- **Don't edit the other side's `.jsonl`** — even to "fix" a typo. Add a
  new record on your own side instead.
- **If you see a "running" status for >2× expected_min**, the runner
  may have crashed or the user killed it. Add a `failed` status with
  `error: "stale running record, presumed crashed"` and re-enqueue with
  a new id.
