"""Render a code-discovery JSON graph to a standalone HTML page using
D3.js v7 (loaded from CDN). The output is a single self-contained HTML
file — no build step, no server, just open it in a browser.

JSON input schema:
  {
    "nodes": [
      {
        "id": "<unique>",
        "label": "<short display>",
        "group": "<color group>",
        "files": ["<rel path>", ...],
        "types": ["<struct/enum>", ...],
        "functions": ["<fn>", ...],
        "description": "..."
      },
      ...
    ],
    "links": [
      {"source": "<id>", "target": "<id>", "type": "<imports|calls|...>"},
      ...
    ]
  }

Usage:
  py scripts/codegraph_render.py -i graph.json -o graph.html
  py scripts/codegraph_render.py < graph.json > graph.html
"""
from __future__ import annotations

import argparse
import json
import sys

HTML_TEMPLATE = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Code Knowledge Graph</title>
<script src="https://d3js.org/d3.v7.min.js"></script>
<style>
  body { margin: 0; font-family: system-ui, -apple-system, sans-serif;
         background: #1a1a1a; color: #e0e0e0; overflow: hidden; }
  #graph svg { width: 100vw; height: 100vh; display: block; }

  .node circle { stroke: #222; stroke-width: 1.5px; cursor: pointer;
                 transition: stroke 0.15s, stroke-width 0.15s; }
  .node.selected circle { stroke: #fff; stroke-width: 3.5px;
                          filter: drop-shadow(0 0 6px #6cf); }
  .node.neighbor circle { stroke: #6cf; stroke-width: 2.5px; }
  .node circle:hover { stroke: #fff; stroke-width: 3px; }

  .node text { font-size: 14px; font-weight: 500; pointer-events: none;
               fill: #f0f0f0;
               text-shadow: 0 0 3px #000, 1px 1px 2px #000, -1px -1px 2px #000; }
  .node.selected text { font-size: 16px; font-weight: 700; fill: #fff;
                        text-shadow: 0 0 4px #6cf, 1px 1px 2px #000; }
  .node.neighbor text { fill: #cfe8ff; }

  /* Dimmed (group filter or search non-match): still visible, clearly de-emphasized. */
  .node.dimmed circle { opacity: 0.18; filter: saturate(0.2); }
  .node.dimmed text   { opacity: 0.25; }

  /* Approved / rejected: a colored ring around the circle so the
     review state is visible on the graph itself, not only in the popup. */
  .node.approved circle { stroke: #5dd870; stroke-width: 3px; }
  .node.rejected circle { stroke: #d85d5d; stroke-width: 3px; stroke-dasharray: 4 3; }

  .link { stroke: #888; stroke-opacity: 0.35; }
  .link.dimmed { stroke-opacity: 0.04; }
  .link.highlight { stroke: #6cf; stroke-opacity: 0.95; stroke-width: 2.5px; }
  .link.fade { stroke-opacity: 0.06; }

  #info { position: fixed; top: 10px; right: 10px; width: 360px; padding: 16px;
          background: rgba(28,28,28,0.97); border: 1px solid #555;
          border-radius: 4px; max-height: 92vh; overflow-y: auto;
          font-size: 14px; line-height: 1.5; box-shadow: 0 2px 12px rgba(0,0,0,0.5); }
  #info h3 { margin: 0 0 6px 0; color: #6cf; font-size: 18px; }
  #info .group-tag { color: #fc6; font-size: 12px; text-transform: uppercase;
                     letter-spacing: 0.5px; margin-bottom: 10px; }
  #info p { margin: 4px 0 10px 0; color: #ddd; }
  #info h4 { margin: 10px 0 4px 0; color: #bbb; font-size: 12px;
             text-transform: uppercase; letter-spacing: 0.5px; }
  #info ul { padding-left: 20px; margin: 2px 0 8px 0; }
  #info li { font-size: 13px; color: #ccc; word-break: break-all; line-height: 1.4; }
  #info code { background: #333; padding: 1px 5px; border-radius: 2px;
               color: #cfc; font-size: 13px; }
  #info pre { background: #111; padding: 10px; border: 1px solid #333;
              border-radius: 3px; font-size: 12px; line-height: 1.4;
              color: #d0e0d0; overflow-x: auto; max-height: 320px;
              white-space: pre; margin: 4px 0 8px 0; }
  #info .rationale { background: rgba(102,153,255,0.08); border-left: 3px solid #6cf;
                     padding: 8px 10px; margin: 8px 0; font-size: 13px;
                     color: #e0e8f0; white-space: pre-wrap; }
  #info .rationale.empty { background: rgba(255,255,255,0.04); border-left-color: #555;
                           color: #888; font-style: italic; }
  #info .actions { display: flex; gap: 8px; margin: 12px 0 6px 0; }
  #info .actions button { flex: 1; padding: 8px 10px; border: 1px solid #555;
                          background: #2a2a2a; color: #ddd; border-radius: 3px;
                          cursor: pointer; font-size: 13px; font-weight: 500; }
  #info .actions button.approve:hover { background: #2a4a30; border-color: #5dd870; color: #5dd870; }
  #info .actions button.reject:hover  { background: #4a2a2a; border-color: #d85d5d; color: #d85d5d; }
  #info .actions button.active.approve { background: #2a4a30; border-color: #5dd870; color: #5dd870; }
  #info .actions button.active.reject  { background: #4a2a2a; border-color: #d85d5d; color: #d85d5d; }
  #info a { color: #6cf; text-decoration: none; }
  #info a:hover { text-decoration: underline; }
  #info .links a { display: inline-block; margin-right: 10px; font-size: 12px; }
  #info .neighbor-link { color: #6cf; cursor: pointer; text-decoration: underline dotted; }
  #info .neighbor-link:hover { color: #fff; }
  details > summary { cursor: pointer; color: #aaa; font-size: 12px;
                      text-transform: uppercase; letter-spacing: 0.5px; padding: 4px 0; }
  details[open] > summary { color: #ddd; }

  #search { position: fixed; top: 10px; left: 10px; padding: 9px 12px;
            background: rgba(28,28,28,0.97); border: 1px solid #555;
            border-radius: 4px; color: #fff; width: 240px; font-size: 14px; }
  #search:focus { outline: none; border-color: #6cf; }

  .legend { position: fixed; bottom: 10px; left: 10px; padding: 10px 12px;
            background: rgba(28,28,28,0.97); border: 1px solid #555;
            border-radius: 4px; font-size: 13px; }
  .legend .row { margin: 4px 0; cursor: pointer; padding: 1px 3px;
                 border-radius: 2px; transition: background 0.1s; }
  .legend .row:hover { background: #333; }
  .legend .row.dimmed { opacity: 0.4; }
  .legend .row.dimmed .swatch { filter: saturate(0.2); }
  .legend .swatch { display: inline-block; width: 14px; height: 14px;
                    margin-right: 8px; vertical-align: middle;
                    border: 1px solid rgba(255,255,255,0.3);
                    border-radius: 2px; }

  .stats { position: fixed; bottom: 10px; right: 10px; padding: 9px 12px;
           background: rgba(28,28,28,0.97); border: 1px solid #555;
           border-radius: 4px; font-size: 12px; color: #bbb; }
</style>
</head>
<body>
  <input type="text" id="search" placeholder="Search nodes (label, type, function)...">
  <div id="graph"></div>
  <div id="info"><em>Click a node to see details. Drag to reposition. Scroll to zoom.</em></div>
  <div class="legend" id="legend"></div>
  <div class="stats" id="stats"></div>
<script>
const DATA = __GRAPH_JSON__;

const width = window.innerWidth, height = window.innerHeight;
const svg = d3.select('#graph').append('svg').attr('width', width).attr('height', height);
const g = svg.append('g');

svg.call(d3.zoom().scaleExtent([0.2, 8]).on('zoom', (e) => g.attr('transform', e.transform)));

// Color by group.
const groups = [...new Set(DATA.nodes.map(d => d.group))].sort();
const color = d3.scaleOrdinal(d3.schemeTableau10).domain(groups);

// Legend (clickable to filter).
const dimmedGroups = new Set();
const legend = d3.select('#legend');
legend.append('div').style('font-weight', 'bold').style('margin-bottom', '4px').text('Groups');
groups.forEach(grp => {
  legend.append('div').attr('class', 'row').attr('data-group', grp)
    .html(`<span class="swatch" style="background:${color(grp)}"></span>${grp}`)
    .on('click', function() {
      if (dimmedGroups.has(grp)) dimmedGroups.delete(grp); else dimmedGroups.add(grp);
      d3.select(this).classed('dimmed', dimmedGroups.has(grp));
      applyClasses();
    });
});

// Render links and nodes.
const link = g.append('g').attr('stroke-linecap', 'round')
  .selectAll('line').data(DATA.links).enter().append('line').attr('class', 'link');

const node = g.append('g').selectAll('g').data(DATA.nodes).enter().append('g')
  .attr('class', 'node').call(d3.drag()
    .on('start', (e,d) => { if (!e.active) sim.alphaTarget(0.3).restart(); d.fx=d.x; d.fy=d.y; })
    .on('drag', (e,d) => { d.fx=e.x; d.fy=e.y; })
    .on('end', (e,d) => { if (!e.active) sim.alphaTarget(0); d.fx=null; d.fy=null; }));

// Size circles by degree (in + out edges).
const degree = {};
DATA.nodes.forEach(n => degree[n.id] = 0);
DATA.links.forEach(l => {
  const s = typeof l.source === 'object' ? l.source.id : l.source;
  const t = typeof l.target === 'object' ? l.target.id : l.target;
  degree[s] = (degree[s] || 0) + 1;
  degree[t] = (degree[t] || 0) + 1;
});
const r = d3.scaleSqrt().domain([0, d3.max(Object.values(degree)) || 1]).range([6, 16]);

node.append('circle')
  .attr('r', d => r(degree[d.id] || 0))
  .attr('fill', d => color(d.group))
  .on('click', (e,d) => selectNode(d))
  .on('mouseover', (e,d) => highlightNeighbors(d, true))
  .on('mouseout', (e,d) => highlightNeighbors(d, false));

node.append('text').text(d => d.label).attr('x', d => r(degree[d.id] || 0) + 3).attr('y', 4);

const sim = d3.forceSimulation(DATA.nodes)
  .force('link', d3.forceLink(DATA.links).id(d => d.id).distance(100).strength(0.4))
  .force('charge', d3.forceManyBody().strength(-280))
  .force('center', d3.forceCenter(width/2, height/2))
  .force('collide', d3.forceCollide().radius(d => r(degree[d.id] || 0) + 4))
  .on('tick', () => {
    link.attr('x1', d => d.source.x).attr('y1', d => d.source.y)
        .attr('x2', d => d.target.x).attr('y2', d => d.target.y);
    node.attr('transform', d => `translate(${d.x},${d.y})`);
  });

// Build a neighbor index: id -> Set of neighbor ids.
const neighborIndex = {};
DATA.nodes.forEach(n => neighborIndex[n.id] = new Set());
DATA.links.forEach(l => {
  const s = typeof l.source === 'object' ? l.source.id : l.source;
  const t = typeof l.target === 'object' ? l.target.id : l.target;
  neighborIndex[s].add(t);
  neighborIndex[t].add(s);
});

let selectedId = null;
function selectNode(d) {
  selectedId = (selectedId === d.id) ? null : d.id;
  applyClasses();
  if (selectedId) showInfo(d); else clearInfo();
}

function clearInfo() {
  d3.select('#info').html('<em>Click a node to see details. Drag to reposition. Scroll to zoom.</em>');
}

// Persistent approval/rejection state in localStorage, keyed by node id.
const APPROVAL_KEY = 'codegraph.review.v1';
function loadApprovals() {
  try { return JSON.parse(localStorage.getItem(APPROVAL_KEY) || '{}'); }
  catch (e) { return {}; }
}
function saveApprovals(obj) {
  localStorage.setItem(APPROVAL_KEY, JSON.stringify(obj));
}
let approvals = loadApprovals();

function setApproval(id, status) {
  if (status === null || approvals[id] === status) {
    delete approvals[id];
  } else {
    approvals[id] = status;
  }
  saveApprovals(approvals);
  applyClasses();
  // Re-render info panel to reflect new state.
  const n = DATA.nodes.find(x => x.id === id);
  if (n) showInfo(n);
}

function escapeHtml(s) {
  if (s == null) return '';
  return String(s)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

function showInfo(d) {
  const info = d3.select('#info');
  const status = approvals[d.id] || null;
  let html = `<h3>${escapeHtml(d.label)}</h3>`;
  html += `<div class="group-tag">${escapeHtml(d.kind || '')} &middot; ${escapeHtml(d.group)}</div>`;

  // Approve/Reject actions — these are how the user defends the choice.
  html += `<div class="actions">`;
  html += `<button class="approve${status === 'approved' ? ' active' : ''}" onclick="setApproval(${JSON.stringify(d.id)}, 'approved')">${status === 'approved' ? 'Approved' : 'Approve'}</button>`;
  html += `<button class="reject${status === 'rejected' ? ' active' : ''}" onclick="setApproval(${JSON.stringify(d.id)}, 'rejected')">${status === 'rejected' ? 'Rejected' : 'Reject'}</button>`;
  html += `</div>`;

  if (d.description) html += `<p>${escapeHtml(d.description)}</p>`;

  // Rationale block — the "why" defense, taken from leading doc-comments.
  html += `<h4>Rationale</h4>`;
  if (d.rationale && d.rationale.trim()) {
    html += `<div class="rationale">${escapeHtml(d.rationale)}</div>`;
  } else {
    html += `<div class="rationale empty">No leading doc-comment found. The "why" needs to be inferred from the source below or from CLAUDE.md / other docs.</div>`;
  }

  // Reference links — open the actual file in editor.
  if (d.abs_path) {
    html += `<h4>References</h4><div class="links">`;
    html += `<a href="vscode://file/${encodeURI(d.abs_path)}" title="Open in VS Code">VS Code</a>`;
    html += `<a href="file:///${encodeURI(d.abs_path)}" title="Open via file://">file://</a>`;
    html += `<a href="#" onclick="navigator.clipboard.writeText(${JSON.stringify(d.abs_path)});return false;" title="Copy path">copy path</a>`;
    html += `</div>`;
  }

  // Source excerpt — the "evidence".
  if (d.excerpt) {
    html += `<details${(d.kind === 'file') ? '' : ' open'}><summary>Source excerpt</summary>`;
    html += `<pre><code>${escapeHtml(d.excerpt)}</code></pre></details>`;
  }

  if (d.files && d.files.length && d.kind !== 'file') {
    html += `<h4>Defined in</h4><ul>`;
    d.files.forEach(f => html += `<li><code>${escapeHtml(f)}</code></li>`);
    html += `</ul>`;
  }
  if (d.types && d.types.length) {
    html += `<h4>Types in this file</h4><ul>`;
    d.types.forEach(t => html += `<li><code>${escapeHtml(t)}</code></li>`);
    html += `</ul>`;
  }
  if (d.functions && d.functions.length) {
    html += `<h4>Public functions in this file</h4><ul>`;
    d.functions.forEach(f => html += `<li><code>${escapeHtml(f)}</code></li>`);
    html += `</ul>`;
  }

  // Neighbors — clickable to jump to that node.
  const neighbors = DATA.links
    .map(l => {
      const s = typeof l.source === 'object' ? l.source.id : l.source;
      const t = typeof l.target === 'object' ? l.target.id : l.target;
      if (s === d.id) return {dir: '→', other: t, type: l.type};
      if (t === d.id) return {dir: '←', other: s, type: l.type};
      return null;
    }).filter(x => x);
  if (neighbors.length) {
    html += `<h4>Connections (${neighbors.length})</h4><ul>`;
    neighbors.forEach(n => {
      const otherNode = DATA.nodes.find(nn => nn.id === n.other);
      const ostatus = approvals[n.other];
      const marker = ostatus === 'approved' ? ' ✓' : (ostatus === 'rejected' ? ' ✗' : '');
      html += `<li>${n.dir} <span class="neighbor-link" onclick="jumpTo(${JSON.stringify(n.other)})">${escapeHtml(otherNode ? otherNode.label : n.other)}</span><small>${marker} (${escapeHtml(n.type || 'link')})</small></li>`;
    });
    html += `</ul>`;
  }
  info.html(html);
}

function jumpTo(id) {
  const n = DATA.nodes.find(x => x.id === id);
  if (!n) return;
  selectedId = id;
  applyClasses();
  showInfo(n);
}

function highlightNeighbors(d, on) {
  link.classed('highlight', l => {
    const s = typeof l.source === 'object' ? l.source.id : l.source;
    const t = typeof l.target === 'object' ? l.target.id : l.target;
    return on && (s === d.id || t === d.id);
  });
}

let searchQuery = '';
function nodeMatchesSearch(d) {
  if (!searchQuery) return true;
  const q = searchQuery.toLowerCase();
  const hay = (d.label + ' ' + d.id + ' ' + (d.types||[]).join(' ') + ' '
               + (d.functions||[]).join(' ') + ' ' + (d.files||[]).join(' '));
  return hay.toLowerCase().includes(q);
}

// Build a fast id->node lookup for class application.
const nodeById = {};
DATA.nodes.forEach(n => { nodeById[n.id] = n; });

function applyClasses() {
  // Per-node classes: dimmed (filtered out), selected (clicked), neighbor,
  // approved/rejected (review state).
  const sel = selectedId;
  const neighbors = sel ? neighborIndex[sel] : null;
  node.classed('dimmed', d =>
    dimmedGroups.has(d.group) || !nodeMatchesSearch(d));
  node.classed('selected', d => d.id === sel);
  node.classed('neighbor', d => neighbors && d.id !== sel && neighbors.has(d.id));
  node.classed('approved', d => approvals[d.id] === 'approved');
  node.classed('rejected', d => approvals[d.id] === 'rejected');

  // Link classes: dimmed if either endpoint is dimmed; highlight if
  // touches the selected node; faded if a node is selected but this
  // link doesn't touch it (so the selected node's edges pop out).
  link.classed('dimmed', l => {
    const s = typeof l.source === 'object' ? l.source.id : l.source;
    const t = typeof l.target === 'object' ? l.target.id : l.target;
    const sNode = nodeById[s];
    const tNode = nodeById[t];
    return (sNode && (dimmedGroups.has(sNode.group) || !nodeMatchesSearch(sNode))) ||
           (tNode && (dimmedGroups.has(tNode.group) || !nodeMatchesSearch(tNode)));
  });
  link.classed('highlight', l => {
    if (!sel) return false;
    const s = typeof l.source === 'object' ? l.source.id : l.source;
    const t = typeof l.target === 'object' ? l.target.id : l.target;
    return s === sel || t === sel;
  });
  link.classed('fade', l => {
    if (!sel) return false;
    const s = typeof l.source === 'object' ? l.source.id : l.source;
    const t = typeof l.target === 'object' ? l.target.id : l.target;
    return !(s === sel || t === sel);
  });
}

document.getElementById('search').addEventListener('input', (e) => {
  searchQuery = e.target.value;
  applyClasses();
});

document.getElementById('stats').innerHTML =
  `${DATA.nodes.length} nodes · ${DATA.links.length} edges · ${groups.length} groups`;
</script>
</body>
</html>
"""


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--input', '-i', help='JSON input file (default: stdin)')
    ap.add_argument('--output', '-o', help='HTML output file (default: stdout)')
    args = ap.parse_args()

    if args.input:
        with open(args.input, 'r', encoding='utf-8') as f:
            graph = json.load(f)
    else:
        graph = json.load(sys.stdin)

    if 'nodes' not in graph or 'links' not in graph:
        sys.stderr.write("Error: input must have 'nodes' and 'links' keys\n")
        sys.exit(2)

    html = HTML_TEMPLATE.replace('__GRAPH_JSON__', json.dumps(graph))

    if args.output:
        with open(args.output, 'w', encoding='utf-8') as f:
            f.write(html)
        sys.stderr.write(f"Wrote {args.output} ({len(graph['nodes'])} nodes, {len(graph['links'])} edges)\n")
    else:
        sys.stdout.write(html)


if __name__ == '__main__':
    main()
