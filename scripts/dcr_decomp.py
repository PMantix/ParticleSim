"""View DCR pulse with V_cell decomposed into Galvani drops + bulk ionic drop."""
import csv
import sys
from pathlib import Path
import matplotlib.pyplot as plt
import numpy as np

run_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "doe_results/dcr_pulse_sweep/v5_components_smoke")

rows = []
with open(run_dir / "dense_series.csv") as f:
    for r in csv.DictReader(f):
        rows.append({
            "t": float(r["t_fs"]),
            "phase": r["phase"],
            "i": float(r["i_applied"]),
            "v_cell": float(r["v_cell"]),
            "v_ma": float(r["v_metal_a"]),
            "v_mb": float(r["v_metal_b"]),
            "v_ba": float(r["v_bulk_a"]),
            "v_bb": float(r["v_bulk_b"]),
        })

t = np.array([r["t"] for r in rows])
v_cell = np.array([r["v_cell"] for r in rows]) * 1000  # mV
v_ma = np.array([r["v_ma"] for r in rows]) * 1000
v_mb = np.array([r["v_mb"] for r in rows]) * 1000
v_ba = np.array([r["v_ba"] for r in rows]) * 1000
v_bb = np.array([r["v_bb"] for r in rows]) * 1000

# Decomposition: V_cell = (V_ma - V_ba) + (V_ba - V_bb) + (V_bb - V_mb)
galvani_a = v_ma - v_ba          # interface drop A (foil A → bulk A)
bulk_drop = v_ba - v_bb          # ionic bulk drop (bulk A → bulk B)
galvani_b = v_bb - v_mb          # interface drop B (bulk B → foil B)

# Sanity check the decomposition equals V_cell
recon = galvani_a + bulk_drop + galvani_b
err = np.abs(v_cell - recon).max()
print(f"decomposition error (max): {err:.3e} mV")

# Phase boundaries
on_start = next((r["t"] for r in rows if r["phase"] == "on"), None)
rest_start = next((r["t"] for r in rows if r["phase"] == "rest"), None)

fig, axes = plt.subplots(3, 1, figsize=(13, 10), sharex=True)

# Top: V_cell + 4 raw potentials
ax = axes[0]
ax.plot(t, v_cell, "-", color="black", lw=1.2, label="V_cell (= V_ma − V_mb)")
ax.plot(t, v_ma, "-", color="tab:blue", lw=0.8, alpha=0.7, label="V_metal_a")
ax.plot(t, v_mb, "-", color="tab:orange", lw=0.8, alpha=0.7, label="V_metal_b")
ax.plot(t, v_ba, "--", color="tab:cyan", lw=0.8, alpha=0.7, label="V_bulk_a")
ax.plot(t, v_bb, "--", color="tab:red", lw=0.8, alpha=0.7, label="V_bulk_b")
if on_start: ax.axvline(on_start, color="tab:green", lw=0.8, ls="--", alpha=0.4)
if rest_start: ax.axvline(rest_start, color="tab:red", lw=0.8, ls="--", alpha=0.4)
ax.set_title("Raw potentials at all 4 probe positions")
ax.set_ylabel("V (mV)")
ax.grid(True, alpha=0.3)
ax.legend(loc="upper right", fontsize=8, ncol=2)

# Middle: decomposition
ax = axes[1]
ax.plot(t, galvani_a, "-", color="tab:blue", lw=1.0, label="Galvani drop A (V_ma − V_ba)")
ax.plot(t, bulk_drop, "-", color="tab:purple", lw=1.5, label="Ionic bulk drop (V_ba − V_bb)")
ax.plot(t, galvani_b, "-", color="tab:orange", lw=1.0, label="Galvani drop B (V_bb − V_mb)")
ax.plot(t, v_cell, "k-", lw=1.0, alpha=0.5, label="V_cell (sum)")
if on_start: ax.axvline(on_start, color="tab:green", lw=0.8, ls="--", alpha=0.4)
if rest_start: ax.axvline(rest_start, color="tab:red", lw=0.8, ls="--", alpha=0.4)
ax.set_title("V_cell decomposed: V_cell = Galvani_A + Bulk_drop + Galvani_B")
ax.set_ylabel("V component (mV)")
ax.grid(True, alpha=0.3)
ax.legend(loc="upper right", fontsize=8)

# Bottom: i(t)
ax = axes[2]
i = np.array([r["i"] for r in rows])
ax.plot(t, i, "-", color="tab:purple", lw=1.0)
if on_start: ax.axvline(on_start, color="tab:green", lw=0.8, ls="--", alpha=0.4, label="pulse start")
if rest_start: ax.axvline(rest_start, color="tab:red", lw=0.8, ls="--", alpha=0.4, label="pulse end")
ax.set_title("Applied I(t)")
ax.set_xlabel("t (fs)")
ax.set_ylabel("I (e/fs)")
ax.grid(True, alpha=0.3)
ax.legend(fontsize=8)

fig.suptitle(f"DCR pulse decomposition — {run_dir.name}", fontsize=12)
fig.tight_layout(rect=(0, 0, 1, 0.97))
out = run_dir.parent / f"{run_dir.name}_decomp.png"
fig.savefig(out, dpi=140)
plt.close(fig)
print(f"wrote {out}")

# Print phase summaries
print("\n=== Phase-mean potentials (mV) ===")
for phase in ["pre_hold", "on", "rest"]:
    mask = np.array([r["phase"] == phase for r in rows])
    if mask.sum() == 0: continue
    print(f"{phase:10} (n={mask.sum()})  V_cell={v_cell[mask].mean():>7.3f}  "
          f"V_ma={v_ma[mask].mean():>7.3f}  V_mb={v_mb[mask].mean():>7.3f}  "
          f"V_ba={v_ba[mask].mean():>7.3f}  V_bb={v_bb[mask].mean():>7.3f}  "
          f"galv_A={galvani_a[mask].mean():>7.3f}  bulk={bulk_drop[mask].mean():>7.3f}  "
          f"galv_B={galvani_b[mask].mean():>7.3f}")
