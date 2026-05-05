"""Grid-resolution DOE for the interface_arc_length metric.

Sweeps Δy ∈ {1, 2, 5, 10, 20} Å on the morphology_demo scenarios, captures
the computed metric per scenario per bin width, and renders:

  - One curve per scenario in `images/morphology_validation/arc_length_grid_doe.png`,
    showing computed arc length vs. bin width on a log-x axis.
  - A `arc_length_grid_doe.csv` row dump.

The point: characterize the bin-width dependence so we can pick a default
where the metric is approximately resolution-stable.

Usage:
    python scripts/morphology_grid_doe.py
"""
from __future__ import annotations

import csv
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

import matplotlib.pyplot as plt

REPO_ROOT = Path(__file__).resolve().parent.parent
BINARY = REPO_ROOT / "target" / "release" / "morphology_demo"
OUT_DIR = REPO_ROOT / "images" / "morphology_validation"

# DOE bin-widths, in Å.
BIN_WIDTHS = [1.0, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0]


def run_one(y_bin: float, tmp_dir: Path) -> dict:
    out_path = tmp_dir / f"run_y{y_bin:g}.json"
    cmd = [
        str(BINARY),
        "--metric", "interface_arc_length",
        "--y-bin", str(y_bin),
        "--out", str(out_path),
    ]
    subprocess.run(cmd, check=True, cwd=REPO_ROOT, capture_output=True)
    with out_path.open() as f:
        return json.load(f)


def main():
    if not BINARY.exists():
        raise SystemExit(
            f"binary not found at {BINARY}. Run "
            f"`cargo build --release --bin morphology_demo` first."
        )

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    # Run DOE.
    rows = []  # one row per (scenario, y_bin)
    scenario_names = None
    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        for y_bin in BIN_WIDTHS:
            data = run_one(y_bin, tmp_dir)
            if scenario_names is None:
                scenario_names = [s["name"] for s in data["scenarios"]]
            for sc in data["scenarios"]:
                rows.append({
                    "scenario": sc["name"],
                    "y_bin": y_bin,
                    "computed": sc["computed"],
                    "expected": sc.get("expected"),
                })

    # Write CSV.
    csv_path = OUT_DIR / "arc_length_grid_doe.csv"
    with csv_path.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["scenario", "y_bin", "computed", "expected"])
        w.writeheader()
        w.writerows(rows)
    print(f"wrote {csv_path}")

    # Render curves.
    fig, ax = plt.subplots(figsize=(9, 5.5))
    by_scenario: dict[str, list[tuple[float, float]]] = {}
    for r in rows:
        by_scenario.setdefault(r["scenario"], []).append((r["y_bin"], r["computed"]))

    cmap = plt.get_cmap("tab10")
    for i, name in enumerate(scenario_names or []):
        pts = sorted(by_scenario.get(name, []))
        if not pts:
            continue
        xs, ys = zip(*pts)
        ax.plot(xs, ys, marker="o", color=cmap(i), label=name, lw=1.5)

    ax.set_xscale("log")
    ax.set_xlabel("y_bin (Å, log scale)")
    ax.set_ylabel("interface_arc_length / lateral_extent")
    ax.axhline(1.0, color="grey", linestyle=":", lw=0.8, label="flat reference")
    ax.set_title("interface_arc_length: grid-resolution DOE")
    ax.grid(True, which="both", alpha=0.3)
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()

    png_path = OUT_DIR / "arc_length_grid_doe.png"
    fig.savefig(png_path, dpi=140)
    plt.close(fig)
    print(f"wrote {png_path}")


if __name__ == "__main__":
    main()
