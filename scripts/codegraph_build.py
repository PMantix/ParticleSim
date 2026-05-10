"""Build a function-level code-knowledge graph from a Rust source tree
using regex extraction (no tree-sitter dependency).

Walks src/, extracts:
  - file nodes (one per .rs file)
  - type nodes (each pub struct, enum, trait declared at module scope)
  - function nodes (each fn declared at module scope or as impl method)

Edges:
  - file -> file: from `use crate::<path>;` references
  - file -> type|function: "contains" (a node belongs to a file)
  - function -> function: heuristic call edges (when function body
    contains another function's bare-name token followed by `(`)
  - function -> type: heuristic use edges (function body references
    a type by name)

Group is the top-level module under src/ (e.g. "simulation", "renderer",
"body"). Top-level files like config.rs go in group "top".

Output JSON shape matches scripts/codegraph_render.py expectations.

Usage:
    py scripts/codegraph_build.py [--src-root src] [--output graph.json]
                                  [--min-fn-body-lines N] [--public-only]
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

# -------- Regexes (line-anchored, ignore_case off) --------
RE_USE = re.compile(r"^\s*use\s+([\w:{}*,\s]+);", re.MULTILINE)
RE_STRUCT = re.compile(r"^\s*(pub\s+)?struct\s+(\w+)", re.MULTILINE)
RE_ENUM = re.compile(r"^\s*(pub\s+)?enum\s+(\w+)", re.MULTILINE)
RE_TRAIT = re.compile(r"^\s*(pub\s+)?trait\s+(\w+)", re.MULTILINE)
RE_TYPE_ALIAS = re.compile(r"^\s*(pub\s+)?type\s+(\w+)", re.MULTILINE)
# Function declarations: capture line, modifier (pub/async/unsafe), name.
RE_FN = re.compile(
    r"^(?P<indent>\s*)"
    r"(?P<vis>pub(?:\s*\([^)]+\))?\s+)?"
    r"(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?"
    r"fn\s+(?P<name>\w+)\s*[(<]",
    re.MULTILINE,
)
# `impl Foo` / `impl Foo<T>` / `impl Trait for Foo`
RE_IMPL = re.compile(
    r"^\s*impl\b[^{]*?(?:for\s+)?(?P<recv>[\w:]+)\s*(?:<[^{]*?>)?\s*\{",
    re.MULTILINE,
)


def strip_comments_and_strings(src: str) -> str:
    """Crude but effective: remove // line comments, /* */ block comments,
    and "..." string literals so our regex matchers don't see them. We
    keep newlines so line numbers stay roughly aligned."""
    # Block comments (non-greedy).
    src = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), src, flags=re.DOTALL)
    # Line comments.
    src = re.sub(r"//[^\n]*", "", src)
    # String literals (simple — doesn't handle raw strings perfectly).
    src = re.sub(r'"(?:\\.|[^"\\])*"', '""', src)
    return src


def find_function_bodies(src: str):
    """Return list of (name, indent, start_idx, end_idx, body_text) for
    each fn declaration. End index found by brace-counting from the first
    `{` after the declaration."""
    out = []
    for m in RE_FN.finditer(src):
        # Find first `{` after the fn name.
        i = m.end()
        depth = 0
        body_start = None
        while i < len(src):
            ch = src[i]
            if ch == ";" and body_start is None:
                # Forward decl (e.g., trait method without body): bail.
                break
            if ch == "{":
                if body_start is None:
                    body_start = i
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0 and body_start is not None:
                    body_end = i
                    out.append(
                        {
                            "name": m.group("name"),
                            "is_pub": bool(m.group("vis")),
                            "decl_start": m.start(),
                            "body_start": body_start,
                            "body_end": body_end,
                            "body": src[body_start + 1 : body_end],
                            "indent": len(m.group("indent")),
                        }
                    )
                    break
            i += 1
    return out


def find_impl_for_function(fn_decl_pos: int, src: str) -> str | None:
    """Walk backwards from fn declaration to find the enclosing `impl`
    receiver type, if any. Returns receiver type name or None."""
    # Find last `impl` whose body brace matches before this fn pos.
    best = None
    for m in RE_IMPL.finditer(src):
        if m.end() > fn_decl_pos:
            break
        # Brace-count from the impl's opening brace forward to ensure
        # fn_decl_pos is still inside it.
        i = m.end()  # position right after '{'
        depth = 1
        while i < len(src) and depth > 0:
            if src[i] == "{":
                depth += 1
            elif src[i] == "}":
                depth -= 1
                if depth == 0:
                    impl_end = i
                    if m.start() < fn_decl_pos < impl_end:
                        best = m.group("recv")
                    break
            i += 1
    return best


def extract_leading_rationale(raw_src: str, decl_start: int) -> str:
    """Walk backwards from a declaration to collect contiguous leading
    doc comments (``///`` or ``//!``) and regular comments (``//``),
    stopping at the first blank line or non-comment code line.
    This is the 'why I made this choice' rationale block that should
    accompany each node in the knowledge graph."""
    # Find the start of the declaration's line.
    line_start = raw_src.rfind("\n", 0, decl_start) + 1
    # Walk upward line by line.
    rationale_lines = []
    pos = line_start - 1  # at the previous newline
    while pos > 0:
        prev_line_start = raw_src.rfind("\n", 0, pos) + 1
        line = raw_src[prev_line_start:pos]
        stripped = line.strip()
        if not stripped:
            break  # blank line ends rationale block
        if stripped.startswith("///") or stripped.startswith("//!") or stripped.startswith("//"):
            rationale_lines.append(line.rstrip())
            pos = prev_line_start - 1
        elif stripped.startswith("#[") or stripped.startswith("#!["):
            # Skip attributes (e.g. #[derive(...)]) — keep walking up.
            pos = prev_line_start - 1
        else:
            break
    rationale_lines.reverse()
    return "\n".join(rationale_lines).strip()


def extract_block_body(clean: str, decl_start: int) -> tuple[int, int]:
    """Find the matching `{ ... }` block following a declaration.
    Returns (body_start, body_end) char indices, or (-1, -1) if no body
    (e.g., type alias `type X = Y;` or unit struct `struct X;`)."""
    i = decl_start
    body_start = None
    depth = 0
    while i < len(clean):
        ch = clean[i]
        if ch == ";" and body_start is None:
            return (-1, -1)
        if ch == "{":
            if body_start is None:
                body_start = i
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0 and body_start is not None:
                return (body_start, i)
        i += 1
    return (-1, -1)


def excerpt_from_raw(raw_src: str, start: int, end: int, max_chars: int = 1500) -> str:
    """Pull excerpt from RAW source (with comments preserved) given char
    offsets that came from the cleaned-source view. Since strip_comments
    preserves character positions (replaces with spaces/newlines), the
    same indices work in raw source. Truncates with ellipsis if needed."""
    text = raw_src[start:end + 1]
    if len(text) > max_chars:
        text = text[:max_chars - 12] + "\n  /* ... */"
    return text


def extract_file_node(rel_path: str, src: str, group: str):
    """Returns (file_node, type_nodes, fn_nodes, fn_calls, type_uses, imports)."""
    clean = strip_comments_and_strings(src)

    types = []
    for rgx, kind in [
        (RE_STRUCT, "struct"),
        (RE_ENUM, "enum"),
        (RE_TRAIT, "trait"),
        (RE_TYPE_ALIAS, "type"),
    ]:
        for m in rgx.finditer(clean):
            is_pub = bool(m.group(1))
            name = m.group(2)
            # Body for excerpt (struct/enum/trait — type aliases have no body).
            decl_line_start = src.rfind("\n", 0, m.start()) + 1
            if kind == "type":
                # Type alias: take from decl line to next ;
                end = src.find(";", m.end())
                excerpt = src[decl_line_start:end + 1] if end != -1 else src[decl_line_start:m.end() + 80]
            else:
                bs, be = extract_block_body(clean, m.end())
                if bs >= 0:
                    excerpt = excerpt_from_raw(src, decl_line_start, be, max_chars=1500)
                else:
                    # Unit struct or similar.
                    end = src.find(";", m.end())
                    excerpt = src[decl_line_start:end + 1] if end != -1 else src[decl_line_start:m.end() + 80]
            rationale = extract_leading_rationale(src, decl_line_start)
            types.append({"name": name, "kind": kind, "is_pub": is_pub, "excerpt": excerpt, "rationale": rationale})

    fns = []
    for fb in find_function_bodies(clean):
        impl_recv = find_impl_for_function(fb["decl_start"], clean) if fb["indent"] > 0 else None
        # Simple name = name; qualified = "Type::name" if impl.
        qual_name = f"{impl_recv}::{fb['name']}" if impl_recv else fb["name"]
        decl_line_start = src.rfind("\n", 0, fb["decl_start"]) + 1
        excerpt = excerpt_from_raw(src, decl_line_start, fb["body_end"], max_chars=2000)
        rationale = extract_leading_rationale(src, decl_line_start)
        fns.append(
            {
                "name": fb["name"],
                "qual_name": qual_name,
                "is_pub": fb["is_pub"],
                "is_method": impl_recv is not None,
                "body": fb["body"],
                "body_lines": fb["body"].count("\n") + 1,
                "excerpt": excerpt,
                "rationale": rationale,
            }
        )

    # `use` statements -> referenced module paths.
    uses = []
    for m in RE_USE.finditer(clean):
        path = m.group(1).strip()
        # Expand `use a::{b, c}` into a::b, a::c.
        if "{" in path:
            base, _, rest = path.partition("{")
            inner = rest.rstrip("}").strip()
            base = base.rstrip(":").strip()
            for piece in inner.split(","):
                piece = piece.strip()
                if piece:
                    uses.append(f"{base}::{piece}" if base else piece)
        else:
            uses.append(path)

    file_node = {
        "rel_path": rel_path,
        "group": group,
    }
    return file_node, types, fns, uses


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--src-root", default="src", help="Root of Rust source tree")
    ap.add_argument("--output", "-o", default="doe_results/code_graph.json")
    ap.add_argument(
        "--min-fn-body-lines",
        type=int,
        default=3,
        help="Skip fn nodes whose body is shorter than this. 0 = include all.",
    )
    ap.add_argument(
        "--public-only",
        action="store_true",
        help="Include only `pub` functions and types as their own nodes.",
    )
    args = ap.parse_args()

    src_root = Path(args.src_root).resolve()
    if not src_root.is_dir():
        sys.stderr.write(f"--src-root {src_root} not a directory\n")
        return 2

    rs_files = sorted(src_root.rglob("*.rs"))
    sys.stderr.write(f"Scanning {len(rs_files)} .rs files under {src_root}\n")

    # Per-file extraction.
    files_data = []
    for f in rs_files:
        rel = f.relative_to(src_root.parent).as_posix()
        # Group = first directory under src/ (or "top" for src/foo.rs).
        parts = f.relative_to(src_root).parts
        if len(parts) == 1:
            group = "top"
        else:
            group = parts[0]
        try:
            text = f.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            text = f.read_text(encoding="latin1")
        file_node, types, fns, uses = extract_file_node(rel, text, group)
        files_data.append((file_node, types, fns, uses))

    # Build nodes.
    nodes = []
    file_id = lambda rp: f"file:{rp}"
    type_id = lambda rp, name: f"type:{rp}::{name}"
    fn_id = lambda rp, qn: f"fn:{rp}::{qn}"

    file_node_lookup = {}  # rel_path -> file_id
    type_name_lookup = defaultdict(list)  # type name -> list of (rel, id)
    fn_name_lookup = defaultdict(list)    # fn simple name -> list of (rel, id)

    repo_root = src_root.parent
    for file_node, types, fns, uses in files_data:
        rp = file_node["rel_path"]
        fid = file_id(rp)
        file_node_lookup[rp] = fid
        abs_path = (repo_root / rp).as_posix()
        # File-level rationale: top-of-file `//!` doc comment, if any.
        rp_full = repo_root / rp
        try:
            file_text = rp_full.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            file_text = rp_full.read_text(encoding="latin1")
        file_rationale_lines = []
        for line in file_text.splitlines():
            s = line.strip()
            if s.startswith("//!") or (file_rationale_lines and s.startswith("//")):
                file_rationale_lines.append(line)
            elif s.startswith("//"):
                file_rationale_lines.append(line)
            elif not s:
                if file_rationale_lines:
                    break
                continue
            else:
                break
        file_rationale = "\n".join(file_rationale_lines).strip()
        nodes.append(
            {
                "id": fid,
                "label": Path(rp).name,
                "group": file_node["group"],
                "kind": "file",
                "files": [rp],
                "abs_path": abs_path,
                "types": [t["name"] for t in types if t.get("is_pub") or not args.public_only],
                "functions": [f["qual_name"] for f in fns if f.get("is_pub") or not args.public_only],
                "description": f"File {rp}: {len(types)} types, {len(fns)} functions.",
                "rationale": file_rationale,
                "excerpt": "\n".join(file_text.splitlines()[:40]),
            }
        )
        for t in types:
            if args.public_only and not t["is_pub"]:
                continue
            tid = type_id(rp, t["name"])
            nodes.append(
                {
                    "id": tid,
                    "label": t["name"],
                    "group": file_node["group"],
                    "kind": t["kind"],
                    "files": [rp],
                    "abs_path": abs_path,
                    "description": f"{t['kind']} {t['name']}{'  (pub)' if t['is_pub'] else ''} in {rp}",
                    "rationale": t.get("rationale", ""),
                    "excerpt": t.get("excerpt", ""),
                }
            )
            type_name_lookup[t["name"]].append((rp, tid))
        for f in fns:
            if args.min_fn_body_lines and f["body_lines"] < args.min_fn_body_lines:
                continue
            if args.public_only and not f["is_pub"]:
                continue
            fid_n = fn_id(rp, f["qual_name"])
            nodes.append(
                {
                    "id": fid_n,
                    "label": f["qual_name"],
                    "group": file_node["group"],
                    "kind": "method" if f["is_method"] else "function",
                    "files": [rp],
                    "abs_path": abs_path,
                    "description": f"{'method' if f['is_method'] else 'fn'} {f['qual_name']} in {rp} ({f['body_lines']} body lines{', pub' if f['is_pub'] else ''})",
                    "rationale": f.get("rationale", ""),
                    "excerpt": f.get("excerpt", ""),
                }
            )
            fn_name_lookup[f["name"]].append((rp, fid_n, f["qual_name"]))

    # Build edges.
    links = []
    seen = set()

    def add_link(s, t, kind):
        key = (s, t, kind)
        if key in seen or s == t:
            return
        seen.add(key)
        links.append({"source": s, "target": t, "type": kind})

    # contains edges (file -> type, file -> fn).
    for n in nodes:
        if n["kind"] == "file":
            continue
        if n["kind"] in ("struct", "enum", "trait", "type"):
            owner = file_id(n["files"][0])
            add_link(owner, n["id"], "contains")
        elif n["kind"] in ("function", "method"):
            owner = file_id(n["files"][0])
            add_link(owner, n["id"], "contains")

    # file -> file via `use` statements.
    for file_node, _types, _fns, uses in files_data:
        rp = file_node["rel_path"]
        sid = file_node_lookup[rp]
        for u in uses:
            # Match `crate::<group>::...` and `super::...` only as cross-file.
            if u.startswith("crate::"):
                target_path = u[len("crate::"):]
                # Try to resolve to a file under src/.
                pieces = target_path.split("::")
                # Drop trailing item (function/struct name) for path-only match.
                # We'll attempt several resolutions.
                cands = []
                for cut in range(len(pieces), 0, -1):
                    candidate_path = "/".join(pieces[:cut]) + ".rs"
                    cands.append(f"src/{candidate_path}")
                    cands.append(f"src/{'/'.join(pieces[:cut])}/mod.rs")
                for c in cands:
                    if c in file_node_lookup:
                        add_link(sid, file_node_lookup[c], "uses")
                        break

    # function -> function call edges and function -> type use edges
    # (heuristic, name-based).
    for file_node, _types, fns, _uses in files_data:
        rp = file_node["rel_path"]
        for f in fns:
            if args.min_fn_body_lines and f["body_lines"] < args.min_fn_body_lines:
                continue
            if args.public_only and not f["is_pub"]:
                continue
            sid = fn_id(rp, f["qual_name"])
            body = f["body"]
            # Tokenize identifiers (letters/digits/_).
            calls = set(re.findall(r"\b([a-z_][a-zA-Z0-9_]*)\s*\(", body))
            for callee in calls:
                if callee == f["name"]:
                    continue  # ignore self-recursion
                # Resolve callee: prefer function in same file, else any
                # other file. If multiple candidates, link to all (small N).
                candidates = fn_name_lookup.get(callee, [])
                local = [c for c in candidates if c[0] == rp]
                target_set = local if local else candidates
                for _, tid, _ in target_set:
                    add_link(sid, tid, "calls")
            # Type uses (find type names in body).
            type_refs = set(re.findall(r"\b([A-Z][A-Za-z0-9_]+)\b", body))
            for tname in type_refs:
                candidates = type_name_lookup.get(tname, [])
                if candidates:
                    # Prefer same-file type if available.
                    local = [c for c in candidates if c[0] == rp]
                    target_set = local if local else candidates[:1]
                    for _, tid in target_set:
                        add_link(sid, tid, "uses_type")

    graph = {"nodes": nodes, "links": links}
    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(graph, indent=2), encoding="utf-8")
    sys.stderr.write(
        f"Wrote {out_path}: {len(nodes)} nodes, {len(links)} edges\n"
    )
    # Print breakdown.
    by_kind = defaultdict(int)
    for n in nodes:
        by_kind[n["kind"]] += 1
    for k, v in sorted(by_kind.items()):
        sys.stderr.write(f"  {k}: {v}\n")
    by_etype = defaultdict(int)
    for l in links:
        by_etype[l["type"]] += 1
    for k, v in sorted(by_etype.items()):
        sys.stderr.write(f"  edge {k}: {v}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
