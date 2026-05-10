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
  .node circle { stroke: #fff; stroke-width: 1.5px; cursor: pointer; }
  .node circle:hover { stroke-width: 3px; }
  .node text { font-size: 12px; pointer-events: none; fill: #ddd;
               text-shadow: 1px 1px 2px #000, -1px -1px 2px #000; }
  .link { stroke: #888; stroke-opacity: 0.4; }
  .link.highlight { stroke: #6cf; stroke-opacity: 0.9; stroke-width: 2px; }

  #info { position: fixed; top: 10px; right: 10px; width: 340px; padding: 14px;
          background: rgba(30,30,30,0.97); border: 1px solid #555;
          border-radius: 4px; max-height: 92vh; overflow-y: auto;
          font-size: 13px; line-height: 1.45; box-shadow: 0 2px 12px rgba(0,0,0,0.5); }
  #info h3 { margin: 0 0 6px 0; color: #6cf; font-size: 16px; }
  #info .group-tag { color: #fc6; font-size: 11px; text-transform: uppercase;
                     letter-spacing: 0.5px; margin-bottom: 8px; }
  #info p { margin: 4px 0 10px 0; color: #ccc; }
  #info h4 { margin: 8px 0 4px 0; color: #aaa; font-size: 11px;
             text-transform: uppercase; letter-spacing: 0.5px; }
  #info ul { padding-left: 18px; margin: 2px 0 6px 0; }
  #info li { font-size: 12px; color: #aaa; word-break: break-all; }
  #info code { background: #333; padding: 1px 4px; border-radius: 2px; color: #cfc; }

  #search { position: fixed; top: 10px; left: 10px; padding: 8px 10px;
            background: rgba(30,30,30,0.97); border: 1px solid #555;
            border-radius: 4px; color: #ddd; width: 220px; font-size: 13px; }
  #search:focus { outline: none; border-color: #6cf; }

  .legend { position: fixed; bottom: 10px; left: 10px; padding: 10px;
            background: rgba(30,30,30,0.97); border: 1px solid #555;
            border-radius: 4px; font-size: 11px; }
  .legend .row { margin: 3px 0; cursor: pointer; }
  .legend .row.dimmed { opacity: 0.3; }
  .legend .swatch { display: inline-block; width: 12px; height: 12px;
                    margin-right: 6px; vertical-align: middle;
                    border: 1px solid rgba(255,255,255,0.3); }

  .stats { position: fixed; bottom: 10px; right: 10px; padding: 8px;
           background: rgba(30,30,30,0.97); border: 1px solid #555;
           border-radius: 4px; font-size: 11px; color: #aaa; }
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
      applyFilter();
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
  .on('click', (e,d) => showInfo(d))
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

function showInfo(d) {
  const info = d3.select('#info');
  let html = `<h3>${d.label}</h3><div class="group-tag">${d.group}</div>`;
  if (d.description) html += `<p>${d.description}</p>`;
  if (d.files && d.files.length) {
    html += `<h4>Files (${d.files.length})</h4><ul>`;
    d.files.forEach(f => html += `<li><code>${f}</code></li>`);
    html += `</ul>`;
  }
  if (d.types && d.types.length) {
    html += `<h4>Key Types</h4><ul>`;
    d.types.forEach(t => html += `<li><code>${t}</code></li>`);
    html += `</ul>`;
  }
  if (d.functions && d.functions.length) {
    html += `<h4>Public Functions</h4><ul>`;
    d.functions.forEach(f => html += `<li><code>${f}</code></li>`);
    html += `</ul>`;
  }
  // Show neighbors.
  const neighbors = DATA.links
    .map(l => {
      const s = typeof l.source === 'object' ? l.source.id : l.source;
      const t = typeof l.target === 'object' ? l.target.id : l.target;
      if (s === d.id) return {dir: '->', other: t, type: l.type};
      if (t === d.id) return {dir: '<-', other: s, type: l.type};
      return null;
    }).filter(x => x);
  if (neighbors.length) {
    html += `<h4>Connections</h4><ul>`;
    neighbors.forEach(n => html += `<li>${n.dir} <code>${n.other}</code> <small>(${n.type || 'link'})</small></li>`);
    html += `</ul>`;
  }
  info.html(html);
}

function highlightNeighbors(d, on) {
  link.classed('highlight', l => {
    const s = typeof l.source === 'object' ? l.source.id : l.source;
    const t = typeof l.target === 'object' ? l.target.id : l.target;
    return on && (s === d.id || t === d.id);
  });
}

let searchQuery = '';
function applyFilter() {
  const q = searchQuery.toLowerCase();
  node.selectAll('circle').attr('opacity', d => {
    if (dimmedGroups.has(d.group)) return 0.1;
    if (!q) return 1;
    const hay = (d.label + ' ' + d.id + ' ' + (d.types||[]).join(' ') + ' '
                 + (d.functions||[]).join(' ') + ' ' + (d.files||[]).join(' '));
    return hay.toLowerCase().includes(q) ? 1 : 0.15;
  });
  node.selectAll('text').attr('opacity', d => dimmedGroups.has(d.group) ? 0.1 : 1);
}
document.getElementById('search').addEventListener('input', (e) => {
  searchQuery = e.target.value;
  applyFilter();
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
