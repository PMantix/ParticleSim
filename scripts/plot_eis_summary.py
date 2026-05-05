"""Multi-panel summary of one EIS sweep — Nyquist + Bode |Z| + Bode phase
+ representative V/I time series. Reads the table printed by
`tests/eis_validation_runs.rs` or `eis_quick_sweep` from a log file.

Usage:
    python scripts/plot_eis_summary.py <log_file> [--ts-dir <dir>] [--out <path>]
"""
from __future__ import annotations

import argparse
import csv
import math
import re
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent


def parse_log(path: Path):
    """Pull the EisPoints table from a quick_sweep / test log.
    Returns list of dicts with frequency, z_real, z_imag, magnitude,
    phase_deg, fit_r2_v, fit_r2_i, fit_v_amp, fit_i_amp.
    """
    text = path.read_text()
    points = []
    # Two formats: quick_sweep table (no [...] prefix) or test print ([%i] prefix).
    # Both have z_real, z_imag, |Z|, phase, R²(V), R²(I), V_amp, I_amp in order.
    # quick_sweep: "   0    5.000e-3    -6.707e-2    -2.516e-2    7.163e-2  -159.4°  0.8614  1.0000  4.298e-2  6.000e-1"
    # test:       "  [0] f=5.000e-3/fs  Z=(-6.443e-2, -2.066e-2)  |Z|=6.766e-2  φ=-162.2°  R²(V)=0.8201  R²(I)=1.0000  V_amp=3.803e-2  I_amp=6.000e-1"
    quick_re = re.compile(
        r"^\s*\d+\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s+([\d.+\-]+)°\s+([\d.]+)\s+([\d.]+)\s+([\d.e+\-]+)\s+([\d.e+\-]+)\s*$",
        re.MULTILINE,
    )
    test_re = re.compile(
        r"\[(\d+)\]\s+f=([\d.e+\-]+)/fs\s+Z=\(([\d.e+\-]+),\s*([\d.e+\-]+)\)\s+\|Z\|=([\d.e+\-]+)\s+φ=([\d.+\-]+)°\s+R²\(V\)=([\d.]+)\s+R²\(I\)=([\d.]+)\s+V_amp=([\d.e+\-]+)\s+I_amp=([\d.e+\-]+)"
    )

    for m in quick_re.finditer(text):
        f, zr, zi, zm, ph, r2v, r2i, va, ia = m.groups()
        points.append({
            "frequency": float(f), "z_real": float(zr), "z_imag": float(zi),
            "magnitude": float(zm), "phase_deg": float(ph),
            "fit_r2_v": float(r2v), "fit_r2_i": float(r2i),
            "fit_v_amp": float(va), "fit_i_amp": float(ia),
        })
    if not points:
        for m in test_re.finditer(text):
            _, f, zr, zi, zm, ph, r2v, r2i, va, ia = m.groups()
            points.append({
                "frequency": float(f), "z_real": float(zr), "z_imag": float(zi),
                "magnitude": float(zm), "phase_deg": float(ph),
                "fit_r2_v": float(r2v), "fit_r2_i": float(r2i),
                "fit_v_amp": float(va), "fit_i_amp": float(ia),
            })
    points.sort(key=lambda p: p["frequency"])
    return points


def find_ts_for_freq(ts_dir: Path, target_freq: float) -> Path | None:
    """Find the eis_timeseries CSV closest in frequency to target_freq."""
    best = None
    best_dev = float("inf")
    for p in ts_dir.glob("*.csv"):
        m = re.search(r"_(\d+\.\d+e[+\-]?\d+)\.csv$", p.name)
        if not m:
            continue
        f = float(m.group(1))
        dev = abs(math.log10(f) - math.log10(target_freq))
        if dev < best_dev:
            best_dev = dev
            best = p
    return best if best_dev < 0.05 else None  # within 12% tolerance


