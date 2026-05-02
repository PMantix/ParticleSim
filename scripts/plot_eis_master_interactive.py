"""Interactive HTML Nyquist + Bode + R² master plot from EIS DOE logs.

Mirrors plot_eis_master.py but renders to a single self-contained HTML
file viewable in any browser with full zoom/pan/hover.

Usage:
    python scripts/plot_eis_master_interactive.py
    python scripts/plot_eis_master_interactive.py --out custom.html
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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("logs", nargs="*", help="log files; default: doe_results/eis_doe_lf/*.log")
    ap.add_argument("--out", default=str(REPO_ROOT / "images" / "eis_validation_runs" / "master_nyquist.html"))
    ap.add_argument("--r2-floor", type=float, default=0.85)
    ap.add_argument("--vamp-min", type=float, default=0.030, help="V_amp lower bound (V); below this fits are noise-dominated")
    ap.add_argument("--vamp-max", type=float, default=0.150, help="V_amp upper bound (V); above this nonlinearity inflates Re/|Z|")
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

    # Group by frequency, pick best R² per freq for "best" trace
    by_freq: Dict[float, List[Dict]] = {}
    for p in all_points:
        key = round(math.log10(p["frequency"]), 2)
        by_freq.setdefault(key, []).append(p)
    best: List[Dict] = []
    for key in sorted(by_freq.keys()):
        pts = by_freq[key]
        ideal = [pp for pp in pts
                 if pp["fit_r2_v"] >= args.r2_floor
                 and args.vamp_min <= pp["fit_v_amp"] <= args.vamp_max]
        if ideal:
            best.append(max(ideal, key=lambda p: p["fit_r2_v"]))
            continue
        relaxed = [pp for pp in pts if pp["fit_r2_v"] >= args.r2_floor]
        if relaxed:
            chosen = max(relaxed, key=lambda p: p["fit_r2_v"])
            chosen["_fallback"] = True
            best.append(chosen)
    best.sort(key=lambda p: p["frequency"])

    # Group all points by amplitude for the Bode and R² panels
    by_amp: Dict[float, List[Dict]] = {}
    for p in all_points:
        by_amp.setdefault(round(p["fit_i_amp"], 8), []).append(p)
    for pts in by_amp.values():
        pts.sort(key=lambda p: p["frequency"])

    fig = make_subplots(
        rows=2, cols=2,
        specs=[
            [{"rowspan": 2}, {}],
            [None, {}],
        ],
        subplot_titles=(
            "Nyquist (best R² ≥ {} per freq)".format(args.r2_floor),
            "Bode |Z| (all amplitudes; ★ = best)",
            "R²(V) vs frequency, by amplitude",
        ),
        column_widths=[0.45, 0.55],
        horizontal_spacing=0.08, vertical_spacing=0.12,
    )

    # ── Nyquist — separate in-range (connected) from fallback (X markers) ──
    in_range = [p for p in best if not p.get("_fallback")]
    fallback = [p for p in best if p.get("_fallback")]
    if in_range:
        fig.add_trace(go.Scatter(
            x=[p["z_real"] for p in in_range],
            y=[-p["z_imag"] for p in in_range],
            mode="lines+markers",
            marker=dict(
                size=11,
                color=[math.log10(p["frequency"]) for p in in_range],
                colorscale="Viridis",
                colorbar=dict(title="log₁₀ f", x=0.42, len=0.95),
                line=dict(color="black", width=1),
            ),
            line=dict(color="rgba(0,0,0,0.5)", width=1, dash="dash"),
            customdata=[(p["frequency"], p["fit_r2_v"], p["fit_v_amp"],
                         p["fit_i_amp"], p["src"]) for p in in_range],
            hovertemplate=(
                "f = %{customdata[0]:.3e} /fs<br>"
                "Re(Z) = %{x:.4e}<br>−Im(Z) = %{y:.4e}<br>"
                "R²(V) = %{customdata[1]:.4f}<br>"
                "V_amp = %{customdata[2]:.3e}<br>"
                "I_amp = %{customdata[3]:.3e}<br>"
                "src = %{customdata[4]}<extra>linear-regime</extra>"
            ),
            name="linear-regime",
        ), row=1, col=1)
    if fallback:
        fig.add_trace(go.Scatter(
            x=[p["z_real"] for p in fallback],
            y=[-p["z_imag"] for p in fallback],
            mode="markers",
            marker=dict(size=8, symbol="x", color="rgba(120,120,120,0.55)"),
            customdata=[(p["frequency"], p["fit_r2_v"], p["fit_v_amp"],
                         p["fit_i_amp"], p["src"]) for p in fallback],
            hovertemplate=(
                "f = %{customdata[0]:.3e} /fs<br>"
                "Re(Z) = %{x:.4e}<br>−Im(Z) = %{y:.4e}<br>"
                "R²(V) = %{customdata[1]:.4f}<br>"
                "V_amp = %{customdata[2]:.3e}<br>"
                "I_amp = %{customdata[3]:.3e}<br>"
                "src = %{customdata[4]}<extra>V_amp out of range</extra>"
            ),
            name="V_amp out of range",
        ), row=1, col=1)
    for p in in_range:
        fig.add_annotation(
            x=p["z_real"], y=-p["z_imag"],
            text=f"  {p['frequency']:.1e}",
            showarrow=False, xanchor="left", yanchor="middle",
            font=dict(size=9), row=1, col=1,
        )

    # ── Bode |Z| — one trace per amplitude + best overlay ──
    palette = ["#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd",
               "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf"]
    for i, (amp, pts) in enumerate(sorted(by_amp.items())):
        fig.add_trace(go.Scatter(
            x=[p["frequency"] for p in pts],
            y=[p["magnitude"] for p in pts],
            mode="lines+markers",
            marker=dict(size=6, color=palette[i % 10]),
            line=dict(color=palette[i % 10], width=1),
            name=f"I={amp:g}",
            legendgroup=f"amp_{amp}",
            customdata=[(p["fit_r2_v"], p["phase_deg"], p["fit_v_amp"], p["src"])
                        for p in pts],
            hovertemplate=(
                "f = %{x:.3e} /fs<br>|Z| = %{y:.3e}<br>"
                "phase = %{customdata[1]:+.1f}°<br>"
                "R²(V) = %{customdata[0]:.4f}<br>"
                "V_amp = %{customdata[2]:.3e}<br>"
                "src = %{customdata[3]}<extra>I=" + f"{amp:g}" + "</extra>"
            ),
        ), row=1, col=2)
    if best:
        fig.add_trace(go.Scatter(
            x=[p["frequency"] for p in best],
            y=[p["magnitude"] for p in best],
            mode="lines+markers",
            marker=dict(size=12, symbol="star", color="black",
                        line=dict(color="white", width=1)),
            line=dict(color="black", width=2),
            name="best",
            customdata=[(p["fit_r2_v"], p["phase_deg"], p["src"]) for p in best],
            hovertemplate=(
                "BEST f = %{x:.3e} /fs<br>|Z| = %{y:.3e}<br>"
                "phase = %{customdata[1]:+.1f}°<br>"
                "R²(V) = %{customdata[0]:.4f}<br>"
                "src = %{customdata[2]}<extra></extra>"
            ),
        ), row=1, col=2)

    # ── R²(V) vs freq, by amplitude ──
    for i, (amp, pts) in enumerate(sorted(by_amp.items())):
        fig.add_trace(go.Scatter(
            x=[p["frequency"] for p in pts],
            y=[p["fit_r2_v"] for p in pts],
            mode="lines+markers",
            marker=dict(size=6, color=palette[i % 10]),
            line=dict(color=palette[i % 10], width=1),
            name=f"I={amp:g}",
            legendgroup=f"amp_{amp}",
            showlegend=False,
            customdata=[(p["magnitude"], p["fit_v_amp"], p["src"]) for p in pts],
            hovertemplate=(
                "f = %{x:.3e} /fs<br>R²(V) = %{y:.4f}<br>"
                "|Z| = %{customdata[0]:.3e}<br>"
                "V_amp = %{customdata[1]:.3e}<br>"
                "src = %{customdata[2]}<extra>I=" + f"{amp:g}" + "</extra>"
            ),
        ), row=2, col=2)
    fig.add_hline(y=args.r2_floor, line=dict(color="red", dash="dash", width=1),
                  annotation_text=f"floor={args.r2_floor}", annotation_position="bottom right",
                  row=2, col=2)
    fig.add_hline(y=0.95, line=dict(color="green", dash="dash", width=1),
                  annotation_text="0.95", annotation_position="bottom right",
                  row=2, col=2)

    # Axes
    fig.update_xaxes(title_text="Re(Z)", row=1, col=1, zeroline=True, zerolinecolor="rgba(0,0,0,0.3)")
    fig.update_yaxes(title_text="−Im(Z)", row=1, col=1, zeroline=True, zerolinecolor="rgba(0,0,0,0.3)")
    fig.update_xaxes(title_text="frequency (1/fs)", type="log", row=1, col=2)
    fig.update_yaxes(title_text="|Z|", type="log", row=1, col=2)
    fig.update_xaxes(title_text="frequency (1/fs)", type="log", row=2, col=2)
    fig.update_yaxes(title_text="R²(V)", row=2, col=2, range=[0, 1.05])

    fig.update_layout(
        title=f"EIS DOE master ({len(paths)} jobs · {len(all_points)} raw pts · {len(best)} best)",
        height=800,
        hovermode="closest",
        legend=dict(orientation="v", yanchor="top", y=1.0, xanchor="left", x=1.02),
    )

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(str(out), include_plotlyjs="cdn")
    print(f"wrote {out}")
    print(f"open with: open '{out}'")


if __name__ == "__main__":
    main()
