"""Fit a 2-RC equivalent-circuit model to dcr_pulse_sweep output.

Reads `dense_series.csv` from a `dcr_pulse_sweep` run, picks the on-phase
samples of one pulse, and fits

    V(t) - V_pre = I·R0 + I·R1·(1 - exp(-t/τ1)) + I·R2·(1 - exp(-t/τ2))

where t = time since pulse onset, I is the applied current amplitude,
R0 is the instantaneous (kinetic-inductance / ohmic) drop, and R1/R2
with τ1/τ2 are the fast / slow arc resistances and time constants.

Produces:
  - <run-dir>/fit_2rc.csv: pulse_idx, R0, R1, R2, tau1, tau2, RMSE_mV
  - <run-dir>/fit_2rc.png: V(t) data + fit overlay, one subplot per pulse

Usage:
    python scripts/fit_dcr_2rc.py --run-dir doe_results/dcr_pulse_sweep/<run_id>
    python scripts/fit_dcr_2rc.py --run-dir <path> --pulse 0   # fit only pulse 0
"""
from __future__ import annotations

import argparse
import csv
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
from scipy.optimize import curve_fit


def two_rc_response(t, R0, R1, R2, tau1, tau2, I):
    """V(t) - V_pre when a step current I is applied at t=0."""
    return I * (R0 + R1 * (1 - np.exp(-t / tau1)) + R2 * (1 - np.exp(-t / tau2)))


def load_series(path: Path):
    """Load dense_series.csv as a structured array of (t_fs, step, phase, i, v)."""
    rows = []
    with path.open() as f:
        reader = csv.DictReader(f)
        for r in reader:
            rows.append({
                "t_fs": float(r["t_fs"]),
                "step": int(r["step"]),
                "phase": r["phase"],
                "i_applied": float(r["i_applied"]),
                "v_cell": float(r["v_cell"]),
            })
    return rows


def split_into_pulses(rows):
    """Return list of (v_pre, on_rows, rest_rows, i_amp) per pulse."""
    pulses = []
    i = 0
    n = len(rows)
    while i < n:
        # Look for a "pre" row.
        if rows[i]["phase"] != "pre":
            i += 1
            continue
        v_pre = rows[i]["v_cell"]
        on_rows = []
        rest_rows = []
        i += 1
        # Collect "on" rows.
        while i < n and rows[i]["phase"] == "on":
            on_rows.append(rows[i])
            i += 1
        # Collect "rest" rows.
        while i < n and rows[i]["phase"] == "rest":
            rest_rows.append(rows[i])
            i += 1
        i_amp = on_rows[0]["i_applied"] if on_rows else 0.0
        pulses.append({"v_pre": v_pre, "on": on_rows, "rest": rest_rows, "i_amp": i_amp})
    return pulses


def fit_pulse(pulse):
    """Fit the on-phase 2-RC response. Returns (params, rmse_mV) or None."""
    on = pulse["on"]
    if len(on) < 6:
        return None
    v_pre = pulse["v_pre"]
    i_amp = pulse["i_amp"]
    if abs(i_amp) < 1e-12:
        return None

    t_pulse_start = on[0]["t_fs"]
    t = np.array([r["t_fs"] - t_pulse_start for r in on])
    v = np.array([r["v_cell"] for r in on])
    dv = v - v_pre

    # Sign-flip target so the fit is in the "applied current = positive →
    # V increases" convention. In our sim, positive dc_current on group A
    # makes V more negative, so dv is negative for positive i_amp.
    # Fit |dv| vs |i_amp| to keep R-values positive in the natural sense.
    sign = -1.0 if i_amp > 0 and np.median(dv) < 0 else 1.0
    dv_eff = sign * dv
    i_eff = abs(i_amp)

    # Initial guesses (heuristic):
    # - τ1 short (fast arc), ~2-5% of pulse duration.
    # - τ2 long (slow arc), ~30-80% of pulse duration.
    # - R0 near zero.
    # - R1+R2 split the steady-state ΔV.
    t_max = t[-1]
    delta_v_total = dv_eff[-1]
    p0 = [
        1e-3,                       # R0
        0.5 * delta_v_total / i_eff if i_eff > 0 else 0.0,  # R1
        0.5 * delta_v_total / i_eff if i_eff > 0 else 0.0,  # R2
        max(t_max * 0.05, 5.0),     # τ1
        max(t_max * 0.5, 20.0),     # τ2
    ]
    bounds = ([-np.inf, 0.0, 0.0, 1.0, 1.0],
              [np.inf, np.inf, np.inf, t_max, t_max * 100])

    try:
        popt, _ = curve_fit(
            lambda tt, R0, R1, R2, tau1, tau2: two_rc_response(tt, R0, R1, R2, tau1, tau2, i_eff),
            t, dv_eff, p0=p0, bounds=bounds, maxfev=20000
        )
    except Exception as e:
        print(f"  curve_fit failed: {e}")
        return None

    R0, R1, R2, tau1, tau2 = popt
    fit = two_rc_response(t, R0, R1, R2, tau1, tau2, i_eff)
    rmse_v = float(np.sqrt(np.mean((fit - dv_eff) ** 2)))
    return {
        "R0": float(R0), "R1": float(R1), "R2": float(R2),
        "tau1": float(tau1), "tau2": float(tau2),
        "rmse_v": rmse_v,
        "rmse_mv": rmse_v * 1000.0,
        "i_amp_eff": i_eff,
        "sign": sign,
        "t_pulse_start_fs": t_pulse_start,
    }


