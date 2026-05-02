"""Plot EIS time-series CSVs (v, i over the recording window) and overlay
the lock-in best-fit sinusoid for V and I.

Useful to visually verify what the EIS lock-in is measuring at each frequency.
The 180°-out-of-phase finding from `Z = (-30.4, -8.5)` etc. should be visible
as I(t) flipping the sign of V(t) over each cycle.

Usage:
    python scripts/plot_eis_timeseries.py [csv1] [csv2] ...
    python scripts/plot_eis_timeseries.py    # plots all eis_timeseries/*.csv
                                              modified within the last 12 hours
"""
from __future__ import annotations

import csv
import math
import re
import sys
import time
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent
TS_DIR = REPO_ROOT / "eis_timeseries"
OUT_DIR = REPO_ROOT / "images" / "eis_validation_runs"
OUT_DIR.mkdir(parents=True, exist_ok=True)


def load_csv(path: Path):
    """Return (header_dc, t_fs, v, i, v_ac, i_ac, actual_i, v_cap, is_rec, freq)."""
    with path.open() as f:
        first = f.readline().strip()
        # First line: "# v_dc=...  i_dc=...  freq=..."
        v_dc = float(re.search(r"v_dc=([-\d.eE+]+)", first).group(1))
        i_dc = float(re.search(r"i_dc=([-\d.eE+]+)", first).group(1))
        freq = float(re.search(r"freq=([-\d.eE+]+)", first).group(1))
        rdr = csv.DictReader(f)
        rows = list(rdr)
    t = np.array([float(r["t_rel_fs"]) for r in rows])
    v = np.array([float(r["v"]) for r in rows])
    i = np.array([float(r["i"]) for r in rows])
    v_ac = np.array([float(r["v_ac"]) for r in rows])
    i_ac = np.array([float(r["i_ac"]) for r in rows])
    actual_i = np.array([float(r["actual_i"]) for r in rows])
    v_cap = np.array([float(r["v_cap"]) for r in rows])
    is_rec = np.array([int(r["is_recording"]) for r in rows], dtype=bool)
    return {
        "v_dc": v_dc, "i_dc": i_dc, "freq": freq,
        "t": t, "v": v, "i": i, "v_ac": v_ac, "i_ac": i_ac,
        "actual_i": actual_i, "v_cap": v_cap, "is_rec": is_rec,
    }


def lock_in_fit(t_rel: np.ndarray, sig: np.ndarray, omega: float):
    """3-parameter LS fit: signal(t) = A·cos(ωt) + B·sin(ωt) + C.
    Returns (A, B, C, amp, phase_deg, r2).
    """
    n = len(t_rel)
    if n < 3:
        return 0.0, 0.0, 0.0, 0.0, 0.0, 0.0
    cos_t = np.cos(omega * t_rel)
    sin_t = np.sin(omega * t_rel)
    M = np.column_stack([cos_t, sin_t, np.ones_like(t_rel)])
    coeffs, *_ = np.linalg.lstsq(M, sig, rcond=None)
    A, B, C = coeffs
    fit = M @ coeffs
    resid = sig - fit
    ss_res = float((resid * resid).sum())
    ac_part = sig - sig.mean()
    ss_tot = float((ac_part * ac_part).sum())
    r2 = 1.0 - ss_res / ss_tot if ss_tot > 1e-30 else 1.0
    amp = math.hypot(A, B)
    # cosine convention: signal = amp·cos(ωt + φ) + dc, where φ = atan2(-B, A)
    phase_deg = math.degrees(math.atan2(-B, A))
    return A, B, C, amp, phase_deg, r2


