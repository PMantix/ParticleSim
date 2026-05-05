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
    csv_path = args.csv or (test_dir / "timeseries.csv")
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
