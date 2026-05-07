# Solvation parameter review

**Version 0.1** — drafted 2026-05-06 on `feature/dipole-solvent-spike` branch.

Goal: align the simulator's molecular and inter-species parameters to defensible physical-chemistry values for **1 M LiPF6 in EC/DMC at 300 K**, so that downstream solvation observables (CIP / SIP / S2IP / FD distribution from `SolvationDiagnostic`) fall in literature ranges. This doc is per-knob, with the user's decision on each one tracked here.

**Scope:** EC, DMC, LithiumIon, ElectrolyteAnion (PF6⁻ proxy). Other solvents (VC, FEC, EMC) included for completeness but not the primary review target.

**Strict rule** (per memory `feedback_no_unauthorized_retuning.md`): nothing changes without explicit per-knob approval.

---

## 1. Per-species mechanical parameters

Source columns: `sim` from `src/species.rs::SPECIES_PROPERTIES`; `real` cited inline. Where a parameter is *intentionally* simplified for the simulator (e.g., 2D-with-3D-Coulomb constraints), it's marked "(by design)" and not flagged as a gap.

### EC — ethylene carbonate (C₃H₄O₃)

| Param | Sim | Real | Source | Match? |
|---|---:|---:|---|---|
| Mass | 88.06 amu | 88.06 g/mol | molecular formula | ✓ |
| Radius (body) | 2.5 Å | ~2.5–3.0 Å (effective vdW radius) | OPLS-AA vdW | ✓ |
| Dipole moment (effective) | 1.00 e·Å | 1.02 e·Å (= 4.91 D) | gas-phase / Onsager-corrected liquid | ✓ |
| Polarizability factor (`polar_offset`) | 1.0 | n/a (proxy) | sim-internal | ✓ (by design) |
| `polar_charge` | 0.40 e | matches given r=2.5 → dipole 1.0 e·Å | derived | ✓ |
| LJ enabled | false | n/a | by design | (by design) |

**Status: EC molecular params look right.**

### DMC — dimethyl carbonate (C₃H₆O₃)

| Param | Sim | Real | Source | Match? |
|---|---:|---:|---|---|
| Mass | 90.08 amu | 90.08 g/mol | molecular formula | ✓ |
| Radius | 2.5 Å | ~2.7 Å (slightly larger than EC) | OPLS-AA vdW | ≈ |
| Dipole moment (effective) | 0.20 e·Å | 0.18–0.20 e·Å (= 0.9 D liquid; 0.43 D gas) | Soetens 1998; gas-phase computational | ✓ |
| Polarizability factor (`polar_offset`) | 0.73 | (lower than EC, reflects lower polarity) | by design | ✓ |
| `polar_charge` | 0.11 e | 0.108 (derived from 0.20 e·Å / 0.73 / 2.5) | derived | ✓ |
| LJ enabled | false | n/a | by design | (by design) |

**Status: DMC molecular params look right.** The 0.91 D liquid dipole vs 0.43 D gas-phase reflects environment-induced enhancement; the sim's 0.20 e·Å lies in the liquid-phase range.

### LithiumIon (Li⁺)

| Param | Sim | Real | Source | Match? |
|---|---:|---:|---|---|
| Mass | 6.94 amu | 6.94 g/mol | atomic mass | ✓ |
| Radius (body) | 0.76 Å | 0.76 Å (Shannon ionic radius, 6-coord) | Shannon 1976 | ✓ |
| Solvated effective radius | (no explicit) | 3.4 Å (Stokes radius in EC) | Stokes-Einstein from D measurements | n/a |
| `polar_offset` | 0.0 | n/a (Li⁺ has no electron) | by design | ✓ |
| `polar_charge` | 1.0 e (default) | +1.0 e | trivial | ✓ |
| LJ enabled | false | n/a | by design | (by design) |

**Status: Li⁺ params look right.** The "Stokes radius" of 3.4 Å represents the dynamically-coordinated ion in solution; that emerges from the simulator naturally if EC stays in the first shell.

### ElectrolyteAnion (PF6⁻ proxy)

