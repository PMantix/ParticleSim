# Physics Model Reference

## Unit System

ParticleSim uses a consistent unit system defined in `src/units.rs`:

| Quantity   | Unit              | Symbol |
|------------|-------------------|--------|
| Length     | Angstrom          | Å      |
| Mass       | Atomic mass unit  | amu    |
| Time       | Femtosecond       | fs     |
| Charge     | Elementary charge | e      |
| Energy     | Simulation units  | ~eV    |

Conversion factors:
- `EV_TO_SIM`: electronvolts to simulation energy units
- `BOLTZMANN_SIM`: Boltzmann constant in simulation units
- `COULOMB_CONST_SIM`: Coulomb constant in simulation units

## Force Calculations

### Coulomb (Electric) Forces
- Computed via Barnes-Hut quadtree (O(N log N))
- `theta` parameter controls accuracy vs speed tradeoff
- Softening parameter `QUADTREE_EPSILON` prevents singularities

### Lennard-Jones Forces
- Applied between metal particles (LithiumMetal, FoilMetal) for cohesion
- Parameters: `LJ_EPSILON_EV = 0.0103 eV`, `LJ_SIGMA_A = 1.80 Å`
- Cutoff: `LJ_CUTOFF_A = 2.2 Å`
- Provides attractive well + repulsive core

### Collision Resolution
- Hard-sphere elastic collisions
- Multiple passes (`COLLISION_PASSES = 7`) for stability
- Uses broccoli spatial tree for broad-phase

### Polar Forces
- EC and DMC solvent molecules have bound electrons
- Electron can drift relative to nucleus creating dipole
- Field gradient on dipole creates solvation forces

## Electron Dynamics

### Electron Hopping
Butler-Volmer kinetics for electron transfer between particles:
- Hop radius: `HOP_RADIUS_FACTOR * body.radius` (~3× radius)
- Transfer coefficient `α = 0.5`
- Rate depends on local overpotential

### Electron Sea Protection
Metals surrounded by other metals resist oxidation:
- `SURROUND_RADIUS_FACTOR = 3.5` (check radius)
- `SURROUND_NEIGHBOR_THRESHOLD = 4` (metal neighbors needed)

## Species Transitions

### LithiumIon ↔ LithiumMetal
- `charge > LITHIUM_ION_THRESHOLD (0.5)` → LithiumIon
- `charge <= 0.0` → LithiumMetal
- Controlled by electron hopping and electrochemical potential

### SEI Formation
- Solvent molecules (EC, VC, FEC, DMC, EMC) can nucleate SEI
- Each species has different `sei_charge_threshold`
- Consumes electrons, forms immobile SEI particles

## Thermodynamics

### Thermostat
- Velocity-rescaling thermostat in `simulation/thermal.rs`
- Target temperature configurable via GUI
- Foil particles optionally excluded from thermostat

### Stack Pressure (Optional)
- Uniaxial boundary pressure from left/right edges
- Simulates mechanical stack pressure in battery cells
- Parameters: `stack_pressure`, `stack_pressure_decay`

## Intercalation Electrodes

Electrode materials (Graphite, LFP, NMC, etc.) support Li+ insertion:
- `lithium_content` field tracks state of charge (0.0-1.0)
- Different electrode potentials vs Li/Li+ for each material
- Intercalation reactions gated by local potential
