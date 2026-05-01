# EIS Similitude Analysis — ParticleSim ↔ Experimental Li-metal/NMC811 Cell

**Status:** Draft v1. Phase 0b of `docs/EIS_AMPLITUDE_STUDY_PLAN.md`. Updated when measured values change.

**Purpose:** establish the dimensionless mapping between the simulator and the experimental cell that motivated this work, so impedance features in the simulator can be interpreted against (and compared to) the experimental observation that increasing AC excitation amplitude causes HF |Z| ↓ and LF |Z| ↑.

**Top-line claim:** the *position of impedance features within each system's accessible frequency band* should map under the dimensionless groups Pe = ωL²/D and ωRC, even though absolute frequencies differ by ~10 orders of magnitude. The mapping supports **qualitative trend comparisons** between simulator and experiment, not quantitative magnitude correspondence — see the caveats section for why.

---

## Reference scales

### Real cell (Honda-RI switching cell)

| Quantity | Symbol | Value | Source |
|---|---|---|---|
| Cell chemistry | — | Li metal ‖ NMC811 in 1 M LiPF6 / EC:EMC:DMC + 3% VC | user |
| Operating temperature | T | ~295 K (22 °C) | user |
| Inter-electrode separation | L | 20 μm (separator thickness) | user |
| Active electrode area | A | 53 cm² | user |
| Bulk Li⁺ diffusivity in electrolyte | D_Li⁺ | ≈ 3 × 10⁻¹⁰ m²/s | literature: typical 1 M LiPF6 / ternary carbonate at 25 °C; VC additive perturbs <10% |
| Diffusion time L²/D | τ_diff | 1.3 s | computed |
| Diffusion crossover frequency 1/(2π·L²/D) | f_W | 0.12 Hz | computed |
| Solution / electrolyte resistance | R_s | 0.05 Ω = 2.65 Ω·cm² | from Nyquist HF intercept |
| SEI/film resistance (HF arc) | R_SEI | ≈ 0.05 Ω = 2.65 Ω·cm² | first arc diameter, area-normalized |
| Charge-transfer resistance (mid arc) | R_ct | ≈ 0.10 Ω = 5.3 Ω·cm² | second arc diameter, area-normalized |
| Double-layer capacitance | C_dl | ≈ 14 mF = 273 μF/cm² | from ω_apex(R_ct) ≈ 691 rad/s |
| Film capacitance | C_SEI | ≈ 3 mF = 57 μF/cm² | from ω_apex(R_SEI) ≈ 6.3 × 10³ rad/s |
| HF semicircle apex | f_apex,SEI | ≈ 1 kHz | user |
| Mid arc apex | f_apex,ct | ≈ 110 Hz | user |
| Debye length in 1 M organic carbonate | λ_D | ≈ 0.4–0.5 nm | textbook (ε_r ≈ 30, 1 M ionic strength) |
| SEI thickness range | L_SEI | ~5 nm initial → ~10–20 nm after 3× 0.02 C cycles | typical Li-metal SEI growth |
| EIS measurement band | f_min, f_max | 0.1 Hz, 1 MHz | user |
| EIS amplitude range (potentiostatic) | ΔV | 10–110 mV | user |
| Conditioning protocol between sweeps | — | 0.02 C × 3 cycles | user |

### Simulator (`measurement_configs/bulk_electrolyte.toml` for D; planned EIS scenario for the rest)

| Quantity | Symbol | Value | Source |
|---|---|---|---|
| Operating temperature target | T | 295 K | `config.rs` thermostat target |
| Inter-electrode separation (planned EIS scenario) | L | 250 Å (= 2.5 × 10⁻⁸ m) | Phase 1 validation scenario default |
| Bulk Li⁺ diffusivity | D_Li⁺ | (6.3 ± 0.5) × 10⁻² Å²/fs = (6.3 ± 0.5) × 10⁻⁷ m²/s | Phase 0a, 200 ks runs at thermally-stable seeds (cluster of 6 of 8 seeds; T held within ~10 K of 295 K) — see `images/bulk_electrolyte_diffusivity/msd_bulk_electrolyte_all_runs.png` |
| Diffusion time L²/D | τ_diff | ≈ 9.9 × 10⁵ fs ≈ 1 ns | computed |
| Diffusion crossover frequency 1/(2π·L²/D) | f_W | ≈ 1.6 × 10⁻⁷ /fs ≈ 160 MHz | computed |
| Charge-transfer resistance (sim) | R_ct,sim | TBD — measured in Phase 1 from validation Nyquist | |
| Double-layer capacitance (sim) | C_dl,sim | TBD — Phase 1 | |
| Debye length (sim) | λ_D,sim | TBD — measured empirically (decay length of screened test-charge potential), per Phase 0b open question 3 | |
| Accessible frequency band (sim) | f_min,sim, f_max,sim | ~10⁻¹⁰ to 1 /fs ≈ 100 kHz to 10¹⁵ Hz | sim time-step / total-run constraints |
| Recommended Phase 5 measurement band | | 10⁻⁹ to 10⁻⁵ /fs ≈ 1 MHz to 10 GHz | spans Pe ≈ 10⁻² to 10² centered on the simulator's f_W |

