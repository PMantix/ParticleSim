"""South polling controller.

Watches coordination/north_jobs.jsonl for new jobs and runs them via
scripts/_south_run.sh (which itself wraps scripts/run_job.sh and writes
a .meta file with EXIT/WALL on completion). Reports claims and
completions via append-only writes to coordination/south_status.jsonl,
following the rules in coordination/PROTOCOL.md.

Single-instance, K=1 (claims at most one new job per poll). Existing
"running" rows for jobs this controller didn't start are still picked
up via their .meta files when they complete, so the controller is the
single point of authority for status updates regardless of who launched
the job.

Stops on:
  - a STOP record in north_jobs.jsonl,
  - a coordination/SOUTH_STOP file appearing,
  - SIGINT.

Usage on Windows:
  py scripts/south_controller.py

Run in the background and tail the controller log:
  bash -c "py scripts/south_controller.py > south_controller.log 2>&1 &"
"""

from __future__ import annotations

import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
NORTH = REPO / "coordination" / "north_jobs.jsonl"
SOUTH = REPO / "coordination" / "south_status.jsonl"
INBOX = REPO / "coordination" / "north_to_south.jsonl"
STOP_FLAG = REPO / "coordination" / "SOUTH_STOP"
META_DIR = REPO / "doe_results" / "eis_doe_lf"

POLL_S = 15
RAYON_THREADS_DEFAULT = 4
MAX_CONCURRENT = 12
HOST = os.environ.get("COMPUTERNAME") or socket.gethostname()
BRANCH = "feature/eis-amplitude-study"


def now_local() -> str:
    """ISO 8601 with explicit local timezone offset (e.g. -04:00 in EDT,
    -05:00 in EST). Each timestamp carries its own offset so DST boundaries
    don't introduce ambiguity, and either side of the protocol can write in
    UTC ('Z') or local ('±HH:MM') and they remain mutually parseable."""
    return datetime.now().astimezone().isoformat(timespec="seconds")


def log(msg: str) -> None:
    print(f"[ctrl {now_local()}] {msg}", flush=True)


def read_jsonl(path: Path) -> list[dict]:
    if not path.exists():
        return []
    out: list[dict] = []
    for ln in path.read_text(encoding="utf-8").splitlines():
        ln = ln.strip()
        if not ln:
            continue
        try:
            out.append(json.loads(ln))
        except json.JSONDecodeError as e:
            log(f"warning: skipping malformed line in {path.name}: {e}")
    return out


def latest_status_for(rows: list[dict], id_: str) -> dict | None:
    last = None
    for r in rows:
        if r.get("id") == id_:
            last = r
    return last


def append_status(row: dict) -> None:
    SOUTH.parent.mkdir(parents=True, exist_ok=True)
    with SOUTH.open("a", encoding="utf-8") as f:
        f.write(json.dumps(row) + "\n")


def run_git(args: list[str], check: bool = False) -> subprocess.CompletedProcess:
    r = subprocess.run(
        ["git", *args], cwd=str(REPO), capture_output=True, text=True
    )
    if check and r.returncode != 0:
        log(f"git {' '.join(args)} -> rc={r.returncode}: {r.stderr.strip()}")
    return r


def commit_pending_http_writes() -> None:
    """The messaging server appends to north_jobs.jsonl /
    north_to_south.jsonl on POST without committing. Those unstaged
    changes block subsequent `git pull --rebase`, so the controller
    commits them on each iteration before pulling."""
    r = run_git(["status", "--porcelain", "--", "coordination/"], check=False)
    if r.returncode != 0 or not r.stdout.strip():
        return
    pending: list[str] = []
    for ln in r.stdout.splitlines():
        if not ln or len(ln) < 4:
            continue
        code = ln[:2]
        path = ln[3:].strip().strip('"')
        # ' M' = modified tracked file. We only auto-commit appends to the
        # two North-owned files; anything else is unexpected and we leave it.
        if code.strip() == "M" and path in (
            "coordination/north_jobs.jsonl",
            "coordination/north_to_south.jsonl",
        ):
            pending.append(path)
    if not pending:
        return
    log(f"auto-commit HTTP writes: {', '.join(pending)}")
    for p in pending:
        run_git(["add", "--", p], check=False)
    r = run_git(
        ["commit", "-m", f"[DOE] HTTP channel: append via messaging_server ({', '.join(pending)})"],
        check=False,
    )
    if r.returncode != 0 and "nothing to commit" not in (r.stdout + r.stderr):
        log(f"auto-commit failed: {r.stdout.strip()} {r.stderr.strip()}")


