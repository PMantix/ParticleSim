# Physics Parameter Inventory — ParticleSim

**Version 1.0** — drafted 2026-05-05 on `feature/eis-amplitude-study`. Prerequisite for `docs/PHYSICS_VALIDATION_PLAN.md`.

## Purpose

A complete catalog of every tunable physics knob in the simulator: current value, defensible target (where one exists), cross-dependencies, and known concerns. Does **not** propose changes — see `PHYSICS_VALIDATION_PLAN.md` for the testing roadmap that uses this inventory as input.

> **Strict rule:** do not edit any value listed here without explicit per-change agreement from the user. The current parameter set is a delicate calibration; sensitivity sweeps will reveal range-of-tolerance, but retuning is a separate decision. See memory `feedback_no_unauthorized_retuning.md`.

## Usage notes

- File:line references reflect the codebase at commit `112b314` on `feature/eis-amplitude-study`. They will drift; re-verify before acting on a specific line.
- Where a knob exists in **both** a global `pub const`/default function (e.g. `DEFAULT_X`) **and** a `SimConfig` field, the `SimConfig` field is the runtime-effective value; the const is only the default seed.
- "Per-species" means the value lives in `src/species.rs::SPECIES_PROPERTIES` and is read by species-keyed lookups.
- **Smoking guns** (per memory `project_physics_smoking_guns.md`) are flagged inline with ⚠️ and consolidated in §17.

---

## 1. Time integration

| Knob | File:line | Default | Units | Notes |
|---|---|---|---|---|
| `DEFAULT_DT_FS` | `src/config.rs:192` | 5.0 | fs | Comment in config admits this is ~10× a typical MD timestep. Old value 0.015 fs was "too small". |
| `SimConfig.dt` | `src/config.rs` (Default impl) | seeded from `DEFAULT_DT_FS` | fs | Runtime-effective. |
| `COLLISION_PASSES` | `src/config.rs:194` (runtime mutex) | 7 | count | Iterative collision-resolution passes per step. ⚠️ Multiplied into the LJ force clamp — see §17. |

**Cross-deps:** every kinetic rate law (`p = 1 - exp(-k·dt)` in electron hopping; `prob = base·dt` in SEI; `prob = base·dt` in intercalation) couples to dt. Halving dt should halve event probabilities and leave the steady-state physics unchanged — that is a Phase 1 invariant.

## 2. Electrostatics

| Knob | File:line | Default | Units | Notes |
|---|---|---|---|---|
| `COULOMB_CONSTANT` | `src/units.rs:26` | 8.988e9 (SI converted) | AMU·Å³/(fs²·e²) | Computed from SI; dimensional analysis verified. |
| `SimConfig.coulomb_constant` | `src/config.rs:334` | seeded from above | same | GUI override. |
| `QUADTREE_EPSILON` | `src/config.rs:204` | 2.0 | Å | Coulomb softening; `e_sq = ε²` added to `r²` in Barnes-Hut force eval. |
| `QUADTREE_THETA` | `src/config.rs:203` | 1.0 | dimensionless | Barnes-Hut opening criterion. |

**Cross-deps:** softening epsilon and Barnes-Hut θ together set the upper bound on force error vs brute-force. Phase 1 has a quadtree-force-error invariant that will pin this.

## 3. Lennard-Jones & soft-core repulsion ⚠️ **(smoking gun)**

### Global LJ

| Knob | File:line | Default | Units | Notes |
|---|---|---|---|---|
| `LJ_EPSILON_EV` | `src/config.rs:104` | 0.0103 | eV | Well depth, single global value. |
| `LJ_SIGMA_A` | `src/config.rs:106` | 1.80 | Å | Characteristic length. **All species share σ.** |
| `LJ_CUTOFF_A` | `src/config.rs:108` | 2.2 | σ-units | Cutoff = 2.2·σ ≈ 3.96 Å. |
| `LJ_FORCE_MAX` | `src/config.rs:116` | 200.0 | AMU·Å/fs² | Force-magnitude clamp. ⚠️ Multiplied by `COLLISION_PASSES` at force-eval — §17. |
| `LJ_CELL_DENSITY_THRESHOLD` | `src/config.rs:118` | 0.001 | particles/Å³ | Switches to cell list above this density. |
| `SimConfig.lj_force_epsilon/sigma/cutoff` | `src/config.rs:331-333` | seeded from above | — | GUI overrides. |

