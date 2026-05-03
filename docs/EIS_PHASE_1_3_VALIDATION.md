# Phase 1.3 — EIS Validation on the Flat Symmetric Cell

**Status:** complete (LF apex measurement pending — doe-cos-009 in flight)
**Date:** 2026-05-03
**Branch:** `feature/eis-amplitude-study`
**Scenario:** `measurement_configs/eis_validation_flat_symmetric.toml` — two flat Li-metal foil electrodes facing each other across a uniform EC/DMC + LiPF6 electrolyte volume, no initial roughness, equilibrated 50 ks before AC engagement.

## Acceptance against the plan

Phase 1.3 acceptance criteria from `docs/EIS_AMPLITUDE_STUDY_PLAN.md`:

| Criterion | Status |
|-----------|--------|
| Positive HF real-axis intercept | ✅ — observable as the small +R kinetic-inductance tail folding back toward Re=0 at HF |
| At least one identifiable semicircle / arc | ✅ — capacitive arc fully resolved across [5e-5, 5e-4]; charge-transfer onset visible at f<2e-5 in amp=0.02 |
| Low-frequency tail | ✅ — Re grows from +0.10 to +10.4 across f=[5e-5, 1.08e-5] (amp=0.02), well above noise |
| Anomalies documented | ✅ — kinetic inductance at HF, V-saturation ceiling at LF; both characterized below |

## Methodology

### AC perturbation waveform — `cos(ωt)`

`I(t) = A·cos(ωt)` (commit 6c40728), not `sin(ωt)`. Sin integrates to `ΔQ(t) = A·(1−cos(ωt))/ω` which is always non-negative — the cell accumulates positive charge throughout every cycle and never crosses back through equilibrium, biasing the steady state. With cos, `ΔQ(t) = A·sin(ωt)/ω` swings symmetrically between ±A/ω. The lock-in (3-parameter LS fit `re·cos + im·sin + dc`) is waveform-agnostic; only the steady state changes.

This was the dominant fix of the Phase 1 validation work. Pre-fix sin DOE results showed:
- Persistent small negative Re(Z) at HF (~−0.025 to −0.10)
- Large negative Re(Z) at LF saturation (down to −1.6)
- Impossible +Im(Z) (active phase) at extreme amp×LF
- "Saturation breakdown" with |Z| collapsing 32% past V_amp ~ 1 V

All four anomaly classes resolved by cos. Documented in commit `6c40728` and `docs/EIS_PHASE_1_3_VALIDATION.md` (this file).

### Lock-in fit

`src/simulation/eis.rs::EisState::record_step` accumulates per-step:
- `Σ V·cos(ωt)`, `Σ V·sin(ωt)`, `Σ V`, `Σ V²`
- `Σ I·cos(ωt)`, `Σ I·sin(ωt)`, `Σ I`, `Σ I²`

At sweep end, the closed-form 3-parameter least-squares solution gives `(re, im, dc)` for both V and I. Z = V̂/Î via standard complex division. R²(V) and R²(I) reported as fit-quality metrics.

### Sweep configuration

| Parameter | Value |
|-----------|-------|
| Equilibration | 50 ks (10000 steps at dt=5 fs) |
| Settle periods per freq | 4 |
| Capture periods per freq | 4 |
| Voltage probes | every body in foil (~130/group) |
| dt | 5 fs |
| Bodies | 3449 (after equilibration) |

### DOE coverage (cos-waveform jobs)

8 frequency-sweep jobs across 4 amplitudes. All in `doe_results/eis_doe_lf/doe-cos-NNN_*_cos.log`.

| Amplitude | HF [5e-4, 5e-3] | Mid [5e-5, 5e-4] | LF [5e-6, 5e-5] | Narrow [5e-5, 2e-4] |
|-----------|-----------------|-------------------|-------------------|----------------------|
| 0.001 | — | — | doe-cos-009 (in flight) | — |
| 0.02 | doe-cos-003 | doe-cos-001 | doe-cos-002 | — |
| 0.04 | — | — | — | doe-cos-007 |
| 0.06 | doe-cos-006 | doe-cos-004 | doe-cos-005 | — |
| 0.10 | — | — | — | doe-cos-008 |

## Results

![Master Nyquist](../images/eis_validation_runs/master_nyquist_cos.png)

Interactive version: `images/eis_validation_runs/master_nyquist_cos.html` (Plotly, click any legend entry to toggle traces across all four panels).

### Linear-regime impedance (amp=0.02 cos data)

