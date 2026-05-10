# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo build --release              # Build optimized binary
cargo run --release                # Run GUI simulation
cargo test --features unit_tests   # Run unit tests
cargo test --features unit_tests -- --nocapture  # Tests with stdout
cargo fmt                          # Format code
cargo clippy                       # Lint
cargo run --release --features doe --bin doe_runner  # DOE parameter sweeps
```

Feature flags: `profiling`, `debug_quadtree`, `thermostat_debug`, `command_debug`, `doe`, `unit_tests`, `debug_bins`

Debug binaries (require `debug_bins` feature):
```bash
cargo run --release --features debug_bins --bin minimal_test
```

To force-rebuild the Quarkstrom dependency: `FRESH_DEPS=1 cargo build --release`

## Architecture

### Threading Model
`main.rs` → `app::run()` sets up two threads communicating via mpsc channel:
- **Main thread**: Quarkstrom renderer + egui GUI
- **Simulation thread**: Physics loop (`simulation_loop.rs`) receives `SimCommand` messages from GUI

### Core Data Flow
1. GUI emits `SimCommand` variants (defined in `src/renderer/state.rs`)
2. `command_loop.rs` dispatches commands, updating `SimConfig` and `Simulation` state
3. `Simulation::step()` runs the physics pipeline each tick

### Physics Pipeline (per step in `simulation/simulation.rs`)
1. **Quadtree build** → O(N log N) Coulomb forces (`forces.rs`)
2. **LJ + polar forces** (`forces.rs`)
3. **Collision resolution** via broccoli spatial tree, 7 passes (`collision.rs`)
4. **Electron hopping** with Butler-Volmer kinetics (`electron_hopping.rs`)
5. **SEI formation** from solvent reduction (`sei.rs`)
6. **Thermostat** — Maxwell-Boltzmann rescaling on liquid species only (`thermal.rs`)
7. **Intercalation** — Li+ insertion/extraction in electrodes (`intercalation.rs`)

### Key Types
- **`Body`** (`body/types.rs`): Particle with pos/vel/charge/species and bound `Electron`s in a `SmallVec<[Electron; 2]>`
- **`Species`** (`body/types.rs`): Enum covering ions, metals, solvents (EC/DMC/VC/FEC/EMC), solid electrolytes (LLZO/LLZT/S40B), electrode materials (Graphite/LFP/NMC/etc.), and SEI
- **`Simulation`** (`simulation/simulation.rs`): Owns `Vec<Body>`, `Quadtree`, `CellList`, `SimConfig`, `SwitchScheduler`, history buffer
- **`SimCommand`** (`renderer/state.rs`): Enum for all GUI→simulation messages
- **`SimConfig`** (`config.rs`): All tunable physics constants and parameters

### Module Responsibilities
- **`src/app/`** — App lifecycle, thread spawning, command dispatch, particle spawning
- **`src/simulation/`** — Physics engine (forces, collisions, electron hopping, thermostat, SEI, intercalation)
- **`src/body/`** — Particle/electron representation, redox potential, species transitions
- **`src/quadtree/`** — Barnes-Hut tree for force calculation and neighbor queries
- **`src/renderer/`** — Quarkstrom GPU rendering, egui GUI tabs, input handling, screen capture
- **`src/renderer/gui/`** — Individual egui tabs (physics, diagnostics, electrodes, charging, measurements, etc.)
- **`src/plotting/`** — Real-time time-series and spatial-profile plots
- **`src/switch_charging/`** — 4-step cyclic electrode switching system
- **`src/doe/`** — Design of Experiments framework; results go to `doe_results/`
- **`src/species.rs`** — Per-species properties (mass, radius, color, LJ params)
- **`src/config.rs`** — Physics constants, defaults, `SimConfig` struct
- **`src/units.rs`** — Unit conversion constants (Å, fs, amu, e)

### Quarkstrom Dependency
Local GPU rendering framework at `../quarkstrom/quarkstrom`. Provides WebGPU rendering, winit window management, and egui integration. The build script (`build.rs`) monitors it for changes.

## Units
Lengths in angstroms (Å), time in femtoseconds (fs), mass in amu, charge in elementary charge (e). Conversion constants in `src/units.rs`.

## Adding a GUI Control
1. Add field with `#[serde(default)]` to `SimConfig` in `config.rs`
2. Add `SimCommand` variant in `renderer/state.rs`
3. Handle command in `app/command_loop.rs`
4. Add widget in the appropriate `renderer/gui/*_tab.rs`

