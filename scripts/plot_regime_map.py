#!/usr/bin/env python3
"""Generate the EIS similitude regime map.

Two horizontal log-frequency bars (experimental cell, simulator) annotated
with each system's diffusion crossover (Pe = 1), EIS measurement band, and
arc-apex frequencies where known. See docs/eis_similitude.md for context.

All numerical inputs are at the top so they can be edited as the simulator's
measurements firm up. Re-run after every Phase 0/1/2 update.
"""

from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt

# ----------------------------------------------------------------------
# INPUTS  (edit these as values firm up)
# ----------------------------------------------------------------------

# --- Real cell (Honda-RI switching cell) ---
real = {
    "label": "Real cell  (20 µm, 1 M LiPF6 / EC:EMC:DMC + 3% VC, 22°C)",
    # Diffusion crossover Pe = 1
    "f_W": 0.12,            # Hz, = 1/(2π · L²/D)
    # EIS sweep band
    "eis_min": 0.1,
    "eis_max": 1.0e6,
    # Nyquist arc apex frequencies
    "arcs": [
        ("R_ct·C_dl",  110.0),     # mid arc
        ("R_SEI·C_SEI", 1000.0),   # HF arc
    ],
    # Color / row position
    "color": "#1f77b4",
    "y": 0.65,
}

# --- Simulator (current Phase 0 estimate, Phase 1 will refine) ---
sim = {
    "label": "Simulator  (planned EIS scenario: L = 250 Å, D ≈ 6.3 × 10⁻² Å²/fs)",
    # 1 fs = 1e-15 s, so frequency in Hz = 1/(period in fs) × 1e15
    "f_W": 1.6e8,           # Hz, = 1.6e-7 /fs
    "eis_min": 1.0e6,       # Hz; recommended Phase 5 measurement band lower bound
    "eis_max": 1.0e10,      # Hz
    "accessible_min": 1.0e5,    # Hz, sim total-run-time constraint
    "accessible_max": 1.0e15,   # Hz, sim time-step constraint
    "arcs": [],             # empty until Phase 1 produces a Nyquist
    "color": "#d62728",
    "y": 0.25,
}

# ----------------------------------------------------------------------
# PLOT
# ----------------------------------------------------------------------

fig, ax = plt.subplots(figsize=(11, 4.2))

f_min_global = 1e-3
f_max_global = 1e16

ax.set_xscale("log")
ax.set_xlim(f_min_global, f_max_global)
ax.set_ylim(0, 1)
ax.set_yticks([])
ax.set_xlabel("frequency  [Hz]   (1 fs ≡ 10¹⁵ Hz)")
ax.set_title("EIS similitude regime map  —  experimental cell vs. ParticleSim")

# Light shading: warburg (Pe < 1) and bulk-resistive (Pe > 1) for each system
def draw_system(ax, sys):
    y = sys["y"]
    h = 0.18

    # Accessible band shading (lighter)
    if "accessible_min" in sys:
        ax.axhspan(y - h * 0.6, y + h * 0.6,
                   xmin=np.log10(sys["accessible_min"] / f_min_global) / np.log10(f_max_global / f_min_global),
                   xmax=np.log10(sys["accessible_max"] / f_min_global) / np.log10(f_max_global / f_min_global),
                   color=sys["color"], alpha=0.06, lw=0)

    # EIS band: solid bar
    ax.hlines(y, sys["eis_min"], sys["eis_max"],
              colors=sys["color"], linewidth=10, alpha=0.55)
    ax.text(np.sqrt(sys["eis_min"] * sys["eis_max"]), y + h * 0.7,
            "EIS sweep band", ha="center", va="bottom",
            fontsize=9, color=sys["color"])
    # f_min, f_max labels
    ax.text(sys["eis_min"], y - h * 0.7, f"  {fmt_hz(sys['eis_min'])}",
            ha="left", va="top", fontsize=8, color=sys["color"])
    ax.text(sys["eis_max"], y - h * 0.7, f"{fmt_hz(sys['eis_max'])}  ",
            ha="right", va="top", fontsize=8, color=sys["color"])

    # Pe = 1 line
    ax.axvline(sys["f_W"], ymin=y - h * 0.9 + 0.001, ymax=y + h * 0.9 + 0.001,
               color=sys["color"], linestyle="--", linewidth=1.6)
    ax.text(sys["f_W"], y + h * 1.1, f"Pe = 1\n{fmt_hz(sys['f_W'])}",
            ha="center", va="bottom", fontsize=8, color=sys["color"],
            bbox=dict(boxstyle="round,pad=0.18", fc="white",
                      ec=sys["color"], lw=0.5, alpha=0.9))

    # Arc apex markers — stack labels vertically so close-together arcs don't overlap
    for k, (label, f) in enumerate(sys["arcs"]):
        ax.plot([f], [y], marker="o", color=sys["color"], markersize=8,
                markeredgecolor="white", markeredgewidth=1.0, zorder=5)
        # Alternate label row depending on index parity
        y_off = h * (1.1 + 0.9 * k)
        ax.text(f, y - y_off, f"{label}\n{fmt_hz(f)}",
                ha="center", va="top", fontsize=7.5, color=sys["color"])
        # Tick mark down to the label row so it's clear which arc the label belongs to
        ax.plot([f, f], [y - h * 0.6, y - y_off + h * 0.05],
                color=sys["color"], linewidth=0.6, alpha=0.5)

    # System label, far left
    ax.text(f_min_global * 1.5, y, sys["label"],
            ha="left", va="center", fontsize=9, color=sys["color"], style="italic")


def fmt_hz(f):
    """Format frequency as e.g. '110 Hz', '1 kHz', '160 MHz'."""
    if f >= 1e9:
        return f"{f/1e9:g} GHz"
    if f >= 1e6:
        return f"{f/1e6:g} MHz"
    if f >= 1e3:
        return f"{f/1e3:g} kHz"
    if f >= 1:
        return f"{f:g} Hz"
    return f"{f*1e3:g} mHz"


draw_system(ax, real)
draw_system(ax, sim)

# Cross-system arrow at Pe = 1
ax.annotate("",
            xy=(sim["f_W"], sim["y"] + 0.02),
            xytext=(real["f_W"], real["y"] - 0.02),
            arrowprops=dict(arrowstyle="->", color="#7f7f7f",
                            connectionstyle="arc3,rad=0.0", lw=1.0,
                            alpha=0.7))
mid_log = 10 ** ((np.log10(real["f_W"]) + np.log10(sim["f_W"])) / 2)
ax.text(mid_log, (real["y"] + sim["y"]) / 2,
        f"  {(sim['f_W'] / real['f_W']):.2g}× ratio\n  in absolute Hz",
        fontsize=8, color="#7f7f7f", ha="left", va="center")

ax.grid(True, axis="x", which="both", alpha=0.25)

out_dir = Path(__file__).resolve().parent.parent / "images" / "eis_similitude"
out_dir.mkdir(parents=True, exist_ok=True)
out_path = out_dir / "regime_map.png"
fig.tight_layout()
fig.savefig(out_path, dpi=150, bbox_inches="tight")
print(f"Wrote {out_path}")
print(f"Pe=1 ratio (sim / real): {sim['f_W'] / real['f_W']:.3g}")
