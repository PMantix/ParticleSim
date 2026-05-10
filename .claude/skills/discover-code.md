---
description: Launch parallel Explore sub-agents to map this codebase, then aggregate findings into a standalone D3.js force-directed knowledge graph at doe_results/code_graph.html. Use when the user wants a navigable map of where code lives and how parts interact.
---

You're going to build a code-knowledge graph. Pipeline:

1. Launch parallel Explore sub-agents (one per module group)
2. Parse their reports into a single JSON graph
3. Render via `scripts/codegraph_render.py` to a self-contained HTML file
4. Tell the user where it landed and what's in it

### Step 1 — Launch Explore agents in parallel

Send a single message with multiple Agent tool calls (`subagent_type: Explore`).
Use the partition below; one sub-agent per `group_id`.

| group_id | paths to explore |
|---|---|
| `app` | `src/app/` |
| `body` | `src/body/` |
| `simulation` | `src/simulation/` |
| `quadtree` | `src/quadtree/` |
| `renderer` | `src/renderer/` |
| `plotting` | `src/plotting/` |
| `switch_charging` | `src/switch_charging/` |
| `doe` | `src/doe/` |
| `bin` | `src/bin/` |
| `top` | `src/config.rs`, `src/units.rs`, `src/species.rs`, `src/init_config.rs`, `src/main.rs`, `src/lib.rs` (whatever exists) |

Prompt each sub-agent with this template (substitute `{group_id}` and `{paths}`):

> Explore the `{group_id}` subsystem of this Rust particle-sim codebase
> ({paths}). Read enough to identify:
> - All `.rs` files in this group
> - The 3-7 most important struct/enum types defined here
> - The 3-7 most important public functions/methods
> - 2-3 sentences on what this subsystem does and how data flows through it
> - Which other subsystems it depends on, from this list:
>   `app, body, simulation, quadtree, renderer, plotting, switch_charging,
>   doe, bin, top`
>
> Return your report in EXACTLY this format (no extra prose):
>
> ```
> NODE_ID: {group_id}
> LABEL: <short label, 1-3 words>
> FILES: <comma-separated relative paths>
> TYPES: <comma-separated struct/enum names>
> FUNCTIONS: <comma-separated function names with module::path>
> DESCRIPTION: <2-3 sentences>
> IMPORTS_FROM: <comma-separated group ids from the list above; omit self>
> ```
>
> Be specific; don't speculate beyond what the code shows. Total under 250 words.

### Step 2 — Aggregate into JSON

After all sub-agents return:

- Parse each report's fields.
- Build the graph as JSON, using each `group_id` as a node `id`. Each
  node entry:

```json
{
  "id": "<group_id>",
  "label": "<LABEL>",
  "group": "<group_id>",
  "files": ["<from FILES>"],
  "types": ["<from TYPES>"],
  "functions": ["<from FUNCTIONS>"],
  "description": "<DESCRIPTION>"
}
```

- Build links from each node's `IMPORTS_FROM`: one link per
  `(source=this_id, target=imported_id, type='imports')`. Skip self-loops.
  Deduplicate identical edges. Skip targets that aren't in the partition.

Save as `doe_results/code_graph.json` (use `git add -f` if committing —
`doe_results/*.json` is not gitignored but be aware of `doe_results/*.csv`).

### Step 3 — Render to HTML

```bash
py scripts/codegraph_render.py -i doe_results/code_graph.json -o doe_results/code_graph.html
```

The renderer produces a single self-contained HTML page with:
- D3 v7 from `https://d3js.org/d3.v7.min.js` (CDN, fetched on first open)
- Force-directed layout, color-coded by group
- Click node → details panel (description, files, types, functions, neighbors)
- Hover node → highlight outgoing/incoming edges
- Search box (matches label/id/types/functions/files)
- Group legend (clickable to toggle visibility)
- Node radius scaled by degree (most-connected = biggest)
- Drag, zoom, pan

### Step 4 — Report to user

Tell them:
- Path: `doe_results/code_graph.html`
- Open in any browser (no server needed; needs internet on first open for D3 CDN)
- N nodes, M edges, K groups
- Optional one-line structural takeaway from the data — e.g. "renderer
  imports from 6 of 9 other groups (most coupled)" or "doe is isolated
  except via bin/top". Pick something concrete from the actual data, not
  generic.

Keep your final user-facing message brief (under 10 lines). The HTML
itself is the artifact.

### Notes

- This skill only reads files; no git ops, no rebuilds.
- Sub-agents should NOT modify files. If they propose changes, ignore.
- If a sub-agent's output deviates from the format, parse what you can
  and note any nodes you couldn't represent.
- Re-running the skill regenerates both the JSON and HTML; previous
  `code_graph.*` files will be overwritten.
- For a finer-grained graph (per-file rather than per-module), a future
  variant of this skill could partition at the file level. The current
  partition at module-group level is the right starting point — most
  important interactions are cross-group, not intra-group.