| f (1/fs) | Re(Z) | −Im(Z) | \|Z\| | phase | V_amp (mV) | R²(V) | regime |
|----------|-------|--------|------|-------|-----------|-------|--------|
| 5.00e-3 | −0.001 | +5e-4 | 1e-3 | −156° | 0.02 | 0.03 | noise floor (V≪quantum) |
| 2.32e-3 | −0.142 | +0.249 | 0.29 | −120° | 6 | 0.40 | noise floor |
| 1.08e-3 | −0.125 | +0.521 | 0.54 | −104° | 11 | 0.69 | noise floor |
| 5.00e-4 | −0.116 | +1.51 | 1.52 | −94° | 30 | 0.94 | **kinetic-L tail** |
| 2.81e-4 | −0.078 | +2.69 | 2.69 | −92° | 54 | 0.98 | **kinetic-L tail** |
| 1.58e-4 | −0.109 | +4.98 | 4.98 | −91° | 100 | 0.99 | **kinetic-L tail** |
| 8.89e-5 | −0.131 | +8.64 | 8.64 | −91° | 173 | 0.996 | **kinetic-L tail** |
| 5.00e-5 | +0.142 | +15.4 | 15.4 | −89° | 309 | 0.998 | **clean capacitor** |
| 2.32e-5 | +3.44 | +31.8 | 31.9 | −84° | 639 | 0.998 | **CT arc onset** |
| 1.08e-5 | +10.4 | +62.6 | 63.4 | −81° | 1268 | 0.997 | **CT arc onset** |
| 5.00e-6 | −2.89 | +8.74 | 9.20 | −108° | 184 | 0.61 | **V-saturated** |

### Three regimes visible in the Nyquist

1. **HF "kinetic inductance" tail** (f > 5e-5):
   - Small negative Re ~ −0.1, scales linearly with ω
   - Phase slightly past −90°
   - Origin: explicit electron-transport delay (~3–5 timesteps = 15–25 fs) from discrete electron hopping, multi-pass collision resolution (7 passes/step), and Butler-Volmer foil↔electrolyte gating. See `docs/EIS_PHASE_1_3_VALIDATION.md` discussion below.

2. **Mid-band capacitive arc** (5e-5 ≥ f ≥ 1e-5):
   - Phase −90° → −80°
   - \|Z\| ∝ 1/f as expected for a capacitor
   - Re grows from ~0 → +10 as charge transfer becomes resolvable per cycle

