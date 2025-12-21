# Multi-Electrode Materials Expansion Plan

## Executive Summary

This document outlines a phased approach to expanding ParticleSim from symmetric lithium metal electrodes to supporting a variety of anode and cathode active materials commonly used in lithium-ion batteries.

**Target Materials:**
- **Cathodes:** NMC (LiNiMnCoO₂), LFP (LiFePO₄), LMFP (LiMnFePO₄), NCA (LiNiCoAlO₂)
- **Anodes:** Graphite, Graphene, Hard Carbon, Silicon Oxide (SiOx), LTO (Li₄Ti₅O₁₂)

---

## Current Architecture Overview

### How Species/Materials Work Today

1. **Species Enum** ([src/body/types.rs](../src/body/types.rs#L13-L26))
   - All particle types defined in a single enum: `LithiumIon`, `LithiumMetal`, `FoilMetal`, electrolyte species, etc.
   - Each species has static properties defined in `SPECIES_PROPERTIES` HashMap

2. **Species Properties** ([src/species.rs](../src/species.rs))
   - `SpeciesProps` struct contains: mass, radius, damping, color, LJ parameters, polar properties, repulsion parameters
   - Properties can be overridden at runtime via GUI

3. **Electrochemical Behavior** ([src/body/redox.rs](../src/body/redox.rs))
   - `apply_redox()` handles Li⁺ ↔ Li metal conversions based on electron count
   - Species-specific charge calculations in `update_charge_from_electrons()`
   - Electron sea protection for bulk metal stability

4. **Electrode Structure** ([src/body/foil.rs](../src/body/foil.rs))
   - `Foil` struct manages current collectors (electron sources/sinks)
   - Contains body_ids of particles belonging to that electrode
   - Handles current control (DC/AC), overpotential control via PID

5. **Electron Transfer** ([src/simulation/electron_hopping.rs](../src/simulation/electron_hopping.rs))
   - Butler-Volmer kinetics for inter-species electron transfer
   - Currently limited to LithiumMetal ↔ LithiumIon ↔ FoilMetal transfers

---

## Key Differences Between Electrode Types

### Lithium Metal (Current Implementation)
- **Mechanism:** Plating/stripping - Li⁺ deposits as Li⁰ metal atoms
- **Structure:** Amorphous/polycrystalline surface growth
- **Capacity:** Theoretically unlimited (limited by Li supply)
- **Voltage:** 0 V vs Li/Li⁺ (reference)

### Intercalation Cathodes (NMC, LFP, LMFP, NCA)
- **Mechanism:** Li⁺ insertion/extraction from crystalline host lattice
- **Structure:** Layered (NMC, NCA), olivine (LFP, LMFP), or spinel frameworks
- **Capacity:** Fixed by stoichiometry (e.g., LiFePO₄ → FePO₄ + Li⁺ + e⁻)
- **Voltage:** Material-dependent (3.2-4.2V vs Li/Li⁺)
- **Key Physics:** 
  - Desolvation energy barrier at surface
  - Solid-state diffusion within host
  - Redox of transition metal (Fe²⁺↔Fe³⁺, Ni²⁺↔Ni⁴⁺, etc.)

### Intercalation Anodes (Graphite, Hard Carbon, LTO)
- **Mechanism:** Li⁺ intercalation between graphene layers or into disordered carbon
- **Structure:** Layered graphite, disordered carbon, or spinel (LTO)
- **Capacity:** LiC₆ (graphite) = 372 mAh/g, Hard carbon ~300-500 mAh/g
- **Voltage:** 0.1-0.2V (graphite), 1.55V (LTO) vs Li/Li⁺

### Alloying Anodes (Silicon Oxide)
- **Mechanism:** Li alloys with Si → Li₄.₄Si (high capacity, large volume change)
- **Capacity:** ~1500-2000 mAh/g (SiOx composites)
- **Challenges:** 300% volume expansion, SEI instability

---

## Proposed Architecture Changes

### Phase 1: Core Data Model Refactoring

#### 1.1 Introduce `ElectrodeRole` Concept

```rust
// New file: src/electrode/role.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElectrodeRole {
    Anode,
    Cathode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElectrodeMechanism {
    /// Li⁺ plates/strips as metallic Li⁰
    Plating,
    /// Li⁺ intercalates into host lattice
    Intercalation {
        max_stoichiometry: f32, // e.g., 1.0 for LiC₆, 1.0 for LiFePO₄
    },
    /// Li alloys with host material
    Alloying {
        max_li_per_host: f32, // e.g., 4.4 for Li₄.₄Si
        volume_expansion: f32, // fractional expansion at full lithiation
    },
}
```

#### 1.2 Expand Species Enum

```rust
// In src/body/types.rs - Phase 1 additions
pub enum Species {
    // === Existing ===
    LithiumIon,
    LithiumMetal,
    FoilMetal,
    ElectrolyteAnion,
    EC, DMC, VC, FEC, EMC,
    LLZO, LLZT, S40B,
    SEI,
    
    // === Phase 1: Graphite Anode ===
    Graphite,           // Delithiated graphite (C₆)
    LithiatedGraphite,  // Partially to fully lithiated (LiₓC₆)
    
    // === Phase 2: Cathode Materials ===
    LFP,     // LiFePO₄ (lithiated)
    FP,      // FePO₄ (delithiated)
    NMC,     // LiNi₀.₈Mn₀.₁Co₀.₁O₂ variants
    NMCDel,  // Delithiated NMC
    
    // === Phase 3: Additional Materials ===
    HardCarbon,
    LithiatedHardCarbon,
    SiliconOxide,
    LithiatedSiliconOxide,
    LTO,        // Li₄Ti₅O₁₂
    LithiatedLTO, // Li₇Ti₅O₁₂
    NCA,
    NCADel,
    LMFP,
    LMFPDel,
}
```

#### 1.3 Create `ElectrodeMaterial` Trait

```rust
// New file: src/electrode/material.rs
pub trait ElectrodeMaterial {
    /// Species when fully lithiated
    fn lithiated_species(&self) -> Species;
    
    /// Species when fully delithiated  
    fn delithiated_species(&self) -> Species;
    
    /// Equilibrium potential vs Li/Li⁺ as function of state of charge (0.0-1.0)
    fn open_circuit_voltage(&self, soc: f32) -> f32;
    
    /// Maximum Li stoichiometry (Li per formula unit)
    fn max_lithium_content(&self) -> f32;
    
    /// Desolvation energy barrier (eV)
    fn desolvation_barrier(&self) -> f32;
    
    /// Solid-state diffusion coefficient (Å²/fs)
    fn diffusion_coefficient(&self) -> f32;
    
    /// Exchange current density for Butler-Volmer kinetics
    fn exchange_current(&self) -> f32;
    
    /// Role (anode or cathode)
    fn role(&self) -> ElectrodeRole;
    
    /// Mechanism type
    fn mechanism(&self) -> ElectrodeMechanism;
}
```

### Phase 2: Intercalation Physics

#### 2.1 State of Charge Tracking

For intercalation electrodes, we need to track lithium content per electrode region:

```rust
// Extended Foil struct or new ActiveMaterial struct
pub struct ActiveMaterialRegion {
    pub id: u64,
    pub body_ids: Vec<u64>,
    pub material: Box<dyn ElectrodeMaterial>,
    
    /// Current lithium content (0.0 = empty, 1.0 = fully lithiated)
    pub state_of_charge: f32,
    
    /// Number of Li sites available
    pub lithium_capacity: usize,
    
    /// Number of Li currently in material
    pub lithium_count: usize,
    
    /// Surface area for rate calculations
    pub surface_area: f32,
}
```

#### 2.2 Desolvation Process

Li⁺ ions approaching an intercalation electrode must shed their solvation shell:

```rust
// New in src/simulation/desolvation.rs
impl Simulation {
    /// Check for Li⁺ ions near electrode surfaces and attempt desolvation
    pub fn perform_desolvation(&mut self) {
        for (i, body) in self.bodies.iter().enumerate() {
            if body.species != Species::LithiumIon {
                continue;
            }
            
            // Find nearby electrode surface particles
            let nearby_electrode = self.find_nearby_electrode_surface(i);
            if let Some((electrode_idx, material)) = nearby_electrode {
                let barrier = material.desolvation_barrier();
                let thermal_energy = self.config.thermal_energy_kt;
                
                // Arrhenius-type probability
                let prob = (-barrier / thermal_energy).exp() * self.dt;
                
                if rand::random::<f32>() < prob {
                    self.desolvate_and_intercalate(i, electrode_idx);
                }
            }
        }
    }
}
```

#### 2.3 Intercalation/Deintercalation

```rust
// New reactions for intercalation electrodes
impl Body {
    pub fn apply_intercalation_redox(&mut self, electrode: &mut ActiveMaterialRegion) {
        match electrode.material.mechanism() {
            ElectrodeMechanism::Intercalation { max_stoichiometry } => {
                // Li⁺ + e⁻ → Li(intercalated)
                // Updates SOC rather than creating new metal particles
                if self.species == Species::LithiumIon && !self.electrons.is_empty() {
                    if electrode.state_of_charge < max_stoichiometry {
                        electrode.lithium_count += 1;
                        electrode.state_of_charge = 
                            electrode.lithium_count as f32 / electrode.lithium_capacity as f32;
                        // Remove the Li⁺ from simulation (absorbed into electrode)
                        self.species = Species::Absorbed; // Or mark for removal
                    }
                }
            }
            // ... other mechanisms
        }
    }
}
```

### Phase 3: Voltage and Rate Expressions

#### 3.1 Open Circuit Voltage (OCV) Curves

Each material needs characteristic OCV curves:

```rust
// Example implementations
impl ElectrodeMaterial for GraphiteAnode {
    fn open_circuit_voltage(&self, soc: f32) -> f32 {
        // Graphite staging plateaus (simplified)
        match soc {
            x if x < 0.05 => 0.2 - 2.0 * x,  // Stage IV → III
            x if x < 0.25 => 0.12,            // Stage II plateau
            x if x < 0.50 => 0.10,            // Stage II → I transition  
            x if x < 0.95 => 0.08,            // Stage I plateau
            _ => 0.01,                         // Near full lithiation
        }
    }
}

impl ElectrodeMaterial for LFPCathode {
    fn open_circuit_voltage(&self, soc: f32) -> f32 {
        // LFP has very flat voltage profile
        3.4 + 0.05 * (soc - 0.5) // Slight tilt around 3.4V
    }
}

impl ElectrodeMaterial for NMCCathode {
    fn open_circuit_voltage(&self, soc: f32) -> f32 {
        // NMC has sloped voltage profile
        4.3 - 0.7 * soc // ~4.3V delithiated, ~3.6V lithiated
    }
}
```

#### 3.2 Butler-Volmer with Material Properties

Extend existing B-V kinetics to use material-specific parameters:

```rust
fn calculate_transfer_rate(
    material: &dyn ElectrodeMaterial,
    overpotential: f32,
    temperature: f32,
) -> f32 {
    let i0 = material.exchange_current();
    let alpha = 0.5; // Transfer coefficient
    let f = 96485.0 / (8.314 * temperature); // F/RT
    
    i0 * ((alpha * f * overpotential).exp() 
        - ((1.0 - alpha) * f * overpotential).exp())
}
```

---

## Implementation Phases

### Phase 1: Graphite Anode (Recommended Starting Point)
**Why start here:**
- Graphite is the most common anode material
- Well-understood intercalation physics
- Can validate intercalation framework before cathodes
- Pairs naturally with existing Li metal counter electrode

**Tasks:**
1. Add `Graphite` and `LithiatedGraphite` species
2. Create `GraphiteAnode` material implementation
3. Add SOC tracking to electrode regions
4. Implement basic intercalation (Li⁺ absorption at surface)
5. Add deintercalation (Li release during discharge)
6. Visualize SOC via particle color gradient

**Estimated complexity:** Medium (2-3 weeks)

### Phase 2: LFP Cathode
**Why second:**
- Simple olivine structure with flat voltage
- Well-characterized kinetics
- Provides first full cell (Graphite||LFP)

**Tasks:**
1. Add `LFP` and `FP` species
2. Create `LFPCathode` material implementation
3. Implement cathode-specific electron flow
4. Add voltage-based current control
5. Test full cell cycling

**Estimated complexity:** Medium (2-3 weeks)

### Phase 3: NMC Cathode
**Why third:**
- Most commercially important cathode
- More complex voltage profile tests OCV implementation
- Multiple Ni/Co/Mn ratios possible

**Tasks:**
1. Add `NMC` and `NMCDel` species  
2. Implement sloped OCV curve
3. Add composition variants (NMC811, NMC622, etc.)

**Estimated complexity:** Medium (2 weeks)

### Phase 4: Alternative Anodes
- Hard Carbon (similar to graphite, different OCV)
- Silicon Oxide (alloying mechanism, volume expansion)
- LTO (high-rate, different voltage window)

### Phase 5: Additional Cathodes
- NCA (similar to NMC)
- LMFP (Mn-doped LFP)

---

## Configuration Changes

### Extended init_config.toml

```toml
[simulation]
domain_width = 600.0
domain_height = 400.0

[[electrodes]]
name = "anode"
material = "Graphite"
role = "anode"
x = -150.0
y = 0.0
width = 50.0
height = 350.0
initial_soc = 1.0  # Start fully lithiated

[[electrodes]]
name = "cathode"
material = "LFP"
role = "cathode"
x = 150.0
y = 0.0
width = 50.0
height = 350.0
initial_soc = 0.0  # Start delithiated (discharged state)

[electrolyte]
molarity = 1.0
salt = "LiPF6"
solvents = [
    { species = "EC", volume_fraction = 0.3 },
    { species = "DMC", volume_fraction = 0.7 },
]
```

### Material Database

Consider a TOML-based material property database:

```toml
# materials.toml
[graphite]
display_name = "Graphite (Synthetic)"
role = "anode"
mechanism = "intercalation"
max_capacity_mah_g = 372
density_g_cm3 = 2.26
particle_radius_a = 2.0
diffusion_coeff = 1e-10  # cm²/s
exchange_current = 0.001

[lfp]
display_name = "LiFePO₄"
role = "cathode"
mechanism = "intercalation"
max_capacity_mah_g = 170
density_g_cm3 = 3.6
nominal_voltage = 3.4
particle_radius_a = 2.5
```

---

## GUI Enhancements

### New Electrode Material Tab
- Material selection dropdown (Graphite, LFP, NMC, etc.)
- Role assignment (anode/cathode)
- SOC indicator with color visualization
- OCV display based on current SOC
- Capacity utilization metrics

### Enhanced Visualization
- Color gradient for SOC (e.g., gold→black for graphite lithiation)
- Separate electrode region highlighting
- Real-time voltage display
- Current/capacity rate visualization

---

## Testing Strategy

### Unit Tests
- OCV curve accuracy for each material
- SOC tracking during intercalation/deintercalation
- Butler-Volmer rate calculations
- Desolvation probability

### Integration Tests
- Half-cell cycling (Li metal vs. graphite)
- Full-cell cycling (graphite vs. LFP)
- Rate capability tests
- Long-term stability

### Validation
- Compare simulated voltage profiles to experimental data
- Verify correct stoichiometry limits
- Check energy conservation

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance degradation with more species | Medium | High | Profile early, optimize hot paths |
| Complex species interactions | High | Medium | Thorough unit testing, staged rollout |
| OCV accuracy issues | Medium | Medium | Use validated empirical fits |
| Breaking existing Li metal simulations | Medium | High | Feature flags, extensive regression tests |
| GUI complexity explosion | Medium | Medium | Progressive disclosure, tabs for advanced features |

---

## File Structure Proposal

```
src/
├── electrode/
│   ├── mod.rs
│   ├── material.rs         # ElectrodeMaterial trait
│   ├── role.rs             # ElectrodeRole enum
│   ├── graphite.rs         # Graphite implementation
│   ├── lfp.rs              # LFP implementation
│   ├── nmc.rs              # NMC implementation
│   ├── silicon.rs          # Silicon oxide implementation
│   └── database.rs         # Material property database
├── simulation/
│   ├── intercalation.rs    # Intercalation physics
│   ├── desolvation.rs      # Desolvation barriers
│   └── ... (existing files)
└── ... (existing structure)
```

---

## Implementation Checklist

### Sketched Code (Created)
- [x] `src/electrode/mod.rs` - Module structure
- [x] `src/electrode/material.rs` - MaterialType enum with OCV curves, colors, kinetics
- [x] `src/electrode/region.rs` - ActiveMaterialRegion with SOC tracking
- [x] `src/electrode/intercalation.rs` - Desolvation and Butler-Volmer physics
- [x] `src/electrode/INTEGRATION_SKETCH.rs` - Integration reference

### Step 1: Wire Up Module
- [ ] Add `pub mod electrode;` to `src/lib.rs`
- [ ] Run `cargo check` to verify module compiles

### Step 2: Add Simulation Fields
- [ ] Add `active_regions: Vec<ActiveMaterialRegion>` to Simulation struct
- [ ] Add `body_to_region: HashMap<u64, u64>` for lookup
- [ ] Add `intercalation_config: IntercalationConfig`
- [ ] Initialize in `Simulation::new()`

### Step 3: Implement perform_intercalation()
- [ ] Add `count_nearby_solvent()` helper
- [ ] Add `get_foil_potential()` helper (or integrate with existing)
- [ ] Implement main intercalation loop
- [ ] Handle particle removal (absorbed Li⁺)
- [ ] Handle Li⁺ spawning (deintercalation)
- [ ] Call from `Simulation::step()`

### Step 4: Renderer Integration
- [ ] Sync `active_regions` to Renderer
- [ ] Modify body coloring to check `body_to_region`
- [ ] Use `region.current_color()` for SOC visualization

### Step 5: Commands and Configuration
- [ ] Add `CreateActiveMaterialRegion` SimCommand
- [ ] Handle command in simulation
- [ ] Add electrode config to `init_config.toml` schema
- [ ] Parse and create regions in scenario loading

### Step 6: Test with Graphite
- [ ] Create test scenario: Li metal vs Graphite half-cell
- [ ] Verify Li⁺ absorption at graphite surface
- [ ] Verify SOC increases and color changes
- [ ] Verify capacity limit works
- [ ] Verify deintercalation when polarity reverses

### Future Steps
- [ ] Add GUI tab for material selection
- [ ] Add LFP cathode configuration
- [ ] Test full cell (Graphite || LFP)
- [ ] Add remaining materials (NMC, NCA, etc.)

### Step 7: Electrochemical Potential Gating (Critical Physics Fix)

**Problem:** Currently, reactions like Li plating and SEI formation happen based purely on electron availability, ignoring thermodynamic favorability. This causes Li plating/SEI at cathode potentials (~3.4V) where they're impossible.

**Solution:** Use local charge state to derive local electrochemical potential, then gate reactions by comparing local potential to reaction equilibrium potential.

**Key insight:** The explicit electron model already encodes local potential through charge:
- Excess electrons → negative charge → lower potential (more reducing)
- Electron deficit → positive charge → higher potential (more oxidizing)

**Implementation:**

- [ ] Add `equilibrium_potential()` method to Species
  ```rust
  Species::LithiumMetal => 0.0,   // Li⁺/Li reference
  Species::Graphite => 0.1,       // Graphite intercalation  
  Species::LFP => 3.4,            // LiFePO₄
  Species::SEI => 0.8,            // EC reduction threshold
  // etc.
  ```

- [ ] Add `local_potential_from_charge()` helper function
  ```rust
  // Map charge to potential: negative charge → low potential, positive → high
  fn local_potential_from_charge(charge: f32) -> f32 {
      const POTENTIAL_PER_CHARGE: f32 = 2.0;  // tunable
      const BASELINE_POTENTIAL: f32 = 2.0;    // V at neutral
      BASELINE_POTENTIAL + charge * POTENTIAL_PER_CHARGE
  }
  ```

- [ ] Gate `apply_redox()` in `src/body/redox.rs`
  - Li⁺ + e⁻ → Li⁰ only if `local_potential < LITHIUM_PLATING_THRESHOLD` (~0V + small overpotential)
  - Prevents lithium plating at cathode potentials

- [ ] Gate `perform_sei_formation()` in `src/simulation/sei.rs`  
  - SEI formation only if `local_potential < SEI_FORMATION_THRESHOLD` (~0.8V for EC)
  - Prevents SEI at cathode

- [ ] Gate intercalation/deintercalation by material OCV
  - Intercalation favorable when `local_potential < material.ocv(soc)`
  - Deintercalation favorable when `local_potential > material.ocv(soc)`

- [ ] Add config constants for thresholds (tunable via GUI)
  ```rust
  pub const LITHIUM_PLATING_POTENTIAL: f32 = 0.0;
  pub const SEI_FORMATION_POTENTIAL: f32 = 0.8;
  pub const POTENTIAL_PER_CHARGE: f32 = 2.0;
  pub const BASELINE_POTENTIAL: f32 = 2.0;
  ```

**Expected behavior after implementation:**
| Reaction | Equilibrium | Where it should happen |
|----------|-------------|------------------------|
| Li⁺ + e⁻ → Li⁰ | 0V | Only at anode (low potential) |
| Solvent → SEI | ~0.8V | Only at anode |
| Li⁺ → Li(graphite) | 0.1V | Only at anode during charge |
| Li⁺ → Li(LFP) | 3.4V | At cathode during discharge |

### Step 8: Lithium Content Tracking & Intercalation Transfer

**Context:** Added `lithium_content: f32` field to Body struct for tracking intercalated lithium (0.0 = delithiated, 1.0 = fully lithiated). SOC-based coloring now uses this field. Initial values set at spawn (cathodes start lithiated, anodes delithiated).

**Implementation:**

- [x] Add `lithium_content` field to Body struct
- [x] Initialize `lithium_content` at spawn based on species (cathodes=1.0, anodes=0.0)
- [x] Update renderer to use `lithium_content` for SOC coloring
- [ ] Implement lithium transfer during intercalation:
  - When Li⁺ is absorbed into electrode particle → increase `lithium_content`
  - When Li⁺ is released from electrode → decrease `lithium_content`
  
- [ ] Add `perform_lithium_intercalation()` to Simulation
  ```rust
  // Detect Li⁺ near electrode surface
  // If local potential favorable (see Step 7), attempt intercalation:
  //   - Remove Li⁺ from simulation (absorbed)
  //   - Increase electrode body's lithium_content
  //   - Transfer electron to electrode
  ```

- [ ] Add `perform_lithium_deintercalation()` to Simulation
  ```rust
  // For electrodes with lithium_content > 0 and favorable potential:
  //   - Spawn Li⁺ near electrode surface
  //   - Decrease electrode body's lithium_content  
  //   - Remove electron from electrode (or allow hopping)
  ```

- [ ] Add capacity limits per electrode material
  ```rust
  // Each electrode body has max Li capacity based on material
  // lithium_content is normalized 0.0-1.0
  // Actual Li count = lithium_content * max_capacity(material)
  ```

- [ ] Connect to GUI SOC slider
  - Initial SOC slider sets `lithium_content` at spawn time
  - Allow different initial SOC for anode vs cathode

**Physics:**
- Intercalation: Li⁺(electrolyte) + e⁻(electrode) → Li(intercalated)
- Deintercalation: Li(intercalated) → Li⁺(electrolyte) + e⁻(electrode)
- Rate depends on: overpotential, material kinetics, desolvation barrier, local Li⁺ concentration

---

## Next Steps

1. **Review sketched code** - Check `src/electrode/` files
2. **Create feature branch** - `feature/multi-electrode-materials`
3. **Wire up module** - Add to lib.rs and cargo check
4. **Implement Step 2-3** - Core simulation integration
5. **Test with graphite half-cell** - Validate intercalation works

---

## References

- Graphite staging: Dahn et al., Phys. Rev. B 44, 9170 (1991)
- LFP kinetics: Malik et al., Nature Materials 10, 587 (2011)
- NMC voltage profiles: Jung et al., J. Electrochem. Soc. 164, A1361 (2017)
- Butler-Volmer in batteries: Newman & Thomas-Alyea, Electrochemical Systems (2004)
