# ParticleSim - Electrochemical N-Body Simulation

This repository contains a modular, parallelized particle simulation for large-scale N-body systems in 2D, written in Rust. It models electrochemical processes with explicit electron polarization, field-driven electron dynamics, and real-time data analysis capabilities.

---

## Features

- **Barnes-Hut Quadtree**: Efficient spatial partitioning for O(N log N) force calculations.
- **Parallel Simulation**: Utilizes Rayon for multi-threading across the available CPU cores.
- **Explicit Electron Dynamics**:  
  - Lithium metal particles include explicit valence electrons that can drift and hop between particles.
  - Electrons polarize in response to local electric fields, providing realistic charge separation visualization.
  - Butler-Volmer kinetics for electron transfer between species (configurable).
- **Accurate Redox Transitions & Charge Conservation**:  
  - Lithium ions (Li⁺) and lithium metal (Li) dynamically update their species based on electron count.
  - Electron hopping with strict conservation rules and distance-dependent rates.
- **Real-time Data Analysis & Plotting**:
  - **Time Series Plots**: Track species populations, foil currents, and electron hop rates over time.
  - **Spatial Profile Plots**: Analyze charge distribution, velocity profiles, and electron counts along X or Y axes.
  - **Full Domain Binning**: Spatial plots always span the entire simulation domain (-350 to +350), regardless of particle locations.
  - **Data Export**: Export plot data in CSV, JSON, or TSV formats for external analysis.
- **Interactive GUI**:  
  - Real-time visualization and controls via [quarkstrom](https://github.com/DeadlockCode/quarkstrom).
  - **Manual Step Button**: Step the simulation forward by one timestep for precise debugging.
  - Particle selection with detailed diagnostics printed to console.
  - Adjustable visualization overlays (velocity vectors, electron deficiency, etc.).
  - **Screen Capture**: Periodic screenshot recording with customizable regions and intervals for creating animations and documentation.
- **Foil Electrodes**:
  - Configurable foil structures that can apply electric currents to drive electrochemical reactions.
  - Real-time current monitoring and analysis.
- **State Management**:
  - Save and load simulation states for reproducible experiments.
  - Pre-configured scenarios for quick setup.
- **Configurable Physics**:  
  - Adjustable Lennard-Jones parameters per species, electron hopping rates, and Butler-Volmer coefficients. Each species can enable or disable LJ forces to model either "metal-like" cohesion or "liquid-like" behavior.
  - Domain bounds, timestep settings, and force cutoffs can be modified at runtime.

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
- **Middle Mouse Button + Drag**: Pan the view
- **Shift + Left Click**: Select a particle for detailed diagnostics
- **Right Mouse Button**: Spawn a new body

### Keyboard Controls
- **Space**: Pause/resume simulation
- **+ / - Keys**: Increase/decrease charge of selected particle
- **Escape**: Deselect current particle
- **E**: Toggle settings menu

### GUI Features
- **Plotting System**: Create real-time plots of simulation data
  - Time series plots for species populations and currents
  - Spatial profile plots for charge and velocity distributions
  - Export data in multiple formats (CSV, JSON, TSV)
- **Screen Capture**: Record simulation screenshots for animations and documentation
  - Periodic automatic capture with configurable intervals
  - Custom region selection for focused recording
  - Manual capture for specific moments
  - PNG output with timestamp-based naming
- **Scenario Controls**: Quick setup with predefined particle arrangements
- **State Management**: Save and load simulation configurations
- **Manual Stepping**: Advance simulation one timestep at a time for debugging

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
- **Configurable Physics**: Modify parameters in real-time through the GUI
- **Comprehensive Testing**: Run the test suite with `cargo test`
- **Performance Profiling**: Enable with `cargo run --features profiling`

---

## License

This project is licensed under the MIT License.

---

## Credits

- Barnes-Hut algorithm implementation optimized for electrochemical simulations
- Built with [quarkstrom](https://github.com/DeadlockCode/quarkstrom) GUI framework
- Inspired by research in computational electrochemistry and particle dynamics

Explore the fascinating world of electrochemical processes at the particle level!