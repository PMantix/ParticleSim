"""Render synthetic-scenario panels for morphology metric validation.

Reads images/morphology_validation/<metric>.json (produced by the
`morphology_demo` Rust binary) and writes a multi-panel PNG showing each
scenario, with metric-specific overlays.

Usage:
    python scripts/plot_morphology_demo.py --metric accessible_surface_atoms
    python scripts/plot_morphology_demo.py --in path.json --out path.png
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle

REPO_ROOT = Path(__file__).resolve().parent.parent

# Species rendering palette. Designed to match approximate Quarkstrom colors
# but in matplotlib-friendly hex.
SPECIES_COLOR = {
    "LithiumMetal":     "#9999cc",
    "FoilMetal":        "#888888",
    "LithiumIon":       "#ffaa33",
    "ElectrolyteAnion": "#22aaff",
    "EC":               "#66cc66",
    "DMC":              "#33aa33",
    "VC":               "#88cc22",
    "FEC":              "#22cc88",
    "EMC":              "#33cc33",
    "LLZO":             "#aaaaaa",
    "LLZT":             "#999999",
    "S40B":             "#888899",
    "SEI":              "#cc6666",
    "Graphite":         "#222222",
    "HardCarbon":       "#444444",
    "SiliconOxide":     "#aa6633",
    "LTO":              "#6666aa",
    "LFP":              "#cc3333",
    "LMFP":             "#cc5555",
    "NMC":              "#aa3366",
    "NCA":              "#aa6699",
}

ACCESSIBLE_HIGHLIGHT = "#ff3300"
INACCESSIBLE_LI = "#9999cc"


def render_accessible_panel(ax, scenario):
    """One scenario panel for accessible_surface_atoms."""
    particles = scenario["particles"]

    # Collect species categories for the legend.
    seen_species = set()

    # Bounding box auto-fit. Pad to make each panel roughly square so radii
    # render at consistent scale across scenarios with very different aspect
    # ratios (tall foils vs. wide dead-Li chains).
    xs = [p["x"] for p in particles]
    ys = [p["y"] for p in particles]
    if not xs:
        ax.set_title(f"{scenario['name']} (empty)")
        return
    x_min, x_max = min(xs), max(xs)
    y_min, y_max = min(ys), max(ys)
    base_pad = max((x_max - x_min) * 0.08, (y_max - y_min) * 0.08, 5.0)

    # Compute centers and half-spans, then enforce a minimum equal half-span
    # so every panel covers ≥ this many Å on each axis.
    cx = (x_min + x_max) * 0.5
    cy = (y_min + y_max) * 0.5
    half_x = (x_max - x_min) * 0.5 + base_pad
    half_y = (y_max - y_min) * 0.5 + base_pad
    half = max(half_x, half_y)
    ax.set_xlim(cx - half, cx + half)
    ax.set_ylim(cy - half, cy + half)
    ax.set_aspect("equal")

    for p in particles:
        species = p["species"]
        x, y, r = p["x"], p["y"], p["r"]
        accessible = p.get("accessible", False)

        if species == "LithiumMetal":
            color = ACCESSIBLE_HIGHLIGHT if accessible else INACCESSIBLE_LI
            edge = "black" if accessible else "none"
            lw = 0.5 if accessible else 0.0
        else:
            color = SPECIES_COLOR.get(species, "#cccccc")
            edge = "none"
            lw = 0.0

        ax.add_patch(Circle((x, y), r, color=color, ec=edge, lw=lw, alpha=0.85))
        seen_species.add(species)

    judgment = scenario.get("judgment", "?")
    if judgment == "PASS":
        judgment_color = "tab:green"
    elif judgment == "FAIL":
        judgment_color = "tab:red"
    else:
        judgment_color = "tab:blue"  # INFO

    expected = scenario.get("expected")
    expected_str = f"{expected:g}" if isinstance(expected, (int, float)) else "n/a"
    ax.set_title(
        f"{scenario['name']}\n"
        f"expected={expected_str}  computed={scenario['computed']:g}  [{judgment}]",
        color=judgment_color,
        fontsize=10,
    )

    # Species legend (compact, only shown species).
    legend_handles = []
    if "LithiumMetal" in seen_species:
        legend_handles.append(
            plt.Line2D([], [], marker="o", linestyle="",
                       markerfacecolor=ACCESSIBLE_HIGHLIGHT, markeredgecolor="black",
                       markersize=8, label="Li accessible")
        )
        legend_handles.append(
            plt.Line2D([], [], marker="o", linestyle="",
                       markerfacecolor=INACCESSIBLE_LI, markeredgecolor="none",
                       markersize=8, label="Li inaccessible")
        )
    for sp in sorted(seen_species - {"LithiumMetal"}):
        legend_handles.append(
            plt.Line2D([], [], marker="o", linestyle="",
                       markerfacecolor=SPECIES_COLOR.get(sp, "#cccccc"),
                       markeredgecolor="none", markersize=8, label=sp)
        )
    ax.legend(handles=legend_handles, loc="upper right", fontsize=7, frameon=True)
    ax.set_xlabel("x (Å)")
    ax.set_ylabel("y (Å)")


DEAD_COLOR = "#ee2222"
ALIVE_COLOR = "#2aaa44"


def render_dead_li_panel(ax, scenario):
    """One scenario panel for dead_li_fraction: particle scatter colored by
    component label — green = foil-connected, red = dead."""
    particles = scenario["particles"]
    if not particles:
        ax.set_title(f"{scenario['name']} (empty)")
        ax.set_aspect("equal")
        return

    xs = [p["x"] for p in particles]
    ys = [p["y"] for p in particles]
    x_min, x_max = min(xs), max(xs)
    y_min, y_max = min(ys), max(ys)
    base_pad = max((x_max - x_min) * 0.1, (y_max - y_min) * 0.1, 5.0)
    cx, cy = (x_min + x_max) * 0.5, (y_min + y_max) * 0.5
    half = max((x_max - x_min) * 0.5 + base_pad, (y_max - y_min) * 0.5 + base_pad)
    ax.set_xlim(cx - half, cx + half)
    ax.set_ylim(cy - half, cy + half)
    ax.set_aspect("equal")

    seen_categories = set()
    for p in particles:
        species = p["species"]
        status = p.get("dead_status", "n/a")
        if species == "LithiumMetal":
            if status == "dead":
                color = DEAD_COLOR
                edge = "black"
                lw = 0.5
                seen_categories.add("Li dead")
            else:
                color = ALIVE_COLOR
                edge = "none"
                lw = 0.0
                seen_categories.add("Li alive")
        else:
            color = SPECIES_COLOR.get(species, "#cccccc")
            edge = "none"
            lw = 0.0
            seen_categories.add(species)
        ax.add_patch(Circle((p["x"], p["y"]), p["r"], color=color, ec=edge, lw=lw, alpha=0.85))

    judgment = scenario.get("judgment", "?")
    if judgment == "PASS":
        judgment_color = "tab:green"
    elif judgment == "FAIL":
        judgment_color = "tab:red"
    else:
        judgment_color = "tab:blue"

    expected = scenario.get("expected")
    expected_str = f"{expected:.3f}" if isinstance(expected, (int, float)) else "n/a"
    ax.set_title(
        f"{scenario['name']}\n"
        f"expected={expected_str}  computed={scenario['computed']:.3f}  [{judgment}]",
        color=judgment_color,
        fontsize=10,
    )

    legend_handles = []
    if "Li alive" in seen_categories:
        legend_handles.append(plt.Line2D([], [], marker="o", linestyle="",
            markerfacecolor=ALIVE_COLOR, markeredgecolor="none",
            markersize=8, label="Li alive (foil-connected)"))
    if "Li dead" in seen_categories:
        legend_handles.append(plt.Line2D([], [], marker="o", linestyle="",
            markerfacecolor=DEAD_COLOR, markeredgecolor="black",
            markersize=8, label="Li dead"))
    for sp in sorted(seen_categories - {"Li alive", "Li dead"}):
        legend_handles.append(plt.Line2D([], [], marker="o", linestyle="",
            markerfacecolor=SPECIES_COLOR.get(sp, "#cccccc"),
            markeredgecolor="none", markersize=8, label=sp))
    ax.legend(handles=legend_handles, loc="upper right", fontsize=7, frameon=True)
    ax.set_xlabel("x (Å)")
    ax.set_ylabel("y (Å)")


def render_arc_length_panel(ax, scenario):
    """One scenario panel for interface_arc_length: particle scatter +
    per-side frontier polyline overlay."""
    particles = scenario["particles"]
    if not particles:
        ax.set_title(f"{scenario['name']} (empty)\nratio={scenario['computed']:.3f}")
        ax.set_aspect("equal")
        return

    xs = [p["x"] for p in particles]
    ys = [p["y"] for p in particles]
    x_min, x_max = min(xs), max(xs)
    y_min, y_max = min(ys), max(ys)
    base_pad = max((x_max - x_min) * 0.1, (y_max - y_min) * 0.1, 5.0)
    cx, cy = (x_min + x_max) * 0.5, (y_min + y_max) * 0.5
    half = max((x_max - x_min) * 0.5 + base_pad, (y_max - y_min) * 0.5 + base_pad)
    ax.set_xlim(cx - half, cx + half)
    ax.set_ylim(cy - half, cy + half)
    ax.set_aspect("equal")

    seen_species = set()
    for p in particles:
        species = p["species"]
        color = SPECIES_COLOR.get(species, "#cccccc")
        ax.add_patch(Circle((p["x"], p["y"]), p["r"], color=color, ec="none", alpha=0.7))
        seen_species.add(species)

    # Frontier polylines.
    frontiers = scenario.get("frontiers", {})
    for label, color in [("left", "#cc0000"), ("right", "#0066cc")]:
        pts = frontiers.get(label, [])
        if len(pts) >= 2:
            fx = [p["x"] for p in pts]
            fy = [p["y"] for p in pts]
            ax.plot(fx, fy, color=color, lw=2.0, marker="o", markersize=3,
                    label=f"{label} frontier")

    judgment = scenario.get("judgment", "?")
    if judgment == "PASS":
        judgment_color = "tab:green"
    elif judgment == "FAIL":
        judgment_color = "tab:red"
    else:
        judgment_color = "tab:blue"

    expected = scenario.get("expected")
    expected_str = f"{expected:.3f}" if isinstance(expected, (int, float)) else "n/a"
    ax.set_title(
        f"{scenario['name']}\n"
        f"expected={expected_str}  computed={scenario['computed']:.3f}  [{judgment}]",
        color=judgment_color,
        fontsize=10,
    )

    if frontiers.get("left") or frontiers.get("right"):
        ax.legend(loc="upper right", fontsize=7, frameon=True)
    ax.set_xlabel("x (Å)")
    ax.set_ylabel("y (Å)")


def render_metric(data, out_path: Path):
    metric = data["metric"]
    scenarios = data["scenarios"]
    n = len(scenarios)
    cols = min(3, n)
    rows = (n + cols - 1) // cols

    fig, axes = plt.subplots(rows, cols, figsize=(cols * 5.0, rows * 4.5))
    if rows * cols == 1:
        axes = np.array([[axes]])
    elif rows == 1:
        axes = np.array([axes])
    elif cols == 1:
        axes = np.array([[ax] for ax in axes])

    for idx, sc in enumerate(scenarios):
        r, c = divmod(idx, cols)
        ax = axes[r][c]
        if metric == "accessible_surface_atoms":
            render_accessible_panel(ax, sc)
        elif metric == "interface_arc_length":
            render_arc_length_panel(ax, sc)
        elif metric == "dead_li_fraction":
            render_dead_li_panel(ax, sc)
        else:
            ax.set_title(f"renderer not implemented: {metric}")
    # Blank any unused slots.
    for idx in range(len(scenarios), rows * cols):
        r, c = divmod(idx, cols)
        axes[r][c].axis("off")

    judged = [sc for sc in scenarios if sc.get("judgment") in ("PASS", "FAIL")]
    pass_count = sum(1 for sc in judged if sc.get("judgment") == "PASS")
    info_count = sum(1 for sc in scenarios if sc.get("judgment") == "INFO")
    info_suffix = f", {info_count} INFO" if info_count else ""

    if metric == "accessible_surface_atoms":
        param = f"contact_factor={data.get('contact_factor', '?')}"
    elif metric == "interface_arc_length":
        param = f"y_bin={data.get('y_bin_angstroms', '?')} Å"
    elif metric == "dead_li_fraction":
        param = (f"cutoff_factor={data.get('cutoff_factor', '?')} "
                 f"(={data.get('cutoff_angstroms', '?')} Å)")
    else:
        param = ""

    fig.suptitle(
        f"morphology metric: {metric}   "
        f"({param}, {pass_count}/{len(judged)} PASS{info_suffix})",
        fontsize=12,
    )
    fig.tight_layout(rect=(0, 0, 1, 0.97))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=140)
    plt.close(fig)
    print(f"wrote {out_path}")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--metric", default=None,
                   help="metric name (accessible_surface_atoms, ...)")
    p.add_argument("--in", dest="in_path", default=None,
                   help="explicit path to JSON input")
    p.add_argument("--out", dest="out_path", default=None,
                   help="explicit path to PNG output")
    args = p.parse_args()

    if args.in_path:
        in_path = Path(args.in_path)
    elif args.metric:
        in_path = REPO_ROOT / f"images/morphology_validation/{args.metric}.json"
    else:
        p.error("specify --metric or --in")

    if args.out_path:
        out_path = Path(args.out_path)
    else:
        out_path = in_path.with_suffix(".png")

    with in_path.open() as f:
        data = json.load(f)

    render_metric(data, out_path)


if __name__ == "__main__":
    main()
