# EIS Amplitude-Dependence Study — ParticleSim Maturation Spec

**Version 1.1** — drafted 2026-05-01. Plan-of-record for the amplitude-study work on `feature/eis-amplitude-study`.

## Changes from v1.0

- **Conditioning waveform** is now an enum (Sinusoidal | SquareWave | AsymmetricPulse), defaulting to SquareWave. The experimental analog is plate-then-strip current-density cycling, not a pure sinusoid.
- **Phase 0 split** into 0a (bulk-electrolyte scenario for D measurement) and 0b (similitude doc + regime map). The original Phase 0 had a circular dependency on Phase 1.
- **2D-with-3D-Coulomb caveat** elevated from a one-liner to a paragraph in the similitude doc. ParticleSim uses 1/r forces on 2D-confined particles; this is neither a 2D log-Coulomb gas nor a 3D bulk system, and the surface-roughness mechanism scales differently than in the experimental cell.
- **RNG state serialization** moved from Phase 7 nice-to-have to a Phase 5 prerequisite. The amplitude study reloads `initial_state_path` between amplitudes and needs deterministic restart to attribute observed differences to A_cond.
- **Conditioning duration** is no longer a free parameter — it is calibrated as the closing sub-task of Phase 4 and consumed as a default in Phase 5.
- **Measurement amplitude** is parameterized by Phase 3's THD results, not chosen blind. Distinguish "measurement amplitude (THD < 1%)" from "harmonics-onset diagnostic (THD > 5%)".
- **R² threshold** tightened from `> 0.8` to `min(fit_r2_v, fit_r2_i) > 0.95`.
- **Amplitude units** are mode-dependent (e/fs in galvanostatic, dimensionless Δratio in potentiostatic). Stamp units into every output CSV header.
- **Output paths**: keep `eis_timeseries/eis_ts_*.csv` for raw per-frequency lock-in inputs (already in use); aggregated study outputs go to `doe_results/<study_name>/`.
- **Switch-charging is explicitly disabled** during conditioning and measurement phases.
- **Scenario preset infrastructure** does not exist yet — Phase 1 introduces it as an explicit sub-task with schema decision.
- **Top-level `tests/` directory** is introduced for integration tests; in-module `#[cfg(feature = "unit_tests")]` continues for unit tests.

---

## Background

The existing EIS module (`src/simulation/eis.rs`, `src/renderer/gui/eis_tab.rs`) implements a clean lock-in detection pipeline with galvanostatic and potentiostatic modes, log-spaced frequency sweeps, a 3-parameter least-squares sinusoidal fit (V = A cos ωt + B sin ωt + C — the constant DC term is automatically subtracted before extracting A,B), R² fit-quality metric on both V and I, and saved-state restore at sweep end. The probe pipeline now supports configurable per-foil-group probe-body counts (`voltage_probes`), per-frequency repeats (`repeats_per_freq`), and a virtual capacitor parameter (`c_virtual`).

The goal of this spec is to mature the harness so it can reproduce, *in silico*, an experimental observation made on a Li-metal / NMC811 / liquid-electrolyte cell:

> Increasing AC excitation amplitude causes (a) reduced impedance at high frequency and (b) increased impedance at low frequency. Hypothesized mechanism: high-amplitude excitation drives interface evolution to a rough/mossy state. The increased active surface area lowers charge-transfer impedance (HF), while tortuous transport through the rough/mossy SEI region raises mass-transport impedance (LF).

Phases 0–4 are validation, instrumentation, and calibration; Phases 5–7 are the experiment itself plus reproducibility plumbing. **Do not proceed past Phase 1 acceptance criteria without first showing a recognizable Randles-style Nyquist on the validation scenario.**

---

## Phase 0a — Bulk-Electrolyte Scenario + D_Li⁺ Measurement

The simulator and the experimental cell live at different absolute scales. Both length and time scales are compressed in the simulator. To defend any in-silico-vs-experiment claim we need to know the simulator's diffusivity, then compute dimensionless groups in Phase 0b.

