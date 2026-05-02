"""status_table.py - render the current South DOE state as a markdown table.

Joins coordination/north_jobs.jsonl (queue) with
coordination/south_status.jsonl (latest status per id) and prints one
row per job, sorted running-first.
"""
from __future__ import annotations

import json
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
NORTH = REPO / "coordination" / "north_jobs.jsonl"
SOUTH = REPO / "coordination" / "south_status.jsonl"

STATUS_RANK = {"running": 0, "queued": 1, "done": 2, "failed": 2, "skipped": 2}


def read_jsonl(path: Path) -> list[dict]:
    if not path.exists():
        return []
    out = []
    for ln in path.read_text(encoding="utf-8").splitlines():
        ln = ln.strip()
        if not ln:
            continue
        try:
            out.append(json.loads(ln))
        except json.JSONDecodeError:
            pass
    return out


def latest_for(rows: list[dict], id_: str) -> dict | None:
    last = None
    for r in rows:
        if r.get("id") == id_:
            last = r
    return last


def short_args(args: list[str]) -> str:
    parts = {}
    i = 0
    while i < len(args):
        a = args[i]
        if a in ("--amplitude", "--f-min", "--f-max", "--seed"):
            parts[a.lstrip("-")] = args[i + 1] if i + 1 < len(args) else ""
            i += 2
        else:
            i += 1
    out = []
    if "amplitude" in parts:
        out.append(f"A={parts['amplitude']}")
    if "f-min" in parts and "f-max" in parts:
        out.append(f"f=[{parts['f-min']},{parts['f-max']}]")
    if "seed" in parts:
        out.append(f"seed={parts['seed']}")
    return " ".join(out)


def fmt_wall(secs) -> str:
    if secs is None:
        return "-"
    s = int(secs)
    if s < 60:
        return f"{s}s"
    m, s = divmod(s, 60)
    if m < 60:
        return f"{m}m{s:02d}s"
    h, m = divmod(m, 60)
    return f"{h}h{m:02d}m"


def main() -> None:
    jobs = read_jsonl(NORTH)
    statuses = read_jsonl(SOUTH)

    rows = []
    for j in jobs:
        id_ = j.get("id")
        if not id_ or id_ == "STOP":
            continue
        last = latest_for(statuses, id_)
        status = last.get("status") if last else "queued"
        wall = last.get("wall_seconds") if last else None
        rc = last.get("exit_code") if last else None
        rows.append(
            (
                id_,
                status,
                short_args(j.get("args", []) or []),
                j.get("expected_min"),
                wall,
                rc,
            )
        )

    rows.sort(key=lambda r: (STATUS_RANK.get(r[1], 9), r[0]))

    counts: dict[str, int] = {}
    for _, status, *_ in rows:
        counts[status] = counts.get(status, 0) + 1

    print("| id | status | args | exp.min | wall | rc |")
    print("|---|---|---|---:|---:|---:|")
    for id_, status, args, exp, wall, rc in rows:
        rc_str = "-" if rc is None else str(rc)
        exp_str = "-" if exp is None else str(exp)
        print(f"| {id_} | {status} | `{args}` | {exp_str} | {fmt_wall(wall)} | {rc_str} |")

    summary = " · ".join(f"{v} {k}" for k, v in sorted(counts.items()))
    print(f"\n_{summary}_")


if __name__ == "__main__":
    main()
