## EIS DCR Pulse + C-rate Replication Plan

**Version 0.1** — drafted 2026-05-04. Plan-of-record for the DCR-pulse / C-rate cycling work on `feature/eis-amplitude-study`.

Parallel to `EIS_AMPLITUDE_STUDY_PLAN.md` (frequency-domain probe). This doc covers the **time-domain pulse probe** of the same Randles-class equivalent circuit, plus the multi-cycle C-rate sweep that exposes plating at high overpotential.

## Background — experimental observation we are replicating

Reference cell data (Honda, NMC811 / Li-metal / liquid electrolyte): an 8-tier C-rate sweep (C/5 → 3C, 5 cycles per tier) with a 10-second DCR pulse at the start of each cycle's discharge. Observations:

1. **R_ohmic flat** (~100–120 mΩ) across all C-rates — purely series, independent of overpotential.
2. **R_polarization shrinks with C-rate** (~250 mΩ at C/5 → ~100 mΩ at 3C) — Butler-Volmer kinetics linearize at higher overpotential, so apparent charge-transfer "resistance" decreases.
3. **2-RC ECM fits the V(t) pulse + relaxation** with RMSE ~0.03 mV: R₀ (ohmic) — R₁‖C₁ (fast arc, charge transfer, τ₁ ≈ 0.6–1.9 s) — R₂‖C₂ (slow arc, diffusion, τ₂ ≈ 7–15 s).
4. **CE drops + charge inflation explodes at 3C** (Q_chg − Q_dchg climbs to 100–150 mAh — out of a ~275 mAh nominal capacity). This is the textbook **Li plating** signature — charge goes in but does not come back as Li-deintercalation.
5. **CCCV hold time spikes at 3C** — parasitic side-reaction symptom, also a plating tell.

Items 1–3 are time-domain analogs of the EIS Nyquist arcs we already extract (R_s + R_ct‖C_dl + Warburg). Items 4–5 are the morphologically interesting part: this is the same experimental→simulation chain we have been wanting to close for the EIS amplitude→Z gap (see `EIS_PHASE_5_AMPLITUDE_MAP.md`).

## Goal

Reproduce, *in silico*:

1. The 2-RC ECM fit on a single DCR pulse, showing R₀, R₁, R₂, τ₁, τ₂ in our cell.
2. The smooth C-rate trend in R_polarization (high C-rate → lower apparent R_pol).
3. The plating signature at high C-rate (charge inflation, CE drop) — coupled to morphology metrics from Phase 4 of the amplitude plan.

Cross-validation: items 1 and 2 should be quantitatively consistent with the Randles arc parameters extracted from the ongoing cos-EIS DOE on the same cell.

## Tier structure (cost-ordered)

| Tier | What | Effort | Status |
|---|---|---|---|
| 1 | **Pulse mode**: 10 s on at fixed amplitude, 30+ s rest. Sweep amplitude (proxy for C-rate). | Small — extends EIS galvanostatic infra; replace AC sinusoid with square pulse. | NOT STARTED |
| 2 | **2-RC ECM fit script**: V(t) = V_∞ + ΔV·(1 − A·exp(−t/τ₁) − B·exp(−t/τ₂)) on pulse-on; mirror on relax. Cross-validate against Nyquist extraction. | Small — pure Python. | NOT STARTED |
| 3 | **Cycle protocol**: charge–rest–pulse–rest–discharge–rest, log per-cycle Q_chg / Q_dchg / CE / charge-inflation. | Medium — state machine. EIS already has phases. | NOT STARTED |
| 4 | **CCCV control** (galvano to V_max, then potentiostatic hold until I drops below threshold). | Medium — needs P or PI feedback loop on V→I. Sim is currently galvanostatic-only. | NOT STARTED |
| 5 | **Plating diagnostics**: Li-metal mass / accessible surface / dead Li tracked across cycles. | Already in progress under `EIS_AMPLITUDE_STUDY_PLAN.md` Phase 4. | DEPENDENCY |
| 6 | **Multi-cycle stability**: 5 cycles per amplitude, 8 amplitudes → ~40 cycles. Cycle ≈ several sim seconds. Whole protocol ≈ hours of compute per run. | Just expensive. | DEPENDENCY |

