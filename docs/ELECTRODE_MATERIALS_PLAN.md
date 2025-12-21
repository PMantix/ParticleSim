# Multi-Electrode Materials Expansion Plan

## Executive Summary

This document outlines a phased approach to expanding ParticleSim from symmetric lithium metal electrodes to supporting a variety of anode and cathode active materials commonly used in lithium-ion batteries.

**Target Materials:**
- **Cathodes:** NMC (LiNiMnCoO₂), LFP (LiFePO₄), LMFP (LiMnFePO₄), NCA (LiNiCoAlO₂)
- **Anodes:** Graphite, Graphene, Hard Carbon, Silicon Oxide (SiOx), LTO (Li₄Ti₅O₁₂)

---

## Design Philosophy: Emergent Intercalation

### The Key Insight

Rather than explicitly modeling intercalation with complex stoichiometry tracking, rate equations, and particle absorption/spawning, we leverage the particle-based simulation's natural physics:

**Real intercalation:**
1. Li⁺ approaches electrode surface
2. Li⁺ desolvates (sheds solvent shell)
3. Li⁺ enters host lattice (gaps between layers/particles)
4. Li⁺ gets reduced by accepting electron from host

**What our simulation already does:**
1. Li⁺ moves toward electrode (electric field + diffusion)
2. Desolvation happens implicitly (solvent sterically excluded from tight spaces)
3. Li⁺ can physically enter gaps between electrode particles
4. Electron hopping reduces Li⁺ → Li⁰ when near electrode material

### Advantages of Emergent Approach

| Aspect | Explicit Tracking | Emergent Approach |
|--------|-------------------|-------------------|
| Capacity limits | Arbitrary counters | Natural geometry limits |
| Stoichiometry | Manual tracking | Implicit from particle count |
| Code complexity | High (many systems) | Low (use existing physics) |
| Physical realism | Abstracted | Actual spatial distribution |
| Concentration gradients | Calculated | Emerge naturally |

### What We Keep

- **MaterialType** - for equilibrium potentials, colors, species identification
- **Species enum entries** - Graphite, LFP, NMC, etc.
- **Potential gating** - controls where reactions happen thermodynamically
- **Electron hopping** - existing Butler-Volmer kinetics
- **SOC visualization** - based on nearby lithium count

### What We Remove (Cleanup Task)

The following explicit intercalation machinery is no longer needed:

- `IntercalationConfig` struct
- `IntercalationResult` / `DeintercalationResult` enums  
- `desolvation_probability()` function
- `butler_volmer_rate()` for intercalation (keep for electron hopping)
- `reaction_direction()` function
- `intercalate_many()` / `deintercalate_many()` methods
- `ActiveMaterialRegion.lithium_count` / `lithium_capacity` tracking
- `IntercalationStats` struct
- Particle absorption/spawning logic

---

## Revised Architecture

### Core Concept: Electrode Particles as Lithium Hosts

Each electrode material particle (Graphite, LFP, etc.) acts as a **host region** for lithium:
- Li⁺ ions can physically occupy gaps between/within electrode particles
- Electrons hop to reduce Li⁺ → Li⁰ when thermodynamically favorable
- Li⁰ stays trapped in electrode region due to geometry
- "State of charge" = density of Li⁰ near electrode particles

### Thermodynamic Gating (Critical)

**Electron donation from active materials must be gated by thermodynamic favorability:**

```rust
// In electron_hopping.rs - when electron hops FROM active material particle
fn can_donate_electron(donor: &Body, acceptor: &Body) -> bool {
    if !ENABLE_POTENTIAL_GATING {
        return true;
    }
    
    // Get the donor's equilibrium potential
    let donor_eq_potential = donor.equilibrium_potential();
    
    // Get local potential from charge (overpotential)
    let local_potential = donor.local_potential();
    
    // Electron donation favorable when local potential < equilibrium
    // (more reducing conditions than equilibrium)
    local_potential < donor_eq_potential + OVERPOTENTIAL_THRESHOLD
}
```

