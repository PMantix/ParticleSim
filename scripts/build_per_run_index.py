"""Build an HTML index page that displays every per-run summary plot
inline, sortable by amplitude or frequency, with key metadata.
"""
from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LOG_DIR = REPO_ROOT / "doe_results" / "eis_doe_lf"
PNG_DIR = REPO_ROOT / "images" / "eis_validation_runs" / "per_run"
OUT = REPO_ROOT / "images" / "eis_validation_runs" / "per_run_index.html"

QUICK_RE = re.compile(
    r"^\s*\d+\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.+\-]+)°\s+([\d.]+)\s+([\d.]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s*$",
    re.MULTILINE,
)


def parse_log_summary(log_path: Path):
    text = log_path.read_text()
    rows = []
    for m in QUICK_RE.finditer(text):
        f, zr, zi, zm, ph, r2v, r2i, va, ia = m.groups()
        rows.append({
            "frequency": float(f),
            "z_real": float(zr),
            "z_imag": float(zi),
            "magnitude": float(zm),
            "phase_deg": float(ph),
            "fit_r2_v": float(r2v),
            "fit_v_amp": float(va),
            "fit_i_amp": float(ia),
        })
    if not rows:
        return None
    iamp = rows[0]["fit_i_amp"]
    fmin = min(r["frequency"] for r in rows)
    fmax = max(r["frequency"] for r in rows)
    meanr2 = sum(r["fit_r2_v"] for r in rows) / len(rows)
    return {
        "name": log_path.stem,
        "i_amp": iamp,
        "n_points": len(rows),
        "fmin": fmin,
        "fmax": fmax,
        "mean_r2": meanr2,
        "rows": rows,
    }


def main():
    summaries = []
    for log in sorted(LOG_DIR.glob("*.log")):
        s = parse_log_summary(log)
        if s:
            summaries.append(s)

    # Sort by amplitude, then frequency
    summaries.sort(key=lambda s: (s["i_amp"], s["fmin"]))

    parts = ["""<!doctype html>
<html><head><meta charset="utf-8"><title>EIS DOE — per-run plots</title>
<style>
body { font-family: -apple-system, sans-serif; margin: 20px; background: #f8f8f8; }
h1 { font-size: 18px; }
.toolbar { position: sticky; top: 0; background: #f8f8f8; padding: 10px 0; border-bottom: 1px solid #ccc; z-index: 100; }
.toolbar button { margin: 0 4px; padding: 6px 10px; cursor: pointer; }
.run { margin: 20px 0; padding: 12px; background: white; border: 1px solid #ddd; border-radius: 4px; }
.run h3 { margin: 0 0 6px 0; font-size: 14px; font-family: monospace; }
.run .meta { font-family: monospace; font-size: 11px; color: #555; margin-bottom: 6px; }
.run table { font-family: monospace; font-size: 10px; border-collapse: collapse; margin-bottom: 8px; }
.run table td, .run table th { padding: 2px 6px; text-align: right; border: 1px solid #eee; }
.run table th { background: #f0f0f0; }
.run img { max-width: 100%; height: auto; border: 1px solid #ccc; }
.r2-good { background: #d4f4d4; }
.r2-mid { background: #fff4cc; }
.r2-bad { background: #ffd4d4; }
.amp-band { font-weight: bold; padding: 8px; margin: 16px 0 4px 0; background: #eef; border-left: 4px solid #88a; }
</style></head><body>
<h1>EIS DOE — per-run summary plots</h1>
<div class="toolbar">
<button onclick="document.querySelectorAll('.run').forEach(e=>e.style.display='block')">show all</button>
<button onclick="filterAmp(0.0006)">I≤0.001</button>
<button onclick="filterAmp(0.01)">I≤0.01</button>
<button onclick="filterAmp(0.1)">I≤0.1</button>
<button onclick="filterAmp(1)">I≤1</button>
<script>
function filterAmp(maxAmp) {
  document.querySelectorAll('.run').forEach(e => {
    const a = parseFloat(e.dataset.iamp);
    e.style.display = (a <= maxAmp) ? 'block' : 'none';
  });
}
</script>
</div>
"""]

    last_amp = None
    for s in summaries:
        if s["i_amp"] != last_amp:
            parts.append(f'<div class="amp-band">I_amp = {s["i_amp"]:.3e}</div>')
            last_amp = s["i_amp"]

        # Mean R² class
        if s["mean_r2"] >= 0.95:
            r2_cls = "r2-good"
        elif s["mean_r2"] >= 0.85:
            r2_cls = "r2-mid"
        else:
            r2_cls = "r2-bad"

        parts.append(f'<div class="run" data-iamp="{s["i_amp"]}">')
        parts.append(f'<h3>{s["name"]}</h3>')
        parts.append(f'<div class="meta">I={s["i_amp"]:.3e} · '
                     f'f∈[{s["fmin"]:.2e}, {s["fmax"]:.2e}] · '
                     f'{s["n_points"]} pts · '
                     f'<span class="{r2_cls}">mean R²(V) = {s["mean_r2"]:.3f}</span></div>')

        # Per-point table
        parts.append('<table><tr><th>f</th><th>Re(Z)</th><th>−Im(Z)</th><th>|Z|</th>'
                     '<th>phase</th><th>R²(V)</th><th>V_amp</th></tr>')
        for r in s["rows"]:
            r2 = r["fit_r2_v"]
            cls = "r2-good" if r2 >= 0.95 else ("r2-mid" if r2 >= 0.85 else "r2-bad")
            parts.append(
                f'<tr><td>{r["frequency"]:.2e}</td>'
                f'<td>{r["z_real"]:+.2e}</td>'
                f'<td>{-r["z_imag"]:+.2e}</td>'
                f'<td>{r["magnitude"]:.2e}</td>'
                f'<td>{r["phase_deg"]:+.1f}°</td>'
                f'<td class="{cls}">{r2:.3f}</td>'
                f'<td>{r["fit_v_amp"]*1000:.0f} mV</td></tr>'
            )
        parts.append('</table>')

        png_path = f"per_run/{s['name']}.png"
        parts.append(f'<img src="{png_path}" loading="lazy" />')
        parts.append('</div>')

    parts.append('</body></html>')
    OUT.write_text("\n".join(parts))
    print(f"wrote {OUT}")
    print(f"open with: open '{OUT}'")


if __name__ == "__main__":
    main()
