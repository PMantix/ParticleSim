"""Visualize the eis_single_case_deep output.

Three panels per cycle:
  1. Particle positions colored by species + charge (16-frame grid montage)
  2. Time series: ac_value, applied_i, v_cell
  3. Summary plot: V_cell vs time with phase shading + foil electron deltas

Usage:
    python scripts/plot_deep_case.py [--in-dir doe_results/eis_single_case_deep]
"""
from __future__ import annotations

import argparse
import csv
import math
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent


def parse_summary(path: Path) -> dict:
    """Parse key=value lines from summary.txt."""
    out = {}
    if not path.exists():
        return out
    for line in path.read_text().splitlines():
        if "=" in line:
            k, v = line.split("=", 1)
            out[k.strip()] = v.strip()
    return out


def fit_phasor(t: np.ndarray, sig: np.ndarray, omega: float):
    """3-parameter LS: signal(t) = re*cos(wt) + im*sin(wt) + dc. Returns (re, im, dc, r2)."""
    cos_t = np.cos(omega * t)
    sin_t = np.sin(omega * t)
    M = np.column_stack([cos_t, sin_t, np.ones_like(t)])
    coeffs, *_ = np.linalg.lstsq(M, sig, rcond=None)
    re, im, dc = float(coeffs[0]), float(coeffs[1]), float(coeffs[2])
    fit = re * cos_t + im * sin_t + dc
    ss_res = float(np.sum((sig - fit) ** 2))
    ss_tot = float(np.sum((sig - sig.mean()) ** 2))
    r2 = 1.0 - ss_res / ss_tot if ss_tot > 0 else 0.0
    return re, im, dc, r2


def compute_z(v_re, v_im, i_re, i_im):
    """Standard EIS phasor division. Returns (re, im) of Z = V/I."""
    denom = i_re * i_re + i_im * i_im
    if denom < 1e-30:
        return float("nan"), float("nan")
    z_real = (v_re * i_re + v_im * i_im) / denom
    z_imag = -((v_im * i_re - v_re * i_im) / denom)  # match sim sign convention
    return z_real, z_imag

# Species color map (consistent with sim's species.rs intent)
SPECIES_COLORS = {
    "FoilMetal": "#444",
    "LithiumMetal": "#888",
    "LithiumIon": "#fc0",        # yellow Li+
    "ElectrolyteAnion": "#06f",  # blue PF6-
    "EC": "#7c7",                 # light green
    "DMC": "#373",                # dark green
}


def load_snapshot(path: Path):
    with path.open() as f:
        header = f.readline().strip()
        meta = {}
        for kv in header.lstrip("#").split():
            if "=" in kv:
                k, v = kv.split("=", 1)
                meta[k] = v
        rdr = csv.DictReader(f)
        rows = list(rdr)
    return meta, rows