### Per-species soft-core repulsion

Defined in `src/species.rs::SPECIES_PROPERTIES`. Acts inside `compute_repulsive_force` (`forces.rs:240-ish`) as `F = K·(1 - r/r0)/r` for `r < r0`, combined as `r0_pair = 0.5·(r0_i + r0_j)`.

| Species | `repulsion_strength` (K) | `repulsion_cutoff` (r0, Å) | Note |
|---|---|---|---|
| LithiumIon, ElectrolyteAnion, LithiumMetal, FoilMetal | 5.0 | 2.0 | |
| EC | 5.0 | 5.0 | "Reduced from 100.0 / 11.0 — represents osmotic pressure" (`species.rs:115` comment) |
| DMC | 5.0 | 5.0 | |
| VC | 5.0 | 5.0 | |
| FEC | 6.0 | 5.0 | Slightly stiffer |
| EMC | 4.5 | 5.5 | Slightly softer + longer range |
| LLZO/LLZT/S40B | 5.0 | 2.0 | Solid electrolytes |
| Graphite/HardCarbon/SiOx/LTO/LFP/LMFP/NMC/NCA/SEI | 5.0 | 2.0 | Electrodes/SEI |

**Smoking-gun concerns:**
- The `100.0/11.0 → 5.0/5.0` reduction comment confirms the values are ad-hoc; no literature target.
- All carbonate solvents share K and r0 except FEC (+0.2 in K) and EMC (-0.5 in K, +0.5 in r0). The justification for these specific deltas is undocumented.
- The repulsion competes with both LJ attraction and Coulomb. With `LJ_SIGMA = 1.80 Å`, two Li+ at r ≈ 2.0 Å have repulsion shoulder + Coulomb — never validated against anything.

**Validation handle:** Phase 1 force-balance + RDF first-peak; Phase 2 Li⁺-EC coordination number.

## 4. Per-species static properties

From `src/species.rs::SPECIES_PROPERTIES`. Mass / radius / damping below the LJ-and-repulsion table above.

| Species | mass (amu) | radius (Å) | damping | LJ enabled | neutral e⁻ | max e⁻ | polar charge |
|---|---|---|---|---|---|---|---|
| LithiumIon | 6.94 | 0.76 | 1.0 | no | 0 | unl. | 1.0 (default) |
| LithiumMetal | 6.94 | 1.52 | 0.01 | yes | 1 | 3 | 1.0 |
| FoilMetal | 1e6 | 1.52 | 0.1 | yes | 1 | 2 | 1.0 |
| ElectrolyteAnion | 145.0 | 2.0 | 1.0 | no | 0 | unl. | 1.0 |
| EC | 88.06 | 2.5 | 1.0 | no | 1 | unl. | 0.40 |
| DMC | 90.08 | 2.5 | 1.0 | no | 1 | unl. | 0.11 |
| VC | 86.0 | 2.4 | 1.0 | no | 1 | unl. | 0.42 |
| FEC | 107.0 | 2.5 | 0.8 | no | 1 | unl. | 0.45 |
| EMC | 104.0 | 2.6 | 1.0 | no | 1 | unl. | 0.11 |
| LLZO | 840.0 | 4.5 | 0.2 | yes | 0 | unl. | 0.05 |
| LLZT | 865.0 | 4.7 | 0.2 | yes | 0 | unl. | 0.06 |
| S40B | 340.0 | 4.2 | 0.25 | yes | 0 | unl. | 0.04 |
| SEI | 100.0 | 2.0 | 0.01 | yes | 0 | unl. | 1.0 |
| Graphite | 72.0 | 1.7 | 0.01 | yes | 1 | 3 | 1.0 |
| HardCarbon | 72.0 | 1.8 | 0.01 | yes | 1 | 3 | 1.0 |
| SiliconOxide | 60.0 | 2.0 | 0.01 | yes | 1 | 3 | 1.0 |
| LTO | 460.0 | 2.5 | 0.01 | yes | 1 | 3 | 1.0 |
| LFP | 158.0 | 2.2 | 0.01 | yes | 1 | 2 | 1.0 |
| LMFP | 158.0 | 2.2 | 0.01 | yes | 1 | 2 | 1.0 |
| NMC | 97.0 | 2.0 | 0.01 | yes | 1 | 2 | 1.0 |
| NCA | 97.0 | 2.0 | 0.01 | yes | 1 | 2 | 1.0 |