def plot_one(data: dict, out_path: Path):
    f = data["freq"]
    omega = 2 * math.pi * f
    t = data["t"]
    is_rec = data["is_rec"]
    v_ac = data["v_ac"]
    i_ac = data["i_ac"]

    # Recording window only — that's what the lock-in fits.
    t_rec = t[is_rec]
    v_rec = v_ac[is_rec]
    i_rec = i_ac[is_rec]
    if len(t_rec) < 3:
        print(f"  WARN: only {len(t_rec)} recording samples in {data}")
        return

    # Re-fit V and I locally (doesn't need to match sim's exact accumulators —
    # just confirm the shapes line up).  Use t relative to recording start so
    # the phase is referenced consistently.
    t0 = t_rec[0]
    t_rec_rel = t_rec - t0
    Av, Bv, Cv, amp_v, phase_v, r2_v = lock_in_fit(t_rec_rel, v_rec, omega)
    Ai, Bi, Ci, amp_i, phase_i, r2_i = lock_in_fit(t_rec_rel, i_rec, omega)

    # Z = V̂ / Î (cosine convention).
    # If V = amp_v cos(ωt+φv), I = amp_i cos(ωt+φi), then phase(Z) = φv - φi.
    z_phase_deg = phase_v - phase_i
    # Wrap to (-180, 180]
    while z_phase_deg > 180.0:
        z_phase_deg -= 360.0
    while z_phase_deg <= -180.0:
        z_phase_deg += 360.0
    z_mag = amp_v / amp_i if amp_i > 1e-30 else float("inf")

    fig, axes = plt.subplots(3, 1, figsize=(10, 9))
    fig.suptitle(
        f"EIS time-series — f = {f:.3e} /fs (T = {1/f:.2e} fs)  amp Δ = 0.005",
        fontsize=12,
    )

    # ── Panel 1: V and I time-series with fits ──
    ax = axes[0]
    ax.plot(t / 1000, v_ac, color="tab:blue", lw=0.7, label=f"V_ac (raw, {len(t)} pts)")
    fit_v_full = Av * np.cos(omega * (t - t0)) + Bv * np.sin(omega * (t - t0)) + Cv
    ax.plot(t / 1000, fit_v_full, color="tab:blue", lw=1.5, ls="--",
            label=f"V fit: amp={amp_v:.3e}, φ={phase_v:+.1f}°, R²={r2_v:.4f}")
    ax2 = ax.twinx()
    ax2.plot(t / 1000, i_ac, color="tab:red", lw=0.7, alpha=0.7, label="I_ac (raw)")
    fit_i_full = Ai * np.cos(omega * (t - t0)) + Bi * np.sin(omega * (t - t0)) + Ci
    ax2.plot(t / 1000, fit_i_full, color="tab:red", lw=1.5, ls="--",
             label=f"I fit: amp={amp_i:.3e}, φ={phase_i:+.1f}°, R²={r2_i:.4f}")

    # Shade settling phase
    if (~is_rec).any():
        t_settle = t[~is_rec]
        ax.axvspan(t_settle.min() / 1000, t_settle.max() / 1000,
                   alpha=0.08, color="gray", zorder=0)
    ax.set_xlabel("t_rel (ps)")
    ax.set_ylabel("V_ac (V)", color="tab:blue")
    ax2.set_ylabel("I_ac (A.U.)", color="tab:red")
    ax.tick_params(axis="y", labelcolor="tab:blue")
    ax2.tick_params(axis="y", labelcolor="tab:red")
    lines1, labels1 = ax.get_legend_handles_labels()
    lines2, labels2 = ax2.get_legend_handles_labels()
    ax.legend(lines1 + lines2, labels1 + labels2, loc="upper right", fontsize=8)
    ax.grid(alpha=0.3)
    ax.set_title("V_ac (blue) and I_ac (red) — fits overlaid as dashed")

    # ── Panel 2: zoomed-in recording window only ──
    ax = axes[1]
    n_zoom = min(len(t_rec_rel), int(2 * (1.0 / f) / np.median(np.diff(t_rec_rel))) + 1)
    n_zoom = min(n_zoom, len(t_rec_rel))
    if n_zoom < 10:
        n_zoom = len(t_rec_rel)
    tz = t_rec_rel[:n_zoom]
    ax.plot(tz / 1000, v_rec[:n_zoom], color="tab:blue", lw=1, label="V_ac")
    ax.plot(tz / 1000, Av * np.cos(omega * tz) + Bv * np.sin(omega * tz),
            color="tab:blue", lw=1.5, ls="--", label="V fit")
    ax2 = ax.twinx()
    ax2.plot(tz / 1000, i_rec[:n_zoom], color="tab:red", lw=1, alpha=0.7, label="I_ac")
    ax2.plot(tz / 1000, Ai * np.cos(omega * tz) + Bi * np.sin(omega * tz),
             color="tab:red", lw=1.5, ls="--", label="I fit")
    ax.set_xlabel("t_rec (ps)")
    ax.set_ylabel("V_ac", color="tab:blue")
    ax2.set_ylabel("I_ac", color="tab:red")
    ax.tick_params(axis="y", labelcolor="tab:blue")
    ax2.tick_params(axis="y", labelcolor="tab:red")
    ax.set_title(f"First ~2 cycles of recording window — phase(Z) = φv − φi = {z_phase_deg:+.1f}°")
    ax.grid(alpha=0.3)

    # ── Panel 3: Lissajous (V vs I) with fit ellipse ──
    ax = axes[2]
    ax.plot(i_rec, v_rec, color="gray", lw=0.5, alpha=0.5, label="raw V vs I (recording)")
    omega_t = omega * t_rec_rel
    v_fit_only = Av * np.cos(omega_t) + Bv * np.sin(omega_t)
    i_fit_only = Ai * np.cos(omega_t) + Bi * np.sin(omega_t)
    ax.plot(i_fit_only, v_fit_only, color="black", lw=1.5,
            label=f"fit: |Z|={z_mag:.3e}  phase(Z)={z_phase_deg:+.1f}°")
    ax.axhline(0, color="k", lw=0.5)
    ax.axvline(0, color="k", lw=0.5)
    ax.set_xlabel("I_ac (A.U.)")
    ax.set_ylabel("V_ac (V)")
    ax.set_title(
        "Lissajous V vs I — line = pure resistor (φ=0); ellipse = capacitive/inductive; "
        "antiphase line (NW/SE) = Z_real < 0"
    )
    ax.legend(loc="best", fontsize=9)
    ax.grid(alpha=0.3)
    ax.set_aspect("auto")

    fig.tight_layout()
    fig.savefig(out_path, dpi=120)
    plt.close(fig)
    print(f"  wrote {out_path}")
    print(f"    re-fit:   |Z|={z_mag:.3e}  phase(Z)={z_phase_deg:+.2f}°  R²(V)={r2_v:.4f}  R²(I)={r2_i:.4f}")
    return {
        "freq": f, "z_mag": z_mag, "z_phase_deg": z_phase_deg,
        "amp_v": amp_v, "amp_i": amp_i, "r2_v": r2_v, "r2_i": r2_i,
    }