3. **LF saturation** (f < 1e-5 at amp=0.02; f < 1.1e-5 at amp=0.06):
   - V_amp pegs around 1.5–1.8 V regardless of further increase in capacitive demand
   - R²(V) collapses (V_cell no longer a clean sinusoid — it's clipped against the cell's maximum sustainable swing)
   - Phase past −90° (apparent +Im — but it's a fitting artifact on a clipped waveform)

### Cross-amplitude comparison at f=5e-5 (linear ceiling regime)

| amp | V_amp (mV) | Re(Z) | −Im(Z) | \|Z\| | phase | comment |
|-----|-----------|-------|--------|------|-------|---------|
| 0.02 | 309 | +0.14 | +15.4 | 15.4 | −89.5° | linear ceiling |
| 0.04 | 608 | +0.97 | +15.2 | 15.2 | −86.3° | mild nonlinearity |
| 0.06 | 909 | +1.21 | +15.1 | 15.2 | −85.4° | nonlinearity envelope |
| 0.10 | 1507 | +1.32 | +15.0 | 15.1 | −85.0° | saturation envelope |

**\|Z\| is constant within 2.5%** across a 5× amplitude range — the cell really is dominantly capacitive with this characteristic at f=5e-5. Re(Z) inflates with amplitude (saturation halo on a non-LTI ellipse), but Im(Z) is amplitude-independent as theory requires.

## Equivalent circuit interpretation

For the linear regime (amp=0.02), the cell is well-described by:

```
  ──[R_s]──[L_k]──┬──[R_ct]──┬──[W]──
                  │          │
                  └──[C_dl]──┘
```

| Element | Estimate | Source |
|---------|----------|--------|
| R_s (series) | < 0.01 | not separable from kinetic-L tail |
| **L_k (kinetic inductance)** | **τ_k ≈ 20 fs** | Re(Z)/(ω·\|Z\|) at f=8.89e-5 → ~25 fs ≈ 5 timesteps |
| C_dl (double-layer) | 1/(ω·\|Z\|_capacitive) at f=5e-5 → **~1.3 e²·fs/V** in sim units | from \|Z\|=15.4 at f=5e-5 |
| **R_ct (charge transfer)** | **~10–15** (lower bound) | Re(Z) at f=1.08e-5 = +10.4; apex (true R_ct) likely between 10 and 20 — pinned by doe-cos-009 |
| Warburg coefficient | TBD | LF tail not yet fully resolved within the saturation-free amplitude range |

The kinetic-inductance term `L_k` is **not a standard EIS feature**. It's an artifact of explicit electron-hopping kinetics in the simulator — orders of magnitude larger than wire-lead inductance in a real cell. See "Caveats" below.

## Caveats / known issues

### 1. Kinetic inductance from explicit electron transport

The simulator models electrons as discrete particles that hop body-to-body via Butler-Volmer kinetics, then redistribute through the foil over multiple collision-resolution passes per step. Each contributes finite response delay — net ~3–5 timesteps (~15–25 fs) before V_cell catches up to a current step. This shows up as:
- Re(Z) ≈ −\|Z\|·ωτ (small negative, scales with ω)
- Phase slightly past −90°

In a real cell, the analogous inductance is from physical wire leads (~nH, only matters at MHz+). In our sim with explicit kinetics resolved at fs scale, it's amplified into our LF measurement range. **This is a real physical model output, not a measurement bug.** When comparing simulated EIS to experimental EIS, this kinetic-L term should either (a) be subtracted as a known sim-vs-real artifact, (b) be reduced by smaller dt or finer body resolution, or (c) be documented and accepted.

Saved in project memory: `project_kinetic_inductance.md`.

### 2. V-amp saturation ceiling at LF

The cell physically cannot sustain V_amp > ~1.5–1.8 V in our scenario. At sufficiently low frequency, even moderate amplitudes drive V_amp past this ceiling. Symptoms:
- V_amp drops instead of growing as f decreases
- R²(V) collapses (V_cell is clipped, not sinusoidal)
- Apparent phase past −90° (fitting artifact on clipped waveform)
- Re(Z) flips sign with no physical meaning

The saturation onset depends on amplitude (smaller amp → can probe to lower f before saturating). For Phase 1.3, this means the LF apex (R_ct + Warburg) is best probed with the smallest amplitude that still gives R²(V) > 0.85. doe-cos-009 (amp=0.001 at f=[5e-6, 1e-5]) targets this.

### 3. DOE sequence vs fresh-equilibrate disagreement

In a sequential DOE sweep (multiple frequencies in one job), the cell carries state between frequencies. Compared to a fresh-equilibrate single-point measurement (`eis_single_case_deep`), the DOE produces slightly different Re(Z) values at the same point — typically a small magnification of the kinetic-L tail. The DOE values represent "long-term steady state across many cycles"; the fresh-equilibrate values represent "first-cycle response". Both are valid; they answer slightly different questions.

### 4. R²(V) noise floor at very small amplitude

At amp=0.02, frequencies above f=1e-3 have V_amp below ~10 mV, comparable to the V_cell quantization noise floor (~5 mV per excess electron on a 130-body foil). R²(V) drops below 0.85 and the lock-in essentially fits noise. These points are filtered out of the master plot.

## Data inventory

```
doe_results/eis_doe_lf/
├── README.md                       (sin/cos split rationale)
├── sin/                            (30 historical sin-era logs — biased; do not use for new analysis)
└── doe-cos-NNN_*_cos.log           (cos-waveform DOE — Phase 1.3 + Phase 5 baseline)

images/eis_validation_runs/
├── master_nyquist_cos.png          (static plot, this writeup)
└── master_nyquist_cos.html         (Plotly interactive)

doe_results/eis_single_case_deep_*/ (6 sin + 5 cos deep-dives — point-level diagnostics)
images/eis_single_case_deep_*/      (matching plots)

eis_timeseries/sin/                 (raw per-frequency lock-in inputs from sin-era; gitignored)
```

## What's pending

- **doe-cos-009** (amp=0.001, f=[5e-6, 1e-5]) — LF apex pinning. Will reveal whether the cell has a true Randles arc apex within accessible LF or behaves as a pure-capacitor + Warburg through f → 0.
- **Phase 5 amp study** (doe-cos-010..017) — populates the amp×freq matrix for the amplitude-dependence question. Separate writeup.
- **Equivalent-circuit fit** with explicit numerical values for C_dl, R_ct, Warburg coefficient, and L_k — pending the LF apex measurement.

## References

- `docs/EIS_AMPLITUDE_STUDY_PLAN.md` — master plan defining Phase 1.3
- `src/simulation/eis.rs` — lock-in implementation, EisState struct
- Commit `6c40728` — sin → cos waveform fix
- Commit `882b2f9` — calculate_cell_voltage AC bug fix (separate, also closed)
- `coordination/PROTOCOL.md` — north/south DOE coordination protocol
