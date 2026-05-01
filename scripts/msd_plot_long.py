#!/usr/bin/env python3
"""Plot MSD vs time from the longer (200 ps target) measure_diffusivity run.

Shows:
  (1) MSD with a linear fit restricted to the clean diffusive window (5-50 ps),
  (2) instantaneous local slope d<r^2>/dt, which exposes the caging plateau,
  (3) T_liquid trace alongside, so the link between T dips and slope dips is visible.
"""

from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt

# (time_fs, <r^2> A^2, T_liquid K)  — 0 fs has no T sample reported beyond initial
data = np.array([
    [     0,     0.00, 295.14],
    [  2000,    27.03, 295.54],
    [  4000,   100.97, 295.41],
    [  6000,   201.74, 294.64],
    [  8000,   321.26, 293.69],
    [ 10000,   446.03, 293.80],
    [ 12000,   603.35, 292.00],
    [ 14000,   776.80, 291.67],
    [ 16000,   972.53, 291.46],
    [ 18000,  1212.24, 292.24],
    [ 20000,  1493.94, 293.48],
    [ 22000,  1820.79, 293.89],
    [ 24000,  2161.49, 294.50],
    [ 26000,  2524.87, 293.76],
    [ 28000,  2925.20, 294.48],
    [ 30000,  3360.53, 294.20],
    [ 32000,  3775.11, 294.93],
    [ 34000,  4222.68, 295.96],
    [ 36000,  4671.34, 296.44],
    [ 38000,  5101.66, 296.90],
    [ 40000,  5538.86, 295.27],
    [ 42000,  6014.30, 294.88],
    [ 44000,  6494.77, 294.73],
    [ 46000,  7074.72, 295.03],
    [ 48000,  7686.48, 295.83],
    [ 50000,  8285.29, 295.52],
    [ 52000,  8868.39, 295.58],
    [ 54000,  9462.49, 294.59],
    [ 56000, 10053.79, 294.25],
    [ 58000, 10688.02, 295.08],
    [ 60000, 11337.74, 295.72],
    [ 62000, 11959.48, 296.39],
    [ 64000, 12610.02, 293.51],
    [ 66000, 13196.67, 292.40],
    [ 68000, 13765.06, 290.64],
    [ 70000, 14315.11, 288.17],
    [ 72000, 14784.61, 287.24],
    [ 74000, 15228.60, 287.65],
    [ 76000, 15744.83, 286.39],
    [ 78000, 16380.63, 286.17],
    [ 80000, 17142.07, 284.36],
    [ 82000, 17947.22, 284.56],
    [ 84000, 18797.68, 284.03],
    [ 86000, 19596.51, 286.15],
    [ 88000, 20387.61, 289.63],
    [ 90000, 21130.94, 290.55],
    [ 92000, 21984.46, 291.52],
    [ 94000, 22867.76, 293.42],
    [ 96000, 23785.30, 293.19],
    [ 98000, 24637.08, 294.33],
    [100000, 25497.84, 295.60],
    [102000, 26265.91, 295.68],
    [104000, 26944.27, 295.22],
    [106000, 27510.39, 295.78],
    [108000, 27921.74, 296.79],
    [110000, 28220.54, 296.49],
    [112000, 28483.84, 296.45],
    [114000, 28781.23, 297.09],
    [116000, 29188.41, 297.39],
    [118000, 29649.54, 297.03],
    [120000, 30059.87, 297.27],
    [122000, 30471.77, 296.02],
    [124000, 30890.19, 294.40],
    [126000, 31319.23, 293.98],
    [128000, 31790.49, 295.82],
    [130000, 32328.02, 294.40],
    [132000, 32826.68, 293.83],
    [134000, 33319.21, 294.21],
    [136000, 33868.43, 292.72],
    [138000, 34291.24, 290.90],
    [140000, 34867.36, 289.79],
    [142000, 35448.18, 289.36],
    [144000, 35980.21, 288.33],
    [146000, 36411.88, 290.61],
    [148000, 36810.06, 287.95],
    [150000, 37161.93, 288.17],
    [152000, 37544.33, 288.34],
    [154000, 37888.19, 288.13],
    [156000, 38277.96, 289.09],
    [158000, 38799.18, 289.97],
    [160000, 39329.71, 287.15],
    [162000, 39945.41, 285.24],
    [164000, 40572.52, 285.50],
    [166000, 41361.75, 284.37],
    [168000, 42101.42, 285.54],
    [170000, 42732.05, 283.63],
    [172000, 43321.42, 283.48],
    [174000, 43796.64, 286.22],
    [176000, 44272.65, 288.13],
    [178000, 44868.46, 289.77],
    [180000, 45511.74, 290.59],
    [182000, 46140.71, 291.70],
    [184000, 46782.02, 293.15],
    [186000, 47521.23, 292.86],
    [188000, 48337.11, 293.80],
    [190000, 49164.80, 295.12],
    [192000, 50038.34, 296.87],
    [194000, 51058.23, 296.93],
    [196000, 51970.72, 296.85],
    [198000, 52877.94, 296.57],
    [200000, 53828.94, 296.54],
])

