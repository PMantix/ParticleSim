"""Render per-test PNG plots from physics-invariant timeseries CSVs.

Reads a CSV produced by `cargo run --bin physics_invariants -- --test <name>`
and writes a PNG alongside it. Per-test plotting logic dispatches by test name.

Usage:
    python scripts/plot_invariant.py --test charge_balance
    python scripts/plot_invariant.py --csv path/to/timeseries.csv --out path.png

When invoked with --test only, default paths are used:
    csv:    doe_results/physics_validation/<test>/timeseries.csv
    result: doe_results/physics_validation/<test>/result.json   (for tolerance lookup)
    out:    doe_results/physics_validation/<test>/<test>.png
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent
RESULTS_ROOT = REPO_ROOT / "doe_results" / "physics_validation"


def _load_result(result_path: Path) -> dict | None:
    if not result_path.exists():
        return None
    with result_path.open() as f:
        return json.load(f)


def _read_csv(csv_path: Path) -> dict:
    """Return columns as a dict of name -> np.ndarray."""
    arr = np.genfromtxt(csv_path, delimiter=",", names=True, dtype=float)
    return {name: arr[name] for name in arr.dtype.names}


def plot_charge_balance(csv_path: Path, out_path: Path, result: dict | None) -> None:
    cols = _read_csv(csv_path)
    step = cols["step"].astype(int)
    sigma_q = cols["sigma_q"]
    abs_dev = cols["abs_dev_from_q0"]

    tol = None
    test_pass = None
    value = None
    seed = None
    n_bodies = None
    if result is not None:
        tol = result.get("tolerance", {}).get("value")
        test_pass = result.get("pass")
        value = result.get("value")
        seed = result.get("seed")
        details = result.get("details", {})
        n_bodies = details.get("n_bodies")

    fig, (ax_top, ax_bot) = plt.subplots(2, 1, figsize=(9, 6.5), sharex=True)

    q0 = sigma_q[0]
    ax_top.axhline(q0, color="#888", linestyle=":", linewidth=1, label=f"Σq₀ = {q0:.3g}")
    if tol is not None:
        ax_top.fill_between(
            step,
            q0 - tol,
            q0 + tol,
            color="#88cc88",
            alpha=0.15,
            label=f"tolerance band ±{tol:.1e} e",
        )
    ax_top.plot(step, sigma_q, color="#1f77b4", linewidth=1.4, marker="o", markersize=3)
    ax_top.set_ylabel("Σ body.charge  (e)")
    ax_top.legend(loc="upper right", fontsize=9)

    if tol is not None:
        ymargin = max(tol * 3.0, 1e-12)
        ax_top.set_ylim(q0 - ymargin, q0 + ymargin)

    nonzero = abs_dev[abs_dev > 0.0]
    if nonzero.size > 0:
        ax_bot.set_yscale("log")
        floor = max(nonzero.min() * 0.5, 1e-20)
        ceiling = max(abs_dev.max(), tol or 0.0) * 2.0
        ax_bot.set_ylim(floor, ceiling)
        ax_bot.plot(step, np.maximum(abs_dev, floor), color="#d62728", linewidth=1.2, marker="o", markersize=2)
        if tol is not None:
            ax_bot.axhline(tol, color="#cc6600", linestyle="--", linewidth=1.2, label=f"tolerance {tol:.1e}")
            ax_bot.legend(loc="upper right", fontsize=9)
    else:
        # All deviations exactly zero — log scale would blow up. Use linear range
        # and annotate.
        if tol is not None:
            ax_bot.set_ylim(-tol * 0.1, tol * 1.5)
            ax_bot.axhline(tol, color="#cc6600", linestyle="--", linewidth=1.2, label=f"tolerance {tol:.1e}")
        ax_bot.plot(step, abs_dev, color="#d62728", linewidth=1.2, marker="o", markersize=2)
        ax_bot.text(
            0.5,
            0.55,
            "All |dev| = 0 exactly\n(electron hops move integer charges atomically;\n"
            "any non-zero value here would indicate a bookkeeping bug)",
            transform=ax_bot.transAxes,
            ha="center",
            va="center",
            fontsize=10,
            color="#444",
            bbox=dict(facecolor="white", edgecolor="#bbb", boxstyle="round,pad=0.6"),
        )
        ax_bot.legend(loc="upper right", fontsize=9)

    ax_bot.set_ylabel("|Σq(t) − Σq₀|  (e)")
    ax_bot.set_xlabel("simulation step")

    status = "PASS" if test_pass else ("FAIL" if test_pass is False else "(no result.json)")
    title_lines = [f"charge_balance — {status}"]
    if value is not None:
        title_lines.append(f"max |dev| = {value:.3e} e")
    meta = []
    if n_bodies is not None:
        meta.append(f"n_bodies={n_bodies}")
    if seed is not None:
        meta.append(f"seed={seed}")
    meta.append(f"steps={int(step.max())}")
    if meta:
        title_lines.append("  ·  ".join(meta))
    fig.suptitle("\n".join(title_lines), fontsize=12)

    fig.tight_layout(rect=(0, 0, 1, 0.94))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_zero_emf_symmetric(csv_path: Path, out_path: Path, result: dict | None) -> None:
    cols = _read_csv(csv_path)
    t = cols["t_fs"]
    q_l = cols["q_left"]
    q_r = cols["q_right"]
    q_d = cols["q_diff"]
    i_l = cols["i_left_e_per_fs"]
    i_r = cols["i_right_e_per_fs"]
    # Hop-activity columns (added 2026-05-05). Tolerate missing columns for
    # backwards-compat with older CSVs.
    e_flips = cols.get("e_flips_in_window")
    distinct = cols.get("distinct_bodies_changed_in_window")
    max_dq = cols.get("max_q_change_in_window")

    tol = None
    test_pass = None
    value = None
    seed = None
    n_bodies = None
    n_foils = None
    equilibrate_fs = None
    measure_fs = None
    hop_activity = None
    if result is not None:
        tol = result.get("tolerance", {}).get("value")
        test_pass = result.get("pass")
        value = result.get("value")
        seed = result.get("seed")
        details = result.get("details", {})
        n_bodies = details.get("n_bodies")
        n_foils = details.get("n_foils")
        equilibrate_fs = details.get("equilibrate_fs")
        measure_fs = details.get("measure_fs")
        hop_activity = details.get("hop_activity")

    # Statistics over second half (matches the binary's logic).
    n_tail = len(t) // 2
    tail_slice = slice(len(t) - n_tail, len(t))

    fig, axs = plt.subplots(2, 3, figsize=(15.0, 8.0), sharex=True)
    (ax_q, ax_qd, ax_flips), (ax_il, ax_ir, ax_dq) = axs

    # Top-left: Q_left & Q_right
    ax_q.plot(t, q_l, color="#1f77b4", linewidth=1.2, label="Q_left")
    ax_q.plot(t, q_r, color="#d62728", linewidth=1.2, label="Q_right")
    ax_q.axhline(0.0, color="#888", linestyle=":", linewidth=1)
    ax_q.set_ylabel("foil net charge  (e)")
    ax_q.legend(loc="upper right", fontsize=9)
    ax_q.set_title("Per-foil net charge")

    # Top-right: Q_diff with tolerance band, mean line, ±std band over tail
    ax_qd.plot(t, q_d, color="#2ca02c", linewidth=1.2)
    ax_qd.axhline(0.0, color="#888", linestyle=":", linewidth=1)
    if tol is not None:
        ax_qd.fill_between(t, -tol, tol, color="#cc6600", alpha=0.10, label=f"tolerance ±{tol:.2f} e")
    if n_tail > 0:
        m = float(np.mean(q_d[tail_slice]))
        s = float(np.std(q_d[tail_slice]))
        ax_qd.axhline(m, color="#cc6600", linestyle="--", linewidth=1, label=f"tail mean={m:+.3e} e")
        ax_qd.axhspan(m - s, m + s, color="#ffaa55", alpha=0.18, label=f"tail ±1σ ({s:.3e} e)")
    ax_qd.set_ylabel("Q_left − Q_right  (e)")
    ax_qd.legend(loc="upper right", fontsize=9)
    ax_qd.set_title("Charge asymmetry  (test metric)")

    # If everything is exactly zero, set explicit y-limits so the plot isn't a
    # zero-thickness line.
    qd_range = float(np.max(np.abs(q_d))) if q_d.size else 0.0
    if qd_range == 0.0 and tol is not None:
        ax_qd.set_ylim(-tol * 1.5, tol * 1.5)
        ax_qd.text(
            0.5,
            0.5,
            "Q_diff exactly zero across full window",
            transform=ax_qd.transAxes,
            ha="center",
            va="center",
            fontsize=10,
            color="#444",
            bbox=dict(facecolor="white", edgecolor="#bbb", boxstyle="round,pad=0.5"),
        )

    # Bottom-left: I_left
    def _plot_current(ax, t, i, label, color):
        ax.plot(t, i, color=color, linewidth=1.0)
        ax.axhline(0.0, color="#888", linestyle=":", linewidth=1)
        if n_tail > 0:
            m = float(np.mean(i[tail_slice]))
            s = float(np.std(i[tail_slice]))
            ax.axhline(m, color=color, linestyle="--", linewidth=1, alpha=0.7,
                       label=f"tail mean={m:+.2e}")
            ax.axhspan(m - s, m + s, color=color, alpha=0.10,
                       label=f"tail ±1σ ({s:.2e})")
        ax.set_ylabel(f"{label}  (e/fs)")
        ax.legend(loc="upper right", fontsize=8)
        ax.set_title(label)
        if i.size and float(np.max(np.abs(i))) == 0.0:
            ax.set_ylim(-1e-3, 1e-3)
            ax.text(
                0.5,
                0.5,
                "exactly zero across window",
                transform=ax.transAxes,
                ha="center",
                va="center",
                fontsize=9,
                color="#666",
                bbox=dict(facecolor="white", edgecolor="#ccc", boxstyle="round,pad=0.4"),
            )

    _plot_current(ax_il, t, i_l, "I_left", "#1f77b4")
    _plot_current(ax_ir, t, i_r, "I_right", "#d62728")

    # Hop-activity panels (column 3)
    def _plot_activity(ax, t, y, label, color, ylabel, is_int):
        if y is None:
            ax.text(0.5, 0.5, "(activity columns missing\nfrom CSV — re-run binary)",
                    transform=ax.transAxes, ha="center", va="center",
                    fontsize=9, color="#999")
            ax.set_title(label)
            return
        if is_int:
            ax.bar(t, y, width=(t[1] - t[0]) * 0.85 if len(t) > 1 else 1.0,
                   color=color, edgecolor="none")
        else:
            ax.plot(t, y, color=color, linewidth=1.0)
        total = float(np.sum(y)) if y.size else 0.0
        peak = float(np.max(y)) if y.size else 0.0
        ax.set_ylabel(ylabel)
        ax.set_title(f"{label}  (Σ={total:.0f}, peak={peak:.3g})")
        if peak == 0.0:
            ax.set_ylim(0, 1)
            ax.text(
                0.5,
                0.55,
                "ZERO\nacross entire window",
                transform=ax.transAxes,
                ha="center",
                va="center",
                fontsize=12,
                color="#a00",
                fontweight="bold",
                bbox=dict(facecolor="#fff8f0", edgecolor="#a00", boxstyle="round,pad=0.6"),
            )

    _plot_activity(
        ax_flips, t, e_flips,
        "Electron-count flips per sample window (any body)",
        "#6a3d9a",
        "flips per window",
        is_int=True,
    )
    _plot_activity(
        ax_dq, t, max_dq,
        "max |Δbody.charge| per sample window",
        "#ff7f0e",
        "max |Δq|  (e)",
        is_int=False,
    )

    for ax in (ax_il, ax_ir, ax_flips, ax_dq):
        ax.set_xlabel("t  (fs, post-equilibration)")

    status = "PASS" if test_pass else ("FAIL" if test_pass is False else "(no result.json)")
    title_lines = [f"zero_emf_symmetric — {status}"]
    if value is not None:
        title_lines.append(f"|<Q_left − Q_right>| = {value:.3e} e")
    if hop_activity is not None:
        flips = hop_activity.get("total_electron_flips", "?")
        bodies_flipped = hop_activity.get("distinct_bodies_flipped", "?")
        if flips == 0:
            title_lines.append(
                f"⚠ hop activity: {flips} flips / {bodies_flipped} bodies — kinetics inactive in this scenario"
            )
        else:
            title_lines.append(f"hop activity: {flips} flips / {bodies_flipped} bodies flipped")
    meta = []
    if n_bodies is not None:
        meta.append(f"n_bodies={n_bodies}")
    if n_foils is not None:
        meta.append(f"n_foils={n_foils}")
    if equilibrate_fs is not None and measure_fs is not None:
        meta.append(f"eq={equilibrate_fs:.0f}fs / meas={measure_fs:.0f}fs")
    if seed is not None:
        meta.append(f"seed={seed}")
    if meta:
        title_lines.append("  ·  ".join(meta))
    fig.suptitle("\n".join(title_lines), fontsize=12)

    fig.tight_layout(rect=(0, 0, 1, 0.91))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_driven_symmetric(csv_path: Path, out_path: Path, result: dict | None) -> None:
    cols = _read_csv(csv_path)
    t = cols["t_fs"]
    q_l = cols["q_left"]
    q_r = cols["q_right"]
    i_l = cols["i_left_e_per_fs"]
    i_r = cols["i_right_e_per_fs"]
    e_flips = cols.get("e_flips_in_window")
    distinct = cols.get("distinct_bodies_changed_in_window")
    max_dq = cols.get("max_q_change_in_window")
    # applied-current columns are constant across the run; pull from the first row
    i_left_applied = float(cols["i_left_applied"][0]) if "i_left_applied" in cols else 0.0
    i_right_applied = float(cols["i_right_applied"][0]) if "i_right_applied" in cols else 0.0

    test_pass = None
    value = None
    seed = None
    n_bodies = None
    n_foils = None
    equilibrate_fs = None
    measure_fs = None
    hop_activity = None
    rel_err_left = None
    rel_err_right = None
    if result is not None:
        test_pass = result.get("pass")
        value = result.get("value")
        seed = result.get("seed")
        details = result.get("details", {})
        n_bodies = details.get("n_bodies")
        n_foils = details.get("n_foils")
        equilibrate_fs = details.get("equilibrate_fs")
        measure_fs = details.get("measure_fs")
        hop_activity = details.get("hop_activity")
        rel_err_left = details.get("rel_err_left")
        rel_err_right = details.get("rel_err_right")

    n_tail = len(t) // 2
    tail_slice = slice(len(t) - n_tail, len(t))

    fig, axs = plt.subplots(2, 3, figsize=(15.0, 8.0), sharex=True)
    (ax_q, ax_il, ax_flips), (ax_qd, ax_ir, ax_dq) = axs

    # Top-left: per-foil cumulative charge (Q_left, Q_right)
    ax_q.plot(t, q_l, color="#1f77b4", linewidth=1.2, label="Q_left")
    ax_q.plot(t, q_r, color="#d62728", linewidth=1.2, label="Q_right")
    ax_q.axhline(0.0, color="#888", linestyle=":", linewidth=1)
    ax_q.set_ylabel("foil net charge  (e)")
    ax_q.legend(loc="best", fontsize=9)
    ax_q.set_title("Per-foil cumulative charge (drive ON)")

    # Bottom-left: charge sum (should hover near 0 if antisymmetric)
    ax_qd.plot(t, q_l + q_r, color="#2ca02c", linewidth=1.2, label="Q_left + Q_right")
    ax_qd.axhline(0.0, color="#888", linestyle=":", linewidth=1)
    ax_qd.set_ylabel("Q_left + Q_right  (e)")
    ax_qd.legend(loc="best", fontsize=9)
    ax_qd.set_title("Antisymmetry check  (sum should ≈ 0)")

    def _plot_current_with_applied(ax, t, i, i_app, label, color):
        ax.plot(t, i, color=color, linewidth=1.0, label=f"measured {label}")
        ax.axhline(i_app, color=color, linestyle="--", linewidth=1.2, alpha=0.8,
                   label=f"applied {label} = {i_app:+.2e}")
        ax.axhline(0.0, color="#888", linestyle=":", linewidth=1)
        if n_tail > 0:
            m = float(np.mean(i[tail_slice]))
            ax.axhline(m, color=color, linestyle="-.", linewidth=1, alpha=0.6,
                       label=f"tail mean = {m:+.2e}")
        ax.set_ylabel(f"{label}  (e/fs)")
        ax.legend(loc="best", fontsize=8)
        ax.set_title(label)

    _plot_current_with_applied(ax_il, t, i_l, i_left_applied, "I_left", "#1f77b4")
    _plot_current_with_applied(ax_ir, t, i_r, i_right_applied, "I_right", "#d62728")

    def _plot_activity(ax, t, y, label, color, ylabel, is_int):
        if y is None:
            ax.text(0.5, 0.5, "(activity columns missing)",
                    transform=ax.transAxes, ha="center", va="center",
                    fontsize=9, color="#999")
            ax.set_title(label)
            return
        if is_int:
            ax.bar(t, y, width=(t[1] - t[0]) * 0.85 if len(t) > 1 else 1.0,
                   color=color, edgecolor="none")
        else:
            ax.plot(t, y, color=color, linewidth=1.0)
        total = float(np.sum(y)) if y.size else 0.0
        peak = float(np.max(y)) if y.size else 0.0
        ax.set_ylabel(ylabel)
        ax.set_title(f"{label}  (Σ={total:.0f}, peak={peak:.3g})")
        if peak == 0.0:
            ax.text(0.5, 0.55, "ZERO across entire window",
                    transform=ax.transAxes, ha="center", va="center",
                    fontsize=11, color="#a00", fontweight="bold",
                    bbox=dict(facecolor="#fff8f0", edgecolor="#a00", boxstyle="round,pad=0.5"))

    _plot_activity(ax_flips, t, e_flips,
                   "Electron-count flips per window",
                   "#6a3d9a", "flips per window", is_int=True)
    _plot_activity(ax_dq, t, max_dq,
                   "max |Δbody.charge| per window",
                   "#ff7f0e", "max |Δq|  (e)", is_int=False)

    for ax in (ax_qd, ax_ir, ax_dq):
        ax.set_xlabel("t  (fs, post-equilibration)")

    status = "PASS" if test_pass else ("FAIL" if test_pass is False else "(no result.json)")
    title_lines = [f"driven_symmetric — {status}"]
    if value is not None and rel_err_left is not None and rel_err_right is not None:
        title_lines.append(
            f"worst rel_err = {value:.3f}  ·  L={rel_err_left:.3f}  R={rel_err_right:.3f}"
        )
    if hop_activity is not None:
        flips = hop_activity.get("total_electron_flips", "?")
        bodies_flipped = hop_activity.get("distinct_bodies_flipped", "?")
        flips_by_species = hop_activity.get("flips_by_species", {})
        if isinstance(flips, int) and flips > 0:
            top = sorted(flips_by_species.items(), key=lambda kv: -int(kv[1]))[:4]
            top_str = ", ".join(f"{k}:{v}" for k, v in top)
            title_lines.append(
                f"hop activity: {flips} flips / {bodies_flipped} bodies  ·  top species: {top_str}"
            )
        else:
            title_lines.append(f"hop activity: {flips} flips / {bodies_flipped} bodies")
    meta = []
    if n_bodies is not None:
        meta.append(f"n_bodies={n_bodies}")
    if n_foils is not None:
        meta.append(f"n_foils={n_foils}")
    if equilibrate_fs is not None and measure_fs is not None:
        meta.append(f"eq={equilibrate_fs:.0f}fs / meas={measure_fs:.0f}fs")
    meta.append(f"applied=±{abs(i_right_applied):.2e} e/fs")
    if seed is not None:
        meta.append(f"seed={seed}")
    title_lines.append("  ·  ".join(meta))
    fig.suptitle("\n".join(title_lines), fontsize=11)

    fig.tight_layout(rect=(0, 0, 1, 0.90))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_nve_energy_drift(csv_path: Path, out_path: Path, result: dict | None) -> None:
    cols = _read_csv(csv_path)
    t = cols["t_fs"]
    ke = cols["ke"]
    pe = cols["pe_coulomb"]
    e_total = cols["e_total"]
    t_kelvin = cols.get("t_kelvin")

    test_pass = None
    value = None
    seed = None
    n_bodies = None
    slope = None
    drift_over_window = None
    r2 = None
    measure_fs = None
    tol = None
    if result is not None:
        test_pass = result.get("pass")
        value = result.get("value")
        seed = result.get("seed")
        details = result.get("details", {})
        n_bodies = details.get("n_bodies")
        slope = details.get("slope_per_fs")
        drift_over_window = details.get("drift_over_window")
        r2 = details.get("linear_fit_r2")
        measure_fs = details.get("measure_fs")
        tol = result.get("tolerance", {}).get("value")

    fig, axs = plt.subplots(3, 1, figsize=(10, 9), sharex=True)
    ax_ke, ax_pe, ax_e = axs

    ax_ke.plot(t, ke, color="#d62728", linewidth=1.2, label="KE")
    ax_ke.set_ylabel("KE (sim units)")
    ax_ke.legend(loc="best", fontsize=9)
    ax_ke.set_title("Kinetic energy (thermostat OFF after t=0)")

    if t_kelvin is not None:
        ax_ke_t = ax_ke.twinx()
        ax_ke_t.plot(t, t_kelvin, color="#888", linewidth=0.8, alpha=0.5, linestyle=":", label="T_eff")
        ax_ke_t.set_ylabel("T_eff  (K)", color="#666")
        ax_ke_t.tick_params(axis="y", colors="#666")
        ax_ke_t.legend(loc="lower right", fontsize=8)

    ax_pe.plot(t, pe, color="#2ca02c", linewidth=1.2, label="PE_coulomb")
    ax_pe.set_ylabel("PE (sim units)")
    ax_pe.legend(loc="best", fontsize=9)
    ax_pe.set_title("Coulomb potential energy (LJ + repulsion not included)")

    ax_e.plot(t, e_total, color="#1f77b4", linewidth=1.4, label="E_total = KE + PE")
    if slope is not None and len(t) > 1:
        # Reproduce the linear fit overlay
        intercept = float(np.mean(e_total) - slope * np.mean(t))
        ax_e.plot(t, slope * t + intercept, color="#ff7f0e", linewidth=1.0,
                  linestyle="--", label=f"linear fit (slope={slope:+.2e} /fs, R²={r2:.3f})")
    ax_e.set_ylabel("E_total (sim units)")
    ax_e.set_xlabel("t  (fs, post-equilibration)")
    ax_e.legend(loc="best", fontsize=9)
    ax_e.set_title("Total energy with linear-drift fit")

    status = "PASS" if test_pass else ("FAIL" if test_pass is False else "(no result.json)")
    title_lines = [f"nve_energy_drift — {status}"]
    if value is not None and tol is not None:
        title_lines.append(f"drift fraction = {value:.4f}  (tol {tol:.1f})")
    if drift_over_window is not None and measure_fs is not None:
        title_lines.append(f"drift over {measure_fs:.0f} fs: {drift_over_window:+.3e} (sim energy)")
    meta = []
    if n_bodies is not None:
        meta.append(f"n_bodies={n_bodies}")
    if seed is not None:
        meta.append(f"seed={seed}")
    if meta:
        title_lines.append("  ·  ".join(meta))
    fig.suptitle("\n".join(title_lines), fontsize=12)

    fig.tight_layout(rect=(0, 0, 1, 0.92))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_quadtree_force_error(csv_path: Path, out_path: Path, result: dict | None) -> None:
    cols = _read_csv(csv_path)
    brute_mag = cols["brute_mag"]
    qt_mag = cols["qt_mag"]
    rel_err = cols["rel_err"]

    title_lines = ["quadtree_force_error"]
    if result is not None:
        title_lines[0] += f" — {'PASS' if result.get('pass') else 'FAIL'}"
        details = result.get("details", {})
        title_lines.append(
            f"L2-normalised err = {result.get('value', 0):.4f}  ·  "
            f"max per-body rel = {details.get('max_rel_err_per_body', 0):.3f}  ·  "
            f"θ = {details.get('quadtree_theta', '?')}  ·  "
            f"n_bodies = {details.get('n_bodies', '?')}"
        )

    fig, axs = plt.subplots(1, 2, figsize=(12, 5))
    ax_scatter, ax_hist = axs

    # Scatter: |F_qt| vs |F_brute| (perfect agreement → diagonal)
    max_v = max(brute_mag.max(), qt_mag.max()) * 1.1 if brute_mag.size else 1
    min_v = max(min(brute_mag.min(), qt_mag.min()), 1e-12)
    ax_scatter.loglog([min_v, max_v], [min_v, max_v], "k--", linewidth=1, alpha=0.5, label="perfect agreement")
    ax_scatter.loglog(brute_mag.clip(min=min_v), qt_mag.clip(min=min_v),
                      "o", color="#1f77b4", markersize=4, alpha=0.5)
    ax_scatter.set_xlabel("|F_brute|")
    ax_scatter.set_ylabel("|F_quadtree|")
    ax_scatter.set_title("Per-body magnitude scatter")
    ax_scatter.legend(loc="best", fontsize=9)
    ax_scatter.grid(True, which="both", alpha=0.3)

    # Histogram of per-body relative errors
    nz = rel_err[rel_err > 0]
    if nz.size:
        ax_hist.hist(nz, bins=30, color="#ff7f0e", edgecolor="white")
        ax_hist.axvline(np.median(nz), color="#cc6600", linestyle="--", linewidth=1, label=f"median = {np.median(nz):.3f}")
    ax_hist.set_xlabel("per-body relative error")
    ax_hist.set_ylabel("count")
    ax_hist.set_title("Distribution of per-body relative errors")
    ax_hist.legend(loc="best", fontsize=9)

    fig.suptitle("\n".join(title_lines), fontsize=12)
    fig.tight_layout(rect=(0, 0, 1, 0.93))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_mb_velocity_distribution(csv_path: Path, out_path: Path, result: dict | None) -> None:
    cols = _read_csv(csv_path)
    v_low = cols["v_low"]
    v_high = cols["v_high"]
    observed = cols["observed"]
    exp_emp = cols["expected_at_T_empirical"]
    exp_target = cols["expected_at_T_target"]
    used = cols.get("used")

    bin_centers = (v_low + v_high) / 2.0
    bin_width = float(v_high[0] - v_low[0]) if v_low.size else 1.0

    title_lines = ["mb_velocity_distribution"]
    if result is not None:
        title_lines[0] += f" — {'PASS' if result.get('pass') else 'FAIL'}"
        details = result.get("details", {})
        title_lines.append(
            f"χ²/dof = {result.get('value', 0):.3f}  ·  "
            f"T_target = {details.get('target_temperature_K', '?'):.0f} K  ·  "
            f"T_empirical = {details.get('empirical_temperature_K', '?'):.1f} K  ·  "
            f"samples = {details.get('n_speed_samples', '?')}"
        )

    fig, ax = plt.subplots(figsize=(10, 6))
    ax.bar(bin_centers, observed, width=bin_width * 0.85, color="#1f77b4", alpha=0.8, label="observed")
    ax.plot(bin_centers, exp_emp, "o-", color="#ff7f0e", linewidth=1.4, markersize=5,
            label="expected MB at T_empirical")
    ax.plot(bin_centers, exp_target, "s--", color="#888", linewidth=1.0, markersize=4, alpha=0.8,
            label="expected MB at T_target")
    if used is not None:
        bins_used = np.where(used > 0.5)[0]
        if len(bins_used):
            v_first = bin_centers[bins_used[0]] - bin_width / 2
            v_last = bin_centers[bins_used[-1]] + bin_width / 2
            ax.axvspan(v_first, v_last, color="#88cc88", alpha=0.08, label="bins used in χ²")
    ax.set_xlabel("speed |v|  (Å/fs)")
    ax.set_ylabel("count")
    ax.legend(loc="best", fontsize=9)
    ax.set_yscale("log")
    ax.set_ylim(0.5, observed.max() * 2 if observed.size else 100)
    fig.suptitle("\n".join(title_lines), fontsize=12)
    fig.tight_layout(rect=(0, 0, 1, 0.92))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_nernst_einstein(csv_path: Path, out_path: Path, result: dict | None) -> None:
    """Plot MSD (with linear D fit) + drift trajectory + σ comparison summary."""
    # csv_path here points to msd.csv. The matching drift.csv lives next to it.
    msd_cols = _read_csv(csv_path)
    t_msd = msd_cols["t_fs"]
    msd_li = msd_cols["msd_li_a2"]
    msd_an = msd_cols["msd_anion_a2"]

    drift_path = csv_path.parent / "drift.csv"
    if drift_path.exists():
        drift_cols = _read_csv(drift_path)
        t_dr = drift_cols["t_fs"]
        v_li = drift_cols["v_li_x"]
        v_an = drift_cols["v_anion_x"]
    else:
        t_dr = v_li = v_an = np.array([])

    title_lines = ["nernst_einstein"]
    ratio = None
    sigma_meas = None
    sigma_ne = None
    d_li = None
    d_an = None
    field_mag = None
    pass_flag = None
    band = (0.5, 2.0)
    if result is not None:
        d = result.get("details", {})
        ratio = result.get("value")
        pass_flag = result.get("pass")
        sigma_meas = d.get("sigma_measured")
        sigma_ne = d.get("sigma_ne_total")
        d_li = d.get("d_li_a2_per_fs")
        d_an = d.get("d_anion_a2_per_fs")
        field_mag = d.get("field_magnitude_simunits")
        tol = result.get("tolerance", {})
        band = (tol.get("min", 0.5), tol.get("max", 2.0))
        title_lines[0] += f" — {'PASS' if pass_flag else 'FAIL'}"
        title_lines.append(
            f"σ_meas/σ_NE = {ratio:.4f}   target band = [{band[0]:.1f}, {band[1]:.1f}]   "
            f"Haven ratio (1/ratio) = {1.0/ratio:.2f}" if ratio else "(no σ ratio)"
        )
        title_lines.append(
            f"D_Li⁺ = {d_li:.3e} Å²/fs    D_anion = {d_an:.3e} Å²/fs    E = {field_mag:.2e} sim_V/Å"
        )

    fig, axs = plt.subplots(2, 2, figsize=(13, 8))
    (ax_msd, ax_drift), (ax_msd_log, ax_sigma) = axs

    # MSD (linear)
    ax_msd.plot(t_msd, msd_li, "o-", color="#ff7f0e", linewidth=1.4, markersize=4, label="Li⁺ MSD")
    ax_msd.plot(t_msd, msd_an, "s-", color="#22aaff", linewidth=1.4, markersize=4, label="anion MSD")
    if d_li is not None:
        ax_msd.plot(t_msd, 4.0 * d_li * t_msd, "--", color="#cc6600", linewidth=1.0,
                    label=f"4·D_Li·t = {d_li*4:.2e}·t")
    if d_an is not None:
        ax_msd.plot(t_msd, 4.0 * d_an * t_msd, "--", color="#003366", linewidth=1.0,
                    label=f"4·D_anion·t = {d_an*4:.2e}·t")
    ax_msd.set_xlabel("t  (fs)")
    ax_msd.set_ylabel("⟨r²⟩  (Å²)")
    ax_msd.set_title("Mean squared displacement (linear)")
    ax_msd.legend(loc="best", fontsize=9)
    ax_msd.grid(True, alpha=0.3)

    # MSD (log-log to show diffusive scaling)
    if t_msd.size and t_msd[1:].min() > 0:
        ax_msd_log.loglog(t_msd[1:], msd_li[1:], "o-", color="#ff7f0e", linewidth=1.4, markersize=4, label="Li⁺")
        ax_msd_log.loglog(t_msd[1:], msd_an[1:], "s-", color="#22aaff", linewidth=1.4, markersize=4, label="anion")
        # ideal slope-1 reference
        ref_t = t_msd[1:]
        ref_y = ref_t * msd_li[1] / t_msd[1] if msd_li[1] > 0 else None
        if ref_y is not None:
            ax_msd_log.loglog(ref_t, ref_y, ":", color="#888", linewidth=1.0, label="slope=1 (diffusive)")
        ax_msd_log.set_xlabel("t  (fs, log)")
        ax_msd_log.set_ylabel("⟨r²⟩  (Å², log)")
        ax_msd_log.set_title("MSD log-log (slope=1 → diffusive)")
        ax_msd_log.legend(loc="best", fontsize=9)
        ax_msd_log.grid(True, which="both", alpha=0.3)

    # Drift trajectory
    if v_li.size:
        ax_drift.plot(t_dr, v_li, "o-", color="#ff7f0e", linewidth=1.0, markersize=3, label="⟨v_Li_x⟩")
        ax_drift.plot(t_dr, v_an, "s-", color="#22aaff", linewidth=1.0, markersize=3, label="⟨v_anion_x⟩")
        ax_drift.axhline(0.0, color="#888", linestyle=":", linewidth=1)
        # Mean of tail
        n_tail = len(t_dr) // 2
        if n_tail > 0:
            mean_li = float(np.mean(v_li[n_tail:]))
            mean_an = float(np.mean(v_an[n_tail:]))
            ax_drift.axhline(mean_li, color="#ff7f0e", linestyle="--", linewidth=1, alpha=0.7,
                             label=f"⟨v_Li⟩_tail = {mean_li:+.2e}")
            ax_drift.axhline(mean_an, color="#22aaff", linestyle="--", linewidth=1, alpha=0.7,
                             label=f"⟨v_anion⟩_tail = {mean_an:+.2e}")
        ax_drift.set_xlabel("t  (fs, post-field-equilibration)")
        ax_drift.set_ylabel("drift velocity  (Å/fs)")
        ax_drift.set_title("Mean per-species x-drift under applied field")
        ax_drift.legend(loc="best", fontsize=8)
        ax_drift.grid(True, alpha=0.3)
    else:
        ax_drift.text(0.5, 0.5, "(drift.csv missing)", transform=ax_drift.transAxes,
                      ha="center", va="center", fontsize=10, color="#999")
        ax_drift.set_title("Drift trajectory")

    # σ summary bar/comparison
    if sigma_meas is not None and sigma_ne is not None:
        labels = ["σ_measured", "σ_NE"]
        vals = [abs(sigma_meas), abs(sigma_ne)]
        colors = ["#33a02c", "#cc6600"]
        ax_sigma.bar(labels, vals, color=colors, edgecolor="white")
        ax_sigma.set_yscale("log")
        ax_sigma.set_ylabel("|σ|  (e²/(sim_energy·fs))")
        ax_sigma.set_title("Conductivity comparison (note log scale)")
        # Annotate with ratio
        if ratio is not None:
            ax_sigma.text(0.5, 0.95, f"σ_meas / σ_NE = {ratio:.4f}", transform=ax_sigma.transAxes,
                          ha="center", va="top", fontsize=11,
                          bbox=dict(facecolor="white", edgecolor="#999", boxstyle="round,pad=0.4"))
            # Show target band
            ax_sigma.axhspan(abs(sigma_ne) * band[0], abs(sigma_ne) * band[1], color="#88cc88",
                             alpha=0.2, label=f"target band [{band[0]:.1f}, {band[1]:.1f}]·σ_NE")
            ax_sigma.legend(loc="lower right", fontsize=9)
    else:
        ax_sigma.text(0.5, 0.5, "(no σ values)", transform=ax_sigma.transAxes,
                      ha="center", va="center", fontsize=10, color="#999")

    fig.suptitle("\n".join(title_lines), fontsize=11)
    fig.tight_layout(rect=(0, 0, 1, 0.92))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_kinetic_inductance_scaling(
    csv_path: Path, out_path: Path, result: dict | None
) -> None:
    """Plot D vs L (should be flat) + τ_KI vs L² (should be linear) with fit overlay."""
    cols = _read_csv(csv_path)
    l = cols["L_angstrom"]
    l2 = cols["L_squared"]
    d_li = cols["d_li"]
    d_an = cols["d_anion"]
    tau_li = cols["tau_ki_li"]
    tau_an = cols["tau_ki_anion"]

    title_lines = ["kinetic_inductance_scaling"]
    pass_flag = None
    d_li_cv = None
    d_an_cv = None
    li_r2 = None
    an_r2 = None
    li_slope = None
    li_intercept = None
    an_slope = None
    an_intercept = None
    if result is not None:
        d = result.get("details", {})
        pass_flag = result.get("pass")
        d_li_cv = d.get("d_li_cv")
        d_an_cv = d.get("d_anion_cv")
        li_r2 = d.get("tau_li_vs_l2_r2")
        an_r2 = d.get("tau_anion_vs_l2_r2")
        li_slope = d.get("tau_li_vs_l2_slope")
        li_intercept = d.get("tau_li_vs_l2_intercept")
        an_slope = d.get("tau_anion_vs_l2_slope")
        an_intercept = d.get("tau_anion_vs_l2_intercept")
        title_lines[0] += f" — {'PASS' if pass_flag else 'FAIL'}"
        title_lines.append(
            f"CV(D_Li⁺) = {d_li_cv:.3f}   CV(D_anion) = {d_an_cv:.3f}   "
            f"R²(τ_Li⁺ vs L²) = {li_r2:.3f}   R²(τ_anion vs L²) = {an_r2:.3f}"
        )

    fig, (ax_d, ax_tau) = plt.subplots(1, 2, figsize=(13, 5.5))

    # Panel 1: D vs L
    ax_d.plot(l, d_li, "o-", color="#ff7f0e", linewidth=1.4, markersize=8, label="D_Li⁺")
    ax_d.plot(l, d_an, "s-", color="#22aaff", linewidth=1.4, markersize=8, label="D_anion")
    if d_li_cv is not None:
        d_li_mean = float(np.mean(d_li))
        d_an_mean = float(np.mean(d_an))
        ax_d.axhline(d_li_mean, color="#ff7f0e", linestyle=":", linewidth=1, alpha=0.7,
                     label=f"⟨D_Li⟩ = {d_li_mean:.2e}")
        ax_d.axhline(d_an_mean, color="#22aaff", linestyle=":", linewidth=1, alpha=0.7,
                     label=f"⟨D_anion⟩ = {d_an_mean:.2e}")
        ax_d.fill_between(l, d_li_mean * (1 - d_li_cv), d_li_mean * (1 + d_li_cv),
                          color="#ff7f0e", alpha=0.10)
        ax_d.fill_between(l, d_an_mean * (1 - d_an_cv), d_an_mean * (1 + d_an_cv),
                          color="#22aaff", alpha=0.10)
    ax_d.set_xlabel("domain L  (Å)")
    ax_d.set_ylabel("D  (Å²/fs)")
    ax_d.set_title("D vs L (should be ~flat for bulk-intrinsic transport)")
    ax_d.legend(loc="best", fontsize=9)
    ax_d.grid(True, alpha=0.3)

    # Panel 2: τ_KI vs L²
    ax_tau.plot(l2, tau_li, "o-", color="#ff7f0e", linewidth=1.4, markersize=8, label="τ_KI Li⁺")
    ax_tau.plot(l2, tau_an, "s-", color="#22aaff", linewidth=1.4, markersize=8, label="τ_KI anion")
    if li_slope is not None and li_intercept is not None:
        l2_range = np.linspace(0, max(l2) * 1.05, 50)
        fit_y = li_slope * l2_range + li_intercept
        ax_tau.plot(l2_range, fit_y, "--", color="#cc6600", linewidth=1.0,
                    label=f"Li⁺ fit: slope={li_slope:.2e} fs/Å²  R²={li_r2:.3f}")
    if an_slope is not None and an_intercept is not None:
        l2_range = np.linspace(0, max(l2) * 1.05, 50)
        fit_y = an_slope * l2_range + an_intercept
        ax_tau.plot(l2_range, fit_y, "--", color="#003366", linewidth=1.0,
                    label=f"anion fit: slope={an_slope:.2e} fs/Å²  R²={an_r2:.3f}")
    ax_tau.set_xlabel("L²  (Å²)")
    ax_tau.set_ylabel("τ_KI = L²/D  (fs)")
    ax_tau.set_title("τ_KI vs L² (similitude check)")
    ax_tau.legend(loc="best", fontsize=9)
    ax_tau.grid(True, alpha=0.3)

    fig.suptitle("\n".join(title_lines), fontsize=11)
    fig.tight_layout(rect=(0, 0, 1, 0.92))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_dipole_spike(csv_path: Path, out_path: Path, result: dict | None) -> None:
    """RDF plot for the dipole-solvent spike, with Phase-2.2 baseline overlay."""
    cols = _read_csv(csv_path)
    r = cols["r_center"]
    g = cols["g_of_r"]
    cum = cols["cumulative_n_per_li"]

    title_lines = ["dipole_spike (Phase-3 smoking-gun #4 test)"]
    n_coord = None
    r_peak = None
    r_min = None
    g_peak = None
    pass_flag = None
    target_band = (3.0, 6.0)
    baseline = 2.18
    delta = None
    bond_pre = None
    bond_post = None
    r_eq = None
    dipole_q = None
    if result is not None:
        d = result.get("details", {})
        n_coord = result.get("value")
        pass_flag = result.get("pass")
        r_peak = d.get("r_peak_angstrom")
        r_min = d.get("r_min_angstrom")
        g_peak = d.get("g_peak")
        baseline = d.get("phase_2_2_baseline", 2.18)
        delta = d.get("delta_from_baseline")
        bond_pre = d.get("bond_length_pre_mean")
        bond_post = d.get("bond_length_post_mean")
        r_eq = d.get("r_eq")
        dipole_q = d.get("dipole_charge")
        tol = result.get("tolerance", {})
        target_band = (tol.get("min", 3.0), tol.get("max", 6.0))
        title_lines[0] += f" — {'PASS' if pass_flag else 'FAIL'}"
        title_lines.append(
            f"n_coord = {n_coord:.2f}  ·  baseline (single-charge EC) = {baseline:.2f}  "
            f"·  Δ = {delta:+.2f}  ({100.0 * delta / baseline:+.1f}%)"
        )
        title_lines.append(
            f"dipole: q = ±{dipole_q}, r_eq = {r_eq} Å  ·  bond stable: pre {bond_pre:.2f} Å → post {bond_post:.2f} Å"
        )

    fig, (ax_g, ax_n) = plt.subplots(2, 1, figsize=(10, 8), sharex=True)

    ax_g.plot(r, g, "-", color="#1f77b4", linewidth=1.4, label="g(r) — Li⁺ vs dipole_neg")
    ax_g.axhline(1.0, color="#888", linestyle=":", linewidth=1, label="g(r)=1 (uniform)")
    if r_peak is not None:
        ax_g.axvline(r_peak, color="#ff7f0e", linestyle="--", linewidth=1.2, alpha=0.7,
                     label=f"r_peak = {r_peak:.2f} Å")
    if r_min is not None:
        ax_g.axvline(r_min, color="#d62728", linestyle="--", linewidth=1.2, alpha=0.7,
                     label=f"r_min = {r_min:.2f} Å")
    ax_g.set_ylabel("g(r)")
    ax_g.set_title("Li⁺ → dipole-neg radial distribution")
    ax_g.legend(loc="best", fontsize=9)
    ax_g.grid(True, alpha=0.3)
    if g.size and g.max() > 5:
        ax_g.set_yscale("log")
        ax_g.set_ylim(1e-3, g.max() * 2)

    ax_n.plot(r, cum, "-", color="#33a02c", linewidth=1.5, label="n(r) integrated")
    ax_n.axhspan(target_band[0], target_band[1], color="#ffe699", alpha=0.4,
                 label=f"target band [{target_band[0]:.1f}, {target_band[1]:.1f}]")
    ax_n.axhline(baseline, color="#9999cc", linestyle="--", linewidth=1.2,
                 label=f"Phase-2.2 baseline (single-charge EC) = {baseline:.2f}")
    if n_coord is not None and r_min is not None:
        ax_n.axhline(n_coord, color="#d62728", linestyle="--", linewidth=1.2,
                     label=f"dipole spike n(r_min) = {n_coord:.2f}")
        ax_n.plot([r_min], [n_coord], "o", color="#d62728", markersize=10)
    ax_n.set_xlabel("r  (Å)")
    ax_n.set_ylabel("cumulative n(r)")
    ax_n.set_title("Integrated coordination — comparison with single-charge baseline")
    ax_n.legend(loc="best", fontsize=9)
    ax_n.grid(True, alpha=0.3)

    fig.suptitle("\n".join(title_lines), fontsize=11)
    fig.tight_layout(rect=(0, 0, 1, 0.91))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_li_ec_coordination(csv_path: Path, out_path: Path, result: dict | None) -> None:
    """Plot Li⁺-EC RDF + cumulative coordination number from rdf.csv."""
    cols = _read_csv(csv_path)
    r = cols["r_center"]
    g = cols["g_of_r"]
    cum = cols["cumulative_n_per_li"]

    title_lines = ["li_ec_coordination"]
    n_coord = None
    r_peak = None
    r_min = None
    g_peak = None
    pass_flag = None
    target_band = (3.0, 6.0)
    n_li = None
    n_ec = None
    rho_ec = None
    n_samples = None
    if result is not None:
        d = result.get("details", {})
        n_coord = result.get("value")
        pass_flag = result.get("pass")
        r_peak = d.get("r_peak_angstrom")
        r_min = d.get("r_min_angstrom")
        g_peak = d.get("g_peak")
        n_li = d.get("n_li_avg")
        n_ec = d.get("n_ec_avg")
        rho_ec = d.get("rho_ec_per_a2")
        n_samples = d.get("n_samples")
        tol = result.get("tolerance", {})
        target_band = (tol.get("min", 3.0), tol.get("max", 6.0))
        title_lines[0] += f" — {'PASS' if pass_flag else 'FAIL'}"
        title_lines.append(
            f"n_coord = {n_coord:.2f} EC/Li⁺  ·  target band = [{target_band[0]:.1f}, {target_band[1]:.1f}]  "
            f"·  r_peak = {r_peak:.2f} Å  ·  g_peak = {g_peak:.1f}"
        )
        meta = []
        if n_li is not None:
            meta.append(f"⟨n_Li⟩={n_li:.0f}")
        if n_ec is not None:
            meta.append(f"⟨n_EC⟩={n_ec:.0f}")
        if rho_ec is not None:
            meta.append(f"ρ_EC={rho_ec:.2e} Å⁻²")
        if n_samples is not None:
            meta.append(f"samples={int(n_samples)}")
        title_lines.append("  ·  ".join(meta))

    fig, (ax_g, ax_n) = plt.subplots(2, 1, figsize=(10, 8), sharex=True)

    # Top: g(r)
    ax_g.plot(r, g, "-", color="#1f77b4", linewidth=1.4, label="g(r)")
    ax_g.axhline(1.0, color="#888", linestyle=":", linewidth=1, label="g(r)=1 (uniform reference)")
    if r_peak is not None:
        ax_g.axvline(r_peak, color="#ff7f0e", linestyle="--", linewidth=1.2, alpha=0.7,
                     label=f"r_peak = {r_peak:.2f} Å")
    if r_min is not None:
        ax_g.axvline(r_min, color="#d62728", linestyle="--", linewidth=1.2, alpha=0.7,
                     label=f"r_min = {r_min:.2f} Å")
    ax_g.set_ylabel("g(r)")
    ax_g.set_title("Li⁺-EC radial distribution function")
    ax_g.legend(loc="best", fontsize=9)
    ax_g.grid(True, alpha=0.3)
    if g.size and g.max() > 50:
        ax_g.set_yscale("log")
        ax_g.set_ylim(1e-3, g.max() * 2)

    # Bottom: cumulative coordination
    ax_n.plot(r, cum, "-", color="#33a02c", linewidth=1.5, label="n(r) = ∫₀ʳ g(r')·ρ·2πr' dr'")
    ax_n.axhspan(target_band[0], target_band[1], color="#ffe699", alpha=0.4,
                 label=f"target band [{target_band[0]:.1f}, {target_band[1]:.1f}]")
    if n_coord is not None and r_min is not None:
        ax_n.axhline(n_coord, color="#d62728", linestyle="--", linewidth=1.2, alpha=0.8,
                     label=f"n(r_min) = {n_coord:.2f}")
        ax_n.plot([r_min], [n_coord], "o", color="#d62728", markersize=10)
    if r_peak is not None:
        ax_n.axvline(r_peak, color="#ff7f0e", linestyle="--", linewidth=1.0, alpha=0.5)
    if r_min is not None:
        ax_n.axvline(r_min, color="#d62728", linestyle="--", linewidth=1.0, alpha=0.5)
    ax_n.set_xlabel("r  (Å)")
    ax_n.set_ylabel("cumulative coordination n(r)")
    ax_n.set_title("Integrated coordination number")
    ax_n.legend(loc="best", fontsize=9)
    ax_n.grid(True, alpha=0.3)

    fig.suptitle("\n".join(title_lines), fontsize=11)
    fig.tight_layout(rect=(0, 0, 1, 0.92))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_tafel_slope(csv_path: Path, out_path: Path, result: dict | None) -> None:
    """Plot the Tafel sweep — sweep_summary.csv with one row per amplitude."""
    cols = _read_csv(csv_path)
    amp = cols["amp_applied"]
    abs_i = cols["abs_i_measured"]
    eta = cols["eta_proxy_volts"]
    accepted = cols["accepted_hops"]

    title_lines = ["tafel_slope"]
    cfg_alpha = None
    cfg_scale = None
    rec_alpha = None
    slope_per_v = None
    intercept = None
    r2 = None
    pass_flag = None
    rel_err = None
    linear_amps = None
    if result is not None:
        d = result.get("details", {})
        cfg_alpha = d.get("configured_alpha")
        cfg_scale = d.get("configured_bv_overpotential_scale")
        rec_alpha = result.get("value")
        slope_per_v = d.get("tafel_slope_per_volt")
        intercept = d.get("tafel_intercept")
        r2 = d.get("tafel_fit_r2")
        rel_err = d.get("alpha_relative_error")
        pass_flag = result.get("pass")
        linear_amps = d.get("linear_regime_amps", [])
        title_lines[0] += f" — {'PASS' if pass_flag else 'FAIL'}"
        title_lines.append(
            f"recovered α = {rec_alpha:.3f}  ·  configured α = {cfg_alpha:.2f}  ·  rel_err = {rel_err:.3f}"
        )
        title_lines.append(
            f"Tafel slope = {slope_per_v:.2f} V⁻¹  ·  R² = {r2:.3f}  ·  scale = {cfg_scale:.3f} V"
        )

    fig, axs = plt.subplots(1, 2, figsize=(13, 5.5))
    ax_iv, ax_iv_log = axs

    # Left panel: linear |I| vs |η|
    ax_iv.plot(np.abs(eta), np.abs(abs_i), "o-", color="#1f77b4", linewidth=1.4, markersize=7,
               label="|I_measured|")
    # Reference: applied amplitude (galvanostatic input)
    ax_iv.plot(np.abs(eta), np.abs(amp), "s--", color="#888", linewidth=1.0, markersize=5,
               alpha=0.7, label="|I_applied|")
    ax_iv.set_xlabel("|η| (V, foil-charge proxy)")
    ax_iv.set_ylabel("|I| (e/fs)")
    ax_iv.set_title("Linear |I| vs |η|")
    ax_iv.legend(loc="best", fontsize=9)
    ax_iv.grid(True, alpha=0.3)

    # Right panel: ln|I| vs |η| with Tafel fit
    keep = np.abs(abs_i) > 0
    eta_keep = np.abs(eta[keep])
    i_keep = np.abs(abs_i[keep])
    ax_iv_log.semilogy(eta_keep, i_keep, "o-", color="#1f77b4", linewidth=1.4, markersize=7,
                      label="|I_measured|")
    ax_iv_log.semilogy(np.abs(eta), np.abs(amp), "s--", color="#888", linewidth=1.0, markersize=5,
                      alpha=0.7, label="|I_applied|")

    # Highlight points used in the linear fit
    if linear_amps:
        used_mask = np.array([float(a) in [float(la) for la in linear_amps] for a in amp])
        if used_mask.any():
            ax_iv_log.semilogy(np.abs(eta[used_mask]), np.abs(abs_i[used_mask]),
                              "o", color="#33a02c", markersize=12, fillstyle="none",
                              markeredgewidth=2, label="used in Tafel fit")

    # Overlay fit line
    if slope_per_v is not None and intercept is not None and np.isfinite(slope_per_v):
        eta_range = np.linspace(eta_keep.min() * 0.8 if eta_keep.size else 0.001,
                                eta_keep.max() * 1.1 if eta_keep.size else 1.0, 50)
        # ln(I) = slope·η + intercept => I = exp(intercept) · exp(slope·η)
        i_fit = np.exp(intercept) * np.exp(slope_per_v * eta_range)
        ax_iv_log.semilogy(eta_range, i_fit, "--", color="#d62728", linewidth=1.5,
                          label=f"Tafel fit: slope = {slope_per_v:.2f} V⁻¹")

    # Annotate ideal Tafel line at configured α
    if cfg_alpha is not None and cfg_scale is not None:
        ideal_slope = cfg_alpha / cfg_scale
        # Anchor at the smallest used η
        if eta_keep.size > 0:
            anchor_eta = eta_keep.min()
            anchor_i = i_keep.min()
            ideal_intercept = np.log(anchor_i) - ideal_slope * anchor_eta
            eta_range = np.linspace(eta_keep.min() * 0.8, eta_keep.max() * 1.1, 50)
            ideal_i = np.exp(ideal_intercept + ideal_slope * eta_range)
            ax_iv_log.semilogy(eta_range, ideal_i, ":", color="#cc6600", linewidth=1.3,
                              label=f"ideal slope α/scale = {ideal_slope:.2f} V⁻¹")

    ax_iv_log.set_xlabel("|η| (V, foil-charge proxy)")
    ax_iv_log.set_ylabel("|I| (e/fs, log scale)")
    ax_iv_log.set_title("ln|I| vs |η| (Tafel)")
    ax_iv_log.legend(loc="best", fontsize=9)
    ax_iv_log.grid(True, which="both", alpha=0.3)

    fig.suptitle("\n".join(title_lines), fontsize=11)
    fig.tight_layout(rect=(0, 0, 1, 0.90))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


def plot_no_spurious_plating(csv_path: Path, out_path: Path, result: dict | None) -> None:
    cols = _read_csv(csv_path)
    t = cols["t_fs"]
    cum = cols["cumulative_transitions"]
    n_li = cols["n_LithiumIon"]
    n_lm = cols["n_LithiumMetal"]
    n_fm = cols["n_FoilMetal"]
    n_other = cols["n_other"]

    title_lines = ["no_spurious_plating"]
    if result is not None:
        title_lines[0] += f" — {'PASS' if result.get('pass') else 'FAIL'}"
        title_lines.append(
            f"total transitions = {int(result.get('value', 0))} events"
        )

    fig, (ax_top, ax_bot) = plt.subplots(2, 1, figsize=(10, 7), sharex=True)
    ax_top.plot(t, n_li, "o-", color="#ff7f0e", label="LithiumIon", markersize=3)
    ax_top.plot(t, n_lm, "s-", color="#9999cc", label="LithiumMetal", markersize=3)
    ax_top.plot(t, n_fm, "^-", color="#888", label="FoilMetal", markersize=3)
    ax_top.plot(t, n_other, "x-", color="#22aaff", label="other", markersize=3, alpha=0.6)
    ax_top.set_ylabel("count")
    ax_top.legend(loc="best", fontsize=9)
    ax_top.set_title("Per-species body counts vs time")

    ax_bot.plot(t, cum, "o-", color="#d62728", linewidth=1.4)
    ax_bot.set_xlabel("t  (fs, post-equilibration)")
    ax_bot.set_ylabel("cumulative transitions")
    ax_bot.set_title("Cumulative species transitions (tolerance = 0)")
    if cum.max() == 0:
        ax_bot.set_ylim(-0.5, 1.0)
        ax_bot.text(0.5, 0.55, "ZERO transitions across measurement window",
                    transform=ax_bot.transAxes, ha="center", va="center",
                    fontsize=12, color="#a00", fontweight="bold",
                    bbox=dict(facecolor="#fff8f0", edgecolor="#a00", boxstyle="round,pad=0.6"))

    fig.suptitle("\n".join(title_lines), fontsize=12)
    fig.tight_layout(rect=(0, 0, 1, 0.93))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)


PLOTTERS = {
    "charge_balance": plot_charge_balance,
    "zero_emf_symmetric": plot_zero_emf_symmetric,
    "driven_symmetric": plot_driven_symmetric,
    "nve_energy_drift": plot_nve_energy_drift,
    "quadtree_force_error": plot_quadtree_force_error,
    "mb_velocity_distribution": plot_mb_velocity_distribution,
    "no_spurious_plating": plot_no_spurious_plating,
    "tafel_slope": plot_tafel_slope,
    "li_ec_coordination": plot_li_ec_coordination,
    "nernst_einstein": plot_nernst_einstein,
    "kinetic_inductance_scaling": plot_kinetic_inductance_scaling,
    "dipole_spike": plot_dipole_spike,
}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--test", help="Test name (uses default paths if --csv/--out not given)")
    ap.add_argument("--csv", type=Path, help="Override CSV input path")
    ap.add_argument("--result", type=Path, help="Override result.json path")
    ap.add_argument("--out", type=Path, help="Override PNG output path")
    args = ap.parse_args()

    if args.test is None and (args.csv is None or args.out is None):
        ap.error("--test is required, or --csv plus --out must be provided")

    test_name = args.test
    if test_name is None:
        ap.error("--test is required to pick a plotter; pass --test charge_balance")

    plotter = PLOTTERS.get(test_name)
    if plotter is None:
        ap.error(
            f"no plotter registered for --test {test_name}. "
            f"Available: {sorted(PLOTTERS)}"
        )

    test_dir = RESULTS_ROOT / test_name
    # Tests use per-test CSV filenames matching the binary's --csv default.
    csv_filenames = {
        "charge_balance": "timeseries.csv",
        "zero_emf_symmetric": "timeseries.csv",
        "driven_symmetric": "timeseries.csv",
        "nve_energy_drift": "timeseries.csv",
        "quadtree_force_error": "per_body.csv",
        "mb_velocity_distribution": "histogram.csv",
        "no_spurious_plating": "timeseries.csv",
        "tafel_slope": "sweep_summary.csv",
        "li_ec_coordination": "rdf.csv",
        "nernst_einstein": "msd.csv",
        "kinetic_inductance_scaling": "sweep_summary.csv",
        "dipole_spike": "rdf.csv",
    }
    default_csv = csv_filenames.get(test_name, "timeseries.csv")
    csv_path = args.csv or (test_dir / default_csv)
    result_path = args.result or (test_dir / "result.json")
    out_path = args.out or (test_dir / f"{test_name}.png")

    if not csv_path.exists():
        ap.error(
            f"CSV not found: {csv_path}\n"
            f"Run the binary first:\n"
            f"  cargo run --release --bin physics_invariants -- --test {test_name}"
        )

    result = _load_result(result_path)
    plotter(csv_path, out_path, result)
    print(f"Wrote {out_path.relative_to(REPO_ROOT) if out_path.is_absolute() else out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