def render(pulses, fits, run_dir: Path, out_path: Path):
    n = len(pulses)
    cols = min(3, n)
    rows = (n + cols - 1) // cols
    fig, axes = plt.subplots(rows, cols, figsize=(cols * 5.0, rows * 3.5), squeeze=False)

    for idx, (pulse, fit) in enumerate(zip(pulses, fits)):
        r, c = divmod(idx, cols)
        ax = axes[r][c]
        on = pulse["on"]
        rest = pulse["rest"]
        v_pre = pulse["v_pre"]
        t_on = np.array([r["t_fs"] - on[0]["t_fs"] for r in on]) if on else np.array([])
        v_on = np.array([r["v_cell"] for r in on]) if on else np.array([])
        ax.plot(t_on, (v_on - v_pre) * 1000, "o", ms=3, color="tab:blue", label="V_data on")
        if rest:
            t_rest = np.array([r["t_fs"] - on[-1]["t_fs"] for r in rest])
            v_rest = np.array([r["v_cell"] for r in rest])
            ax.plot(t_rest + t_on[-1] if t_on.size else t_rest,
                    (v_rest - v_pre) * 1000, "x", ms=3, color="tab:gray", label="V_data rest")
        if fit:
            t_fit = np.linspace(0, t_on[-1] if t_on.size else 1.0, 200)
            v_fit = two_rc_response(
                t_fit, fit["R0"], fit["R1"], fit["R2"], fit["tau1"], fit["tau2"], fit["i_amp_eff"]
            )
            # Apply sign flip back to plot in original convention.
            ax.plot(t_fit, fit["sign"] * v_fit * 1000, "-", color="tab:red",
                    label=f"2-RC fit (RMSE={fit['rmse_mv']:.3f} mV)")
            param_str = (
                f"R0={fit['R0']:.3e}, R1={fit['R1']:.3e}, R2={fit['R2']:.3e}\n"
                f"τ1={fit['tau1']:.1f} fs, τ2={fit['tau2']:.1f} fs"
            )
            ax.text(0.02, 0.98, param_str, transform=ax.transAxes,
                    fontsize=8, va="top", ha="left",
                    bbox=dict(boxstyle="round", facecolor="white", alpha=0.8))
        ax.set_title(f"pulse {idx}  (I={pulse['i_amp']:.3e})")
        ax.set_xlabel("t since pulse start (fs)")
        ax.set_ylabel("V_cell − V_pre (mV)")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=7, loc="lower right")

    for idx in range(len(pulses), rows * cols):
        r, c = divmod(idx, cols)
        axes[r][c].axis("off")

    fig.suptitle(f"DCR 2-RC fit — {run_dir.name}", fontsize=12)
    fig.tight_layout(rect=(0, 0, 1, 0.96))
    fig.savefig(out_path, dpi=140)
    plt.close(fig)
    print(f"wrote {out_path}")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--run-dir", required=True, help="DCR pulse sweep output directory")
    p.add_argument("--pulse", type=int, default=None,
                   help="if set, fit only this pulse index (0-based)")
    args = p.parse_args()

    run_dir = Path(args.run_dir)
    series_path = run_dir / "dense_series.csv"
    if not series_path.exists():
        raise SystemExit(f"missing {series_path}")

    rows = load_series(series_path)
    pulses = split_into_pulses(rows)
    print(f"loaded {len(pulses)} pulses from {series_path}")

    if args.pulse is not None:
        pulses = [pulses[args.pulse]]

    fits = []
    for idx, pulse in enumerate(pulses):
        print(f"\nPulse {idx}: I={pulse['i_amp']:.3e}, "
              f"on_samples={len(pulse['on'])}, rest_samples={len(pulse['rest'])}")
        fit = fit_pulse(pulse)
        fits.append(fit)
        if fit:
            print(f"  R0={fit['R0']:.3e}  R1={fit['R1']:.3e}  R2={fit['R2']:.3e}")
            print(f"  τ1={fit['tau1']:.1f} fs  τ2={fit['tau2']:.1f} fs  RMSE={fit['rmse_mv']:.3f} mV")
        else:
            print("  fit failed")

    # Write CSV.
    csv_path = run_dir / "fit_2rc.csv"
    with csv_path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["pulse_idx", "R0", "R1", "R2", "tau1_fs", "tau2_fs", "rmse_mV", "i_amp_eff"])
        for idx, fit in enumerate(fits):
            if fit:
                w.writerow([
                    idx, fit["R0"], fit["R1"], fit["R2"],
                    fit["tau1"], fit["tau2"], fit["rmse_mv"], fit["i_amp_eff"],
                ])
            else:
                w.writerow([idx, "", "", "", "", "", "", ""])
    print(f"\nwrote {csv_path}")

    png_path = run_dir / "fit_2rc.png"
    render(pulses, fits, run_dir, png_path)


if __name__ == "__main__":
    main()
