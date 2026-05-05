"""Compare 2-RC fit parameters across multiple dcr_pulse_sweep runs.

Loads `fit_2rc.csv` from each run directory (must be pre-generated via
`fit_dcr_2rc.py`), pulls the pulse-0 row from each, and produces a
4-panel figure: R0, R1, R2, τ1, τ2 vs run amplitude.

Usage:
    python scripts/dcr_amplitude_compare.py \
        --runs doe_results/dcr_pulse_sweep/dcr-long-001_amp1e-2 \
               doe_results/dcr_pulse_sweep/dcr-long-002_amp2e-2 \
               doe_results/dcr_pulse_sweep/dcr-long-003_amp6e-2 \
        --out doe_results/dcr_pulse_sweep/amplitude_compare.png
"""
from __future__ import annotations

import argparse
import csv
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np


def load_first_pulse_fit(run_dir: Path) -> dict | None:
    """Read fit_2rc.csv and return the pulse-0 row + i_amp."""
    fit_path = run_dir / "fit_2rc.csv"
    summary_path = run_dir / "summary.txt"
    if not fit_path.exists():
        print(f"missing {fit_path}")
        return None

    with fit_path.open() as f:
        reader = csv.DictReader(f)
        rows = list(reader)
    if not rows:
        return None
    row = rows[0]
    if not row.get("R0"):
        return None

    # Parse amplitude from summary.txt (the Rust binary writes it).
    amp = None
    if summary_path.exists():
        for line in summary_path.read_text().splitlines():
            if line.startswith("amplitude="):
                amp = float(line.split("=", 1)[1].strip())
                break
    if amp is None:
        # Fall back to i_amp_eff from the fit row.
        amp = float(row.get("i_amp_eff", 0.0))

    return {
        "amp": amp,
        "R0": float(row["R0"]),
        "R1": float(row["R1"]),
        "R2": float(row["R2"]),
        "tau1": float(row["tau1_fs"]),
        "tau2": float(row["tau2_fs"]),
        "rmse_mv": float(row["rmse_mV"]),
        "name": run_dir.name,
    }


def render(rows, out_path: Path):
    rows = sorted(rows, key=lambda r: r["amp"])
    amps = np.array([r["amp"] for r in rows])
    R0 = np.array([r["R0"] for r in rows])
    R1 = np.array([r["R1"] for r in rows])
    R2 = np.array([r["R2"] for r in rows])
    tau1 = np.array([r["tau1"] for r in rows])
    tau2 = np.array([r["tau2"] for r in rows])
    rmse = np.array([r["rmse_mv"] for r in rows])

    fig, axes = plt.subplots(2, 3, figsize=(15, 8))

    axes[0, 0].plot(amps, R0, "o-", color="tab:blue")
    axes[0, 0].set_title("R₀ vs amplitude")
    axes[0, 0].set_xlabel("amplitude (e/fs)")
    axes[0, 0].set_ylabel("R₀")
    axes[0, 0].grid(True, alpha=0.3)
    axes[0, 0].set_xscale("log")

    axes[0, 1].plot(amps, R1, "o-", color="tab:orange", label="R₁ (fast)")
    axes[0, 1].plot(amps, R2, "s-", color="tab:red", label="R₂ (slow)")
    axes[0, 1].plot(amps, R1 + R2, "^--", color="tab:purple", label="R₁+R₂ = R_pol")
    axes[0, 1].set_title("Polarization resistances vs amplitude")
    axes[0, 1].set_xlabel("amplitude (e/fs)")
    axes[0, 1].set_ylabel("R")
    axes[0, 1].legend()
    axes[0, 1].grid(True, alpha=0.3)
    axes[0, 1].set_xscale("log")

    axes[0, 2].plot(amps, rmse, "o-", color="tab:gray")
    axes[0, 2].set_title("Fit quality (RMSE)")
    axes[0, 2].set_xlabel("amplitude (e/fs)")
    axes[0, 2].set_ylabel("RMSE (mV)")
    axes[0, 2].grid(True, alpha=0.3)
    axes[0, 2].set_xscale("log")

    axes[1, 0].plot(amps, tau1, "o-", color="tab:orange")
    axes[1, 0].set_title("τ₁ (fast arc) vs amplitude")
    axes[1, 0].set_xlabel("amplitude (e/fs)")
    axes[1, 0].set_ylabel("τ₁ (fs)")
    axes[1, 0].grid(True, alpha=0.3)
    axes[1, 0].set_xscale("log")

    axes[1, 1].plot(amps, tau2, "s-", color="tab:red")
    axes[1, 1].set_title("τ₂ (slow arc) vs amplitude")
    axes[1, 1].set_xlabel("amplitude (e/fs)")
    axes[1, 1].set_ylabel("τ₂ (fs)")
    axes[1, 1].grid(True, alpha=0.3)
    axes[1, 1].set_xscale("log")

    # Per-amp ΔV for sanity.
    delta_v = (R0 + R1 + R2) * amps * 1000
    axes[1, 2].plot(amps, delta_v, "o-", color="tab:green")
    axes[1, 2].set_title("Predicted steady-state ΔV (mV)")
    axes[1, 2].set_xlabel("amplitude (e/fs)")
    axes[1, 2].set_ylabel("ΔV_∞ (mV)")
    axes[1, 2].grid(True, alpha=0.3)
    axes[1, 2].set_xscale("log")

    fig.suptitle("DCR 2-RC fit parameters vs amplitude", fontsize=13)
    fig.tight_layout(rect=(0, 0, 1, 0.96))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)
    print(f"wrote {out_path}")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--runs", nargs="+", required=True,
                   help="Run directories containing fit_2rc.csv + summary.txt")
    p.add_argument("--out", default="doe_results/dcr_pulse_sweep/amplitude_compare.png")
    args = p.parse_args()

    rows = []
    for run_path in args.runs:
        fit = load_first_pulse_fit(Path(run_path))
        if fit:
            rows.append(fit)
            print(f"{fit['name']:50} amp={fit['amp']:.3e}  "
                  f"R0={fit['R0']:.3e}  R1={fit['R1']:.3e}  R2={fit['R2']:.3e}  "
                  f"τ1={fit['tau1']:.0f}  τ2={fit['tau2']:.0f}  RMSE={fit['rmse_mv']:.3f}mV")

    if not rows:
        raise SystemExit("no usable fit rows found")

    render(rows, Path(args.out))


if __name__ == "__main__":
    main()