t  = data[:, 0]
r2 = data[:, 1]
T  = data[:, 2]

# Linear fit on whole range (for comparison)
slope_all, intercept_all = np.polyfit(t, r2, 1)
ss_res = np.sum((r2 - (slope_all * t + intercept_all)) ** 2)
ss_tot = np.sum((r2 - r2.mean()) ** 2)
r2_all = 1 - ss_res / ss_tot

# Linear fit on the "clean" diffusive window: 5 ps - 50 ps
mask = (t >= 5000) & (t <= 50000)
slope_win, intercept_win = np.polyfit(t[mask], r2[mask], 1)
res_w = np.sum((r2[mask] - (slope_win * t[mask] + intercept_win)) ** 2)
tot_w = np.sum((r2[mask] - r2[mask].mean()) ** 2)
r2_win = 1 - res_w / tot_w

D_all = slope_all / 4.0  # 2D Einstein relation
D_win = slope_win / 4.0
D_win_SI = D_win * 1e-5

# Local slope via centered finite differences (units: A^2 / fs)
local_slope = np.gradient(r2, t)

fig = plt.figure(figsize=(10, 9))
gs = fig.add_gridspec(3, 1, height_ratios=[3, 1.2, 1.0], hspace=0.08)
ax_msd = fig.add_subplot(gs[0])
ax_slp = fig.add_subplot(gs[1], sharex=ax_msd)
ax_T   = fig.add_subplot(gs[2], sharex=ax_msd)

# --- MSD panel ---
ax_msd.plot(t, r2, "o", color="#1f77b4", markersize=4,
            label=r"simulated $\langle r^2 \rangle$")

t_fit = np.linspace(0, t.max(), 200)
ax_msd.plot(t_fit, slope_all * t_fit + intercept_all, ":", color="#7f7f7f",
            linewidth=1.5,
            label=(f"fit (full range): D = {D_all:.3e} Å²/fs, "
                   f"R² = {r2_all:.3f}"))

t_win = np.linspace(t[mask].min(), t[mask].max(), 50)
ax_msd.plot(t_win, slope_win * t_win + intercept_win, "-", color="#d62728",
            linewidth=2.2,
            label=(f"fit (5–50 ps window): D = {D_win:.3e} Å²/fs "
                   f"= {D_win_SI:.3e} m²/s\n R² = {r2_win:.4f}"))

# Shade caging plateau
ax_msd.axvspan(105000, 115000, color="#ffeeba", alpha=0.6, label="caging plateau")

ax_msd.set_ylabel(r"$\langle r^2 \rangle$  [Å²]")
ax_msd.set_title("Li⁺ MSD in bulk electrolyte — full 200 ps run")
ax_msd.grid(True, alpha=0.3)
ax_msd.legend(loc="upper left", fontsize=9, framealpha=0.92)

# --- Local slope panel ---
ax_slp.plot(t, local_slope, "-", color="#2ca02c", linewidth=1.4)
ax_slp.axhline(slope_win, color="#d62728", linestyle="--", linewidth=1.0,
               label=f"fit slope = {slope_win:.3e}")
ax_slp.axvspan(105000, 115000, color="#ffeeba", alpha=0.6)
ax_slp.set_ylabel(r"$d\langle r^2 \rangle / dt$" + "\n[Å²/fs]")
ax_slp.grid(True, alpha=0.3)
ax_slp.legend(loc="lower left", fontsize=8)

# --- Temperature panel ---
ax_T.plot(t, T, "-", color="#9467bd", linewidth=1.3)
ax_T.axhline(295.0, color="black", linestyle=":", linewidth=0.8,
             label="295 K target")
ax_T.axvspan(105000, 115000, color="#ffeeba", alpha=0.6)
ax_T.set_xlabel("time  [fs]")
ax_T.set_ylabel(r"$T_{\rm liquid}$  [K]")
ax_T.grid(True, alpha=0.3)
ax_T.legend(loc="lower left", fontsize=8)

# Hide x tick labels on the upper two panels
plt.setp(ax_msd.get_xticklabels(), visible=False)
plt.setp(ax_slp.get_xticklabels(), visible=False)

out_dir = Path(__file__).resolve().parent.parent / "images" / "bulk_electrolyte_diffusivity"
out_dir.mkdir(parents=True, exist_ok=True)
out_path = str(out_dir / "msd_bulk_electrolyte_full.png")
fig.savefig(out_path, dpi=150, bbox_inches="tight")
print(f"Wrote {out_path}")
print(f"D (full)   = {D_all:.4e} A^2/fs   ({D_all * 1e-5:.4e} m^2/s)   R^2={r2_all:.4f}")
print(f"D (5-50ps) = {D_win:.4e} A^2/fs   ({D_win_SI:.4e} m^2/s)   R^2={r2_win:.4f}")
