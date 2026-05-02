"""Stitch a master Nyquist + Bode from multiple eis_quick_sweep / test logs.

Reads every log file passed as an argument (or every log in
doe_results/eis_doe_lf/), parses the EisPoints table, picks the
highest-R²(V) point at each frequency (across all jobs), and renders:
  - Nyquist (Re vs -Im, color = log f)
  - Bode |Z| vs f (with all amplitudes overlaid as scatter, "best" highlighted)
  - Bode phase vs f
  - R² vs f, faceted by amplitude
"""
from __future__ import annotations

import argparse
import math
import re
import sys
from pathlib import Path
from typing import Dict, List

import matplotlib.pyplot as plt
import numpy as np

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
    ap.add_argument("--out", default=str(REPO_ROOT / "images" / "eis_validation_runs" / "master_nyquist.png"))
    ap.add_argument("--r2-floor", type=float, default=0.85, help="discard points with R²(V) below this from 'best' selection")
    ap.add_argument("--vamp-min", type=float, default=0.030, help="V_amp lower bound (V) — below this, fits are noise-dominated")
    ap.add_argument("--vamp-max", type=float, default=0.150, help="V_amp upper bound (V) — above this, nonlinearity inflates |Z| and Re(Z)")
    ap.add_argument("--phase-min", type=float, default=-100.0, help="Phase floor (deg) — below this is the saturation regime (V lags I by >90°), unphysical for passive cell")
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
    if not all_points:
        print("No EisPoints parsed.", file=sys.stderr)
        sys.exit(1)
    print(f"Parsed {len(all_points)} points from {len(paths)} log files.")

    # Group by frequency (round to nearest 1% to merge near-duplicates)
    by_freq: Dict[float, List[Dict]] = {}
    for p in all_points:
        # Bucket key — log10(f) rounded to 2 decimals merges 5.000e-4 and 5.014e-4 etc.
        key = round(math.log10(p["frequency"]), 2)
        by_freq.setdefault(key, []).append(p)

    # Pick best per frequency: prefer points in linear V_amp range with R² >= floor.
    # Fall back to relaxed criteria only if no point clears the bar.
    best: List[Dict] = []
    for key in sorted(by_freq.keys()):
        pts = by_freq[key]
        ideal = [p for p in pts
                 if p["fit_r2_v"] >= args.r2_floor
                 and args.vamp_min <= p["fit_v_amp"] <= args.vamp_max
                 and p["phase_deg"] >= args.phase_min]
        if ideal:
            best.append(max(ideal, key=lambda p: p["fit_r2_v"]))
            continue
        # Fallback: highest R² regardless of V_amp / phase (flagged in stdout)
        relaxed = [p for p in pts if p["fit_r2_v"] >= args.r2_floor]
        if relaxed:
            chosen = max(relaxed, key=lambda p: p["fit_r2_v"])
            chosen["_fallback"] = True
            best.append(chosen)

    # Sort best by frequency for plotting
    best.sort(key=lambda p: p["frequency"])
    print(f"\n{len(best)} 'best' points (R²(V) ≥ {args.r2_floor}, V_amp ∈ [{args.vamp_min}, {args.vamp_max}] V; * = fallback outside V_amp range):")
    print(f"  {'f':>10}  {'Re(Z)':>10}  {'-Im(Z)':>10}  {'|Z|':>10}  {'phase':>8}  {'R²(V)':>7}  {'V_amp':>10}  {'I_amp':>10}  src")
    for p in best:
        flag = "*" if p.get("_fallback") else " "
        print(f"  {flag} {p['frequency']:>10.3e}  {p['z_real']:>+10.3e}  {-p['z_imag']:>10.3e}  "
              f"{p['magnitude']:>10.3e}  {p['phase_deg']:>+7.1f}°  {p['fit_r2_v']:>7.4f}  "
              f"{p['fit_v_amp']:>10.3e}  {p['fit_i_amp']:>10.3e}  {p['src']}")

    # Plot
    fig = plt.figure(figsize=(15, 10))
    gs = fig.add_gridspec(3, 3, hspace=0.5, wspace=0.35)
    ax_nyq = fig.add_subplot(gs[0:2, 0])
    ax_mag = fig.add_subplot(gs[0, 1:])
    ax_phs = fig.add_subplot(gs[1, 1:])
    ax_r2 = fig.add_subplot(gs[2, :])

    # Nyquist — separate in-range (solid, connected) from fallback (X, faded)
    in_range = [p for p in best if not p.get("_fallback")]
    fallback = [p for p in best if p.get("_fallback")]
    if in_range:
        re_z = np.array([p["z_real"] for p in in_range])
        im_z = np.array([-p["z_imag"] for p in in_range])
        freqs = np.array([p["frequency"] for p in in_range])
        # Connect only the in-range points so the trace reflects clean physics
        ax_nyq.plot(re_z, im_z, "k--", lw=1.0, alpha=0.7, zorder=2)
        sc = ax_nyq.scatter(re_z, im_z, c=np.log10(freqs), cmap="viridis",
                            s=90, edgecolor="k", zorder=4, label="linear-regime")
        for p in in_range:
            ax_nyq.annotate(f"  {p['frequency']:.2e}",
                            (p["z_real"], -p["z_imag"]),
                            fontsize=7, va="center", ha="left")
        plt.colorbar(sc, ax=ax_nyq, label="log₁₀ f (1/fs)")
    if fallback:
        re_z_fb = np.array([p["z_real"] for p in fallback])
        im_z_fb = np.array([-p["z_imag"] for p in fallback])
        ax_nyq.scatter(re_z_fb, im_z_fb, marker="x", s=50, c="gray",
                       alpha=0.5, zorder=3, label="V_amp out of range")
    ax_nyq.axhline(0, color="k", lw=0.5)
    ax_nyq.axvline(0, color="k", lw=0.5)
    ax_nyq.set_xlabel("Re(Z)")
    ax_nyq.set_ylabel("−Im(Z)")
    ax_nyq.set_title(f"Master Nyquist — {len(in_range)} linear-regime ★ + {len(fallback)} fallback ✕")
    ax_nyq.grid(alpha=0.3)
    if fallback:
        ax_nyq.legend(loc="best", fontsize=8)

    # Bode |Z| — all points scatter, best highlighted
    by_amp: Dict[float, List[Dict]] = {}
    for p in all_points:
        by_amp.setdefault(round(p["fit_i_amp"], 6), []).append(p)
    cmap = plt.cm.tab10
    for i, (amp, pts) in enumerate(sorted(by_amp.items())):
        pts.sort(key=lambda p: p["frequency"])
        fs = [p["frequency"] for p in pts]
        zs = [p["magnitude"] for p in pts]
        ax_mag.loglog(fs, zs, "o-", color=cmap(i % 10), alpha=0.6, lw=0.8, label=f"I={amp:g}")
    if best:
        bf = [p["frequency"] for p in best]
        bz = [p["magnitude"] for p in best]
        ax_mag.loglog(bf, bz, "k*-", lw=1.5, ms=10, label="best")
    ax_mag.set_xlabel("frequency (1/fs)")
    ax_mag.set_ylabel("|Z|")
    ax_mag.set_title("Bode magnitude — all amplitudes (★ = best)")
    ax_mag.grid(alpha=0.3, which="both")
    ax_mag.legend(loc="best", fontsize=8)

    # Bode phase — best only
    if best:
        ax_phs.semilogx(bf, [p["phase_deg"] for p in best], "ko-", lw=1)
        for h in (-90, -180, 0):
            ax_phs.axhline(h, color="gray", lw=0.5, ls="--", alpha=0.5)
    ax_phs.set_xlabel("frequency (1/fs)")
    ax_phs.set_ylabel("phase (deg)")
    ax_phs.set_title("Bode phase (best points)")
    ax_phs.grid(alpha=0.3, which="both")

    # R² heatmap-ish: faceted by amplitude
    for i, (amp, pts) in enumerate(sorted(by_amp.items())):
        pts.sort(key=lambda p: p["frequency"])
        fs = [p["frequency"] for p in pts]
        r2s = [p["fit_r2_v"] for p in pts]
        ax_r2.semilogx(fs, r2s, "o-", color=cmap(i % 10), alpha=0.7, lw=1, label=f"I={amp:g}")
    ax_r2.axhline(args.r2_floor, color="r", lw=0.8, ls="--", alpha=0.5, label=f"floor={args.r2_floor}")
    ax_r2.axhline(0.95, color="g", lw=0.8, ls="--", alpha=0.5, label="0.95")
    ax_r2.set_xlabel("frequency (1/fs)")
    ax_r2.set_ylabel("R²(V)")
    ax_r2.set_title("Fit quality vs frequency, by amplitude")
    ax_r2.set_ylim(0, 1.05)
    ax_r2.grid(alpha=0.3, which="both")
    ax_r2.legend(loc="best", fontsize=7, ncol=4)

    fig.suptitle(f"EIS DOE master — {len(paths)} jobs, {len(all_points)} raw points, {len(best)} best", fontsize=11)
    fig.savefig(args.out, dpi=120, bbox_inches="tight")
    plt.close(fig)
    print(f"\nwrote {args.out}")


if __name__ == "__main__":
    main()
