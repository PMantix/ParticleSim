# ParticleSim - Electrochemical N-Body Simulation

This repository contains a modular, parallelized particle simulation for large-scale N-body systems in 2D, written in Rust. It models electrochemical processes with explicit electron polarization, field-driven electron dynamics, and real-time data analysis capabilities.

---

## Features

### Core Physics
- **Barnes-Hut Quadtree**: Efficient spatial partitioning for O(N log N) force calculations
- **Parallel Simulation**: Rayon-based multi-threading across available CPU cores
- **Explicit Electron Dynamics**:
  - Lithium metal particles include explicit valence electrons that drift and hop
  - Electrons polarize in response to local electric fields
  - Butler-Volmer kinetics for inter-species electron transfer
- **Accurate Redox Transitions**: Li⁺/Li species transitions with strict charge conservation
- **Polar Solvents**: EC, DMC, VC, FEC, and EMC with bound electrons and polarization forces
- **Solid Electrolytes**: LLZO, LLZT, and S40B scaffolds for interfacial studies, ceramic separators, and sulfide glass layers
- **Optional 2.5D Out-of-Plane**: Particles can temporarily move vertically to bypass 2D crowding

### Electrochemical Control
- **Switch-Charging System** (NEW):
  - 4-step cyclic switching between foil pairs for symmetric charging
  - **Global or per-step setpoints**: Configure once or customize each step
  - **Current mode**: Direct electron injection (e/fs) with sign polarity
  - **Overpotential mode**: PID voltage control with complementary targets (V and 2−V)
  - JSON import/export for configuration sharing
  - Real-time step monitoring and configuration validation
- **Foil Electrodes**:
  - Configurable structures that apply electric currents
  - Group linking (parallel or opposite modes)
  - Real-time current and overpotential monitoring

### Collision Dynamics
- **Soft Collision System** (NEW):
  - Per-species toggles for collision softness (default: Li⁺ only)
  - Anion soft collisions configurable independently
  - Softness factor scales correction forces (0.0 = hard, 1.0 = very soft)

### Visualization & Analysis
- **Advanced Field Visualization** (ENHANCED):
  - **Isoline controls**: Count, fidelity, adaptive refinement near crossings
  - **Percentile clipping**: Dynamic range with bias and margin adjustments
  - **Filled isobands**: Translucent bands between levels
  - **Nonlinear distribution**: Gamma warp for perceptual level spacing
  - **Color mapping**: Strength and gamma controls for contrast
- **Measurement Tool** (NEW):
  - Directional projection mode: Define an axis for aligned measurements
  - History recording with switch-charging metadata
  - CSV export with simulation context
- **Real-time Data Plotting**:
  - Time series plots (species populations, currents, hop rates)
  - Spatial profile plots (charge, velocity, field distributions)
  - Full domain binning regardless of particle locations
  - Export in CSV, JSON, or TSV formats
- **Interactive GUI**:
  - Manual step-by-step execution for debugging
  - Particle selection with detailed diagnostics
  - Screen capture with customizable regions
  - Playback controls with history scrubbing

#### Measurements & CSV Logging (Updated)
- Point-based CSV: records only the leading-edge position per measurement point. One row per timestep: `frame,time_fs,<label1>_edge,<label2>_edge,...`. Auto-named as `Point-based_*` under `doe_results/`.
- Foil-based CSV: one row per timestep with grouped columns for each foil (ordered by foil ID): `mode_f<ID>`, `setpoint_f<ID>`, `actual_ratio_f<ID>`, `delta_electrons_f<ID>`, `li_metal_count_f<ID>`. “Actual ratio” matches the foil electron ratio from the charging tab. Field toggles in the Measurements UI control which groups are populated.
- Time-based CSV: domain-wide solvation fractions (CIP, SIP, S2IP, FD) plus charging context (mode and setpoint/current). Auto-named as `Time-based_*` under `doe_results/`.
- All three CSV toggles live in the Measurements section and default to enabled. Filenames auto-name consistently and can be overridden in the UI.

### Workflow & Configuration
- **State Management**: Save/load simulation states for reproducible experiments
- **Scenario Presets**: Quick setup with pre-configured arrangements
- **Configurable Physics**: Runtime adjustment of LJ parameters, hopping rates, Butler-Volmer coefficients
- **Domain Control**: Adjustable bounds, timestep, force cutoffs

## Units

Lengths are measured in angstroms (Å), time in femtoseconds (fs), charge in
elementary charge (e) and mass in atomic mass units (amu). See
[docs/physics.md](docs/physics.md) for the complete unit system and conversion
guidelines.

---

## Getting Started