### Tasks

1. Add a scenario preset `BulkElectrolyteOnly` (an electrolyte volume of EC/DMC mix and Li⁺/anion ions matching salt molarity, no electrodes, periodic-style boundary handling consistent with the rest of the sim). Lateral and inter-axis dimensions chosen to give a clear bulk in the middle.
2. Add a small Rust binary `src/bin/measure_diffusivity.rs`:
   - Loads a TOML scenario.
   - Equilibrates for `T_equilibrate_fs` (default 50,000 fs ≈ 10,000 steps at the default 5 fs timestep).
   - Tracks Li⁺ mean-squared displacement vs. t for `T_measure_fs` (default 50,000 fs).
   - Restricts the MSD population to a **bulk window** in the middle of the domain (default: middle 50% along each axis) so that ions near domain boundaries don't bias the slope.
   - Removes COM drift before computing displacements.
   - Fits the slope `<r²>/(4t)` (2D self-diffusion). Reports D in Å²/fs and converts to m²/s for human-readable output.
   - Wired into `cargo run --release --bin measure_diffusivity -- <scenario.toml>`.

### Acceptance criteria

- `cargo run --release --bin measure_diffusivity -- measurement_configs/bulk_electrolyte.toml` produces a single number for D_Li⁺ in Å²/fs.
- The reported D is stable to within ~10% across two independent runs (different RNG seeds), confirming the equilibration and measurement windows are long enough.

---

## Phase 0b — Similitude Analysis & Regime Map

### Reference scales (table to fill in with sim-side values)

| Quantity | Real Li-metal cell (typical) | ParticleSim (defaults to verify in 0b) |
|---|---|---|
| Electrode separation L | 25–100 μm | ~100–300 Å |
| Li⁺ diffusivity D | ~10⁻¹⁰ m²/s | from Phase 0a |
| Diffusion time L²/D | ~10–100 s | ~10⁴–10⁵ fs |
| Diffusion crossover f_W ≈ 1/(2π·L²/D) | ~10–100 mHz | ~1–10 GHz (≈ 1e-6 to 1e-5 /fs) |
| Charge-transfer R_ct·C_dl | μs–ms | ps |
| Semicircle apex frequency | Hz–kHz | THz (~ 1e-3 /fs) |

### Dimensionless groups

For both systems:
- **Pe ≈ ωL²/D** — dimensionless transport frequency. Warburg-tail behavior at Pe < 1; bulk-resistive at Pe >> 1.
- **ωR_ct·C_dl** — dimensionless charge-transfer frequency. Semicircle apex at ωR_ct·C_dl ≈ 1.
- **L_DL / L** — Debye length to electrode separation. Not preserved by L-scaling — depends on particle density, charge, and the sim's hybrid 2D-with-3D-Coulomb force law. Relevant to the HF probe (Phase 2).
- **L_SEI / L** — SEI/dead-Li layer thickness to electrode separation. Not under similitude; whatever the sim produces. To be measured in Phase 4 morphology metrics.

### The 2D-with-3D-Coulomb caveat (must be in the doc)

ParticleSim is a 2D simulator (particles confined to a plane, `z=0`) but uses true 3D Coulomb forces (`k·q/r`, not 2D logarithmic). This is verified in `src/quadtree/quadtree.rs` line 374 and `src/simulation/simulation.rs` line ~1414. Implications for the similitude argument:

- **Screening behavior is hybrid.** It is neither the standard 2D Debye length (which would arise from logarithmic 2D Coulomb) nor the canonical 3D Debye length. Computing a "Debye length" in this simulator means measuring the actual screened-potential decay length empirically, not plugging into a textbook formula.
- **Active-area scaling under roughness is dimensionally different.** In a 3D experimental cell, surface roughness produces a 2D interface area; in this simulator, it produces a 1D interface curve. The mechanism predicted in the experiment ("rough surface → larger active area → lower R_ct") still applies qualitatively, but the *magnitude* of the active-area increase per unit roughness is not directly comparable.
- **Therefore we predict trends, not magnitudes.** The Phase 6 predicted-signature claims (HF |Z| ↓, LF |Z| ↑ with A_cond) should be read as monotonic trends matching the experiment, not as quantitative correspondence.

