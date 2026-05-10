---
description: Connect to South compute node — check status, read messages, send jobs
allowed-tools: Bash(./scripts/north_p2p.sh *), Bash(git fetch *), Bash(git log *), Bash(git merge *), Monitor
---

# South Coordination

South is a remote compute node (Ryzen 7950X3D) that runs batch simulation jobs. This skill manages communication and job dispatch.

## Connection

LAN messaging server at `http://192.168.1.184:8765`. Token stored in `.doe_token`.
Communication via `scripts/north_p2p.sh`:

```bash
# Check if South is reachable
./scripts/north_p2p.sh health

# Read South's messages (from line N onward)
./scripts/north_p2p.sh tail south_to_north.jsonl <after_line>

# Send a message to South
./scripts/north_p2p.sh post north_to_south.jsonl '<json_object>'
```

Git fallback: `coordination/north_to_south.jsonl` on `feature/eis-amplitude-study`.

## When invoked

1. Check if South is reachable (`health`)
2. Read latest messages from South (`tail south_to_north.jsonl`)
3. Check git for any new commits from South (`git fetch origin && git log`)
4. Report status to the user
5. If the user wants to send a job or message, format it as JSON and post via `north_p2p.sh post`

## Starting a monitor

To watch for South updates continuously:
```bash
Monitor: poll south_to_north.jsonl and git every 30s
```

## Sending jobs

Format jobs as JSON with fields: `ts`, `from: "north"`, `to: "south"`, `kind: "job-request"`, `msg`.
Always include:
- The git commit to build from
- The branch (`feature/physics-validation-framework`)
- Exact build and run commands
- Output file paths
- Whether to preserve currently running jobs

## Pulling results

When South pushes results:
```bash
git fetch origin
git merge origin/feature/physics-validation-framework --no-edit
```