| Param | Sim | Real | Source | Match? |
|---|---:|---:|---|---|
| Mass | 145.0 amu | 144.96 g/mol | molecular formula | ✓ |
| Radius (body) | **2.0 Å** | **2.55 Å** (vdW radius) | Marcus 1997 | **gap (~22% small)** |
| Charge | -1.0 e | -1.0 e | trivial | ✓ |
| `polar_offset` | 0.3 | (some polarizability, PF6⁻ is mildly polarizable) | OPLS-AA assigns small partial charges to F atoms | ≈ |
| LJ enabled | false | n/a | by design | (by design) |

**Status: anion radius is too small.** Real PF6⁻ vdW radius ≈ 2.55 Å (Marcus, *Pure Appl. Chem.* 1997, "Ionic Radii in Aqueous Solutions"). Sim uses 2.0 Å. Effect: Li⁺-PF6⁻ contact distance is `r_Li + r_anion = 0.76 + 2.0 = 2.76 Å` in sim, vs `0.76 + 2.55 = 3.31 Å` real. The simulator's ions can approach ~0.55 Å closer than reality → stronger Coulomb attraction, more contact-pair formation.

**Recommendation 1 (open):** raise `ElectrolyteAnion.radius` from 2.0 → 2.55 Å.

---

## 2. Inter-species interactions

This is where most of the visible solvation behavior comes from.

### 2.1 Li⁺ ↔ EC (the binding interaction that *should* form the first solvation shell)

**Current model:**
- Coulomb between Li⁺ (+1.0) and EC body (q=0): zero direct.
- Polarization: under `SingleOffset` mode, EC's bound electron shifts in Li⁺ field → effective dipole appears → produces an **induced** Li-EC attraction. Magnitude depends on `polar_charge × polar_offset × radius` (= 1.0 e·Å for EC) and the resulting field gradient.
- Under `ConjugatePair` mode, EC is treated as an explicit +0.4 / −0.4 dipole pair. Field at Li⁺ position is the sum of these two contributions.
- **No Lennard-Jones attraction.** EC has `lj_enabled = false`. Li⁺ has `lj_enabled = false`.
- **Soft-core repulsion:** EC has `enable_repulsion = true` (k=5, r0=5). Li⁺ has `enable_repulsion = false`. Per `compute_repulsive_force`, repulsion only fires when **both** species have it enabled. So **no repulsion between Li⁺ and EC.**

**Real Li⁺-O(EC) interaction:**
- Mostly electrostatic (Li⁺ is hard sphere, EC carbonyl O carries δ⁻).
- Binding energy: ~25–35 kJ/mol per Li-O contact (DFT, Cui et al. 2022; Yu et al. 2018). In sim units = 0.26–0.36 eV = 10–14 k_BT.
- Equilibrium Li-O distance: ~2.0 Å (DFT; Bogle et al. 2013).
- Real Li⁺ in EC bulk has ~4 EC molecules in first shell, each via Li-O coordination.

**Gaps:**
1. **No short-range attraction beyond Coulomb-via-dipole.** Real Li-O has a Pauli-repulsion + dispersion well around r=2 Å. Sim has only Coulomb-via-induced-dipole, which is softer and longer-ranged.
2. **No short-range repulsion at all.** Sim's Li⁺ can approach EC body indefinitely (limited only by the quadtree softening at r=2 Å). Real Li⁺ would feel hard-sphere repulsion from EC's electron cloud at ~2 Å.

**Possible knobs to engage:**
- (a) Enable LJ on Li⁺ AND EC with appropriate ε (depth) and σ (range) representing Li-O attraction.
- (b) Enable repulsion on Li⁺ with `repulsion_cutoff` set to give `r0_pair = (Li_r0 + EC_r0)/2 ≈ 3.5–4.0 Å` for repulsion structure.
- (c) Switch to `ConjugatePair` dipole model — gives stronger orientation-dependent Li-EC attraction.

### 2.2 Li⁺ ↔ PF6⁻ (the ion pair we want to break up)