### Tasks

1. Add `docs/eis_similitude.md` containing:
   - The reference-scales table above with sim-side values populated from Phase 0a output and a representative measurement scenario.
   - A regime map plotting the simulator's accessible frequency range (1e-10 to 1 in 1/fs) annotated with the predicted Pe < 1 region (Warburg) and Pe ~ 1 crossover.
   - The 2D-with-3D-Coulomb caveat paragraph above (verbatim or close).
   - Explicit similitude claim: *the position of impedance features within each system's accessible frequency band should map under the dimensionless groups, even though absolute frequencies do not. The experimental "HF↓ / LF↑ with amplitude" signature should appear in the simulator as the higher-Pe impedance dropping and the lowest-Pe-reachable impedance rising with conditioning amplitude — but as a qualitative trend, not a quantitative magnitude.*
   - Caveats restated: Debye length not under similitude; SEI thickness ratio not under similitude; mechanism magnitude differs in 2D.
2. Link `docs/eis_similitude.md` from the README under the EIS section.

### Acceptance criteria

- `docs/eis_similitude.md` exists and is linked from the README.
- D_Li⁺ measured and recorded.
- Regime map produced as a PNG and committed.
- Measurement-band frequencies for downstream phases (5–6) explicitly span the predicted Pe ~ 1 crossover region within the sim, so both kinetic and transport-limited regimes are accessible.

---

## Phase 1 — Validate EIS on a Static Idealized Interface

Goal: demonstrate that small-signal EIS on a known-good configuration produces an interpretable, Randles-like Nyquist.

### Sub-task 1.0 — Scenario preset infrastructure

There is no scenario-preset loader today. `src/scenario.rs` only loads `init_config.toml`. `measurement_configs/` is parsed as `ManualMeasurement` configs, not scenarios. Phase 1 introduces this:

- Decide the schema. Recommended: extend `init_config.toml`'s shape with an optional top-level `[scenario_preset]` block carrying a name + parameters, registered against a small `ScenarioPreset` enum in `src/scenario.rs`. Reuses existing TOML parsing.
- Alternative: a separate loader keyed off `--preset <name>` CLI flag plus a directory of preset TOMLs. Heavier; only worth it if we expect many presets.

### Sub-task 1.1 — `EisValidationFlatSymmetric` preset

- Two flat Li-metal foil electrodes facing each other across a uniform electrolyte volume (EC/DMC mix, anion concentration matching salt molarity).
- No initial roughness, no dendrites, no dead Li.
- Domain size ≥ 200 Å in the inter-electrode direction, ≥ 100 Å lateral.
- Equilibration phase: run for `T_equilibrate_fs` (default 50,000 fs) with thermostat enabled before EIS engages.

### Sub-task 1.2 — Integration test

Introduce `tests/eis_validation_runs.rs` (this introduces the top-level `tests/` convention; existing `#[cfg(feature = "unit_tests")]` in-module tests remain the pattern for unit tests). Loads the validation scenario and runs a short EIS sweep headlessly, asserting:
- The sweep completes without panics.
- All `EisPoint`s have `min(fit_r2_v, fit_r2_i) > 0.95`. (Tightened from 0.8 — 0.95 catches bad probe placement / amplitude-too-small / settle-too-short far more reliably than the looser threshold; clean small-signal lock-in fits hit 0.99 routinely.)
- The high-frequency intercept on the real axis (Z_imag near 0) is positive and finite.

### Sub-task 1.3 — Manual full sweep

Run a full sweep manually (10 points/decade across the full reachable range, potentiostatic mode, small amplitude e.g. 1e-3 in Δratio units). Save the resulting `study_baseline_nyquist.csv` and a generated PNG plot to `doe_results/eis_validation/`.

