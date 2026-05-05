"""messaging_server.py — peer-to-peer JSONL messaging server for the DOE
coordination protocol.

Replaces GitHub round-trip with direct LAN HTTP for the four coordination
JSONL files. GitHub remains the durable backup (controller still pushes),
but reads/writes between South and North can flow peer-to-peer with
sub-second latency once the firewall lets it through.

Endpoints (all require Authorization: Bearer <DOE_AUTH_TOKEN>):

    GET  /health                                  -> 'ok\\n'
    GET  /index                                   -> JSON {file: line_count}
    GET  /files/<name>                            -> full file contents
    GET  /files/<name>?after=<n>                  -> lines past line index n
    POST /files/<name>                            -> append body (one+ JSON
                                                     lines) to <name>

ALLOWED files: north_jobs.jsonl, south_status.jsonl,
north_to_south.jsonl, south_to_north.jsonl.

POST is restricted by file ownership (North can only write to north_*; the
South controller writes to south_* directly on disk and doesn't need this
endpoint, but it's exposed in case the inverse direction is ever wanted).

Token is read from `DOE_AUTH_TOKEN` env var; if unset, the server fails to
start (no anonymous access). The token can be generated with
`scripts/messaging_token.py` (one-shot).

Run:
    DOE_AUTH_TOKEN=$(cat .doe_token) py scripts/messaging_server.py
"""

from __future__ import annotations

import hmac
import json
import os
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

REPO = Path(__file__).resolve().parent.parent
COORD = REPO / "coordination"

ALLOWED = {
    "north_jobs.jsonl",
    "south_status.jsonl",
    "north_to_south.jsonl",
    "south_to_north.jsonl",
}

# Files that callers from "outside" are allowed to POST to. South-owned
# files are written directly on disk by the controller; we don't expose a
# POST endpoint for them through the server. (Keeps the server narrow.)
WRITABLE_VIA_HTTP = {
    "north_jobs.jsonl",
    "north_to_south.jsonl",
}


def safe_token() -> str:
    tok = os.environ.get("DOE_AUTH_TOKEN", "").strip()
    if not tok:
        sys.stderr.write(
            "[srv] DOE_AUTH_TOKEN not set; refusing to start.\n"
            "[srv] Generate with: py scripts/messaging_token.py > .doe_token\n"
        )
        sys.exit(2)
    return tok


TOKEN = safe_token()


def constant_eq(a: str, b: str) -> bool:
    return hmac.compare_digest(a.encode("utf-8"), b.encode("utf-8"))


class Handler(BaseHTTPRequestHandler):
    server_version = "doe-msg/0.1"

    def log_message(self, fmt: str, *args) -> None:
        sys.stdout.write(f"[srv {self.log_date_time_string()}] {fmt % args}\n")
        sys.stdout.flush()

    def auth_ok(self) -> bool:
        h = self.headers.get("Authorization", "")
        if not h.startswith("Bearer "):
            return False
        return constant_eq(h[7:].strip(), TOKEN)

    def reply(self, code: int, body: bytes = b"", ctype: str = "text/plain") -> None:
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if body:
            self.wfile.write(body)

    def reply_json(self, code: int, obj) -> None:
        self.reply(code, (json.dumps(obj) + "\n").encode("utf-8"), "application/json")

    def do_GET(self) -> None:
        if not self.auth_ok():
            self.reply(401, b"unauthorized\n")
            return

        u = urlparse(self.path)

        if u.path == "/health":
            self.reply(200, b"ok\n")
            return

        if u.path == "/index":
            counts: dict[str, int] = {}
            for n in ALLOWED:
                p = COORD / n
                counts[n] = sum(1 for _ in p.open()) if p.exists() else 0
            self.reply_json(200, counts)
            return

        if u.path.startswith("/files/"):
            name = u.path.split("/", 2)[2]
            if name not in ALLOWED:
                self.reply(404, b"file not allowed\n")
                return
            p = COORD / name
            if not p.exists():
                self.reply(200, b"")
                return
            after = int(parse_qs(u.query).get("after", ["0"])[0])
            with p.open(encoding="utf-8") as f:
                lines = f.readlines()
            content = "".join(lines[after:]).encode("utf-8")
            self.reply(200, content, "application/x-ndjson")
            return

        self.reply(404, b"not found\n")

    def do_POST(self) -> None:
        if not self.auth_ok():
            self.reply(401, b"unauthorized\n")
            return

        u = urlparse(self.path)
        if not u.path.startswith("/files/"):
            self.reply(404, b"not found\n")
            return

        name = u.path.split("/", 2)[2]
        if name not in ALLOWED:
            self.reply(404, b"file not allowed\n")
            return
        if name not in WRITABLE_VIA_HTTP:
            self.reply(403, b"file is south-owned; not writable via HTTP\n")
            return

        length = int(self.headers.get("Content-Length", "0") or "0")
        if length <= 0 or length > 1_000_000:
            self.reply(400, b"empty or oversized body\n")
            return
        body = self.rfile.read(length).decode("utf-8")

        # Validate: every non-empty line must parse as JSON.
        good_lines = []
        for ln in body.splitlines():
            ln = ln.strip()
            if not ln:
                continue
            try:
                json.loads(ln)
            except json.JSONDecodeError as e:
                self.reply(400, f"invalid JSON line: {e}\n".encode("utf-8"))
                return
            good_lines.append(ln)
        if not good_lines:
            self.reply(400, b"no JSON lines in body\n")
            return

        p = COORD / name
        p.parent.mkdir(parents=True, exist_ok=True)
        with p.open("a", encoding="utf-8") as f:
            for ln in good_lines:
                f.write(ln + "\n")

        sys.stdout.write(
            f"[srv] append {name} +{len(good_lines)} line(s)\n"
        )
        sys.stdout.flush()
        self.reply_json(200, {"appended": len(good_lines), "file": name})


def main() -> None:
    host = os.environ.get("DOE_HOST", "0.0.0.0")
    port = int(os.environ.get("DOE_PORT", "8765"))
    server = ThreadingHTTPServer((host, port), Handler)
    print(
        f"[srv] DOE messaging server listening on http://{host}:{port}",
        flush=True,
    )
    print(f"[srv] coord dir: {COORD}", flush=True)
    print(f"[srv] auth: bearer-token (length {len(TOKEN)})", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("[srv] shutting down", flush=True)


if __name__ == "__main__":
    main()