Notes:
- Damping 0.01 marks "stationary" species (electrodes, SEI, Li metal). Liquids run damping 1.0.
- Mass 1e6 amu on FoilMetal makes it quasi-immovable (typical metals are 27–63.5 amu); this is intentional for galvanostatic studies.
- The `polar_charge` column is the magnitude of the partial charge used by the polar-offset interaction model; see §5.

## 5. Polar solvent model ⚠️ **(smoking gun: single-charge vs. dipole)**

ParticleSim represents solvent polarity as a **single-charge** model: each solvent body carries one effective `polar_charge` magnitude and an `electron_drift_radius_factor` per species (see `ELECTRON_DRIFT_RADIUS_FACTOR_*` in `src/config.rs:38-46`). The bound electron orbits at a species-specific offset, producing an instantaneous dipole moment driven by external field.

| Species | `electron_drift_radius_factor` | polar_charge |
|---|---|---|
| EC, VC, LithiumMetal, FoilMetal | 1.00 | EC=0.40, VC=0.42, metals=1.0 |
| DMC | 0.73 | 0.11 |
| FEC | 0.90 | 0.45 |
| EMC | 0.75 | 0.11 |
| LLZO | 0.20 | 0.05 |
| LLZT | 0.20 | 0.06 |
| S40B | 0.22 | 0.04 |
| ELECTRON_MAX_SPEED_FACTOR | 10.2 (global, `src/config.rs:47`) | — |

**Smoking-gun concerns:**
- A true molecular dipole has two opposite point charges (or a permanent vector dipole). The current single-charge + drifting electron model produces a dipole only when an external field induces drift; with no field it averages to zero and provides no permanent polarization energy.
- This may misrepresent solvent screening, Li⁺-solvent coordination, and the dielectric response that drives EIS.
- Model change (not parameter tune): would touch `Body`, `forces.rs`, polar-offset usage in `electron_hopping.rs`. Out-of-scope for parameter validation; tracked as a future spike.

**Validation handle:** Phase 2 Li⁺-EC coordination number from RDF; Phase 3 dipole-vs-single comparison binary on identical scenario.

## 6. Electron hopping & Butler-Volmer ⚠️ **(smoking gun)**

### Hopping rate law (used when B-V disabled)

| Knob | File:line | Default | Units | Notes |
|---|---|---|---|---|
| `HOP_RATE_K0` | `src/config.rs:49` | 1.0 | fs⁻¹ | Pre-exponential. |
| `HOP_TRANSFER_COEFF` | `src/config.rs:51` | 0.5 | dimensionless | α. |
| `HOP_ACTIVATION_ENERGY` | `src/config.rs:53` | 0.025 | sim energy units | Numerically equal to `BV_OVERPOTENTIAL_SCALE`. Acts as k_B·T. |
| `HOP_RADIUS_FACTOR` | `src/config.rs:48` | 3.0 | radius multiplier | Search radius for hop targets. |
| `default_hop_alignment_bias` | `src/config.rs:55` | 0.01 | dimensionless gain | Field-alignment bias on hop selection. |
| `hop_vacancy_polarization_gain` | `src/config.rs:365` | 300.0 | dimensionless gain | ⚠️ Very high; "scales the influence of local valence-electron offset on hop selection". Ad-hoc. |

### Butler-Volmer