### Acceptance criteria

- Test passes.
- Manual Nyquist shows: positive HF real-axis intercept, at least one identifiable semicircle (or arc), and a low-frequency tail (positive-slope tail or curl). Document any anomalies inline in `doe_results/eis_validation/README.md`.
- If no recognizable Randles features appear, **stop and triage** — likely culprits: voltage probe (Phase 2), domain size, settle/recording periods, or amplitude too large/small.

---

## Phase 2 — Tighten the Voltage Probe

The current probe (`compute_eis_voltage_by_potential` in `src/simulation/simulation.rs:1366`) computes the Coulomb potential at each foil group's centroid, summing over all bodies *except* the probe group's own particles, with a 5 Å softcore minimum (`R_MIN`).

**Concern:** 5 Å is comparable to or larger than Stern + diffuse double-layer thickness. This systematically discards the densest charge-accumulation contribution — the very thing that produces double-layer / charge-transfer impedance. Likely effect: blunted or missing HF semicircle.

### Decisions already made

- **Default probe will be the bulk-reference-plane variant.** The legacy centroid-with-softcore probe stays in the codebase as a selectable diagnostic for comparison, but is no longer the default.
- **Probe planes are placed at fixed lab-frame offsets** from each foil's *initial* position, not tracking the evolving interface. Tracking the moving frontier would alias morphology change into spurious impedance change. Trade-off: as the interface advances toward the probe, the local potential reading is contaminated by accumulated Li metal — handled by placing the planes far enough from the initial foil that no realistic growth excursion will reach them, plus a runtime sanity check.
- **DC drift in the probe is automatically handled.** `ls_fit_re_im` (`eis.rs:571`) fits a constant DC term `+ C` alongside the AC components. Any slow drift in bulk space charge or interface position contributes only to C, not to A,B. So even though the probe-plane potential reading drifts as morphology evolves, the impedance extraction is clean.

### Tasks