## Phase 1 — Pulse mode + ECM fit (do this first)

### Tasks

1. **Pulse waveform generator.** New `WaveformKind::Pulse { duration_on_fs, duration_rest_fs }` variant in `src/simulation/eis.rs` (or a new sibling module — TBD). Repeats a square-wave at amplitude A.
2. **Sweep harness.** A `dcr_pulse_sweep` binary analogous to `eis_quick_sweep` that runs N pulses at amplitude A, rest in between, at a fixed cell SOC.
3. **CSV schema.** Per-pulse rows with: `pulse_idx, t_on_fs, t_off_fs, V_pre_pulse, V_pulse_end, V_relax_end, I_amplitude, Q_integrated_on, Q_integrated_off`. Plus dense V(t) / I(t) for the first pulse (for fit-quality diagnostics) — separate file similar to `eis_timeseries/`.
4. **2-RC fit.** `scripts/fit_dcr_2rc.py`: load pulse CSV + dense V(t), fit `V(t) = V_∞ + ΔV·(1 − A·exp(−t/τ₁) − B·exp(−t/τ₂))` (4 params per side: ΔV, A, τ₁, τ₂; B = 1 − A under the constraint that V settles to ΔV). Report R₀ = (V_pulse_start⁺ − V_pre_pulse) / I, RMSE, fit confidence.
5. **C-rate × amplitude mapping.** We do not have an absolute C-rate calibration in the sim. For Phase 1, treat amplitude as a stand-in (the existing 0.001…0.40 ladder is 8 amplitudes — same count as the experimental C-rate tiers). Tag the lowest amplitude in the linear EIS regime as our "C/5 analog" and rescale.
6. **Cross-validate vs Nyquist.** R₀ from pulse should equal R_s extrapolated from cos-DOE Nyquist at f→∞. R₁ from pulse should bracket the fast Randles arc diameter. R₂ from pulse should match the LF diffusion ascent slope. If they disagree by more than 2×, the pulse fit window is wrong.

### Acceptance criteria

- `dcr_pulse_sweep` lands a single-amplitude run that produces a clean V(t) pulse trace at the equilibrated cell SOC.
- 2-RC fit returns R₀, R₁, R₂, τ₁, τ₂ with RMSE < 1 mV.
- The smallest-amplitude pulse extracts an R_pol consistent with the EIS Nyquist arc diameter at the corresponding frequency band, within a factor of 2.

## Phase 2 — Multi-amplitude pulse sweep (matches experimental C-rate sweep)

### Tasks

1. Run `dcr_pulse_sweep` at all 8 ladder amplitudes (analogs of C/5, C/3, C/2, 0.7C, 1C, 1.5C, 2C, 3C).
2. Plot R₀, R₁, R₂, τ₁, τ₂ vs amplitude (analog of the figure-2 ECM-parameter panel).
3. Report whether the sim shows the expected R_pol-vs-overpotential decay (Butler-Volmer linearization signature).

### Acceptance criteria

- R₀ flat across amplitudes (within 10%) — confirms ohmic component is amplitude-independent.
- R_pol = R₁ + R₂ trends downward with amplitude. If it doesn't, the sim's BV kinetics differ qualitatively from the cell, which is itself a finding worth documenting.
- τ₁ stays in a narrow band (charge-transfer relaxation should be intrinsic to interface kinetics, not overpotential).
- τ₂ may drift (diffusion τ depends on concentration gradient depth, which depends on pulse duration vs C-rate).

## Phase 3 — Multi-cycle protocol (charge–rest–pulse–rest–discharge–rest)

### Tasks

1. Cycle state machine (extend the EIS run-loop). States: Idle → Charge_CC → Charge_Rest → DCR_Pulse → Pulse_Rest → Discharge_CC → Discharge_Rest → Idle (or repeat).
2. Per-cycle metrics: Q_chg, Q_dchg, CE = Q_dchg/Q_chg, ΔQ = Q_chg − Q_dchg.
3. Hook morphology metrics into per-cycle output (depends on Phase 4 of `EIS_AMPLITUDE_STUDY_PLAN.md`).
4. CCCV is **not** implemented yet — Phase 3 v1 uses pure CC charge to a fixed cumulative Q. CCCV is Phase 4 of this doc.