| Knob | File:line | Default | Units | Notes |
|---|---|---|---|---|
| `BV_ENABLED` | `src/config.rs:92` | true | bool | Master switch. |
| `BV_EXCHANGE_CURRENT` | `src/config.rs:94` | 0.1 | sim current | i₀. |
| `BV_TRANSFER_COEFF` | `src/config.rs:96` | 0.5 | dimensionless | α. |
| `BV_OVERPOTENTIAL_SCALE` | `src/config.rs:98` | 0.025 | sim voltage | RT/(nF) equivalent. |
| `SimConfig.bv_*` | `src/config.rs:292-296` | seeded from above | — | GUI overrides. |

### Hardcoded thresholds inside `electron_hopping.rs`

| Constant | Approx line | Value | Effect |
|---|---|---|---|
| Min field-alignment for hop | `electron_hopping.rs:230` | `1e-3` | Below this, hop blocked. |
| Electrode-conduction min alignment | `electron_hopping.rs:227` | `0.5` | Foil↔electrode paths bypass strict alignment. |

**Smoking-gun concerns:**
- The vacancy-polarization gain of 300.0 is an order of magnitude larger than every other gain in the codebase. Origin undocumented.
- `HOP_ACTIVATION_ENERGY` and `BV_OVERPOTENTIAL_SCALE` are both 0.025 — almost certainly the same physical quantity (k_B·T at 300 K in eV-equivalent), expressed twice. Should consolidate, but that's a refactor not a tune.
- Hardcoded magic thresholds (1e-3, 0.5) are not exposed to DOE sweeps.

**Validation handle:** Phase 1 zero-EMF symmetric cell (must produce zero current at rest); Phase 2 Tafel slope on a deliberately overpotentialed cell.

### Electron-sea protection (clustering)

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `ENABLE_ELECTRON_SEA_PROTECTION` | `src/config.rs:170` | true | Metals surrounded by ≥4 metals resist oxidation. |
| `SURROUND_RADIUS_FACTOR` | `src/config.rs:172` | 3.5 | Multiplier on body radius. |
| `SURROUND_NEIGHBOR_THRESHOLD` | `src/config.rs:174` | 4 | Count threshold. |
| `SURROUND_MOVE_THRESHOLD` | `src/config.rs:176` | 0.5 | Recompute trigger. |
| `SURROUND_CHECK_INTERVAL` | `src/config.rs:178` | 10 frames | Cap on recompute frequency. |

Couples with electron hopping; if disabled, isolated Li-metal can spuriously oxidize at zero overpotential.

## 7. Thermostat

| Knob | File:line | Default | Units | Notes |
|---|---|---|---|---|
| `DEFAULT_TEMPERATURE` | `src/config.rs:255` | 300.0 | K | Maxwell-Boltzmann target. |
| `SimConfig.temperature` | `src/config.rs:336` | seeded | K | GUI override. |
| `SimConfig.thermostat_interval_fs` | `src/config.rs:339` | 1.0 | fs | Apply every N fs. |
| `BOLTZMANN_CONSTANT` | `src/units.rs:36` | 8.617e-5 (SI converted) | AMU·Å²/(fs²·K) | Used in T = KE/(N_dof·k_B/2). |
| `SimConfig.damping_base` | `src/config.rs:327` | 1.00 | dimensionless | Role unclear from grep — needs investigation. |

### Hardcoded in `thermal.rs`

| Constant | Approx line | Value | Effect |
|---|---|---|---|
| Lower KE bootstrap threshold | `thermal.rs:129` | `1e-3` K | Below this, velocities re-initialized to MB. |
| Velocity scale clamp | `thermal.rs:144` | `[0.1, 10.0]` | Prevents extreme rescaling. ⚠️ If system is far from target, clamping silently absorbs energy — Phase 1 NVE-drift test must equilibrate past this regime first. |
| Liquids-only application | `thermal.rs:45` | filter to LithiumIon, ElectrolyteAnion, EC, DMC | Metals not thermostatted. Design choice; documented inline. |

