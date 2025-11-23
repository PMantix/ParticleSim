# Codex Agent Instructions

This repository is a Rust project that simulates electrochemical particle systems with advanced visualization and control features.

## Recent Major Features (October 2025)

### Switch-Charging System
- **Location**: `src/switch_charging/mod.rs`, applied in `src/simulation/simulation.rs`
- **Purpose**: 4-step cyclic electrode switching for symmetric battery charging
- **Key Concepts**:
  - **Global vs Per-Step Setpoints**: Configure active/inactive control once (global) or customize each step
  - **Complementary Overpotential**: In overpotential mode, positive foils get target V, negative foils get 2−V
  - **Legacy Step Setpoints**: Disabled/grayed when Active/Inactive mode is enabled (avoid confusion)
- **UI**: Complete tab in GUI with role assignment, mode selection, JSON import/export

### Soft Collision System
- **Location**: `src/simulation/collision.rs`, config in `src/config.rs`, UI in `src/renderer/gui/soft_dynamics_tab.rs`
- **Purpose**: Per-species toggles for collision softness (reduce hard corrections for specified species)
- **Defaults**: Li⁺ softness enabled, anions disabled
- **Softness Factor**: 0.0 = hard collisions (unchanged), 1.0 = fully suppressed (not recommended)

### SEI Formation Controls
- **Location**: `src/simulation/sei.rs::perform_sei_formation`, config in `src/config.rs`
- **Electron Bookkeeping**: `sei_electrons_per_event` decrements the supplying metal’s excess electrons for every solvent that reduces to SEI.
- **Kinetic Thresholds**: `sei_charge_threshold_{vc,fec,ec,emc,dmc}` express the |e| charge required on a local metal patch before that solvent is eligible (VC < FEC < EC < EMC < DMC).
- **Growth Geometry**: `sei_radius_scale` inflates the resulting SEI radius relative to the parent solvent (minimum clamp at the SEI species radius) to mimic polymeric buildup.

### Measurement Tool
- **Location**: `src/renderer/mod.rs`, `src/renderer/gui/measurement_tab.rs`, `src/renderer/input.rs`
- **Purpose**: Precision distance tracking with directional projection
- **Features**:
  - Select start point, optionally define measurement axis
  - Live distance projection onto defined direction
  - History recording with switch-charging metadata (step, mode, value)
  - CSV export with simulation time and context

### Advanced Visualization Controls
- **Location**: `src/renderer/gui/visualization_tab.rs`, `src/renderer/draw/field.rs`
- **Isoline Enhancements**:
  - Count, fidelity (target samples), adaptive refinement around crossings
  - Percentile clipping with bias and margin adjustments
  - Filled isobands with translucent alpha blending
  - Nonlinear level distribution (gamma warp for perceptual spacing)
  - Color strength and gamma for contrast tuning

## Thermostat & Temperature Notes
- "Liquid temperature" includes LithiumIon, ElectrolyteAnion, EC, DMC, VC, FEC, and EMC (COM drift removed)
- Maxwell–Boltzmann thermostat rescales only liquid species (metals excluded) every configured interval
- Detailed thermostat diagnostics gated behind cargo feature `thermostat_debug`:
  - Enable via: `cargo run --features thermostat_debug --release`
  - Provides scaling factors, per-frame liquid KE, and sampling of representative particles

## Performance Notes
- History system uses ring buffer (configurable capacity via `config::PLAYBACK_HISTORY_FRAMES`)
- Compressed history system implemented (`src/simulation/compressed_history.rs`) but not yet activated
- For large simulations (>10k particles), consider reducing history capacity or enabling delta compression
- Hot paths: force calculations (`src/simulation/forces.rs`), collisions (`src/simulation/collision.rs`), switch-charging application

## Programmatic Checks
- Run `cargo check` before committing changes to verify the project compiles
- Run `cargo build --release` for performance testing (debug builds are 10-30× slower)
- The `cargo test` suite relies on features that do not function in the Codex environment, so it is unnecessary to run `cargo test` here

## Module Guides
Each major module under `src/` has its own `AGENT.md` file describing the module and key files.

## Key Configuration Files
- `init_config.toml`: Initial particle setup (circles, rectangles, random spawn)
- `src/config.rs`: Physics constants and runtime-configurable parameters
- `src/species.rs`: Per-species properties (mass, radius, LJ parameters, colors)
