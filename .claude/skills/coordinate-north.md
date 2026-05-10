---
description: Bring up the South-side DOE coordination infrastructure (controller, messaging server, monitor) and start listening for messages from North via the protocol in coordination/PROTOCOL.md. Invoke at session start or any time you want to ensure the coordination stack is live.
---

You are the South side of a two-Claude DOE coordination protocol described
in `coordination/PROTOCOL.md`. North runs on the user's Mac. Your job: bring
up (or verify) the coordination infrastructure on this Windows host, then
pick up where the workflow left off.

Execute these steps in order. Don't ask for confirmation between them — the
user invoked this skill specifically to skip that.

### 1. Pull and survey

- `git pull --rebase origin feature/eis-amplitude-study` (use explicit args
  to avoid worktree FETCH_HEAD races; see CLAUDE.md "Operational Pitfalls").
- Read the last 5 lines of each: `coordination/north_jobs.jsonl`,
  `coordination/south_status.jsonl`, `coordination/north_to_south.jsonl`,
  `coordination/south_to_north.jsonl`. This tells you what's queued, what's
  running, and what North said most recently.

### 2. Verify or start the South controller

- Check if `scripts/south_controller.py` is already running via PowerShell:
  `Get-Process | Where-Object { $_.ProcessName -match '^py$|^python$' }`.
  Look for one started during this session or earlier today.
- If not running, launch with:
  `py scripts/south_controller.py >> south_controller.log 2>&1`
  via the Bash tool with `run_in_background: true`. Note the task ID.

### 3. Verify or start the LAN messaging server

- Check if a process is listening on TCP/8765:
  `Get-NetTCPConnection -LocalPort 8765 -State Listen -ErrorAction SilentlyContinue`.
- If not, launch with:
  `DOE_AUTH_TOKEN=$(cat .doe_token) py scripts/messaging_server.py >> messaging_server.log 2>&1`
  via the Bash tool with `run_in_background: true`. Note the task ID.
- The token in `.doe_token` is gitignored; if missing, generate a new one
  with `py scripts/messaging_token.py > .doe_token` and post the new token
  to North via `coordination/south_to_north.jsonl` so they can update
  their client.

### 4. Attach a Monitor to the controller log

- Use the Monitor tool with `persistent: true` and command:
  `tail -F south_controller.log 2>/dev/null | grep --line-buffered -Ev '\] idle '`
- This filter excludes the per-poll "idle" lines so only claims,
  completions, errors, and inbound messages from North surface as
  notifications.

### 5. Report state and stand by

After the infrastructure is up, render a single tight status table for the
user covering:

- Done / Running / Queued / Skipped counts (use `scripts/status_table.py`)
- Last message in `coordination/north_to_south.jsonl` (subject + first line
  of body) so the user sees what North is currently asking for, if anything
- Any process IDs the user might need (controller, messaging server)

Then stop and wait. From here, react to Monitor notifications as they
arrive — completions, claims, North messages — per the established
patterns:

- On a `north->south msg` event, read the full body from
  `coordination/north_to_south.jsonl` and act on it (queue work, push a
  reply, restart binary, etc.).
- On a `completion <id>` event, parse the result file and post a brief
  finding summary to North via `coordination/south_to_north.jsonl` if
  there's anything beyond `exit=0` worth flagging.
- Pull-rebase fails with "unstaged changes" during your own commit windows
  are expected and self-heal; ignore them in narration.

If you hit any of the pitfalls listed in CLAUDE.md (especially the
`taskkill //IM bash.exe` trap or the worktree FETCH_HEAD race), do not
attempt to work around them — fix root cause as documented there.
