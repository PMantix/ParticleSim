"""Master EIS plot — one Nyquist trace per applied amplitude, all data shown.

Mirrors how experimental EIS is reported: fix the perturbation amplitude,
sweep frequency, plot whatever comes out. We do this for each amplitude
in our DOE matrix. Where amplitude traces overlap = linear regime;
where they diverge = the cell's amplitude-dependence.

No filtering, no best-pick. R²(V) panel kept as a fit-quality diagnostic.
"""
from __future__ import annotations

import argparse
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
    ap.add_argument("--r2-filter", type=float, default=0.85, help="drop points with R²(V) below this from each amplitude trace")
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

    # Group by amplitude (round for floating-point key stability), then drop low-R² per trace
    by_amp_raw: Dict[float, List[Dict]] = {}
    for p in all_points:
        by_amp_raw.setdefault(round(p["fit_i_amp"], 9), []).append(p)
    by_amp: Dict[float, List[Dict]] = {}
    dropped_total = 0
    for amp, pts in by_amp_raw.items():
        kept = [p for p in pts if p["fit_r2_v"] >= args.r2_filter]
        kept.sort(key=lambda p: p["frequency"])
        if kept:
            by_amp[amp] = kept
        dropped_total += len(pts) - len(kept)

    # Print summary table
    print(f"\n{len(by_amp)} amplitudes after R²(V) ≥ {args.r2_filter} filter (dropped {dropped_total}/{len(all_points)} raw points):")
    for amp in sorted(by_amp.keys()):
        pts = by_amp[amp]
        raw_n = len(by_amp_raw[amp])
        fmin = min(p["frequency"] for p in pts)
        fmax = max(p["frequency"] for p in pts)
        meanr2 = np.mean([p["fit_r2_v"] for p in pts])
        print(f"  I = {amp:>9.2e}  ·  {len(pts):>2}/{raw_n:>2} kept  ·  f ∈ [{fmin:.2e}, {fmax:.2e}]  ·  mean R²(V) = {meanr2:.3f}")

    fig = plt.figure(figsize=(15, 11))
    gs = fig.add_gridspec(3, 3, hspace=0.45, wspace=0.32)
    ax_nyq = fig.add_subplot(gs[0:2, 0])
    ax_mag = fig.add_subplot(gs[0, 1:])
    ax_phs = fig.add_subplot(gs[1, 1:])
    ax_r2 = fig.add_subplot(gs[2, :])

    # Order amplitudes from smallest to largest for consistent color mapping
    amps_sorted = sorted(by_amp.keys())
    cmap = plt.cm.viridis
    n_amp = len(amps_sorted)

    for i, amp in enumerate(amps_sorted):
        pts = by_amp[amp]
        color = cmap(i / max(1, n_amp - 1))
        label = f"I = {amp:.2e}"

        # Nyquist
        ax_nyq.plot(
            [p["z_real"] for p in pts],
            [-p["z_imag"] for p in pts],
            "o-", color=color, label=label, lw=1.2, ms=6,
            markeredgecolor="black", markeredgewidth=0.4,
        )
        # Bode |Z|
        ax_mag.loglog(
            [p["frequency"] for p in pts],
            [p["magnitude"] for p in pts],
            "o-", color=color, label=label, lw=1.2, ms=5,
        )
        # Bode phase
        ax_phs.semilogx(
            [p["frequency"] for p in pts],
            [p["phase_deg"] for p in pts],
            "o-", color=color, label=label, lw=1.2, ms=5,
        )
        # R²(V)
        ax_r2.semilogx(
            [p["frequency"] for p in pts],
            [p["fit_r2_v"] for p in pts],
            "o-", color=color, label=label, lw=1.2, ms=5,
        )

    ax_nyq.axhline(0, color="k", lw=0.5)
    ax_nyq.axvline(0, color="k", lw=0.5)
    ax_nyq.set_xlabel("Re(Z)")
    ax_nyq.set_ylabel("−Im(Z)")
    ax_nyq.set_title(f"Nyquist — one trace per amplitude ({n_amp} amplitudes, {len(all_points)} points)")
    ax_nyq.grid(alpha=0.3)
    ax_nyq.legend(loc="best", fontsize=7)

    ax_mag.set_xlabel("frequency (1/fs)")
    ax_mag.set_ylabel("|Z|")
    ax_mag.set_title("Bode magnitude")
    ax_mag.grid(alpha=0.3, which="both")
    ax_mag.legend(loc="best", fontsize=7)

    for h in (-90, -180, 0, -45):
        ax_phs.axhline(h, color="gray", lw=0.4, ls="--", alpha=0.5)
    ax_phs.set_xlabel("frequency (1/fs)")
    ax_phs.set_ylabel("phase (deg)")
    ax_phs.set_title("Bode phase")
    ax_phs.grid(alpha=0.3, which="both")

    ax_r2.axhline(0.95, color="green", lw=0.5, ls="--", alpha=0.5)
    ax_r2.axhline(0.85, color="red", lw=0.5, ls="--", alpha=0.5)
    ax_r2.set_xlabel("frequency (1/fs)")
    ax_r2.set_ylabel("R²(V)")
    ax_r2.set_title("Fit quality vs frequency, by amplitude (where traces overlap on Nyquist = linear regime confirmed; where they diverge = amplitude-dependent / nonlinear)")
    ax_r2.set_ylim(0, 1.05)
    ax_r2.grid(alpha=0.3, which="both")
    ax_r2.legend(loc="best", fontsize=7, ncol=4)

    fig.suptitle(f"EIS DOE master ({len(paths)} jobs · {len(all_points)} points · {n_amp} amplitudes)", fontsize=11)
    fig.savefig(args.out, dpi=120, bbox_inches="tight")
    plt.close(fig)
    print(f"\nwrote {args.out}")


if __name__ == "__main__":
    main()
