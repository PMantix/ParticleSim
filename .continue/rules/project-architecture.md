# ParticleSim Project Architecture

ParticleSim is a 2D particle-based lithium-ion battery simulation written in Rust. It models electrochemical behavior including ion transport, electron hopping, and electrode interactions at the molecular scale.

## Core Directory Structure

- `src/` - Main source code
  - `main.rs` - Entry point, launches the GUI application
  - `lib.rs` - Library root, exports all modules
  - `config.rs` - Centralized physics constants and simulation parameters
  - `species.rs` - Species properties (mass, radius, LJ parameters, colors)
  - `units.rs` - Unit conversion constants (eV, amu, Angstroms, femtoseconds)

## Key Modules

### `src/body/`
Particle (Body) representation and chemistry:
- `types.rs` - `Species` enum and `Body` struct with position, velocity, charge, electrons
- `electron.rs` - Electron struct for bound electrons on particles
- `redox.rs` - Local potential calculations for electrochemical reactions
- `foil.rs` - Foil-specific body helpers

### `src/simulation/`
Physics engine:
- `simulation.rs` - Main `Simulation` struct holding bodies, quadtree, config
- `forces.rs` - Coulomb (electric field), LJ, polar, and stack pressure forces
- `collision.rs` - Elastic/inelastic collision resolution
- `electron_hopping.rs` - Electron transfer between particles (Butler-Volmer kinetics)
- `thermal.rs` - Velocity-rescaling thermostat
- `intercalation.rs` - Li+ insertion/extraction in electrode materials
- `sei.rs` - SEI layer formation chemistry
- `history.rs` - Playback buffer for time-travel controls

### `src/quadtree/`
Barnes-Hut quadtree for O(N log N) force calculations:
- `quadtree.rs` - Main quadtree struct, `build()`, `field()`, `find_neighbors_within()`
- `node.rs` - Quadtree node with center-of-charge and bounding box

### `src/renderer/`
GPU rendering and GUI:
- `mod.rs` - Renderer coordination
- `state.rs` - `SimCommand` enum for GUI↔simulation communication
- `gui/` - egui-based tabs (physics, diagnostics, scenario, DOE, measurements)
- `draw/` - Particle and field visualization

### `src/app/`
Application lifecycle:
- `mod.rs` - App struct, initialization, main loop dispatch
- `command_loop.rs` - Handles `SimCommand` messages from GUI
- `simulation_loop.rs` - Physics update tick
- `spawn.rs` - Particle creation helpers (`add_random`, `add_foil`, `add_rectangle`)

### `src/doe/`
Design of Experiments framework for parameter sweeps

### `src/scenario.rs`
Scenario system for battery configurations (CC charge/discharge, GITT, etc.)

## Dependency: Quarkstrom

ParticleSim uses a local dependency `quarkstrom` (at `../quarkstrom/quarkstrom`) for GPU rendering. Quarkstrom provides:
- WebGPU-based particle rendering
- Window management and input handling
- egui integration for the GUI

## Key Types

```rust
// Particle species in the simulation
enum Species {
    LithiumIon,      // Li+ in electrolyte
    LithiumMetal,    // Li0 deposited metal
    FoilMetal,       // Current collector
    ElectrolyteAnion,// PF6- counter-ion
    EC, DMC, VC, FEC, EMC,  // Solvent molecules
    SEI,             // Solid electrolyte interphase
    // Electrode materials:
    Graphite, HardCarbon, SiliconOxide, LTO,  // Anodes
    LFP, LMFP, NMC, NCA,  // Cathodes
}

// Main particle struct
struct Body {
    pos: Vec2,       // Position (Angstroms)
    vel: Vec2,       // Velocity (Å/fs)
    mass: f32,       // Mass (amu)
    radius: f32,     // Radius (Å)
    charge: f32,     // Net charge (e)
    species: Species,
    electrons: SmallVec<[Electron; 2]>,  // Bound electrons
    lithium_content: f32,  // For intercalation electrodes (0-1)
}
```

## Physics Constants (in `config.rs`)

- Units: Angstroms (length), amu (mass), femtoseconds (time), elementary charge (charge)
- `DEFAULT_DT_FS = 5.0` - Timestep in femtoseconds
- `LJ_EPSILON_EV = 0.0103` - Lennard-Jones well depth
- `LJ_SIGMA_A = 1.80` - LJ equilibrium distance
- `LITHIUM_ION_THRESHOLD = 0.5` - Charge threshold for Li+/Li0 transition
- `HOP_RADIUS_FACTOR = 3.0` - Electron hopping search radius

## Build Commands

```bash
cargo build --release           # Build main binary
cargo run --release             # Run simulation GUI
cargo build --release --features doe  # Build with DOE runner
```