## 8. Quadtree

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `QUADTREE_THETA` | `src/config.rs:203` | 1.0 | Barnes-Hut. |
| `QUADTREE_EPSILON` | `src/config.rs:204` | 2.0 Å | Softening. |
| `QUADTREE_LEAF_CAPACITY` | `src/config.rs:205` | 1 | One particle per leaf — maximizes tree depth. Ad-hoc. |
| `QUADTREE_THREAD_CAPACITY` | `src/config.rs:206` | 1024 | Parallel-build threshold. |

Hardcoded sentinels inside `quadtree.rs`: subdivision `quad.size < 1e-6`, position degeneracy `mag_sq < 1e-12`, mass/charge floor `> 1e-6`, idle-iteration cap 1000.

## 9. Collision resolution

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `COLLISION_PASSES` | `src/config.rs:194` | 7 | See §1, §17. |
| `LI_COLLISION_SOFTNESS` | `src/config.rs:225` | 0.8 | [0..1]; 1=very soft. |
| `SimConfig.li_collision_softness` | `src/config.rs:348` | seeded | GUI override. |
| `SimConfig.soft_collision_lithium_ion` | `src/config.rs:350` | true | Apply to Li⁺. |
| `SimConfig.soft_collision_anion` | `src/config.rs:352` | false | Apply to anion (off). |

Hardcoded in `collision.rs`: positional-correction weight `1.5·d·v/d²` (line ~334), degenerate-pair fallback angle hash `(i ^ (j << 13)) · TAU/1024` (line ~269), NaN-sanitization thresholds.

## 10. SEI formation

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `SimConfig.sei_formation_enabled` | `src/config.rs:373` | true | |
| `SimConfig.sei_formation_probability` | `src/config.rs:375` | 0.01 | per fs base rate |
| `SimConfig.sei_formation_bias` | `src/config.rs:377` | 1.0 | multiplier |
| `SimConfig.sei_electrons_per_event` | `src/config.rs:379` | 1 | electrons consumed |
| `SimConfig.sei_radius_scale` | `src/config.rs:381` | 1.1 | post-conversion radius scale |
| `sei_charge_threshold_vc/fec/ec/emc/dmc` | `src/config.rs:383-391` | 0.6/0.8/1.0/1.2/1.5 (\|e\| units) | Per-solvent kinetic threshold |
| `SEI_FORMATION_POTENTIAL` | `src/config.rs:161` | 1.0 V | Thermodynamic gate (vs Li/Li⁺) |
| `BASELINE_POTENTIAL` | `src/config.rs:157` | 2.0 V | Zero-charge offset |
| `POTENTIAL_PER_CHARGE` | `src/config.rs:159` | 2.0 V/e | Linear potential model slope |
| `ENABLE_POTENTIAL_GATING` | `src/config.rs:163` | true | If false, kinetics dominate |

Hardcoded in `sei.rs`: metal search radius `2.5·body.radius` (line ~48), metal charge threshold `≥ -0.01` (line ~63), velocity damping on conversion `solvent.vel *= 0.1` (line ~142).

## 11. Intercalation

`src/simulation/intercalation.rs` — almost everything is hardcoded, none of it in `SimConfig`.

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `INTERCALATION_DISTANCE_FACTOR` | `intercalation.rs:11` | 2.5 | Search radius multiplier. **Not in SimConfig.** |
| `BASE_INTERCALATION_PROBABILITY` | `intercalation.rs:14` | 0.01 | Per-fs base rate. **Not in SimConfig.** |

Plus hardcoded literals: deintercalation Li⁺ spawn radius `±10` units (line ~186), cathode search distance `dx²+dy² < 400` (line ~211), spawned electron orbital position `radius·polar_offset()`, spawned Li⁺ velocity scale `(rand−0.5)·0.1`.

**Concern:** intercalation kinetics are entirely non-tunable from `SimConfig`. Anyone running a DOE that perturbs hop rates without also perturbing intercalation will see asymmetric results.

## 12. Stack pressure

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `SimConfig.stack_pressure_enabled` | `src/config.rs:399` | false | |
| `SimConfig.stack_pressure` | `src/config.rs:403` | 100.0 | Boundary force magnitude. |
| `SimConfig.stack_pressure_decay` | `src/config.rs:407` | 10.0 Å | Linear decay length. |