def load_ts_csv(path: Path):
    with path.open() as f:
        first = f.readline().strip()
        m_freq = re.search(r"freq=([\d.e+\-]+)", first)
        freq = float(m_freq.group(1)) if m_freq else 0.0
        rdr = csv.DictReader(f)
        rows = list(rdr)
    return {
        "freq": freq,
        "t": np.array([float(r["t_rel_fs"]) for r in rows]),
        "v_ac": np.array([float(r["v_ac"]) for r in rows]),
        "i_ac": np.array([float(r["i_ac"]) for r in rows]),
        "is_rec": np.array([int(r["is_recording"]) for r in rows], dtype=bool),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("log", help="path to log file with EisPoints table")
    ap.add_argument("--ts-dir", default=str(REPO_ROOT / "eis_timeseries"))
    ap.add_argument("--out", default=str(REPO_ROOT / "images" / "eis_validation_runs" / "summary.png"))
    args = ap.parse_args()

    points = parse_log(Path(args.log))
    if not points:
        print(f"No EisPoints found in {args.log}", file=sys.stderr)
        sys.exit(1)
    print(f"Parsed {len(points)} EisPoints from {args.log}")

    fig = plt.figure(figsize=(14, 10))
    gs = fig.add_gridspec(3, 3, hspace=0.4, wspace=0.35)

    ax_nyq = fig.add_subplot(gs[0:2, 0])
    ax_mag = fig.add_subplot(gs[0, 1:])
    ax_phs = fig.add_subplot(gs[1, 1:])
    ax_ts = fig.add_subplot(gs[2, :])

    # ── Nyquist ──
    re_z = np.array([p["z_real"] for p in points])
    im_z = np.array([p["z_imag"] for p in points])
    freqs = np.array([p["frequency"] for p in points])
    r2v = np.array([p["fit_r2_v"] for p in points])

    # Standard EIS convention: plot Re(Z) on x, -Im(Z) on y (so capacitive arc bows up).
    sc = ax_nyq.scatter(re_z, -im_z, c=np.log10(freqs), cmap="viridis", s=80, edgecolor="k", zorder=3)
    ax_nyq.plot(re_z, -im_z, "k--", lw=0.8, alpha=0.6, zorder=2)
    for p in points:
        ax_nyq.annotate(
            f"  {p['frequency']:.2e}\n  R²={p['fit_r2_v']:.3f}",
            (p["z_real"], -p["z_imag"]),
            fontsize=8, va="center", ha="left",
        )
    ax_nyq.axhline(0, color="k", lw=0.5)
    ax_nyq.axvline(0, color="k", lw=0.5)
    ax_nyq.set_xlabel("Re(Z)")
    ax_nyq.set_ylabel("−Im(Z)")
    ax_nyq.set_title("Nyquist (color = log₁₀ f)")
    ax_nyq.grid(alpha=0.3)
    plt.colorbar(sc, ax=ax_nyq, label="log₁₀ f (1/fs)")

    # ── Bode |Z| ──
    ax_mag.loglog(freqs, [p["magnitude"] for p in points], "o-", color="tab:blue")
    ax_mag.set_xlabel("frequency (1/fs)")
    ax_mag.set_ylabel("|Z|")
    ax_mag.set_title("Bode magnitude")
    ax_mag.grid(alpha=0.3, which="both")

    # ── Bode phase ──
    ax_phs.semilogx(freqs, [p["phase_deg"] for p in points], "o-", color="tab:red")
    ax_phs.axhline(0, color="k", lw=0.5)
    ax_phs.axhline(-90, color="gray", lw=0.5, ls="--")
    ax_phs.axhline(-180, color="gray", lw=0.5, ls=":")
    ax_phs.set_xlabel("frequency (1/fs)")
    ax_phs.set_ylabel("phase (deg)")
    ax_phs.set_title("Bode phase")
    ax_phs.grid(alpha=0.3, which="both")

    # ── Representative time-series (mid frequency) ──
    if len(points) >= 2:
        target_idx = len(points) // 2
        target_freq = points[target_idx]["frequency"]
    else:
        target_freq = points[0]["frequency"]
    ts_path = find_ts_for_freq(Path(args.ts_dir), target_freq)
    if ts_path:
        ts = load_ts_csv(ts_path)
        # Show only recording window
        mask = ts["is_rec"]
        t = ts["t"][mask] / 1000  # ps
        v = ts["v_ac"][mask]
        i = ts["i_ac"][mask]
        ax_ts.plot(t, v, color="tab:blue", lw=0.8, label=f"V_ac (raw, recording window)")
        ax_ts2 = ax_ts.twinx()
        ax_ts2.plot(t, i, color="tab:red", lw=0.8, alpha=0.7, label="I_ac (applied sinusoid)")
        ax_ts.set_xlabel("t (ps)")
        ax_ts.set_ylabel("V_ac (V)", color="tab:blue")
        ax_ts2.set_ylabel("I_ac (e/fs)", color="tab:red")
        ax_ts.tick_params(axis="y", labelcolor="tab:blue")
        ax_ts2.tick_params(axis="y", labelcolor="tab:red")
        ax_ts.set_title(f"Representative time-series @ f = {ts['freq']:.3e} /fs (R²(V) = {points[target_idx]['fit_r2_v']:.3f})")
        ax_ts.grid(alpha=0.3)
        # Combined legend
        h1, l1 = ax_ts.get_legend_handles_labels()
        h2, l2 = ax_ts2.get_legend_handles_labels()
        ax_ts.legend(h1 + h2, l1 + l2, loc="upper right", fontsize=8)
    else:
        ax_ts.text(0.5, 0.5, f"no time-series CSV found near f={target_freq:.2e}",
                   ha="center", va="center", transform=ax_ts.transAxes)

    fig.suptitle(f"EIS sweep summary — {Path(args.log).name}", fontsize=11)
    fig.savefig(args.out, dpi=120, bbox_inches="tight")
    plt.close(fig)
    print(f"wrote {args.out}")

    # Also print summary table
    print("\n=== Points ===")
    print(f"{'f':>10}  {'Re(Z)':>10}  {'Im(Z)':>10}  {'|Z|':>10}  {'phase':>8}  {'R²(V)':>7}  {'V_amp':>10}  {'I_amp':>10}")
    for p in points:
        print(f"{p['frequency']:>10.3e}  {p['z_real']:>+10.3e}  {p['z_imag']:>+10.3e}  "
              f"{p['magnitude']:>10.3e}  {p['phase_deg']:>+7.1f}°  {p['fit_r2_v']:>7.4f}  "
              f"{p['fit_v_amp']:>10.3e}  {p['fit_i_amp']:>10.3e}")


if __name__ == "__main__":
    main()