def git_pull_rebase() -> None:
    commit_pending_http_writes()
    r = run_git(["pull", "--rebase"], check=False)
    if r.returncode != 0:
        msg = r.stderr.strip()
        # Stale "unstaged changes" race during my own edits is expected and
        # noisy. Ignore that one specific case; surface anything else.
        if "You have unstaged changes" in msg:
            return
        log(f"pull --rebase failed: {msg}")


def git_commit_push(msg: str, paths: list[str]) -> None:
    for p in paths:
        if p:
            run_git(["add", "--", p], check=False)
    r = run_git(["commit", "-m", msg], check=False)
    if r.returncode != 0 and "nothing to commit" not in (r.stdout + r.stderr):
        log(f"commit failed: {r.stdout.strip()} {r.stderr.strip()}")
        return
    git_pull_rebase()
    r = run_git(["push", "origin", BRANCH], check=False)
    if r.returncode != 0:
        log(f"push failed: {r.stderr.strip()}")


def parse_meta(path: Path) -> dict | None:
    if not path.exists():
        return None
    out: dict[str, str] = {}
    for ln in path.read_text(encoding="utf-8").splitlines():
        if "=" in ln:
            k, v = ln.split("=", 1)
            out[k.strip()] = v.strip()
    if "EXIT" not in out:
        return None
    return out


def process_completions(jobs: list[dict], statuses: list[dict]) -> int:
    handled = 0
    for j in jobs:
        id_ = j.get("id")
        if not id_ or id_ == "STOP":
            continue
        last = latest_status_for(statuses, id_)
        if not last or last.get("status") != "running":
            continue
        meta_path = META_DIR / f"{id_}.meta"
        meta = parse_meta(meta_path)
        if meta is None:
            continue
        exit_code = int(meta["EXIT"])
        wall = int(meta.get("WALL", "0") or 0)
        log_path = j.get("log", "")
        status = "done" if exit_code == 0 else "failed"
        row: dict = {
            "id": id_,
            "ts": now_local(),
            "status": status,
            "exit_code": exit_code,
            "log": log_path,
            "wall_seconds": wall,
        }
        if exit_code != 0:
            row["error"] = f"binary exited with code {exit_code}"
        append_status(row)
        try:
            meta_path.rename(meta_path.with_suffix(".meta.done"))
        except OSError as e:
            log(f"could not rename {meta_path.name}: {e}")
        log(f"completion {id_}: exit={exit_code} wall={wall}s")
        # For binaries that produce sibling output files alongside the log
        # (e.g. dcr_pulse_sweep: dense_series.csv, pulse_summary.csv,
        # summary.txt), also stage the log's parent directory so all
        # outputs get committed. eis_quick_sweep's siblings live in
        # gitignored eis_timeseries/ so the parent stage is harmless there.
        paths_to_stage = [
            str(SOUTH.relative_to(REPO)).replace("\\", "/"),
            log_path,
        ]
        if log_path:
            log_parent = str(Path(log_path).parent).replace("\\", "/")
            if log_parent and log_parent != "." and log_parent not in paths_to_stage:
                paths_to_stage.append(log_parent)
        git_commit_push(
            f"[DOE] finish {id_} (exit={exit_code}, wall={wall}s)",
            paths_to_stage,
        )
        handled += 1
    return handled


def next_unclaimed(jobs: list[dict], statuses: list[dict]) -> dict | None:
    seen = {r.get("id") for r in statuses}
    for j in jobs:
        if j.get("id") not in seen:
            return j
    return None


def process_inbox(seen_count: int) -> int:
    rows = read_jsonl(INBOX)
    for r in rows[seen_count:]:
        ts = r.get("ts", "?")
        subj = r.get("subject", "(no subject)")
        ref = r.get("ref")
        suffix = f" (ref={ref})" if ref else ""
        log(f"north->south msg [{ts}] {subj}{suffix}")
        body = (r.get("body") or "").strip()
        if body:
            for ln in body.splitlines():
                log(f"  | {ln}")
    return len(rows)


