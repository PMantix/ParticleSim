# Phase 5 — Amplitude-Dependence Map (probe amplitude × frequency)

**Status:** in-progress (DOE landing — doe-cos-010..017 in flight on south)
**Date drafted:** 2026-05-03
**Branch:** `feature/eis-amplitude-study`
**Prerequisites:** Phase 1.3 complete (validation cell + cos waveform + lock-in verified — see `EIS_PHASE_1_3_VALIDATION.md`).

## Scope

This writeup covers the **probe-amplitude linearity map**: how the lock-in's reported Z depends on the AC excitation amplitude when applied to the same fresh-equilibrated cell, across the accessible frequency range.

This is foundational input for the larger Phase 5 (conditioning + measurement) protocol described in `docs/EIS_AMPLITUDE_STUDY_PLAN.md`. Specifically:

- **Phase 3 (Linearity & Harmonic Diagnostics)** — establishes the small-signal regime where the lock-in's LTI premise holds. This map directly answers that.
- **Phase 5 (Conditioning + Measurement)** — uses a *small* probe amplitude (chosen from this map) and a *large* conditioning amplitude (separate from probe). This map identifies what "small enough" means.

The full Phase 5 conditioning+measurement state machine (loading saved states between amplitude variants, running conditioning waveforms, then probing) is **not** addressed here — it requires Phase 4 morphology metrics and RNG state serialization (Phase 5 prerequisite from the plan).

## Important: this map does NOT yet replicate the experimental amplitude-impedance observation

The experimental motivating observation (`docs/EIS_AMPLITUDE_STUDY_PLAN.md` Background) is:

> Increasing AC excitation amplitude causes (a) reduced impedance at high frequency and (b) increased impedance at low frequency. Hypothesized mechanism: high-amplitude excitation drives interface evolution to a rough/mossy state. The increased active surface area lowers charge-transfer impedance (HF), while tortuous transport through the rough/mossy SEI region raises mass-transport impedance (LF).

**Our current data does NOT show this.** |Z| is amplitude-invariant to within 1–4% across the 2.5× amplitude range tested — the cell's measured impedance does not depend on the probe amplitude (within the linear and quasi-linear regimes). This is not a contradiction with the experimental observation; it is a methodological gap:

- **Experiment:** amplitude conditions the morphology (over many cycles, possibly many seconds), and then EIS is measured *on the conditioned cell* (typically at small amplitude). |Z|(post-conditioning) depends on A_conditioning because morphology evolved.
- **This DOE:** each job starts from a fresh-equilibrate baseline and runs only ~8 cycles per frequency. The cell never carries morphological history between amplitudes. We measure how the lock-in's *probe-amplitude response* of a *fixed cell* depends on amplitude — which is fundamentally a different question.

To replicate the experimental phenomenon we need:

1. **Phase 4 (morphology metrics)** — implement `MorphologyMetrics` struct measuring `interface_arc_length`, `interface_roughness_rms`, `dead_li_fraction`, `accessible_surface_atoms`. Scaffold landed in `src/simulation/morphology.rs`.
2. **Phase 5 prerequisite (RNG state serialization)** — required to deterministically load the same baseline state for each amplitude variant.
3. **Phase 5 state machine (`EisAmplitudeStudy`)** — drives a conditioning waveform at A_cond for `conditioning_duration_fs`, then probes at a small fixed measurement amplitude, then resets state and repeats with a new A_cond.
4. **Phase 4.3 (conditioning-duration calibration)** — pick `conditioning_duration_fs` from morphology saturation curve.

Until these are built, we cannot observe the experimental amplitude→morphology→impedance chain. The current Phase 3 / Phase 5-amplitude-map characterization is necessary but not sufficient.

### What within-job evidence of morphology evolution would look like

Even in the absence of Phase 4/5, hints of morphology evolution would appear in our existing deep-dive captures as:

- **Cycle-to-cycle drift in V_cell envelope** within a single sweep — observed at amp=0.40 (Lissajous shows 4 distinct loop traces) but driven by voltage clipping, not morphology.
- **Net cumulative foil electron-count drift** — pure cycle-balanced AC integrates to zero per cycle; non-zero drift indicates asymmetric plating/stripping.
- **Asymmetric forward vs reverse cycle shapes** — different Butler-Volmer kinetics for ox vs red would manifest as in-phase voltage offsets ~constant in V (which we *do* observe — the −2 mV in-phase noise floor signature).

The −2 mV constant in-phase voltage we observed at all amplitudes is consistent with kinetic asymmetry between forward and reverse foil↔electrolyte reactions. This is a hint of incipient asymmetry — but not enough to drive morphological evolution in 8 cycles.

## Question being answered

For our validation cell (flat symmetric Li-metal foils, EC/DMC + LiPF6 electrolyte), at each frequency in the accessible range:

1. **Where is the linear-regime ceiling?** (Below this V_amp, Z is amplitude-independent within fit noise.)
2. **What does Z look like above the ceiling?** (Re inflation from envelope distortion; |Z| changes; phase shifts from −90°.)
3. **Where is the V-saturation breakdown?** (V_amp pegs at the cell's max sustainable swing; lock-in fits no longer mean anything.)
4. **At what amplitude does THD exceed 1% / 5%?** (Standard EIS small-signal validity thresholds — Phase 3 acceptance criterion.)

## DOE coverage

### Existing (from Phase 1.3 cos DOE)

| Amplitude | HF [5e-4, 5e-3] | Mid [5e-5, 5e-4] | LF [5e-6, 5e-5] | Narrow [5e-5, 2e-4] |
|-----------|-----------------|-------------------|-------------------|----------------------|
| 0.001 | — | — | doe-cos-009 (in flight) | — |
| 0.02 | doe-cos-003 | doe-cos-001 | doe-cos-002 | — |
| 0.04 | — | — | — | doe-cos-007 |
| 0.06 | doe-cos-006 | doe-cos-004 | doe-cos-005 | — |
| 0.10 | — | — | — | doe-cos-008 |

### Queued (Phase 5 amplitude map fill-in — doe-cos-010..017)

| Amplitude | HF [5e-4, 5e-3] | Mid [5e-5, 5e-4] | LF [5e-6, 5e-5] |
|-----------|-----------------|-------------------|-------------------|
| 3e-4 (ultra-low — noise floor) | — | doe-cos-017 | — |
| 0.005 (linear plateau check) | — | doe-cos-010 | doe-cos-011 |
| 0.04 (gap-fill) | doe-cos-012 | (have doe-cos-007) | doe-cos-013 |
| 0.10 (gap-fill) | doe-cos-014 | (have doe-cos-008) | — |
| 0.20 (deep breakdown) | — | doe-cos-015 | — |
| 0.40 (extreme breakdown) | — | doe-cos-016 | — |

After all 17 cos jobs land, the matrix has 6 amplitudes × 3 freq bands well-covered, with extra coverage in the linearity-onset and breakdown regimes.

## Preliminary findings (from existing data)

### Linear-regime ceiling vs frequency (from Phase 1.3 cos DOE)

At f=5e-5 (mid-band):

| amp | V_amp (mV) | \|Z\| | Re(Z) | phase | regime |
|-----|-----------|------|-------|-------|--------|
| 0.02 | 309 | 15.4 | +0.14 | −89.5° | linear ceiling |
| 0.04 | 608 | 15.2 | +0.97 | −86.3° | mild nonlinearity |
| 0.06 | 909 | 15.1 | +1.21 | −85.4° | nonlinearity envelope |
| 0.10 | 1507 | 15.1 | +1.32 | −85.0° | saturation envelope |

**\|Z\| stable to 2.5%** across the 5× amplitude range — the cell is robustly capacitive at this characteristic. Re(Z) inflates with amplitude as a "saturation halo" on a non-LTI ellipse, but Im(Z) is amplitude-independent as theory requires.

**Linear ceiling at f=5e-5 is between V_amp=309 mV (clean) and V_amp=608 mV (mild nonlinearity).** Likely around V_amp ≈ 400 mV.

### V-saturation breakdown vs frequency

| amp | first f at which V_amp peaks | V_amp at peak (mV) |
|-----|------------------------------|-------------------|
| 0.02 | f=1.08e-5 | 1268 |
| 0.06 | f=2.32e-5 | 1807 |

The cell physically cannot sustain V_amp > ~1.5–1.8 V. Below the saturation frequency, V drops *back* below this peak (clipped, R²(V) collapses). Saturation onset is amplitude-dependent: smaller amp can probe to lower f before saturating.

This implies the **LF probe budget** for any given amplitude:

```
  f_min(amp)  ≈  V_sat / (amp · |Z|_capacitive(f_min))
```

For V_sat ≈ 1.6 V and the capacitive |Z| ∝ 1/f, this gives f_min ∝ amp. Concretely:
- amp=0.02 → f_min ≈ 1e-5
- amp=0.06 → f_min ≈ 2e-5
- amp=0.001 (doe-cos-009) → f_min ≈ 5e-7  (extends the LF reach by ~20×)

## Pending — to be filled in as DOE lands

### Open questions

- **Where is the LF Randles arc apex?** doe-cos-009 (amp=0.001 at f=[5e-6, 1e-5]) targets this. If apex visible: extract R_ct (the diameter), V_amp_at_apex.
- **THD at large amplitude.** doe-cos-015/016 (amp=0.20, 0.40) should produce THD > 5% per Phase 3 acceptance. Need to extend lock-in to report H₂, H₃ — currently we only have R²(V) as a fit-quality proxy. Phase 3 task list item.
- **Is the linear plateau truly flat below amp=0.02?** doe-cos-010/011 (amp=0.005) and doe-cos-017 (amp=3e-4) test this. Predicts Re(Z) → 0 at very small amp (only kinetic-L tail remains), \|Z\| unchanged.

### Plots to produce when data is complete

1. **Re(Z) vs amplitude at fixed frequency** — picks out the nonlinearity envelope shape. Should show:
   - Flat plateau at small amp (Re ≈ Re_kinetic_L only)
   - Inflation onset at amp ≈ V_linear / |Z|
   - Plateau or further growth in the non-LTI regime
   - Possible flip past breakdown
2. **|Z| vs amplitude at fixed frequency** — should be flat across the linear regime, then collapse in the breakdown regime.
3. **Phase vs amplitude at fixed frequency** — most sensitive to regime change.
4. **LF probe budget map** — V_amp_max sustainable as function of f, overlaid with predicted V_amp = amp · |Z|(f) lines for each tested amplitude.

## References

- `docs/EIS_AMPLITUDE_STUDY_PLAN.md` — master plan defining Phases 3 and 5
- `docs/EIS_PHASE_1_3_VALIDATION.md` — validation of the underlying methodology
- `coordination/north_jobs.jsonl` — DOE queue including doe-cos-010..017
- `images/eis_validation_runs/master_nyquist_cos.html` — interactive Nyquist updated as DOE lands
