# EIS Amplitude Floor — empirical determination

**Status:** v1, 2026-05-01. Result of running `find_amplitude_floor` on the validation scenario; supersedes the spec's pre-Phase-1 "use 1e-3 in Δratio units" placeholder.

## Purpose

Phase 1 of `EIS_AMPLITUDE_STUDY_PLAN.md` requires a small-signal EIS perturbation amplitude that is (a) **above** the simulator's quantization noise floor and (b) **below** the onset of nonlinearity. This document records the empirical sweep that fixes the lower bound.

## Setup

- Scenario: `measurement_configs/eis_validation_flat_symmetric.toml`
- Tool: `cargo run --release --bin find_amplitude_floor`
- Equilibrate 50 ks; per-amplitude: 30 ks settle / 5 ks rest; record current + V_cell over the second half of each settle window
- Voltage probe: `plotting::analysis::calculate_cell_voltage` (Coulomb sum at foil centroids — same number the GUI's CellVoltage plot displays)
- Current measurement: `Foil::electron_delta_since_measure` read-and-cleared each step (same `actual_i` quantity the EIS lock-in uses)

## Results

| Δratio | I_a (e/fs) | I_b (e/fs) | V_cell (mV) | Notes |
|---|---|---|---|---|
| baseline (0) | 0 | 0 | +0.00 | no charging |
| 0.0005 | +6.7e-5 | +2.0e-4 | −2.07 | quantization-limited |
| 0.0010 | +6.7e-5 | +2.0e-4 | −4.80 | quantization-limited |
| 0.0020 | +6.7e-5 | 0 | −12.88 | crossing into linear |
| 0.0050 | +1.3e-4 | +1.3e-4 | −35.96 | linear |
| 0.0100 | +1.3e-4 | +6.7e-5 | −59.12 | linear ✓ |
| 0.0200 | +3.3e-4 | −1.3e-4 | −117.73 | linear ✓ |
| 0.0500 | 0 | −2.0e-4 | −296.20 | linear ✓ |

CSV: `doe_results/eis_validation/amplitude_floor.csv`

## Findings

**1. ΔV_cell scales linearly with Δratio above ~Δ=0.002.** Empirical conversion factor is **−6.0 V per unit of Δ**, equivalent to ≈ **6 mV per 0.001 of Δratio**. This factor differs from a naive per-body charge-state estimate (−4 V/Δ) because `calculate_cell_voltage` is a Coulomb sum at foil centroids over all bodies — it captures geometry effects (discrete excess charge → localized potential at the centroid) plus electrolyte response.

**2. Quantization floor ≈ Δ = 0.001** (≈ 5 mV cell potential). One excess electron on a 1040-body foil produces ~5–10 mV at the centroid, so any Δ below this rounds to the same single-electron state and reads ~5 mV regardless. Don't sweep below 0.001.

**3. Butler-Volmer currents are kinetics-limited.** Even at Δ=0.05 (V_cell ≈ 300 mV), only ~3 electrons cross the recording window. The simulator's effective Butler-Volmer rate constants are *much* smaller than a real Li/Li cell, so the chronoamperometric current is small even at large overpotential. **For EIS this is fine** — the lock-in extracts the AC component independently of the DC throughput — but it's a calibration finding to keep in mind for any direct quantitative comparison to experimental currents.

**4. |I_a| ≈ |I_b| only at the highest Δ.** At small Δ the currents are at or below the 1-electron / 15-ks window resolution (6.7×10⁻⁵ e/fs), so signs are indeterminate. At Δ=0.02 and 0.05 the magnitudes are similar with opposite signs (as expected for a symmetric cell), confirming the measurement physics.

## Decisions

- **EIS measurement amplitude (Phase 1, validation): Δ = 0.005** ≈ 35 mV cell potential. Comfortably in the linear regime, well above quantization, mid-range of the user's experimental 10–110 mV.
- **EIS amplitude-study sweep range (Phase 5): Δ ∈ [0.0017, 0.018]** ≈ 10 to 110 mV cell potential — directly mirrors the experimental amplitude range.
- **Phase 3 (THD diagnostics) re-checks the upper bound.** The "linear regime ends at" point isn't determined here; that's what THD harmonics will tell us.

## Caveats / open questions

- **Quantization floor depends on foil-body count.** With 1040 bodies, 1 excess electron = ~5–10 mV at the centroid. A larger foil (more bodies) would have a finer quantization. If we change scenario geometry, the floor moves.
- **Conversion factor depends on geometry too.** The −6 V/Δ value is specific to this validation scenario (250 Å gap, 50×200 Å foils, ~1370 electrolyte particles). Other scenarios will have different effective factors. `verify_potential_conversion` re-computes it.
- **PID oscillation ±20% around the time-average is real.** With default PID gains the foil ratio bounces around its target by ~1 electron, producing ~10 mV instantaneous noise on V_cell. Time-averaging over the second half of a 30-ks settle window smooths this out, but for finer-grained measurements (e.g., AC lock-in) we may need to tune PID gains.

## Related

- Spec: `docs/EIS_AMPLITUDE_STUDY_PLAN.md` Phase 1, 3, 5
- Similitude: `docs/eis_similitude.md` (Δratio-to-mV mapping is empirical here, supersedes the doc's predicted 4×Δ)
- Verification binary: `src/bin/verify_potential_conversion.rs`
- Floor binary: `src/bin/find_amplitude_floor.rs`