### Acceptance criteria

- 5 cycles at a low amplitude show CE > 95% (no significant plating).
- 5 cycles at the highest tested amplitude reproduce the experimental "charge inflation" trend qualitatively (ΔQ > 0 and growing with amplitude).

## Phase 4 — CCCV control

### Tasks

1. Add a `Potentiostatic` mode that closes a P or PI loop on V_cell → I_target. Existing galvano mode keeps a fixed I; potentiostatic mode keeps a fixed V.
2. CCCV protocol: switch from galvano (CC) to potentiostatic (CV) when V_cell first reaches V_max. Hold until |I| drops below a threshold.
3. Record CCCV hold time per cycle.

### Acceptance criteria

- Stable potentiostatic hold (no I oscillation > 10% of mean) after switching.
- CCCV hold time grows monotonically with charge amplitude in the high-amplitude regime (matches experimental trend).

## Phase 5 — Plating + morphology coupling

This phase is the experimental→simulation closure on plating. It depends on Phase 4 of `EIS_AMPLITUDE_STUDY_PLAN.md` (morphology metrics in `src/simulation/morphology.rs`) being complete.

### Tasks

1. After each cycle, log: `interface_arc_length`, `interface_roughness_rms`, `dead_li_fraction`, `accessible_surface_atoms`.
2. Correlate per-cycle ΔQ (charge inflation) with cumulative dead_li_fraction.
3. Look for the experimentally-observed sign: plating events leave dead Li → accessible surface area grows or fragments → R_pol behavior shifts in subsequent cycles.

### Acceptance criteria

- ΔQ vs cumulative dead_li_fraction shows a positive correlation across the C-rate sweep.
- 3C-analog amplitude produces visible morphology change (qualitative, screenshot-confirmed) over 5 cycles, while C/5-analog does not.

## Open questions

1. **Absolute C-rate calibration.** We don't have one. Pinning sim-amplitude to physical C-rate requires either (a) a separate cell-capacity DOE, or (b) accepting that we report only normalized C-rates (C/5–3C ratio). v1 takes (b).
2. **Pulse duration vs sim time scale.** Experimental pulse = 10 s. Our diffusion timescale is ~ns–μs depending on D_Li⁺. We may need to use a shorter pulse and rescale τ₁, τ₂ accordingly. The Phase 0b similitude doc from `EIS_AMPLITUDE_STUDY_PLAN.md` should cover this once written.
3. **CCCV vs pure CC.** Pure CC charging to a fixed Q is sufficient for the plating story but loses the CCCV-hold-time signature. Decide before Phase 3 whether to defer Phase 4 or run them in parallel.
4. **Cycle compute cost.** 5 cycles × 8 amplitudes × tens of seconds per cycle = many hours. Need to budget cluster time before committing to Phase 3.

## Progress log

Append-only. Date-stamp every entry. Reference commits where possible.

### 2026-05-04 — Plan drafted
- Initial draft after the user shared the experimental DCR-pulse / C-rate sweep figures (NMC811 / Li-metal / liquid electrolyte).
- Tier structure 1–6 + phase split 1–5 agreed in the planning conversation.
- Decided: Tiers 1+2 (pulse mode + 2-RC fit) start in parallel with Phase 4 morphology work from `EIS_AMPLITUDE_STUDY_PLAN.md`.
- No implementation work yet; this doc precedes any code.
- Companion morphology work (`src/simulation/morphology.rs#accessible_surface_atoms`) landed unrelated to this plan but is a Phase 5 dependency here.

## References

- `docs/EIS_AMPLITUDE_STUDY_PLAN.md` — frequency-domain master plan; Phase 4 morphology and Phase 5 conditioning are dependencies of this doc's Phase 5.
- `docs/EIS_PHASE_1_3_VALIDATION.md` — Randles-arc Nyquist baseline against which pulse-mode R₀/R₁/R₂ will be cross-validated.
- `docs/EIS_PHASE_5_AMPLITUDE_MAP.md` — the methodological gap this plan helps close (amplitude → morphology → impedance chain).
- `src/simulation/eis.rs` — galvanostatic infra to extend with a pulse waveform.
- `src/simulation/morphology.rs` — Phase 4 metrics consumed in Phase 5 here.