1. Promote `R_MIN` to a configurable field on `SimConfig`: `eis_voltage_probe_softcore_angstroms: f32` (default 5.0). Keeps the legacy probe usable.
2. Add `compute_eis_voltage_via_reference_planes`:
   - Probe planes parallel to each foil at a configurable bulk offset from the foil's *initial* lab-frame position (default 30 Å into electrolyte). Offset stored in `SimConfig.eis_probe_plane_offset_angstroms`.
   - Compute the mean Coulomb potential over a uniform grid of probe points on each plane (default 10×10 grid spanning the foil's lateral extent). Probe points are massless evaluation points only.
   - Sum over **all bodies except the foil group being probed** (preserves the existing exclusion semantics from `compute_eis_voltage_by_potential`).
   - V = ⟨φ_groupA_plane⟩ − ⟨φ_groupB_plane⟩.
   - At sweep start, log the minimum distance from the probe plane to any LithiumMetal particle. If during the sweep this distance falls below a safety threshold (default 10 Å), emit a warning that probe contamination is likely.
3. Add `EisVoltageProbeKind { BulkReferencePlanes, CentroidWithSoftcore }` (with `BulkReferencePlanes` as the default `Default` impl). Expose via a GUI dropdown in the EIS tab.
4. Re-run Phase 1 validation with the new default and confirm Nyquist quality. Save both probe variants' Nyquists side-by-side at `doe_results/eis_validation/probe_comparison.md`.

### Acceptance criteria

- Bulk-reference-plane probe implemented and is the default.
- Legacy centroid probe still selectable.
- Phase 1 validation Nyquist looks better (clearer semicircle, less HF noise) with the new probe than with the old.
- Probe-contamination warning fires correctly when artificially triggered (e.g. by placing probe plane very close to foil in a test scenario).

---

## Phase 3 — Linearity & Harmonic Diagnostics

The lock-in's small-signal premise breaks at amplitudes large enough to drive interface evolution. We need to detect this so we know what amplitudes are safe for measurement vs. which are conditioning-only.

### Tasks

1. Generalize `ls_fit_re_im` (or add a sibling fn) to fit harmonics: `signal(t) = A₀ + Σ_{k=1..K} [Aₖ cos(kωt) + Bₖ sin(kωt)]`. Default K=3.
2. Add fields to `EisPoint`:
   - `harmonic_2_magnitude: f64`
   - `harmonic_3_magnitude: f64`
   - `thd: f64` — `sqrt(H₂² + H₃²) / H₁`
3. Display THD per frequency point in the EIS tab. Two thresholds:
   - **Yellow at THD > 1%** — onset of nonlinearity. Above this, the small-signal premise weakens; not safe for impedance measurement.
   - **Red at THD > 5%** — strong harmonic generation. Useful as a confirmation that we're in the regime that drives morphology evolution.
4. Export THD in the existing CSV.

### Acceptance criteria

- At small amplitude on the validation scenario, all THD values < 1%.
- At deliberately large amplitude (e.g. 100× the small-signal value), THD > 5% for at least some frequencies — confirms detector is working.

---

## Phase 4 — Morphology Metrics + Conditioning-Duration Calibration

### Sub-task 4.1 — Morphology metrics

Add `src/simulation/morphology.rs` with `MorphologyMetrics`:

- `interface_arc_length_per_unit_lateral: f32` — arc length of the Li-metal / electrolyte boundary normalized by lateral domain extent. Flat = 1.0; rough = >> 1.0. Implement via marching squares on a binary Li-metal occupancy field at chosen grid resolution (5 Å suggested; Li radius ~1.5 Å, so features <5 Å are not resolved — acceptable for this experiment).
- `interface_roughness_rms_angstroms: f32` — RMS deviation of the Li-metal frontier (per-column highest LithiumMetal y-coordinate, or x for vertical foils) from its mean.
- `dead_li_fraction: f32` — fraction of LithiumMetal particles disconnected from the percolating cluster touching the foil. Connected-component analysis on a particle-proximity graph; cutoff radius `2.5 × Li_metal_radius` (derived from species data, not hand-waved). Use existing `cell_list.rs:57` `find_neighbors_within` rather than rebuilding adjacency.
- `accessible_surface_atoms: u32` — count of LithiumMetal particles within one neighbor radius of an electrolyte-species particle (proxy for active surface area).

`compute_morphology_metrics(&Simulation) -> MorphologyMetrics` cheap enough to call once per N frames (default N = 1000) without significant slowdown.

### Sub-task 4.2 — CSV + GUI

- Extend the existing measurement CSV system with a `morphology` toggle producing rows `frame,time_fs,arc_length_norm,roughness_rms,dead_li_frac,accessible_atoms`. Aggregated study outputs land in `doe_results/<study_name>/morphology.csv`; raw timeseries can land alongside for ad-hoc plotting.
- Display live values in a new section of the EIS tab.

### Sub-task 4.3 — Conditioning-duration calibration

Run a single calibration shot to derive Phase 5's default conditioning duration:

- Pick one representative conditioning waveform + amplitude (will be the largest amplitude planned for the actual study, since saturation timescale is shortest there but other amplitudes need to run *at least* this duration).
- Drive that conditioning + log morphology metrics every 1000 steps.
- Plot morphology metric (recommend `interface_roughness_rms_angstroms` or `accessible_surface_atoms`) vs. t.
- Define `conditioning_duration_fs` as the time to reach 80% of the saturation value, or first plateau if that's clearer. Document the chosen metric and threshold.
- Output: `doe_results/calibration/conditioning_duration.csv` + a PNG of the saturation curve + a one-paragraph note in the study_config defaults.

**Why the largest-amplitude case sets the duration:** lower amplitudes evolve more slowly. Using the largest-amplitude saturation time as the study duration means lower-amplitude runs may not have fully saturated; that's an acceptable part of the comparison, since a non-saturated low-amplitude run still tells us "morphology has barely changed" which is the expected and informative result.

### Acceptance criteria

- On the flat validation scenario, `arc_length_norm ≈ 1.0` and `roughness_rms < 5 Å`.
- After a deliberate high-amplitude conditioning run, both rise visibly.
- Sub-task 4.3 produces a saturation plot and a chosen `conditioning_duration_fs` default, recorded in `doe_results/calibration/conditioning_duration.csv` and in the study config schema as the default.

---

## Phase 5 — Conditioning + Measurement Protocol (`EisAmplitudeStudy`)

This is the core experiment. Architecturally it is an outer state machine that orchestrates conditioning, equilibration, and measurement EIS sweeps.

### Phase 5 prerequisite — RNG state serialization

The amplitude study reloads `initial_state_path` between conditioning amplitudes so that each amplitude starts from identical initial conditions. Today, `src/io.rs:12` `SimulationState` does **not** serialize RNG state; the stochastic Butler-Volmer acceptance and thermostat noise restart from a process-default seed on load, so identical-seeming initial conditions still diverge non-deterministically.

Before Phase 5 implementation begins, extend `SimulationState` to serialize and restore the RNG state(s) used by:
- Butler-Volmer acceptance (`fastrand` calls in electron hopping)
- Thermostat noise (Maxwell-Boltzmann rescaling)
- Any other stochastic kernel used in the per-step pipeline

Acceptance for the prerequisite: a save → load → run-N-steps cycle on a deterministic harness produces bit-equivalent body positions/velocities to a no-save run-N-steps from the same starting state. (Note: due to Rayon parallel-reduction order non-determinism, "bit-equivalent" may not be achievable for all kernels; in that case document which kernels are deterministic-with-RNG and which require seed-replicated-but-not-bit-identical comparisons.)

### Tasks

1. Add `src/simulation/eis_amplitude_study.rs` with:

   ```rust
   pub enum ConditioningWaveform {
       /// Pure sinusoid at conditioning_frequency. Reference / comparison case.
       /// Zero time-average: comparable to AC-only excitation.
       Sinusoidal,
       /// Symmetric square wave at conditioning_frequency.
       /// Best analog for symmetric plate-strip current-density cycling. Default.
       SquareWave,
       /// Asymmetric pulse train. Plate phase and strip phase have independent
       /// duration and magnitude — captures asymmetric C-rate experiments and
       /// is the closest analog for one-sided plating studies.
       AsymmetricPulse {
           plate_duration_fs: f32,
           strip_duration_fs: f32,
           plate_magnitude: f32,
           strip_magnitude: f32,
       },
   }

   pub struct EisAmplitudeStudyConfig {
       pub conditioning_amplitudes: Vec<f32>,    // in mode-dependent units (e/fs or Δratio)
       pub conditioning_waveform: ConditioningWaveform,
       pub conditioning_frequency: f32,          // 1/fs (used by Sinusoidal and SquareWave)
       pub conditioning_duration_fs: f32,        // default from Phase 4 calibration
       pub equilibration_duration_fs: f32,
       pub measurement_amplitude: f32,           // small-signal, set from Phase 3 results (THD < 1%)
       pub measurement_frequencies: Vec<f32>,    // log-spaced; spans Pe ~ 1 region per Phase 0b
       pub measurement_periods_per_freq: usize,
       pub measurement_settle_periods: usize,
       pub mode: EisMode,                        // galvanostatic or potentiostatic
       pub initial_state_path: PathBuf,
       pub rng_seed: u64,
       pub disable_switch_charging: bool,        // default true; conditioning is the only driver
   }
   ```

2. Add `EisAmplitudeStudyState` outer state machine with phases:
   - `LoadInitial` — load saved sim state from `initial_state_path` (RNG state restored per Phase 5 prerequisite). Disables switch-charging if configured.
   - `Conditioning(amp, t_remaining)` — drive the configured `conditioning_waveform` at amplitude `amp`. Switch-charging stays disabled.
   - `Equilibrating(t_remaining)` — turn off conditioning, let system relax. Monitor a fast-decay metric (kinetic energy of mobile species, or integrated current after shutoff) for sanity.
   - `Measuring(EisState)` — run inner small-signal sweep at `measurement_amplitude` and `measurement_frequencies`.
   - `LogResults` — write morphology + spectrum to CSVs.
   - `NextAmplitude` or `Done`.

3. Output files in `doe_results/<study_name>/`:
   - `study_summary.csv`: one row per conditioning amplitude with morphology metrics + key spectral features (HF |Z|, LF |Z|, semicircle diameter if fittable). Header includes `# units: amplitude=<e/fs|delta_ratio>` based on `mode`.
   - `study_spectra.csv`: long-format `(amplitude, frequency, z_real, z_imag, fit_r2_v, fit_r2_i, thd, harmonic_2_mag, harmonic_3_mag)`.
   - `study_morphology.csv`: long-format morphology timeseries per amplitude.
   - `study_config.toml`: copy of the config for reproducibility, plus a `# units:` comment annotating amplitude semantics.
   - `state_after_amp_<N>.bin`: saved sim state after each conditioning step (optional, controlled by flag).
   - **Raw per-frequency lock-in inputs continue to land in `eis_timeseries/eis_ts_*.csv`** as today; the study aggregates from those into the files above.

4. Add an "EIS Amplitude Study" sub-section to the EIS tab with:
   - Conditioning amplitude list editor.
   - Conditioning waveform selector with parameter editing for the asymmetric case.
   - Single measurement amplitude (with hint pointing to Phase 3 THD output).
   - Conditioning/equilibration durations (default from Phase 4 calibration).
   - "Run Study" button + progress indicator showing current amplitude and current sub-phase.
   - "Stop" / "Pause" controls.

### Acceptance criteria

- A study with 3 conditioning amplitudes (small, medium, large) completes end-to-end without manual intervention.
- All output CSVs and saved states are present and well-formed; CSV headers stamp the units string.
- The morphology metrics for the largest conditioning amplitude show clear divergence from the flat baseline.
- Switch-charging confirmed disabled throughout (post-run check on the saved state).

---

## Phase 6 — Analysis Tooling

### Tasks

1. Add `scripts/analyze_eis_amplitude_study.py`. Inputs: a study directory. Outputs (PNG to study dir):
   - `nyquist_overlay.png` — Nyquist plots for each conditioning amplitude on shared axes.
   - `bode_overlay.png` — |Z| and phase vs f, log-log.
   - `hf_impedance_vs_amplitude.png` — |Z(f_max)| vs A_cond.
   - `lf_impedance_vs_amplitude.png` — |Z(f_min)| vs A_cond.
   - `morphology_vs_amplitude.png` — roughness, dead-Li fraction, accessible atoms vs A_cond.
   - `mechanism_check.png` — 2-panel: HF impedance vs accessible_surface_atoms, LF impedance vs roughness_rms. Linear/log fits annotated.

2. Document the predicted signature in the script header:

   > **Predicted signature (corroborates the rough/mossy mechanism, qualitatively — magnitudes are not directly comparable to the experiment because of the 2D-with-3D-Coulomb similitude caveat in `docs/eis_similitude.md`):**
   > - HF |Z| decreases monotonically with A_cond.
   > - LF |Z| increases monotonically with A_cond.
   > - Roughness, dead-Li fraction, and accessible-atom count all increase with A_cond.
   > - HF impedance correlates negatively with accessible_surface_atoms.
   > - LF impedance correlates positively with roughness_rms.

### Acceptance criteria

- Script runs end-to-end on a completed Phase 5 output.
- All PNGs generated and visually informative.

---

## Phase 7 — Reproducibility & Provenance

(Most of the heavy lifting — RNG state serialization — is already a Phase 5 prerequisite. This phase finishes the provenance plumbing.)

### Tasks

1. Stamp every output CSV header with: study name, git commit SHA, full `study_config.toml` embedded as comment lines.
2. Add git SHA stamping to `build.rs`. Today `build.rs` only watches Quarkstrom; extend it to capture `git rev-parse HEAD` and emit it as a `cargo:rustc-env=GIT_SHA=<sha>` directive so the compiled binary can read `env!("GIT_SHA")` at runtime. Use the `vergen` crate or a one-line `Command::new("git").args(["rev-parse", "HEAD"])` invocation.
3. Add `scripts/rerun_study.sh` that takes a study directory and reproduces it from `study_config.toml`.

### Acceptance criteria

- A study rerun from the same seed and same git SHA produces statistically reproducible Z(ω) and morphology metrics. (Bit-identity is *not* claimed — Rayon parallel-reduction order means floating-point order varies. What is claimed: same seed → same stochastic kernel decisions → trajectories diverge only in floating-point reduction noise, not in reaction events.)
- Git SHA appears in every output CSV header.

---

## Out of Scope (explicitly)

- Quantitative match to experimental Z(ω) magnitudes — see Phase 0b's 2D-with-3D-Coulomb caveat.
- Multi-electrode 3D extension — current 2D Barnes-Hut treatment is what we have.
- SEI chemistry beyond what the existing solvent / anion species set supports.
- Full implicit-electrolyte continuum coupling.
- Bit-identical reproducibility (see Phase 7).

---

## Open Questions to Resolve During Implementation

1. **Conditioning frequency choice (for Sinusoidal and SquareWave).** Should it be inside or outside the measurement band? Suggest: pick a single conditioning frequency *below* the measurement f_min so the conditioning AC does not contaminate the measurement bandwidth via residual transients during equilibration. Express this choice in dimensionless terms (Pe of the conditioning frequency) so it is comparable across configurations.

2. **Equilibration sufficiency.** How long is enough for transients to die after conditioning shutoff? Suggest: monitor a fast-decay metric (mean kinetic energy of mobile species, or integrated current after AC shutoff) and proceed when it falls below threshold, but cap with `equilibration_duration_fs` so a stuck system doesn't hang the study.

3. **Debye length in the simulator.** Compute it empirically (decay length of a screened test-charge potential) from current default config and report alongside the Phase 0b dimensionless analysis. If it is comparable to or smaller than the bulk-probe-plane offset (Phase 2 default 30 Å), the new probe is well-placed for HF resolution. If it is much larger, reconsider the offset.

4. **AsymmetricPulse parameter defaults.** Once the SquareWave and Sinusoidal cases produce a baseline, is asymmetric pulsing worth running as a third comparison? Probably yes for a one-sided plating analog, but not until SquareWave is characterized.

---

## Suggested Implementation Order

1. **Phase 0a** (bulk scenario + diffusivity binary; ~half a day)
2. **Phase 0b** (similitude doc + regime map; ~hours, doc-mostly)
3. **Phase 1** (validation scenario + scenario preset infra + first Randles; this is the heaviest validation lift; everything depends on it)
4. **Phase 2** (fix likely root cause of HF blunting before measurement)
5. **Phase 3** (cheap, decoupled — can run partially in parallel with Phase 4)
6. **Phase 4** including the calibration sub-task 4.3 (independent of Phases 2–3 conceptually but should land after them so the calibration runs on a trusted probe + linearity baseline)
7. **Phase 5 prerequisite** (RNG state serialization)
8. **Phase 5** (the study state machine)
9. **Phase 6** (analysis script)
10. **Phase 7** (final pass; git SHA stamping can be threaded earlier if convenient)

---

*Spec version 1.1 — handoff document. Per-phase implementation rhythm: I outline the phase, you sign off, I implement + cargo build/test, I send a checkpoint with artifacts, you sign off before next phase. Visual GUI verification (tab layout, Nyquist rendering, color coding) is yours; I can't see the running app.*
