#!/usr/bin/env python3
"""Plot MSD vs time from measure_diffusivity output for bulk_electrolyte scenario."""

from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt

# (time_fs, <r^2> in A^2)
data = np.array([
    [    0,     0.00],
    [ 1000,     7.10],
    [ 2000,    27.03],
    [ 3000,    59.92],
    [ 4000,   100.97],
    [ 5000,   147.21],
    [ 6000,   201.74],
    [ 7000,   261.98],
    [ 8000,   321.26],
    [ 9000,   381.00],
    [10000,   446.03],
    [11000,   518.99],
    [12000,   603.35],
    [13000,   686.45],
    [14000,   776.80],
    [15000,   870.22],
    [16000,   972.53],
    [17000,  1088.56],
    [18000,  1212.24],
    [19000,  1345.56],
    [20000,  1493.94],
    [21000,  1653.70],
    [22000,  1820.79],
    [23000,  1986.73],
    [24000,  2161.49],
    [25000,  2341.60],
    [26000,  2524.87],
    [27000,  2719.58],
    [28000,  2925.20],
    [29000,  3145.55],
    [30000,  3360.53],
    [31000,  3564.57],
    [32000,  3775.11],
    [33000,  3992.40],
    [34000,  4222.68],
    [35000,  4455.34],
    [36000,  4671.34],
    [37000,  4881.04],
    [38000,  5101.66],
    [39000,  5315.79],
    [40000,  5538.86],
    [41000,  5774.30],
    [42000,  6014.30],
    [43000,  6252.07],
    [44000,  6494.77],
    [45000,  6773.77],
    [46000,  7074.72],
    [47000,  7373.25],
    [48000,  7686.48],
    [49000,  7987.21],
    [50000,  8285.29],
])

t  = data[:, 0]
r2 = data[:, 1]

# Reported fit from the run
slope     = 1.6836e-1     # A^2 / fs
intercept = -1.2404e3     # A^2
r_squared = 0.9528
D         = slope / 4.0   # A^2 / fs (2D Einstein relation)
D_SI      = D * 1e-5      # convert A^2/fs -> m^2/s

fit_line = slope * t + intercept

fig, (ax_main, ax_resid) = plt.subplots(
    2, 1, figsize=(9, 7),
    gridspec_kw={"height_ratios": [3, 1]}, sharex=True
)

# Main MSD plot
ax_main.plot(t, r2, "o", color="#1f77b4", markersize=5,
             label=r"simulated $\langle r^2 \rangle$")
ax_main.plot(t, fit_line, "--", color="#d62728", linewidth=1.8,
             label=(f"linear fit: slope = {slope:.4e} Å²/fs\n"
                    f"R² = {r_squared:.4f}"))

ax_main.set_ylabel(r"$\langle r^2 \rangle$  [Å²]")
ax_main.set_title(
    f"Li⁺ MSD in bulk electrolyte  —  "
    f"D = {D:.3e} Å²/fs  =  {D_SI:.3e} m²/s\n"
    f"(2D, 79 tracked ions, T ≈ 295 K)"
)
ax_main.grid(True, alpha=0.3)
ax_main.legend(loc="upper left", framealpha=0.9)

# Residuals: how much the fit misses each data point
residuals = r2 - fit_line
ax_resid.axhline(0, color="black", linewidth=0.8)
ax_resid.plot(t, residuals, "o-", color="#2ca02c", markersize=4)
ax_resid.set_xlabel("time  [fs]")
ax_resid.set_ylabel("residual  [Å²]")
ax_resid.grid(True, alpha=0.3)

fig.tight_layout()

out_dir = Path(__file__).resolve().parent.parent / "images" / "bulk_electrolyte_diffusivity"
out_dir.mkdir(parents=True, exist_ok=True)
out_path = str(out_dir / "msd_bulk_electrolyte.png")
fig.savefig(out_path, dpi=150)
print(f"Wrote {out_path}")