The "experimental D_Li⁺ ≈ 3 × 10⁻¹⁰ m²/s" and "simulator D_Li⁺ ≈ 6.3 × 10⁻⁷ m²/s" correspond to a ratio of ≈ 2,100× — i.e., the simulator's transport is ~2,000× faster than the real cell, because the simulator is in compressed time/length units. **This ratio is not a free parameter we can tune;** it is what falls out of the simulator's mass/charge/temperature units. We adapt by choosing measurement-band frequencies that put us at comparable Pe in each system, not by trying to match Hz.

---

## Dimensionless groups

### Pe = ω · L² / D — diffusion-time vs perturbation-time

Tells us whether the system has time to diffuse during one perturbation period.
- **Pe ≪ 1:** Warburg / diffusion-limited regime. Concentration profile follows the perturbation.
- **Pe ≈ 1:** crossover. Most informative regime for transport-property extraction.
- **Pe ≫ 1:** bulk-resistive regime. Concentration is effectively frozen on the perturbation timescale.

| Pe range | Real cell freq | Simulator freq |
|---|---|---|
| Pe = 0.01 (deep Warburg) | ~1 mHz | ~1.6 MHz (~6 × 10⁻⁹ /fs) |
| Pe = 1 (crossover) | ~0.12 Hz | ~160 MHz (~1.6 × 10⁻⁷ /fs) |
| Pe = 100 (bulk-resistive) | ~12 Hz | ~16 GHz (~1.6 × 10⁻⁵ /fs) |

The experimental EIS band (0.1 Hz – 1 MHz, 7 decades) sits *above* Pe ≈ 1 — i.e., the experiment barely touches the Warburg regime at its lowest frequency, and is otherwise in the bulk-resistive regime. To cover the same Pe range, the simulator's measurement band should run from ~1 MHz to ~10 GHz, centered near 160 MHz.

### ω · R · C — kinetic-relaxation timescale

Each arc in the Nyquist has its own R·C time constant; the arc apex sits at ω = 1/(RC).

| Process | Real cell | Simulator (TBD post-Phase 1) |
|---|---|---|
| SEI / film | R_SEI·C_SEI ≈ 1.6 × 10⁻⁴ s → f_apex ≈ 1 kHz | — |
| Charge transfer | R_ct·C_dl ≈ 1.4 × 10⁻³ s → f_apex ≈ 110 Hz | — |