1. **Install [Rust](https://www.rust-lang.org/tools/install)**
2. **Clone this repository:**
   ```sh
   git clone https://github.com/your-username/ParticleSim.git
   cd ParticleSim
   ```
3. **Build and run:**
   ```sh
   cargo run --release
   ```

---

## Controls

### Mouse Controls
- **Scroll Wheel**: Zoom in/out
- **Middle Mouse Button + Drag** (or **Alt/Option + Left Drag** on trackpads): Pan the view
- **Shift + Left Click**: Select a particle for detailed diagnostics
- **Right Mouse Button**: Spawn a new body

### Keyboard Controls
- **Space**: Pause/resume simulation
- **+ / - Keys**: Increase/decrease charge of selected particle
- **Escape**: Deselect current particle
- **E**: Toggle settings menu

### GUI Features
- **Switch-Charging Tab**: Complete electrode control system
  - Electrode role assignments (+A, +B, -A, -B)
  - Global or per-step active/inactive setpoints
  - Current and overpotential modes
  - Real-time step indicators and dwell counters
  - JSON configuration import/export
- **Measurement Tab**: Precision distance tracking and CSV controls
  - Select start point and define directional axis
  - Live distance projection onto defined direction
  - History recording with switch-charging metadata
  - CSV export with simulation time and step context
  - Consolidated CSV controls: Enable Time-based / Foil-based / Point-based, with auto-named files
  - Editing the set of points auto-restarts Point-based recording to keep the CSV header in sync
- **Visualization Tab**: Advanced rendering controls
  - Isoline count, fidelity, and adaptive refinement
  - Filled isobands with alpha blending
  - Percentile clipping with bias and margin
  - Color strength and gamma for perceptual tuning
- **Soft Dynamics Tab**: Collision behavior tuning
  - Per-species toggles (Li⁺, anions)
  - Softness factor slider (0.0 = hard, 1.0 = very soft)
- **Plotting System**: Real-time data analysis
  - Time series plots for species populations and currents
  - Spatial profile plots for charge and velocity distributions
  - Export data in CSV, JSON, or TSV formats
- **Screen Capture**: Animation and documentation support
  - Periodic automatic capture with configurable intervals
  - Custom region selection and timestamp-based naming
- **Scenario Controls**: Quick setup with predefined particle arrangements
- **State Management**: Save and load complete simulation configurations
- **Manual Stepping**: Frame-by-frame execution for debugging

---

## Project Structure

- **`src/main.rs`** — Entry point and main simulation loop
- **`src/simulation/`** — Core simulation logic, force calculations, and electron dynamics
- **`src/body/`** — Particle definitions, species types, and electron management
- **`src/quadtree/`** — Barnes-Hut spatial partitioning for efficient force calculations
- **`src/renderer/`** — Visualization, GUI controls, and user interaction
- **`src/plotting/`** — Real-time data analysis and plotting system
- **`src/config.rs`** — Configurable simulation parameters and constants
- **`src/io.rs`** — State saving/loading functionality
- **`docs/physics.md`** — Physics documentation including default parameters

### Key Modules
- **Simulation Engine**: Handles particle dynamics, force calculations, and electron hopping
- **Plotting System**: Provides time series and spatial analysis with full-domain binning
- **Renderer**: GUI interface with interactive controls and visualization options
- **Quadtree**: Spatial optimization for large particle counts

## Data Analysis & Visualization

### Plotting System
The simulation includes a comprehensive plotting system for real-time data analysis:

- **Time Series Plots**:
  - Species population tracking (Li+, Li metal, anions)
  - Foil current monitoring
  - Electron hop rate analysis

- **Spatial Profile Plots**:
  - Charge distribution along X or Y axes
  - Velocity profiles across the simulation domain
  - Electron count distributions
  - Local field strength analysis

- **Full Domain Coverage**:
  - All spatial plots automatically bin data across the entire simulation domain (-350 to +350)
  - Empty regions are included in the analysis, not just areas with particles
  - Consistent binning ensures reproducible analysis

- **Data Export**:
  - Export plot data in CSV, JSON, or TSV formats
  - Timestamped files for data preservation
  - Metadata includes plot configuration and parameters

### Usage
1. Open the plotting window from the GUI
2. Select plot type (Time Series or Spatial Profile)
3. Choose quantity to analyze (charge, velocity, species count, etc.)
4. Configure sampling mode and update frequency
5. Export data for external analysis when needed

---

## Development & Debugging

- **Manual Stepping**: Use the GUI step button for frame-by-frame analysis
- **Particle Selection**: Click particles to view detailed diagnostics in console
- **State Management**: Save/load simulation states for reproducible experiments
- **Playback History**: Scrub through the recorded history buffer, adjust replay speed, and seamlessly resume live simulation. Save files now include frame, time, and thermostat metadata while remaining backward compatible.
- **Configurable Physics**: Modify parameters in real-time through the GUI
- **Comprehensive Testing**: Run the test suite with `cargo test`
- **Performance Profiling**: Enable with `cargo run --features profiling`
- **Thermostat Debug (Optional)**: Enable detailed thermostat logging (scales, per-frame temperature diagnostics) with `--features thermostat_debug`.

### Temperature Definitions
The simulation distinguishes between:
- **Liquid Temperature**: Computed over LithiumIon, ElectrolyteAnion, EC, DMC, VC, FEC, and EMC particles (center-of-mass drift removed). This represents the thermal state of the mobile electrolyte phase.
- **Global Temperature**: Computed over all dynamic bodies (excluding constrained metals’ bulk drift via COM subtraction). Useful for detecting runaway energy in non-thermostatted components.

Only liquid species (Li⁺, anion, EC, DMC, VC, FEC, EMC) are rescaled by the Maxwell–Boltzmann thermostat. Metals (LithiumMetal, FoilMetal) are excluded to preserve electrode dynamics. During bootstrap (very low initial KE) all liquid species are assigned randomized velocities to seed a Maxwellian distribution.

---

## License

This project is licensed under the MIT License.

---

## Credits

- Barnes-Hut algorithm implementation optimized for electrochemical simulations
- Built with [quarkstrom](https://github.com/DeadlockCode/quarkstrom) GUI framework
- Inspired by research in computational electrochemistry and particle dynamics

Explore the fascinating world of electrochemical processes at the particle level!