## 13. Out-of-plane (z) dynamics

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `DOMAIN_DEPTH` | `src/config.rs:214` | 1.0 Å | ±z half-extent. |
| `OUT_OF_PLANE_ENABLED` | `src/config.rs:215` | false | |
| `Z_STIFFNESS` | `src/config.rs:216` | 1.0 | Hookean stiffness. |
| `Z_DAMPING` | `src/config.rs:217` | 0.5 | Viscous coefficient. |
| `MAX_Z` | `src/config.rs:218` | `DOMAIN_DEPTH` | Boundary clamp. |
| `SimConfig.z_*` | `src/config.rs:341-345` | seeded | GUI overrides. |
| `SimConfig.enable_z_many_body_forces` | `src/config.rs:345` | false | Expensive, off by default. |

## 14. Induced external field

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `SimConfig.induced_field_gain` | `src/config.rs:356` | 0.0 | Disabled by default. |
| `SimConfig.induced_field_smoothing` | `src/config.rs:358` | 0.9 | High smoothing → ~10-step lag. |
| `SimConfig.induced_field_use_direction` | `src/config.rs:360` | true | Infer direction from foil centroids. |
| `SimConfig.induced_field_overpot_scale` | `src/config.rs:362` | 100.0 | Overpotential→drive scale. |

## 15. Foil parameters

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `SimConfig.foil_mass` | `src/config.rs:395` | 1.0e6 amu | Quasi-stationary; comment notes Cu ≈ 63.5 / Al ≈ 27. |
| `FOIL_MAX_ELECTRONS` | `src/config.rs:145` | 2 | |
| `FOIL_NEUTRAL_ELECTRONS` | `src/config.rs:124` | 1 | |

## 16. Domain & initialization

| Knob | File:line | Default | Notes |
|---|---|---|---|
| `CLUMP_RADIUS` | `src/config.rs:211` | 20.0 Å | Initial spawn clump. |
| `DOMAIN_BOUNDS` | `src/config.rs:212` | 350.0 Å | Half-width of domain. |

---

## 17. Cross-coupling red flags

These are structural concerns that complicate parameter validation. Each one means "you cannot validate parameter X in isolation; you must also vary or hold Y fixed."

1. **`LJ_FORCE_MAX × COLLISION_PASSES` coupling** (`forces.rs:221`). The maximum LJ force per particle per step is `COLLISION_PASSES · LJ_FORCE_MAX`. Changing `COLLISION_PASSES` for performance silently rescales force clamping. Phase 1 invariant: NVE energy drift at fixed dt should not depend on `COLLISION_PASSES` — if it does, this coupling is the cause.

2. **`HOP_ACTIVATION_ENERGY ≡ BV_OVERPOTENTIAL_SCALE = 0.025`**. Two distinct knobs hold the same physical quantity (k_B·T equivalent). A user editing one without the other introduces an inconsistency that no test currently catches.

3. **`dt = 5.0 fs` is large**. Any rate law of form `p = 1 - exp(-k·dt)` can only be linearized when `k·dt ≪ 1`. For `HOP_RATE_K0 = 1.0 fs⁻¹` this is `k·dt = 5.0` — well into the saturating regime. Phase 1 dt-halving invariant will reveal whether this matters in practice.

4. **Hardcoded literals not in `SimConfig`** — at minimum:
   - `electron_hopping.rs:230` alignment threshold `1e-3`
   - `electron_hopping.rs:227` electrode min alignment `0.5`
   - `intercalation.rs:11` distance factor `2.5`
   - `intercalation.rs:14` base probability `0.01`
   - `sei.rs:48` metal search radius `2.5·r`
   - `sei.rs:142` post-conversion velocity damping `0.1`
   - `thermal.rs:144` velocity scale clamp `[0.1, 10.0]`
   - `quadtree.rs` various sentinels (`1e-6`, `1e-12`, `1e-8`)
   
   None of these can be DOE-swept without code changes. The validation plan needs to either (a) cover them with synthetic-stress tests that pin their effective range, or (b) propose promoting a subset to `SimConfig`. **Promotion requires user approval** per the no-retuning rule.