def main():
    if len(sys.argv) > 1:
        paths = [Path(p) for p in sys.argv[1:]]
    else:
        # All CSVs in eis_timeseries/ modified within the last 12 hours.
        cutoff = time.time() - 12 * 3600
        paths = sorted(p for p in TS_DIR.glob("*.csv") if p.stat().st_mtime >= cutoff)
    if not paths:
        print("No CSVs to plot. Pass file paths or wait for fresh runs.")
        sys.exit(1)
    print(f"Plotting {len(paths)} CSV(s):")
    summary = []
    for p in paths:
        print(f"  {p.name}")
        data = load_csv(p)
        out = OUT_DIR / (p.stem + ".png")
        result = plot_one(data, out)
        if result:
            result["src"] = p.name
            summary.append(result)

    if len(summary) >= 2:
        print("\nSummary (re-fit from CSV, compare to sim's [EIS] Completed lines):")
        print(f"  {'src':<35}  {'freq':>10}  {'|Z|':>10}  {'phase':>9}  {'R²(V)':>7}  {'R²(I)':>7}")
        for r in summary:
            print(f"  {r['src']:<35}  {r['freq']:>10.3e}  {r['z_mag']:>10.3e}  "
                  f"{r['z_phase_deg']:>+8.2f}°  {r['r2_v']:>7.4f}  {r['r2_i']:>7.4f}")


if __name__ == "__main__":
    main()