**Current model:**
- Coulomb between Li⁺ (+1) and PF6⁻ (−1): full 14.4 / r² in sim units.
- **No repulsion** (both species have `enable_repulsion = false`).
- **No LJ** (both have `lj_enabled = false`).
- **No restoring force** at short range — purely the quadtree softening kicks in below r=2 Å.

**Real Li⁺-PF6⁻ interaction:**
- Coulomb-dominated at typical separations (~5–10 Å).
- At contact (r ≈ 3.3 Å), Pauli repulsion from anion electron cloud takes over.
- Solvent screening (effective ε ≈ 90 in pure EC at 300 K) reduces effective Coulomb by ~factor of 90.

**Gaps:**
1. **No solvent dielectric screening.** The simulator computes Coulomb at full vacuum strength, ε_r=1. Real EC has bulk ε_r ≈ 90, so Li-anion interaction at long distance is 90× weaker than sim. (At short distance, the local first-shell screening is much weaker; a typical estimate is ε_eff ≈ 5–10 between an ion pair separated by 1 solvent layer.)
2. **No short-range repulsion.** Li⁺ and PF6⁻ can sit at any distance r > 2 Å in this sim; real ions would have a hard-wall at r ≈ 3.3 Å.

**Possible knobs:**
- (a) Enable repulsion on both Li⁺ and PF6⁻ to introduce hard-sphere contact at r ≈ 3.3 Å.
- (b) Add explicit dielectric screening (would require a model change — e.g., Yukawa potential or distance-dependent ε).

### 2.3 EC-EC (the constraint that limits multi-EC packing around Li⁺)

**Current model:**
- Coulomb (zero between neutral EC bodies, but ConjugatePair adds dipole-dipole).
- **Soft-core repulsion**: k=5, r0=5 Å. Pair-r0 = 5 Å (since both ECs).
- No LJ.

**Real EC-EC interaction:**
- van der Waals contact at r ≈ 5 Å (matches sim r0).
- Dipole-dipole interaction depends on orientation; orientation-averaged Keesom term is attractive but weak (~k_BT at 5 Å).
- No hard-sphere overlap below ~5 Å.

**Gap (potential):** the sim's r0 = 5.0 Å is reasonable for the molecular footprint, but combined with Li⁺-EC having NO repulsion, the geometric situation is asymmetric: ECs can't get close to each other (good), but Li⁺ can sit between them with no resistance (questionable). This may be why CIP fraction is high — Li⁺ can wedge between ECs and find its way to the anion.

### 2.4 EC ↔ PF6⁻

Currently: no repulsion (anion has it disabled), no LJ. Only weak Coulomb-via-dipole.

Real: PF6⁻ is poorly solvated by EC (PF6⁻ has weak Lewis basicity), so a small EC-anion repulsion is realistic. The simulator's "EC pulls Li⁺ but not PF6⁻" asymmetry is roughly right qualitatively.

---

## 3. Dipole-physics mode

The user has noted that the `physics monopole to dipole` setting (= `DipoleModel::SingleOffset` vs `DipoleModel::ConjugatePair`) changes solvation outcomes substantially.

| Mode | What it computes | Gives Li-EC attraction? | Gives EC-EC dipole-dipole? |
|---|---|---|---|
| `SingleOffset` (default) | Field difference at nucleus vs electron of body i, due to *neighbors' net charges only*. | Weak (induced dipole only) | No |
| `ConjugatePair` | Same, plus explicit +q_eff / −q_eff fields from EC/DMC dipoles. | Stronger (explicit dipole field) | Yes |

**`ConjugatePair` is the more physical of the two.** `SingleOffset` neglects dipole sources entirely, treating EC/DMC as pure neutrals for far-field purposes — which under-represents the molecular polarity. The classifier output the user shared (CIP/S2IP ≈ 0.2/0.8 with default settings) is consistent with a too-weak Li-EC interaction.

**Recommendation 2 (open):** make `DipoleModel::ConjugatePair` the default. (Currently `SingleOffset` is the default per `config.rs:487`.)

---

## 4. Critical observations summary

