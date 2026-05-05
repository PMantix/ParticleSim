"""Sweep driven_symmetric across drive amplitudes.

Runs the physics_invariants binary at multiple --drive-amplitude values,
collects per-run result.json files, and renders an aggregate plot showing
how the kinetic gates respond as drive grows. Designed to answer:

  - At what drive amplitude do hops start firing?
  - Which gate dominates rejection at each amplitude?
  - Does measured current track applied current linearly across amplitudes?

The zero-drive case (no kinetics fire) is overlaid as a reference where
useful.

Usage:
    cargo build --release --bin physics_invariants
    python scripts/drive_sweep.py
    # → doe_results/physics_validation/drive_sweep/drive_sweep.png

By default sweeps 1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1 e/fs. Pass --amps to
override. --no-recompute reuses existing result.json files.
"""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent
BINARY = REPO_ROOT / "target" / "release" / "physics_invariants"
OUT_ROOT = REPO_ROOT / "doe_results" / "physics_validation" / "drive_sweep"

DEFAULT_AMPS = [1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1]


def _label(amp: float) -> str:
    return f"amp{amp:.0e}".replace("+0", "+").replace("-0", "-")


def run_one(amp: float, no_recompute: bool = False) -> dict:
    out_dir = OUT_ROOT / _label(amp)
    result_path = out_dir / "result.json"
    if no_recompute and result_path.exists():
        with result_path.open() as f:
            return json.load(f)
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(BINARY),
        "--test", "driven_symmetric",
        "--drive-amplitude", f"{amp}",
        "--out", str(result_path),
        "--csv", str(out_dir / "timeseries.csv"),
        # nonexistent baseline path → binary skips comparison without writing
        "--baseline", str(out_dir / "no_baseline.json"),
    ]
    print(f"[run] amp={amp:.3e}")
    # Don't fail the sweep if a single-amp test reports pass=false (low-amp
    # may legitimately fail the kinetics-engage threshold). Result JSON is
    # still written.
    subprocess.run(cmd, cwd=REPO_ROOT, check=False, capture_output=False)
    if not result_path.exists():
        raise RuntimeError(f"result.json not produced at {result_path}")
    with result_path.open() as f:
        return json.load(f)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--amps", type=float, nargs="+", default=DEFAULT_AMPS,
                    help="Drive amplitudes (e/fs)")
    ap.add_argument("--no-recompute", action="store_true",
                    help="Reuse existing result.json files where present")
    ap.add_argument("--out", type=Path, default=OUT_ROOT / "drive_sweep.png")
    args = ap.parse_args()

    if not BINARY.exists():
        print(f"Binary not found at {BINARY}. Build first:")
        print("  cargo build --release --bin physics_invariants")
        return 1

    results = []
    for amp in args.amps:
        results.append((amp, run_one(amp, args.no_recompute)))

    zero_drive_path = REPO_ROOT / "doe_results/physics_validation/zero_emf_symmetric/result.json"
    zero = None
    if zero_drive_path.exists():
        with zero_drive_path.open() as f:
            zero = json.load(f)

    amps = np.array([r[0] for r in results])
    flips = np.array([r[1]["details"]["hop_activity"]["total_electron_flips"] for r in results])
    accepted = np.array([r[1]["details"]["hop_gates"]["accepted"] for r in results])
    candidates = np.array([r[1]["details"]["hop_gates"]["candidates_reached_predicate"] for r in results])
    rej_align = np.array([r[1]["details"]["hop_gates"]["rejected_by_alignment"] for r in results])
    rej_dphi = np.array([r[1]["details"]["hop_gates"]["rejected_by_dphi"] for r in results])
    rej_rate = np.array([r[1]["details"]["hop_gates"]["rejected_by_rate"] for r in results])
    rej_rand = np.array([r[1]["details"]["hop_gates"]["rejected_by_random"] for r in results])
    dst_filt_species = np.array([r[1]["details"]["hop_gates"]["dst_filtered_by_species"] for r in results])
    dst_filt_other = np.array([r[1]["details"]["hop_gates"]["dst_filtered_other"] for r in results])
    rel_err = np.array([r[1]["value"] for r in results])
    pass_flags = np.array([r[1]["pass"] for r in results], dtype=bool)
    mean_i_left = np.array([r[1]["details"]["mean_i_left_e_per_fs"] for r in results])
    mean_i_right = np.array([r[1]["details"]["mean_i_right_e_per_fs"] for r in results])

    fig, axs = plt.subplots(3, 1, figsize=(11, 12))

    # Panel 1: hop counts vs drive (log-log)
    ax = axs[0]
    ax.loglog(amps, np.maximum(flips, 0.5), "o-", color="#6a3d9a",
              linewidth=1.6, markersize=8, label="total electron-count flips")
    ax.loglog(amps, np.maximum(accepted, 0.5), "s-", color="#33a02c",
              linewidth=1.6, markersize=7, label="accepted hops")
    ax.loglog(amps, np.maximum(candidates, 0.5), "^-", color="#666",
              linewidth=1.0, markersize=5, alpha=0.7,
              label="candidates reaching predicate")
    if zero:
        zf = zero["details"]["hop_activity"]["total_electron_flips"]
        zc = zero["details"]["hop_gates"]["candidates_reached_predicate"]
        ax.axhline(max(zf, 0.5), color="#aaa", linestyle=":",
                   label=f"zero-drive flips ({zf})")
        ax.axhline(max(zc, 0.5), color="#ddd", linestyle=":",
                   label=f"zero-drive candidates ({zc})")
    ax.set_xlabel("|drive amplitude|  (e/fs)")
    ax.set_ylabel("count over measurement window")
    ax.set_title("Hop activity vs drive amplitude")
    ax.legend(loc="best", fontsize=9)
    ax.grid(True, which="both", alpha=0.3)

    # Panel 2: applied vs measured current per foil + rel_err
    ax = axs[1]
    ax_left = ax
    ax_left.loglog(amps, amps, "k--", linewidth=1, alpha=0.6, label="ideal: |I_meas| = |I_app|")
    ax_left.loglog(amps, np.abs(mean_i_left), "v-", color="#1f77b4",
                   linewidth=1.4, markersize=7, label="|I_left| measured")
    ax_left.loglog(amps, np.abs(mean_i_right), "^-", color="#d62728",
                   linewidth=1.4, markersize=7, label="|I_right| measured")
    ax_left.set_xlabel("|drive amplitude|  (e/fs)")
    ax_left.set_ylabel("|measured current|  (e/fs)")
    ax_left.set_title("Current tracking: applied vs measured")
    ax_left.legend(loc="best", fontsize=9)
    ax_left.grid(True, which="both", alpha=0.3)
    ax_right = ax_left.twinx()
    ax_right.semilogx(amps, rel_err, "o-", color="#ff7f0e",
                      linewidth=1.2, markersize=7, label="worst rel_err")
    ax_right.axhline(0.5, color="#cc6600", linestyle=":", alpha=0.6)
    ax_right.set_ylabel("worst rel. err. (vs applied)", color="#cc6600")
    ax_right.tick_params(axis="y", colors="#cc6600")
    ax_right.set_ylim(0, max(1.2, rel_err.max() * 1.1) if rel_err.size else 1.0)
    ax_right.legend(loc="lower right", fontsize=9)

    # Panel 3: per-predicate gate fates as fraction of predicate-evaluated candidates
    ax = axs[2]
    den = np.maximum(candidates, 1)
    masked = candidates > 0
    if masked.any():
        ax.semilogx(amps[masked], accepted[masked] / den[masked],
                    "o-", color="#33a02c", linewidth=1.6, markersize=8, label="accepted")
        ax.semilogx(amps[masked], rej_align[masked] / den[masked],
                    "s-", color="#6a3d9a", linewidth=1.4, markersize=7, label="rejected by alignment")
        ax.semilogx(amps[masked], rej_rand[masked] / den[masked],
                    "d-", color="#1f77b4", linewidth=1.4, markersize=7, label="rejected by random")
        sum_dphi_rate = rej_dphi[masked] + rej_rate[masked]
        ax.semilogx(amps[masked], sum_dphi_rate / den[masked],
                    "x-", color="#ff7f0e", linewidth=1.2, markersize=7,
                    label="rejected by d_phi + rate")
    if (~masked).any():
        for amp_i in amps[~masked]:
            ax.axvline(amp_i, color="#fcc", linestyle=":", linewidth=1)
        ax.text(0.02, 0.95,
                "drive amps with 0 candidates\nreaching predicate are\n"
                "shown as light vertical lines",
                transform=ax.transAxes, ha="left", va="top",
                fontsize=8, color="#a44")
    ax.set_xlabel("|drive amplitude|  (e/fs)")
    ax.set_ylabel("fraction of evaluated candidates")
    ax.set_title("Per-dst gate fates vs drive (only amps with predicate evals shown)")
    ax.legend(loc="best", fontsize=9)
    ax.set_ylim(-0.05, 1.05)
    ax.grid(True, which="both", alpha=0.3)

    title = "driven_symmetric — drive amplitude sweep"
    pass_summary = f"  ·  pass: {sum(pass_flags)}/{len(pass_flags)}"
    fig.suptitle(title + pass_summary, fontsize=14)
    fig.tight_layout(rect=(0, 0, 1, 0.97))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(args.out, dpi=140)
    plt.close(fig)
    rel = args.out.relative_to(REPO_ROOT) if args.out.is_absolute() else args.out
    print(f"Wrote {rel}")

    # Also dump a compact CSV summary
    csv_path = args.out.with_suffix(".csv")
    with csv_path.open("w") as f:
        f.write("amp_e_per_fs,total_flips,accepted,candidates,rej_align,rej_dphi,rej_rate,rej_rand,"
                "dst_filt_species,dst_filt_other,mean_i_left,mean_i_right,rel_err,pass\n")
        for i, amp in enumerate(amps):
            f.write(f"{amp:.6e},{flips[i]},{accepted[i]},{candidates[i]},"
                    f"{rej_align[i]},{rej_dphi[i]},{rej_rate[i]},{rej_rand[i]},"
                    f"{dst_filt_species[i]},{dst_filt_other[i]},"
                    f"{mean_i_left[i]:.6e},{mean_i_right[i]:.6e},{rel_err[i]:.6e},{int(pass_flags[i])}\n")
    print(f"Summary CSV: {csv_path.relative_to(REPO_ROOT) if csv_path.is_absolute() else csv_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