def render_snapshot(rows, meta, ax):
    by_species = {}
    for r in rows:
        by_species.setdefault(r["species"], []).append((float(r["x"]), float(r["y"]), float(r["charge"])))
    for species, pts in by_species.items():
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        color = SPECIES_COLORS.get(species, "#999")
        # FoilMetal/LithiumMetal: color by charge (red=neg, blue=pos, gray=0)
        if species in ("FoilMetal", "LithiumMetal"):
            charges = np.array([p[2] for p in pts])
            ax.scatter(xs, ys, s=8, c=charges, cmap="RdBu", vmin=-0.5, vmax=0.5,
                       edgecolor="none")
        else:
            ax.scatter(xs, ys, s=4, color=color, alpha=0.8, edgecolor="none")
    ax.set_aspect("equal")
    # Annotation
    t = float(meta.get("time_fs", 0))
    v = float(meta.get("v_cell", 0))
    i = float(meta.get("applied_i", 0))
    ax.set_title(f"t={t/1000:.2f} ps · V={v*1000:+.1f} mV · I={i:+.4f}", fontsize=8)
    ax.set_xticks([])
    ax.set_yticks([])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--in-dir", default=str(REPO_ROOT / "doe_results" / "eis_single_case_deep"))
    ap.add_argument("--out-dir", default=str(REPO_ROOT / "images" / "eis_single_case_deep"))
    ap.add_argument("--ref-re-z", type=float, default=None,
                    help="DOE reference Re(Z) to annotate on plot")
    ap.add_argument("--ref-im-z", type=float, default=None,
                    help="DOE reference Im(Z) to annotate on plot")
    ap.add_argument("--ref-source", type=str, default="DOE",
                    help="label for reference source (e.g. doe-021)")
    args = ap.parse_args()

    in_dir = Path(args.in_dir)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    summary = parse_summary(in_dir / "summary.txt")
    freq = float(summary.get("freq", 0.0))
    amplitude = float(summary.get("amplitude", 0.0))
    omega = 2 * math.pi * freq

    snapshot_dir = in_dir / "snapshots"
    snapshot_paths = sorted(snapshot_dir.glob("frame_*.csv"))
    print(f"Found {len(snapshot_paths)} snapshots in {snapshot_dir}")

    # === Panel 1: Snapshot grid (4×8 = 32 frames) ===
    n = len(snapshot_paths)
    cols = 8
    rows = int(np.ceil(n / cols))
    fig, axes = plt.subplots(rows, cols, figsize=(cols * 2.0, rows * 2.0))
    axes = np.array(axes).reshape(-1)
    for i, sp in enumerate(snapshot_paths):
        meta, body_rows = load_snapshot(sp)
        render_snapshot(body_rows, meta, axes[i])
    for i in range(n, len(axes)):
        axes[i].axis("off")
    fig.suptitle(
        f"Particle snapshots across one cycle — eis_single_case_deep\n"
        f"foils: red=more electrons (negative), blue=more positive; electrolyte: yellow=Li+, blue=PF6-, green=solvent",
        fontsize=10,
    )
    fig.tight_layout()
    snapshot_grid_path = out_dir / "snapshot_grid.png"
    fig.savefig(snapshot_grid_path, dpi=120)
    plt.close(fig)
    print(f"wrote {snapshot_grid_path}")

    # === Panel 2: Series time-domain ===
    series_path = in_dir / "series.csv"
    with series_path.open() as f:
        rdr = csv.DictReader(f)
        rows = list(rdr)
    t = np.array([float(r["time_fs"]) for r in rows])
    ac = np.array([float(r["ac_value"]) for r in rows])
    appi = np.array([float(r["applied_i"]) for r in rows])
    v_calc = np.array([float(r["v_cell_calc"]) for r in rows])
    v_eis = np.array([float(r["v_cell_eis"]) for r in rows])
    fa = np.array([int(r["foil_a_de"]) for r in rows])
    fb = np.array([int(r["foil_b_de"]) for r in rows])

    # === Compute local phasor fit (V/I → Z) on the full recorded series ===
    # The DOE lock-in fits the SAME signals; we replicate it here for direct comparison.
    t_rel = t - t[0]
    i_re, i_im, _, i_r2 = fit_phasor(t_rel, appi, omega)
    v_re_eis, v_im_eis, _, v_r2_eis = fit_phasor(t_rel, v_eis, omega)
    v_re_calc, v_im_calc, _, v_r2_calc = fit_phasor(t_rel, v_calc, omega)
    z_eis_re, z_eis_im = compute_z(v_re_eis, v_im_eis, i_re, i_im)
    z_calc_re, z_calc_im = compute_z(v_re_calc, v_im_calc, i_re, i_im)
    i_amp_fit = math.hypot(i_re, i_im)
    v_amp_eis = math.hypot(v_re_eis, v_im_eis)
    v_amp_calc = math.hypot(v_re_calc, v_im_calc)

    fig, axes = plt.subplots(5, 1, figsize=(11, 12), sharex=True)
    axes[0].plot(t / 1000, ac, color="purple")
    axes[0].set_ylabel("ac_value (commanded\nfor group A, e/fs)")
    axes[0].grid(alpha=0.3)
    axes[0].axhline(0, color="k", lw=0.4)

    axes[1].plot(t / 1000, appi, color="tab:red")
    axes[1].set_ylabel("applied_i\n(EIS lock-in I, e/fs)")
    axes[1].grid(alpha=0.3)
    axes[1].axhline(0, color="k", lw=0.4)

    axes[2].plot(t / 1000, v_calc * 1000, color="tab:blue", lw=0.8, label="calculate_cell_voltage")
    axes[2].set_ylabel("V_cell (mV)\nplotting/analysis.rs")
    axes[2].grid(alpha=0.3)
    axes[2].axhline(0, color="k", lw=0.4)
    axes[2].legend(fontsize=8, loc="upper right")
    calc_box = (f"local fit (calc):\n"
                f"  V_amp = {v_amp_calc*1000:.2f} mV (R²={v_r2_calc:.3f})\n"
                f"  Re(Z) = {z_calc_re:+.4f}\n"
                f" -Im(Z) = {-z_calc_im:+.4f}\n"
                f"  |Z|   = {math.hypot(z_calc_re, z_calc_im):.4f}\n"
                f"  phase = {math.degrees(math.atan2(z_calc_im, z_calc_re)):+.1f}°")
    axes[2].text(0.01, 0.02, calc_box, transform=axes[2].transAxes,
                 fontsize=8, family="monospace", va="bottom", ha="left",
                 bbox=dict(facecolor="white", edgecolor="tab:blue", alpha=0.85, pad=3))

    axes[3].plot(t / 1000, v_eis * 1000, color="tab:green", lw=0.8, label="compute_eis_voltage_by_potential")
    axes[3].set_ylabel("V_cell (mV)\nsimulation.rs\n(used by EIS lock-in)")
    axes[3].grid(alpha=0.3)
    axes[3].axhline(0, color="k", lw=0.4)
    axes[3].legend(fontsize=8, loc="upper right")
    # EIS local fit annotation, plus DOE reference (if provided)
    eis_lines = [
        f"local fit (eis):",
        f"  V_amp = {v_amp_eis*1000:.2f} mV (R²={v_r2_eis:.3f})",
        f"  I_amp = {i_amp_fit:.4e} (R²={i_r2:.3f})",
        f"  Re(Z) = {z_eis_re:+.4f}",
        f" -Im(Z) = {-z_eis_im:+.4f}",
        f"  |Z|   = {math.hypot(z_eis_re, z_eis_im):.4f}",
        f"  phase = {math.degrees(math.atan2(z_eis_im, z_eis_re)):+.1f}°",
    ]
    if args.ref_re_z is not None and args.ref_im_z is not None:
        eis_lines += [
            "",
            f"{args.ref_source} reference:",
            f"  Re(Z) = {args.ref_re_z:+.4f}",
            f" -Im(Z) = {-args.ref_im_z:+.4f}",
            f"  Δ Re   = {z_eis_re - args.ref_re_z:+.4f}",
            f"  Δ-Im   = {(-z_eis_im) - (-args.ref_im_z):+.4f}",
        ]
    axes[3].text(0.01, 0.02, "\n".join(eis_lines), transform=axes[3].transAxes,
                 fontsize=8, family="monospace", va="bottom", ha="left",
                 bbox=dict(facecolor="white", edgecolor="tab:green", alpha=0.85, pad=3))

    axes[4].plot(t / 1000, fa.cumsum(), color="tab:orange", label="foil A cumulative ΔN_e")
    axes[4].plot(t / 1000, fb.cumsum(), color="tab:green", label="foil B cumulative ΔN_e")
    axes[4].set_ylabel("foil electron count\n(cumulative since recording start)")
    axes[4].set_xlabel("time (ps)")
    axes[4].grid(alpha=0.3)
    axes[4].axhline(0, color="k", lw=0.4)
    axes[4].legend()

    series_plot = out_dir / "series.png"
    title = f"{in_dir.name} — I_amp={amplitude:g}, f={freq:.3e} 1/fs"
    if args.ref_re_z is not None:
        title += f"\nDOE ref ({args.ref_source}): Re(Z)={args.ref_re_z:+.4f}, -Im(Z)={-args.ref_im_z:+.4f}"
    fig.suptitle(title, fontsize=11)
    fig.tight_layout()
    fig.savefig(series_plot, dpi=120)
    plt.close(fig)
    print(f"wrote {series_plot}")

    # === Panel 3: Phase plot V_cell vs applied_I ===
    fig, ax = plt.subplots(figsize=(7, 7))
    ax.plot(appi, v_eis * 1000, color="black", lw=1)
    ax.set_xlabel("applied_i (e/fs)")
    ax.set_ylabel("V_cell (mV)")
    ax.axhline(0, color="gray", lw=0.5)
    ax.axvline(0, color="gray", lw=0.5)
    liss_title = (f"Lissajous: V_cell vs I_applied (one cycle)\n"
                  f"local fit: Re(Z)={z_eis_re:+.4f}, -Im(Z)={-z_eis_im:+.4f}, "
                  f"phase={math.degrees(math.atan2(z_eis_im, z_eis_re)):+.1f}°")
    if args.ref_re_z is not None:
        liss_title += f"\n{args.ref_source}: Re(Z)={args.ref_re_z:+.4f}, -Im(Z)={-args.ref_im_z:+.4f}"
    ax.set_title(liss_title)
    ax.grid(alpha=0.3)
    lissajous_path = out_dir / "lissajous.png"
    fig.tight_layout()
    fig.savefig(lissajous_path, dpi=120)
    plt.close(fig)
    print(f"wrote {lissajous_path}")


if __name__ == "__main__":
    main()
