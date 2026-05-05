"""Test the 'sign convention bug' hypothesis on Re(Z) values.

For each representative CSV, re-fit V and I locally and compute Z under:
  A. Standard convention (matches the sim's lock-in)
  B. V cos/sin swapped (simulates phase-axis confusion)
  C. I cos/sin swapped
  D. Both swapped

Compare Re(Z), Im(Z), |Z|, phase across all four. If the sim's lock-in
has a sign-convention bug, one of B/C/D should give Re(Z) > 0 with a
"more physical" result. If A/B/C/D all agree on |Z| but disagree on Re
sign, it's a convention question. If A matches the log and the others
diverge, the sim's reading is mathematically correct.
"""
from __future__ import annotations

import csv
import math
import re
import sys
from pathlib import Path

import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent

# Representative CSVs from local runs (integration test = I=0.6 sweep)
CASES = [
    ("HF saturated (I=0.6, V≈40 mV)",     "eis_ts_001_5.000e-3.csv",  5.000e-3,  -0.062, -0.025, "HF-sat"),
    ("Mid saturated (I=0.6, V≈127 mV)",   "eis_ts_002_2.154e-3.csv",  2.154e-3,  -0.184, -0.104, "mid-sat"),
    ("Bridge regime (I=0.6, V=365 mV)",   "eis_ts_003_9.283e-4.csv",  9.283e-4,  -0.293, -0.171, "bridge"),
    ("LF saturated (I=0.6, V=378 mV)",    "eis_ts_004_4.000e-4.csv",  4.000e-4,  -0.545, -0.316, "LF-sat"),
    ("LF small amp (low V_amp)",          "eis_ts_001_5.000e-6.csv",  5.000e-6,  None, None, "LF-low-V"),
]


def load_csv_recording(path: Path):
    """Return (t_rec, v_rec, i_rec, freq) — only the recording-phase samples."""
    with path.open() as f:
        first = f.readline().strip()
        m = re.search(r"freq=([\d.e+\-]+)", first)
        freq = float(m.group(1)) if m else 0.0
        rdr = csv.DictReader(f)
        rows = list(rdr)
    t = np.array([float(r["t_rel_fs"]) for r in rows])
    v = np.array([float(r["v"]) for r in rows])
    i = np.array([float(r["i"]) for r in rows])
    is_rec = np.array([int(r["is_recording"]) for r in rows], dtype=bool)
    return t[is_rec], v[is_rec], i[is_rec], freq


def fit_phasor(t: np.ndarray, sig: np.ndarray, omega: float):
    """3-parameter LS: signal(t) = re·cos + im·sin + dc. Returns (re, im, dc)."""
    cos_t = np.cos(omega * t)
    sin_t = np.sin(omega * t)
    M = np.column_stack([cos_t, sin_t, np.ones_like(t)])
    coeffs, *_ = np.linalg.lstsq(M, sig, rcond=None)
    return float(coeffs[0]), float(coeffs[1]), float(coeffs[2])


def z_from_phasors(v_re, v_im, i_re, i_im):
    """Standard complex division Z = V/I with both phasors as A·cos + B·sin."""
    denom = i_re * i_re + i_im * i_im
    if denom < 1e-30:
        return float("nan"), float("nan")
    z_real = (v_re * i_re + v_im * i_im) / denom
    z_imag = -((v_im * i_re - v_re * i_im) / denom)  # match sim sign convention
    return z_real, z_imag


def main():
    print(f"{'case':<35}  {'conv':<22}  {'Re(Z)':>11}  {'-Im(Z)':>11}  {'|Z|':>10}  {'phase':>8}")
    print("-" * 115)
    for desc, fname, freq, exp_re, exp_im, regime in CASES:
        path = REPO_ROOT / "eis_timeseries" / fname
        if not path.exists():
            print(f"  SKIP: {path.name} not found")
            continue
        t_rec, v_rec, i_rec, csv_freq = load_csv_recording(path)
        # Reference t to recording start
        t0 = t_rec[0]
        t_rel = t_rec - t0
        omega = 2 * math.pi * freq
        v_re, v_im, _ = fit_phasor(t_rel, v_rec, omega)
        i_re, i_im, _ = fit_phasor(t_rel, i_rec, omega)

        conventions = [
            ("A standard",                v_re,  v_im,   i_re,  i_im),
            ("B swap V cos<->sin",         v_im,  v_re,   i_re,  i_im),
            ("C swap I cos<->sin",         v_re,  v_im,   i_im,  i_re),
            ("D swap both",                v_im,  v_re,   i_im,  i_re),
            ("E negate v_re",             -v_re,  v_im,   i_re,  i_im),
            ("F negate i_re",              v_re,  v_im,  -i_re,  i_im),
            ("- sim log (reported)",         None,  None,   None,  None),
        ]

        print(f"\n{desc} ({regime})  f={freq:.2e}, V_amp={math.hypot(v_re, v_im):.3e}, I_amp={math.hypot(i_re, i_im):.3e}")
        for name, vr, vi, ir, ii in conventions:
            if vr is None:
                if exp_re is None:
                    continue  # no log reference for this case
                z_real, z_imag = exp_re, exp_im
            else:
                z_real, z_imag = z_from_phasors(vr, vi, ir, ii)
            mag = math.hypot(z_real, z_imag)
            phase = math.degrees(math.atan2(z_imag, z_real))
            print(f"  {name:<22}  {z_real:>+11.3e}  {-z_imag:>+11.3e}  {mag:>10.3e}  {phase:>+7.1f}°")


if __name__ == "__main__":
    main()