## Adding a New Species
1. Add variant to `Species` enum in `body/types.rs`
2. Add properties in `species.rs` `SPECIES_PROPERTIES` map
3. Add neutral electron count in `config.rs`
4. Update `Body::update_species()` for auto-conversion rules
5. Update electron hopping rules in `simulation/electron_hopping.rs`

## Headless Binaries (`src/bin/`)
Measurement and validation binaries that run without the GUI. Auto-discovered by cargo (no Cargo.toml entry needed).

Key binaries:
- `e1_baseline_dcr` — Potentiostatic DCR: rest→pulse→rest. Args: `<ratio> [pulse_ps] [snap_dir]`
- `e2_repeatability` — 3 consecutive pulses on same cell with snapshots. Args: `<ratio> [snap_dir]`
- `potentiostatic_sweep` — Sweep voltage steps, measure steady-state current
- `steady_state_search` — Galvanostatic sweep across electrode sizes/currents
- `tafel_slope` — Galvanostatic Tafel sweep, recover BV transfer coefficient α
- `nernst_einstein` — Haven ratio from MSD + drift velocity
- `physics_invariants` — Phase 1 invariant tests (charge balance, energy drift, etc.)
- `bfs_debug` — Verify electrode geometry and BFS body count
- `ion_motion_diag` — Track ion kinematics step-by-step

### Spawning geometry in headless binaries
`SimCommand::AddRectangle` and `SimCommand::AddFoil` interpret `x, y` as **bottom-left origin**, not center. The TOML loader converts center→origin via `to_origin_coords()`. Headless binaries must do the same: `origin = center - size/2`. Verify with `bfs_debug` that electrode BFS finds the expected body count.

## Coordinate with South
South is a compute node (Ryzen 7950X3D) running batch jobs. Communication:
- **LAN**: `http://192.168.1.184:8765` (token in `.doe_token`), via `scripts/north_p2p.sh`
- **Git fallback**: `coordination/north_to_south.jsonl` on `feature/eis-amplitude-study`
- South polls origin every 15s; push jobs via either channel
- Results land in `doe_results/` subdirectories; South commits and pushes

## Important Physics Notes
- **Li⁺ excluded from thermostat** — ion velocity is Coulomb-driven. Only solvents + anions are thermostated (`thermal.rs`, `utils.rs`).
- **Reduction snap** — when Li⁺ reduces to Li metal via electron hop, the new metal is placed adjacent to the donor surface atom. Lost momentum distributed to nearby liquid particles (`electron_hopping.rs`).
- **Species lock** — `Body.species_lock_until` (default 10 fs) prevents ping-pong oscillation between metal↔ion states.
- **Foil current signs** — `+current` adds electrons to foil (deposition side), `-current` removes electrons (stripping side). Paired transfers only fire when one foil has surplus and another has deficit.
- **Electrode η measurement** — use `calculate_foil_electron_ratio()` (BFS from foil through connected cluster), not foil-body charge alone. The foil-only measurement misses charge that hopped into the metal cluster.
- **Potentiostatic > galvanostatic for DCR** — galvanostatic at particle scale produces ~10¹¹ C-rates. Use overpotential mode for impedance measurements.

## Coding Conventions
- `parking_lot::Mutex` over `std::sync::Mutex`
- `SmallVec` for small fixed-size collections (e.g., bound electrons)
- `#[inline]` on small, hot functions
- Use `profile_scope!` macro when `profiling` feature is enabled
- Physics constants go in `config.rs`, species properties in `species.rs`
- Unit tests gated behind `#[cfg(feature = "unit_tests")]`; integration tests in `debug/` directory
