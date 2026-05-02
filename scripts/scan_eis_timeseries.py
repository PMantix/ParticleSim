"""Bulk-analyze every CSV in eis_timeseries/ and print a sortable summary
(file, mtime, freq, R²(V), R²(I), |Z|, phase) so we can spot which historical
runs produced clean lock-in fits. No plots.
"""
from __future__ import annotations

import csv
import math
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent
TS_DIR = REPO_ROOT / "eis_timeseries"


def lock_in_fit(t_rel: np.ndarray, sig: np.ndarray, omega: float):
    n = len(t_rel)
    if n < 3:
        return float("nan"), float("nan"), float("nan")
    cos_t = np.cos(omega * t_rel)
    sin_t = np.sin(omega * t_rel)
    M = np.column_stack([cos_t, sin_t, np.ones_like(t_rel)])
    try:
        coeffs, *_ = np.linalg.lstsq(M, sig, rcond=None)
    except np.linalg.LinAlgError:
        return float("nan"), float("nan"), float("nan")
    A, B, C = coeffs
    fit = M @ coeffs
    resid = sig - fit
    ss_res = float((resid * resid).sum())
    ac_part = sig - sig.mean()
    ss_tot = float((ac_part * ac_part).sum())
    r2 = 1.0 - ss_res / ss_tot if ss_tot > 1e-30 else 1.0
    amp = math.hypot(A, B)
    phase_deg = math.degrees(math.atan2(-B, A))
    return amp, phase_deg, r2


def analyze(path: Path):
    with path.open() as f:
        first = f.readline().strip()
        m_v_dc = re.search(r"v_dc=([-\d.eE+]+)", first)
        m_i_dc = re.search(r"i_dc=([-\d.eE+]+)", first)
        m_freq = re.search(r"freq=([-\d.eE+]+)", first)
        if not (m_v_dc and m_i_dc and m_freq):
            return None
        freq = float(m_freq.group(1))
        rdr = csv.DictReader(f)
        rows = list(rdr)
    if not rows:
        return None
    t = np.array([float(r["t_rel_fs"]) for r in rows])
    v_ac = np.array([float(r["v_ac"]) for r in rows])
    i_ac = np.array([float(r["i_ac"]) for r in rows])
    is_rec = np.array([int(r["is_recording"]) for r in rows], dtype=bool)
    if is_rec.sum() < 3:
        return None
    t_rec = t[is_rec]
    v_rec = v_ac[is_rec]
    i_rec = i_ac[is_rec]
    t0 = t_rec[0]
    t_rec_rel = t_rec - t0
    omega = 2 * math.pi * freq

    amp_v, phase_v, r2_v = lock_in_fit(t_rec_rel, v_rec, omega)
    amp_i, phase_i, r2_i = lock_in_fit(t_rec_rel, i_rec, omega)
    z_mag = amp_v / amp_i if (amp_i and amp_i > 1e-30) else float("inf")
    z_phase = phase_v - phase_i
    while z_phase > 180:
        z_phase -= 360
    while z_phase <= -180:
        z_phase += 360

    return {
        "name": path.name,
        "mtime": path.stat().st_mtime,
        "freq": freq,
        "n_rec": int(is_rec.sum()),
        "amp_v": amp_v,
        "amp_i": amp_i,
        "r2_v": r2_v,
        "r2_i": r2_i,
        "z_mag": z_mag,
        "z_phase": z_phase,
    }


def main():
    paths = sorted(TS_DIR.glob("*.csv"))
    rows = []
    for p in paths:
        try:
            r = analyze(p)
        except Exception as e:
            print(f"  skip {p.name}: {e}", file=sys.stderr)
            continue
        if r:
            rows.append(r)

    print(f"Scanned {len(rows)} CSVs.\n")
    print(f"{'mtime':<17}  {'file':<32}  {'freq':>10}  {'n_rec':>6}  "
          f"{'V_amp':>10}  {'I_amp':>10}  {'R²(V)':>7}  {'R²(I)':>7}  "
          f"{'|Z|':>10}  {'phase':>8}")
    rows.sort(key=lambda r: (r["mtime"], r["freq"]))
    for r in rows:
        ts = datetime.fromtimestamp(r["mtime"]).strftime("%Y-%m-%d %H:%M")
        print(f"{ts:<17}  {r['name']:<32}  {r['freq']:>10.3e}  {r['n_rec']:>6}  "
              f"{r['amp_v']:>10.3e}  {r['amp_i']:>10.3e}  {r['r2_v']:>7.4f}  "
              f"{r['r2_i']:>7.4f}  {r['z_mag']:>10.3e}  {r['z_phase']:>+7.1f}°")

    print(f"\n--- Files with R²(V) ≥ 0.95 AND R²(I) ≥ 0.5 ---")
    good = [r for r in rows if r["r2_v"] >= 0.95 and r["r2_i"] >= 0.5]
    for r in good:
        ts = datetime.fromtimestamp(r["mtime"]).strftime("%Y-%m-%d %H:%M")
        print(f"  {ts}  {r['name']}  freq={r['freq']:.3e}  R²(V)={r['r2_v']:.3f}  "
              f"R²(I)={r['r2_i']:.3f}  |Z|={r['z_mag']:.3e}  phase={r['z_phase']:+.1f}°")
    print(f"\n--- Files with R²(V) ≥ 0.95 AND ANY R²(I) (Galvanostatic possible) ---")
    g2 = [r for r in rows if r["r2_v"] >= 0.95]
    print(f"  {len(g2)} files")


if __name__ == "__main__":
    main()