The simulator does not yet have an SEI/film + charge-transfer separation visible in a Nyquist (no validation run done; that's Phase 1). After Phase 1, we should be able to populate this row and check the simulator's mapping at Pe·ωRC space.

### λ_D / L — screening vs geometry

| Quantity | Real cell | Simulator |
|---|---|---|
| L_DL / L | ~ 5 × 10⁻⁵ (0.5 nm / 20 μm) | TBD — depends on simulator's empirical Debye length |
| L_SEI / L | ~ 5 × 10⁻⁴ (10 nm / 20 μm) | TBD — depends on whether/where SEI forms in the sim |

These ratios are **not under similitude control.** L_DL depends on particle density, charge, and the simulator's hybrid 2D-with-3D-Coulomb force law (see caveats); L_SEI depends on whatever grows from SEI formation kinetics in the sim. Both ratios will be measured during Phase 1/2 and reported; matching the real-cell values is not expected.

---

## Caveats — why this is qualitative, not quantitative

These are listed in order of how strongly they limit quantitative comparison.

### 1. Solvation-shell-mediated transport, not free liquid

The simulator at 1 M LiPF6 / EC:DMC visibly forms a structured local arrangement of solvent molecules around Li⁺ ions and around each other. This is **the simulator's representation of the solvation shell** of the solvated salt at 1 M concentration — it is not a phase artifact and not a glass. Transport in this regime is dominated by *cage-hopping* of solvated Li⁺ between solvent shells, with a characteristic cage-residence time visible in the MSD as a sub-diffusive caged regime followed by Fickian linear behavior.

**Implication:** Li⁺ diffusivity in the simulator measures *solvation-shell-mediated transport*, which is mechanistically the dominant transport mode in real concentrated electrolytes too — but the *rate* depends on the simulator's parameters (LJ ε, σ, polar coupling strengths). The 2,000× faster diffusivity vs the real cell partly reflects compressed scales and partly reflects parameters that haven't been tuned for transport-rate fidelity.

### 2. 2D simulator with 3D Coulomb forces

ParticleSim is 2D (particles confined to a plane, z = 0) but uses true 3D Coulomb forces (`k·q/r`, not 2D logarithmic — verified at `quadtree.rs:374`, `simulation.rs:~1414`). This is a hybrid that is neither a 2D log-Coulomb gas nor 3D bulk:

- **Screening behavior is hybrid.** "Debye length" must be measured empirically from the decay of a screened test-charge potential rather than computed from a textbook formula. Open question for Phase 1/2.
- **Active-area scaling under roughness is dimensionally different.** Real surface roughness produces a 2D interface area; in the simulator, it produces a 1D interface curve. The "rough surface → larger active area → lower R_ct" mechanism still applies *qualitatively*, but the *magnitude* of the active-area increase per unit roughness differs.

### 3. Thermostat does not hold isothermal conditions across seeds

Phase 0a found that the thermostat's effective steady-state T_liquid varies by seed: across 8 seeds, final T ranged from 222 K to 297 K against a 295 K target. D_Li⁺ varies ~4× as a result. The user hypothesizes this is downstream of weak short-range repulsion in the force model leading to void/cluster formation; that investigation is deferred to `docs/THERMOSTAT_AND_VOID_INVESTIGATION.md`.

**Implication for similitude:** when comparing simulator runs to each other (e.g., across conditioning amplitudes in Phase 5), thermal-state variability is a confound on the order of 4× in D and likely a similar factor in EIS impedance magnitudes. Two mitigations:
1. **Pre-screen seeds**: only accept runs where final T_liquid is within ±5 K of target (this rejects pathological cold-crashed runs).
2. **Replicate amplitudes across multiple seeds** and average impedance results (per the spec's Phase 5 design).

### 4. Boundary conditions: clamping, not periodic

When particles reach a domain edge they are clamped, not wrapped. This produces a wall-clamping artifact in MSD measurements (occasional `<r²>` decreases when many tracked particles hit walls and the COM drift continues for the rest). Mitigated for Phase 0a by using a bulk-window restriction. For EIS, the relevant bulk-window equivalent is keeping the measurement domain large enough that the diffuse double layer doesn't reach the lateral walls.

---

## Implications for the EIS amplitude study (`docs/EIS_AMPLITUDE_STUDY_PLAN.md`)

1. **Measurement-band choice for Phase 5** should span Pe ≈ 10⁻² to 10² in *simulator units*, i.e., approximately 10⁻⁹ to 10⁻⁵ /fs. Using log-spaced points, that's 4 decades — comparable in dimensionless terms to the experimental 7-decade band but compressed because we don't need to extend as far from the Pe ≈ 1 crossover to capture both regimes.

2. **Predicted-signature claims (Phase 6)** should be framed as monotonic trends matching the experiment (HF |Z| ↓, LF |Z| ↑ with conditioning amplitude), explicitly *not* as quantitative correspondence in Ω·cm² values.

3. **Same-amplitude vs separated-amplitude protocol.** The user's experimental protocol uses a single amplitude per EIS sweep — the sweep both drives morphology change and measures impedance. The spec's `EisAmplitudeStudy` design separates these into a high-amplitude *conditioning* phase followed by a small-signal *measurement* phase, which is more general. Both should be runnable:
   - **Single-amplitude (experimental analog):** set `conditioning_amplitude == measurement_amplitude` and skip the separate conditioning phase. This replicates the experiment exactly.
   - **Separated-amplitude (spec design):** as currently specified. Lets us disentangle "drive" from "measure" and check whether the HF↓/LF↑ signature comes from morphology evolution alone (visible in separated runs) or only manifests under simultaneous drive+measure.
   We should ensure Phase 5's `EisAmplitudeStudyConfig` exposes both modes.

4. **Conditioning current density in the experiment is 0.02 C** — i.e., the cell is cycled at C/50 between EIS measurements. In the simulator, this maps to a galvanostatic conditioning current scaled by area and active-material content. Numerical mapping between sim and experimental conditioning currents will be added when Phase 5 is implemented.

---

## Open questions

1. **Empirical Debye length in the simulator.** Run a screened-test-charge experiment in the bulk-electrolyte scenario: place a small test charge at the center, wait for equilibration, sample the radial potential, fit `φ(r) ∝ exp(-r/λ_D) / r` to the decay. Until measured, L_DL/L is unknown.

2. **Cage-residence time.** From the MSD plot, the cage-hopping crossover sits around 5–20 ks of measurement time. Express this as a dimensionless residence time τ_cage·D/L² and compare to the experimental cell's solvent-shell exchange time. May explain why the simulator's "Warburg" features in EIS appear at different relative frequencies than expected.

3. **Whether the simulator's R_ct, C_dl reproduce the double-arc structure** of the experimental Nyquist. Phase 1 produces the first simulator Nyquist; if it shows a single arc rather than two, the SEI/film + charge-transfer separation either isn't happening in the simulator (no SEI yet?) or the time constants are too close to resolve.

---

## Regime map

See `images/eis_similitude/regime_map.png` (generated by `scripts/plot_regime_map.py`). Two horizontal log-frequency bars — one for the experimental cell, one for the simulator — annotated with each system's Pe = 1 line, EIS measurement band, and arc-apex frequencies where known.

---

## References (placeholders to fill in if you have specific values)

- D_Li⁺ in LiPF6 carbonate electrolytes: Logan & Newman; Zugmann et al., *J. Electrochem. Soc.* 158 (2011); Stewart & Newman, *J. Electrochem. Soc.* 155 (2008).
- Li-metal SEI thickness: Aurbach et al.; Peled & Menkin, *J. Electrochem. Soc.* 164 (2017).
- Butler-Volmer kinetics in batteries: Newman & Thomas-Alyea, *Electrochemical Systems* (2004).
