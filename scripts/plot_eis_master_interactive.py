"""Interactive HTML EIS master plot — one trace per applied amplitude.

Companion to plot_eis_master.py. Same data, same logic, but renders to
self-contained HTML for browser zoom/pan/hover.
"""
from __future__ import annotations

import argparse
import math
import re
import sys
from pathlib import Path
from typing import Dict, List

import plotly.graph_objects as go
from plotly.subplots import make_subplots

REPO_ROOT = Path(__file__).resolve().parent.parent

QUICK_RE = re.compile(
    r"^\s*\d+\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.+\-]+)°\s+([\d.]+)\s+([\d.]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s*$",
    re.MULTILINE,
)


def parse_log(path: Path):
    text = path.read_text()
    points = []
    for m in QUICK_RE.finditer(text):
        f, zr, zi, zm, ph, r2v, r2i, va, ia = m.groups()
        points.append({
            "frequency": float(f),
            "z_real": float(zr),
            "z_imag": float(zi),
            "magnitude": float(zm),
            "phase_deg": float(ph),
            "fit_r2_v": float(r2v),
            "fit_r2_i": float(r2i),
            "fit_v_amp": float(va),
            "fit_i_amp": float(ia),
            "src": path.stem,
        })
    return points


def viridis_hex(t: float) -> str:
    """Approximate viridis colormap as hex string for plotly."""
    import matplotlib.cm as cm
    import matplotlib.colors as mcolors
    rgba = cm.viridis(t)
    return mcolors.to_hex(rgba)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("logs", nargs="*", help="log files; default: doe_results/eis_doe_lf/*.log")
    ap.add_argument("--out", default=str(REPO_ROOT / "images" / "eis_validation_runs" / "master_nyquist.html"))
    args = ap.parse_args()

    if args.logs:
        paths = [Path(p) for p in args.logs]
    else:
        paths = sorted((REPO_ROOT / "doe_results" / "eis_doe_lf").glob("*.log"))
    if not paths:
        print("No log files found.", file=sys.stderr)
        sys.exit(1)

    all_points: List[Dict] = []
    for p in paths:
        all_points.extend(parse_log(p))
    print(f"Parsed {len(all_points)} points from {len(paths)} log files.")

    by_amp: Dict[float, List[Dict]] = {}
    for p in all_points:
        by_amp.setdefault(round(p["fit_i_amp"], 9), []).append(p)
    for pts in by_amp.values():
        pts.sort(key=lambda p: p["frequency"])

    fig = make_subplots(
        rows=2, cols=2,
        specs=[[{"rowspan": 2}, {}], [None, {}]],
        subplot_titles=(
            "Nyquist — one trace per amplitude",
            "Bode |Z|",
            "R²(V) vs frequency",
        ),
        column_widths=[0.45, 0.55],
        horizontal_spacing=0.08, vertical_spacing=0.12,
    )

    amps_sorted = sorted(by_amp.keys())
    n_amp = len(amps_sorted)

    for i, amp in enumerate(amps_sorted):
        pts = by_amp[amp]
        color = viridis_hex(i / max(1, n_amp - 1))
        label = f"I={amp:.2e}"
        legendgroup = f"amp_{amp}"

        # Nyquist
        fig.add_trace(go.Scatter(
            x=[p["z_real"] for p in pts],
            y=[-p["z_imag"] for p in pts],
            mode="lines+markers",
            marker=dict(size=8, color=color, line=dict(color="black", width=0.4)),
            line=dict(color=color, width=1.5),
            name=label,
            legendgroup=legendgroup,
            customdata=[(p["frequency"], p["fit_r2_v"], p["fit_v_amp"], p["src"]) for p in pts],
            hovertemplate=(
                "f = %{customdata[0]:.3e} /fs<br>"
                "Re(Z) = %{x:.4e}<br>−Im(Z) = %{y:.4e}<br>"
                "R²(V) = %{customdata[1]:.4f}<br>V_amp = %{customdata[2]:.3e}<br>"
                "src = %{customdata[3]}<extra>" + label + "</extra>"
            ),
        ), row=1, col=1)

        # Bode |Z|
        fig.add_trace(go.Scatter(
            x=[p["frequency"] for p in pts],
            y=[p["magnitude"] for p in pts],
            mode="lines+markers",
            marker=dict(size=6, color=color),
            line=dict(color=color, width=1.5),
            name=label,
            legendgroup=legendgroup, showlegend=False,
            customdata=[(p["phase_deg"], p["fit_r2_v"], p["fit_v_amp"], p["src"]) for p in pts],
            hovertemplate=(
                "f = %{x:.3e} /fs<br>|Z| = %{y:.3e}<br>"
                "phase = %{customdata[0]:+.1f}°<br>R²(V) = %{customdata[1]:.4f}<br>"
                "V_amp = %{customdata[2]:.3e}<br>src = %{customdata[3]}<extra>" + label + "</extra>"
            ),
        ), row=1, col=2)

        # R²
        fig.add_trace(go.Scatter(
            x=[p["frequency"] for p in pts],
            y=[p["fit_r2_v"] for p in pts],
            mode="lines+markers",
            marker=dict(size=6, color=color),
            line=dict(color=color, width=1.5),
            name=label,
            legendgroup=legendgroup, showlegend=False,
            customdata=[(p["magnitude"], p["fit_v_amp"], p["src"]) for p in pts],
            hovertemplate=(
                "f = %{x:.3e} /fs<br>R²(V) = %{y:.4f}<br>"
                "|Z| = %{customdata[0]:.3e}<br>V_amp = %{customdata[1]:.3e}<br>"
                "src = %{customdata[2]}<extra>" + label + "</extra>"
            ),
        ), row=2, col=2)

    fig.add_hline(y=0.95, line=dict(color="green", dash="dash", width=1), row=2, col=2)
    fig.add_hline(y=0.85, line=dict(color="red", dash="dash", width=1), row=2, col=2)

    fig.update_xaxes(title_text="Re(Z)", row=1, col=1, zeroline=True, zerolinecolor="rgba(0,0,0,0.3)")
    fig.update_yaxes(title_text="−Im(Z)", row=1, col=1, zeroline=True, zerolinecolor="rgba(0,0,0,0.3)")
    fig.update_xaxes(title_text="frequency (1/fs)", type="log", row=1, col=2)
    fig.update_yaxes(title_text="|Z|", type="log", row=1, col=2)
    fig.update_xaxes(title_text="frequency (1/fs)", type="log", row=2, col=2)
    fig.update_yaxes(title_text="R²(V)", row=2, col=2, range=[0, 1.05])

    fig.update_layout(
        title=f"EIS DOE master — {len(paths)} jobs · {len(all_points)} pts · {n_amp} amplitudes "
              f"<br><sub>Where amplitude traces overlap on Nyquist = linear regime; where they fan out = amplitude-dependence (Phase 5 signal)</sub>",
        height=900,
        hovermode="closest",
        legend=dict(orientation="v", yanchor="top", y=1.0, xanchor="left", x=1.02),
    )

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(str(out), include_plotlyjs="cdn")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