**Effect by material:**
- **Graphite (0.1V):** Donates electrons easily (low barrier)
- **LFP (3.4V):** Only donates electrons at high potentials (cathode conditions)
- **Li metal (0.0V):** Most reducing, donates most readily

This naturally prevents:
- Li plating at cathode (electrons can't reduce Li⁺ at 3.4V)
- Unwanted reduction at high potentials

### SOC Visualization via Nearby Lithium Count

Instead of tracking `lithium_content` per particle, we **count nearby lithium atoms**:

```rust
// For rendering - count Li within cutoff distance of electrode particle
fn calculate_local_soc(electrode_body: &Body, all_bodies: &[Body]) -> f32 {
    let cutoff = electrode_body.radius * 3.0; // tunable
    let mut li_count = 0;
    let mut max_capacity = estimate_capacity_from_volume(electrode_body);
    
    for body in all_bodies {
        if body.species == Species::LithiumMetal {
            let dist = (electrode_body.pos - body.pos).length();
            if dist < cutoff {
                li_count += 1;
            }
        }
    }
    
    (li_count as f32 / max_capacity).clamp(0.0, 1.0)
}
```

This provides:
- Real-time SOC visualization
- No state to track or synchronize
- Natural gradients visible in simulation

---

## Current Architecture (What Works Today)

### Species/Materials
- **Species Enum** in [src/body/types.rs](src/body/types.rs) - includes electrode materials: Graphite, HardCarbon, SiliconOxide, LTO, LFP, LMFP, NMC, NCA
- **Electron hopping** extended to all electrode materials
- **Potential gating** prevents reactions at wrong potentials

### Electrochemical Behavior
- `apply_redox()` handles Li⁺ ↔ Li⁰ with potential gating
- SEI formation gated by potential
- Butler-Volmer kinetics for electron transfer

### What Still Needs Work
- Electron hopping needs thermodynamic gating for donation
- SOC visualization needs nearby-lithium counting
- Cleanup of unused intercalation code

---

## Key Differences Between Electrode Types

### Lithium Metal (Current Implementation)
- **Mechanism:** Plating/stripping - Li⁺ deposits as Li⁰ metal atoms
- **Voltage:** 0 V vs Li/Li⁺ (reference)
- **Behavior:** Li⁺ reduced anywhere with excess electrons at low potential

### Intercalation Materials (Graphite, LFP, NMC, etc.)
- **Mechanism:** Li⁺ enters gaps, gets reduced, stays trapped
- **Voltage:** Material-dependent (0.1-4.2V vs Li/Li⁺)
- **Behavior:** Geometry naturally limits capacity; potential gates reactions

**The key insight:** In a particle simulation, intercalation IS just "Li enters gaps and gets reduced" - no special machinery needed!

---

## Revised Implementation Phases

### Phase 1: Species & Basic Framework ✅ COMPLETE
- [x] Add electrode material Species (Graphite, LFP, NMC, etc.)
- [x] Add electron hopping support for all materials
- [x] Add equilibrium potentials per species
- [x] Implement potential gating for Li plating
- [x] Implement potential gating for SEI formation

### Phase 2: Thermodynamic Electron Donation Gating
**Goal:** Electrons only hop FROM electrode materials when thermodynamically favorable

- [ ] Modify `attempt_electron_hop()` in electron_hopping.rs
  - Add check: `donor.local_potential() < donor.equilibrium_potential() + threshold`
  - This prevents cathode materials from reducing Li⁺ inappropriately
  
- [ ] Add `ELECTRON_DONATION_OVERPOTENTIAL` config constant
  - Small threshold (~0.1V) to allow some overpotential driving

**Expected behavior:**
| Material | Eq. Potential | Donates electron when... |
|----------|---------------|--------------------------|
| Li metal | 0.0V | local_potential < 0.1V |
| Graphite | 0.1V | local_potential < 0.2V |
| LTO | 1.55V | local_potential < 1.65V |
| LFP | 3.4V | local_potential < 3.5V |
| NMC | 3.8V | local_potential < 3.9V |

### Phase 3: SOC Visualization via Nearby Lithium Count
**Goal:** Color electrode particles by local lithium density

- [ ] Add `count_nearby_lithium()` function
  ```rust
  fn count_nearby_lithium(body_idx: usize, bodies: &[Body], cutoff: f32) -> usize
  ```

- [ ] Add `calculate_local_soc()` for rendering
  - Count Li⁰ within `radius * 3.0` of electrode particle
  - Normalize by estimated capacity (based on particle size)
  
- [ ] Update renderer to use nearby lithium count
  - Replace `body.lithium_content` with dynamic calculation
  - Can cache per-frame for performance

- [ ] Remove `lithium_content` field from Body (cleanup)

### Phase 4: Cleanup Unused Code
**Goal:** Remove explicit intercalation machinery that's no longer needed

- [ ] Remove from `src/electrode/intercalation.rs`:
  - [ ] `IntercalationConfig` struct
  - [ ] `IntercalationResult` enum
  - [ ] `DeintercalationResult` enum  
  - [ ] `desolvation_probability()` function
  - [ ] `butler_volmer_rate()` function (intercalation-specific)
  - [ ] `reaction_direction()` function
  - [ ] `ReactionDirection` enum
  - [ ] `IntercalationStats` struct

- [ ] Remove from `src/electrode/region.rs`:
  - [ ] `lithium_count` field
  - [ ] `lithium_capacity` field
  - [ ] `intercalate_many()` method
  - [ ] `deintercalate_many()` method

- [ ] Remove from `src/electrode/material.rs`:
  - [ ] `StorageMechanism` enum (or keep if useful for docs)
  - [ ] `mechanism()` method
  - [ ] `max_stoichiometry()` method
  - [ ] `desolvation_barrier()` method

- [ ] Remove from Simulation:
  - [ ] `intercalation_config` field (if added)
  - [ ] Any particle absorption/spawning for intercalation

- [ ] Clean up dead code warnings

### Phase 5: Testing & Validation
- [ ] Test graphite anode: Li⁺ enters gaps, gets reduced at low potential
- [ ] Test LFP cathode: Li⁺ enters gaps, gets reduced at cathode potential
- [ ] Verify Li metal still plates normally at anode
- [ ] Verify no Li plating at cathode potentials
- [ ] Verify SEI only forms at anode
- [ ] Verify SOC coloring reflects actual lithium distribution

---

## Configuration (Simplified)

With emergent intercalation, configuration is simpler - just specify electrode materials:

```toml
[simulation]
domain_width = 600.0
domain_height = 400.0

[[electrodes]]
name = "anode"
material = "Graphite"  # Just the material type
x = -150.0
y = 0.0
width = 50.0
height = 350.0
# No SOC tracking needed - geometry handles capacity

[[electrodes]]
name = "cathode"
material = "LFP"
x = 150.0
y = 0.0
width = 50.0
height = 350.0

[electrolyte]
molarity = 1.0
```

---

## GUI Enhancements

### Updated Visualization
- **SOC coloring by nearby Li count** - dynamic, no tracking needed
- Color gradient based on local lithium density
- Real-time updates as Li moves in/out

### Material Selection
- Material dropdown per electrode
- Equilibrium potential display
- No complex capacity/stoichiometry controls needed

---

## File Structure (Simplified)

```
src/
├── electrode/
│   ├── mod.rs              # Keep
│   ├── material.rs         # Keep (equilibrium potentials, colors)
│   ├── region.rs           # Simplify (remove lithium tracking)
│   └── intercalation.rs    # REMOVE (no longer needed)
├── simulation/
│   └── ... (existing files, no new intercalation.rs)
└── ... (existing structure)
```

---

## Implementation Checklist (Revised for Emergent Approach)

### Completed ✅
- [x] Species enum with electrode materials
- [x] Equilibrium potentials per species
- [x] Potential gating for Li plating
- [x] Potential gating for SEI formation
- [x] Electron hopping for all electrode materials
- [x] Basic SOC coloring infrastructure

### Phase 2: Thermodynamic Electron Donation (Next)
- [ ] Gate electron donation by material equilibrium potential
- [ ] Add `ELECTRON_DONATION_OVERPOTENTIAL` constant
- [ ] Test: LFP only donates electrons at cathode potentials
- [ ] Test: Graphite donates electrons at anode potentials

### Phase 3: SOC Visualization
- [ ] Implement `count_nearby_lithium()` function
- [ ] Update renderer to color by nearby Li count
- [ ] Remove `lithium_content` field from Body
- [ ] Test visualization reflects actual Li distribution

### Phase 4: Code Cleanup
- [ ] Remove `IntercalationConfig` struct
- [ ] Remove `IntercalationResult` / `DeintercalationResult` enums
- [ ] Remove `desolvation_probability()` function
- [ ] Remove `butler_volmer_rate()` for intercalation
- [ ] Remove `reaction_direction()` / `ReactionDirection`
- [ ] Remove `IntercalationStats` struct
- [ ] Simplify `ActiveMaterialRegion` (remove Li tracking)
- [ ] Remove unused methods from `MaterialType`
- [ ] Clean up dead code warnings

### Phase 5: Testing
- [ ] Test graphite half-cell
- [ ] Test LFP half-cell
- [ ] Test full cell (Graphite || LFP)
- [ ] Verify Li metal still works correctly
- [ ] Verify SEI only at anode

---

## Step 7: Electrochemical Potential Gating ✅ COMPLETE

**Solution:** Use local charge state to derive local electrochemical potential, then gate reactions by comparing local potential to reaction equilibrium potential.

**Key insight:** The explicit electron model already encodes local potential through charge:
- Excess electrons → negative charge → lower potential (more reducing)
- Electron deficit → positive charge → higher potential (more oxidizing)

**Implemented:**
- [x] `equilibrium_potential()` method on Body (per-species)
- [x] `local_potential_from_charge()` helper function
- [x] `local_potential()` method on Body
- [x] Gated Li⁺ → Li⁰ in `apply_redox()`
- [x] Gated SEI formation in `perform_sei_formation()`
- [x] Config constants: `LITHIUM_PLATING_POTENTIAL`, `SEI_FORMATION_POTENTIAL`, etc.

---

## Step 8: Emergent SOC Visualization (Replaces explicit tracking)

**Revised approach:** Instead of tracking `lithium_content` per particle, we dynamically count nearby lithium for visualization.

**Current state:**
- [x] `lithium_content` field exists (but will be replaced)
- [x] Renderer uses `lithium_content` for coloring

**Revised implementation:**
- [ ] Add `count_nearby_lithium(body_idx, bodies, cutoff) -> usize`
- [ ] Calculate local SOC = nearby_li_count / estimated_capacity
- [ ] Update renderer to use dynamic count instead of stored field
- [ ] Remove `lithium_content` field from Body struct
- [ ] Remove `lithium_content` initialization code

**Benefits:**
- No state synchronization needed
- Accurate representation of actual Li distribution
- Natural concentration gradients visible
- Simpler code

---

## Step 9: Thermodynamic Electron Donation Gating

**Goal:** Prevent electrons from leaving electrode materials unless thermodynamically favorable.

**Implementation:**
- [ ] In `attempt_electron_hop()`, check donor's equilibrium potential
- [ ] Electron donation allowed when: `local_potential < equilibrium_potential + threshold`
- [ ] Add `ELECTRON_DONATION_OVERPOTENTIAL` constant (~0.1V)

**Expected behavior:**
- LFP (3.4V) only donates electrons when local potential < ~3.5V
- Graphite (0.1V) donates electrons at anode potentials
- Li metal (0.0V) most readily donates electrons

This completes the thermodynamic consistency of the simulation.

---

## Next Steps

1. **Phase 2:** Implement thermodynamic electron donation gating
2. **Phase 3:** Implement nearby-lithium SOC visualization
3. **Phase 4:** Clean up unused intercalation code
4. **Phase 5:** Test with graphite/LFP electrodes
- Butler-Volmer in batteries: Newman & Thomas-Alyea, Electrochemical Systems (2004)