5. **`hop_vacancy_polarization_gain = 300.0`** (`config.rs:365`). An order of magnitude larger than every other gain. Worth a dedicated sensitivity sweep in Phase 2.

6. **Foil mass 1e6 amu**. Decouples nucleus dynamics from electron dynamics in the limit of infinite inertia. Foil electrons can move (hop on/off) while the foil body cannot. Whether this introduces systematic charge-momentum mismatch under high-amplitude EIS forcing is unknown.

7. **Thermostat clamp `[0.1, 10.0]`**. If the system is far from target (spawned cold, or cooled by a measurement), the clamp silently absorbs/injects energy. Phase 1 MB-distribution invariant must use a long pre-equilibration that takes the clamp out of play.

8. **Single-charge polar solvent vs. dipole** (§5). Not a parameter cross-dep — a model choice. Architecturally orthogonal to everything else but affects Li⁺ coordination, screening, and dielectric response. Out of scope for parameter validation; tracked separately.

---

## 18. Parameter → validation-phase mapping

Cross-references the inventory to `PHYSICS_VALIDATION_PLAN.md` phases. "Phase 1" = invariants (must hold regardless of values), "Phase 2" = dimensionless ratios, "Phase 3" = absolute targets.

| Knob/area | Phase 1 invariant | Phase 2 dimensionless | Phase 3 absolute |
|---|---|---|---|
| Total charge | `charge_balance` | — | — |
| Symmetric-cell EMF | `zero_emf_symmetric` | — | — |
| Velocity distribution | `mb_velocity_distribution` | — | — |
| Energy conservation | `nve_energy_drift` | — | — |
| Quadtree force error | `quadtree_force_error` | — | — |
| Spurious plating at 0 current | `no_spurious_plating` | — | — |
| dt convergence | (variant of nve_drift) | — | — |
| `COLLISION_PASSES` coupling | (variant of nve_drift) | — | — |
| **LJ + repulsion (smoking gun)** | NVE drift sensitive to ε, σ | RDF first-peak position; Li⁺-EC coordination # | — |
| **Polar solvent model (smoking gun)** | — | Li⁺-EC coord #; dielectric response | dipole-vs-single comparison |
| **Hopping/BV (smoking gun)** | zero-EMF must be 0 | Tafel slope (η vs ln I) | — |
| Diffusion coefficient | — | Nernst-Einstein σ vs D₊/D₋ | D_Li⁺ in EC/DMC bulk (Phase 0a binary exists) |
| Conductivity | — | (paired with above) | 1 M conductivity |
| Electrochemical window | — | — | redox window vs literature |
| Stack pressure / z-dynamics | NVE drift independence | — | — |
| Intercalation kinetics | (no current invariant) | — | (none yet) |
| SEI kinetics | (no current invariant) | — | (none yet) |

---

## Appendix A — file map

- `src/config.rs` — `SimConfig`, defaults, global `pub const` values (lines 1–512)
- `src/species.rs` — `SPECIES_PROPERTIES` table (lines 1–629)
- `src/units.rs` — SI ↔ sim unit conversions
- `src/body/types.rs` — `Body`, `Species`, `Electron`
- `src/body/redox.rs` — `donation_overpotential`, `can_donate_electron`, local-potential model
- `src/simulation/forces.rs` — Coulomb (Barnes-Hut), LJ, soft-core repulsion, stack pressure
- `src/simulation/electron_hopping.rs` — hop rate law, alignment gating
- `src/simulation/sei.rs` — solvent reduction → SEI
- `src/simulation/intercalation.rs` — Li⁺ insertion/extraction
- `src/simulation/collision.rs` — broccoli-tree collision passes
- `src/simulation/thermal.rs` — Maxwell-Boltzmann thermostat
- `src/quadtree/quadtree.rs` — Barnes-Hut tree build & traversal

## Appendix B — knobs added since this inventory

Append-only. Date-stamp every entry. Update the inventory body when promoting from this list.

*(none yet)*