| Observation | Effect on solvation | Severity |
|---|---|---|
| EC dipole moment 1.0 e·Å vs real 1.02 e·Å | Negligible | none |
| DMC dipole 0.20 e·Å vs real ~0.18 e·Å | Negligible | none |
| Anion radius 2.0 Å vs real 2.55 Å | Allows tighter Li-anion CIP than reality | moderate |
| Default = `SingleOffset` (neglects EC dipole field) | Under-represents Li-EC attraction → CIP dominates | **high** |
| No LJ ε between Li⁺ and EC | Misses the ~25 kJ/mol Li-O binding | **high** |
| No repulsion on Li⁺ or anion | Allows unphysical close approaches; no hard-sphere structure | moderate |
| No solvent dielectric screening | Coulomb between ion pair is too strong at large r | high (architectural) |

---

## 5. Recommended ordering for review-and-adjust

Each is a single knob (or pair) with a clear physical justification. I recommend testing them **one at a time** with the CIP/SIP/S2IP/FD classifier as the readout, so we can attribute changes:

| # | Knob | Sim → Recommended | Reasoning |
|---|---|---|---|
| **R1** | `DipoleModel` default | `SingleOffset` → `ConjugatePair` | Real EC has a static dipole; SingleOffset ignores its far field |
| **R2** | `ElectrolyteAnion.radius` | 2.0 → 2.55 Å | Match real PF6⁻ vdW radius |
| **R3** | Enable Li⁺ repulsion | `enable_repulsion: false` → `true` (with r0 ≈ 1.5 Å) | Real Li⁺ has hard-sphere contact at ~3.3 Å with anion |
| **R4** | Enable PF6⁻ repulsion | `enable_repulsion: false` → `true` (already pair-r0 = 2.55+2.55 = 5.1 if anion radius set) | Combined with R3, gives proper Li-anion contact distance |
| **R5** | Add Li-EC LJ pair (custom) | n/a → ε ≈ 25 kJ/mol, σ ≈ 2 Å | Real Li-O binding is ~25 kJ/mol; currently absent |
| **R6** | Solvent dielectric screening | n/a → architectural change | Long-term; would require new force law |

**R1 and R2 are tiny single-line changes** (one config field each, no architectural risk). They would close the gap to literature for the simplest physical reasons. I'd suggest:

1. Run the GUI with `ConjugatePair` selected (R1, no code change needed — just toggle the dropdown). Read off CIP/SIP/S2IP/FD.
2. If the user authorises, change the default in config.rs:487 (one-line change).
3. Independently, test R2 by changing `ElectrolyteAnion.radius`. Read off the classifier.
4. If R1+R2 land us in the literature band, stop — solvation is "right enough" and we move to the next phase.
5. If not, R3/R4 get added.
6. R5 (Li-EC LJ) requires species code modifications and is bigger scope.
7. R6 (dielectric) is a real architectural change and should wait.

---

## 6. What to share with another agent for downstream work

This doc + the `SolvationDiagnostic` reference (`src/diagnostics/solvation.rs`) are sufficient context for someone to take over:

- The classifier already exists and is reliable.
- The dipole-physics toggle already exists.
- The species table is the input layer.
- The "before" baseline (default settings) per user observation: CIP/SIP/S2IP/FD ≈ 0.2 / 0 / 0.8 / 0.
- Literature target: roughly CIP 0.10–0.30, SIP 0.40–0.55, S2IP 0.10–0.30, FD 0.05–0.15.

---

## Open items (per-knob status)

*To be filled in as user authorises adjustments and we measure outcomes.*

- [ ] R1 — `DipoleModel` default `ConjugatePair`. Pre-CIP/SIP/S2IP/FD: 0.2/0/0.8/0. Post: TBD.
- [ ] R2 — `ElectrolyteAnion.radius` 2.0→2.55. Pre: TBD. Post: TBD.
- [ ] R3 — Li⁺ repulsion enable. Pre: TBD. Post: TBD.
- [ ] R4 — PF6⁻ repulsion enable. Pre: TBD. Post: TBD.
- [ ] R5 — Li-EC LJ pair. Pre: TBD. Post: TBD.
- [ ] R6 — Dielectric screening (architectural; deferred).