def count_running(jobs: list[dict], statuses: list[dict]) -> int:
    n = 0
    for j in jobs:
        id_ = j.get("id")
        if not id_ or id_ == "STOP":
            continue
        last = latest_status_for(statuses, id_)
        if last and last.get("status") == "running":
            n += 1
    return n


def claim_and_launch(j: dict) -> None:
    id_ = j["id"]
    binary = j.get("binary", "eis_quick_sweep")
    log_path = j.get("log", f"doe_results/eis_doe_lf/{id_}.log")
    args = j.get("args", []) or []

    rayon = RAYON_THREADS_DEFAULT
    note = f"south_controller.py: scripts/run_job.sh, RAYON_NUM_THREADS={rayon}"
    row = {
        "id": id_,
        "ts": now_local(),
        "status": "running",
        "host": HOST,
        "note": note,
    }
    append_status(row)
    log(f"claim {id_}; pushing running row")
    git_commit_push(
        f"[DOE] start {id_}",
        [str(SOUTH.relative_to(REPO)).replace("\\", "/")],
    )

    bash = shutil.which("bash")
    if bash is None:
        log("ERROR: bash not on PATH; cannot launch job")
        return

    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(rayon)

    creationflags = 0
    if sys.platform == "win32":
        creationflags = 0x00000008  # DETACHED_PROCESS

    cmd = [
        bash,
        "scripts/_south_run.sh",
        id_,
        log_path,
        binary,
        "--",
        *args,
    ]
    log(f"launching {id_}: {' '.join(args)}")
    subprocess.Popen(
        cmd,
        cwd=str(REPO),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=env,
        creationflags=creationflags,
        close_fds=True,
    )


def stopping_via_flag() -> bool:
    if STOP_FLAG.exists():
        log("SOUTH_STOP flag present, exiting")
        try:
            STOP_FLAG.unlink()
        except OSError:
            pass
        return True
    return False


def ack_stop_record() -> None:
    row = {
        "id": "STOP",
        "ts": now_local(),
        "status": "done",
        "host": HOST,
        "note": "south_controller.py acked STOP record, exiting",
    }
    append_status(row)
    git_commit_push(
        "[DOE] South: stop ack",
        [str(SOUTH.relative_to(REPO)).replace("\\", "/")],
    )


def main() -> int:
    # Force UTF-8 stdout/stderr with errors='replace' so messages from North
    # (which may contain non-cp1252 characters like arrows or Greek letters)
    # don't crash the controller on Windows. cp1252 is the default Windows
    # console codepage and can't encode e.g. U+2192.
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass

    log(
        f"controller starting (poll={POLL_S}s, max_concurrent={MAX_CONCURRENT}, "
        f"rayon_threads={RAYON_THREADS_DEFAULT}, host={HOST}, branch={BRANCH})"
    )

    inbox_seen = 0  # surface any pre-existing messages on first iteration

    def handle_sigint(signum, frame):
        log("SIGINT received, exiting")
        sys.exit(0)

    try:
        signal.signal(signal.SIGINT, handle_sigint)
    except (ValueError, AttributeError):
        pass

    while True:
        if stopping_via_flag():
            break

        git_pull_rebase()
        inbox_seen = process_inbox(inbox_seen)
        jobs = read_jsonl(NORTH)
        statuses = read_jsonl(SOUTH)
        process_completions(jobs, statuses)

        # Re-read after possible commits
        jobs = read_jsonl(NORTH)
        statuses = read_jsonl(SOUTH)

        nxt = next_unclaimed(jobs, statuses)
        if nxt is None:
            log("idle (no new jobs)")
        elif nxt.get("id") == "STOP":
            log("STOP record encountered, exiting")
            ack_stop_record()
            break
        else:
            running = count_running(jobs, statuses)
            if running >= MAX_CONCURRENT:
                log(
                    f"idle (concurrency cap: {running}/{MAX_CONCURRENT} running, "
                    f"holding {nxt.get('id')})"
                )
            else:
                claim_and_launch(nxt)

        time.sleep(POLL_S)

    log("controller exited")
    return 0


if __name__ == "__main__":
    sys.exit(main())
