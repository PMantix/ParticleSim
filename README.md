# Barnes-Hut N-Body Simulation (Rust)

This repository contains a modular, parallelized Barnes-Hut simulation for large-scale N-body systems in 2D, written in Rust. Its ultimate goal is to model electrochemical charging/discharging with explicit electron polarization and field-driven electron dynamics. Enjoy experimenting with state-of-the-art debug features and interactive controls!

---

## Features

- **Barnes-Hut Quadtree**: Efficient spatial partitioning for O(N log N) force calculations.
- **Parallel Simulation**: Utilizes Rayon for multi-threading.
- **Explicit Electron Polarization**:  
  - Lithium metal particles now include explicit valence electrons.
  - Electrons polarize in response to local electric fields (background plus inter-particle fields), providing a realistic visualization of charge separation.
- **Accurate Redox Transitions & Charge Conservation**:  
  - Lithium ions (Li⁺) and lithium metal (Li) update their species and charge according to electron count.
  - Electron hopping is implemented with strict conservation rules.
- **Live Force Tracking & Debugging**:  
  - Separate accumulation and visualization of Coulomb and Lennard-Jones (LJ) forces.
  - Debug prints (configurable via the GUI) display per-body force vectors, acceleration, and velocity.
- **Interactive GUI**:  
  - Real-time visualization and controls via [quarkstrom](https://github.com/DeadlockCode/quarkstrom).
  - **Manual Step Button**: Step the simulation forward by one timestep per click for precise debugging.
  - Particle selection prints detailed diagnostics (including position, velocity, acceleration, LJ and Coulomb forces) to the console.
  - Adjustable visualization overlays (e.g., velocity vectors, charge density, field isolines, and force ratio overlays).
- **Scenario Controls**:  
  - Quick simulation setups with commands to add circles, rectangles, and foil-based structures.
  - Easily clear all particles and experiment with various initial conditions.
- **Configurable Parameters**:  
  - All key physics constants and GUI visualization parameters are accessible and modifiable.
  - Experiment with different timestep settings and damping factors.
- **Extensible and Modular Codebase**:  
  - Well-structured separation of simulation, quadtree, rendering, and state management.
  - New force laws, diagnostic features, or spatial partitioning strategies can be added easily.

---

## Getting Started

1. **Install [Rust](https://www.rust-lang.org/tools/install)**
2. **Clone this repository:**
   ```sh
   git clone https://github.com/your-username/barnes-hut-simulation.git
   cd barnes-hut-simulation
   ```
3. **Build and run:**
   ```sh
   cargo run --release
   ```
   > **Note:** On non-Windows platforms, see [this issue](https://github.com/DeadlockCode/n-body/issues/1) for additional dependencies.

---

## Controls

- **Scrolling**: Zoom in/out.
- **Middle Mouse Button**: Pan the view.
- **Shift + Left Click**: Select a particle.  
  The console will print detailed diagnostics for the selected particle, including:
  - **ID, Position, Velocity, and Acceleration**
  - **LJ Force** and **Coulomb Force** vectors (tracked and updated separately).
- **+ / - Keys**: Increase or decrease the charge of the selected particle.
- **Escape**: Deselect the current particle.
- **Right Mouse Button**: Spawn a new body (with adjustable mass by moving the mouse).
- **Space**: Pause/resume simulation.
- **E**: Open the settings menu (toggle quadtree visualization, field isolines, etc.).

**Additional GUI Controls:**
- **Manual Step**:  
  A new "Step Simulation" button in the GUI allows you to advance the simulation by one timestep per click.
- **Scenario Setup**:  
  - **Delete All Particles**: Clears the simulation space immediately.
  - **Add Circle / Rectangle / Foil**: Spawn configured groups of particles quickly—ideal for rapid prototyping and testing.
- **Force Visualization Overlays**:  
  Toggle overlays for velocity vectors, charge density, and force ratio display.

---

## Project Structure

- **`src/renderer/`** — Rendering and GUI logic (includes the new diagnostic output and manual step button).
- **`src/quadtree/`** — Barnes-Hut quadtree spatial partitioning for efficiency.
- **`src/simulation.rs`** — Simulation logic and physics; integrates force laws and electron dynamics.
- **`src/body.rs`** — Contains the `Body` struct and methods; now supports explicit electron and polarization handling.
- **`src/config.rs`** — Configurable simulation parameters, including force constants and visualization options.
- **`src/main.rs`** — Entry point, handles threading, command processing, and the main simulation loop.

---

## Extending & Debugging

- **Force Tracking**:  
  The simulation maintains separate debug fields for LJ and Coulomb forces in each `Body`. These are accumulated during force calculations for later inspection.
- **Manual Stepping**:  
  Use the new GUI button to advance one timestep at a time for detailed debugging.
- **Adding New Features**:  
  Extend physics or visualization modules easily thanks to the modular code organization.
- **Testing**:
  Run the comprehensive test suite with:
  ```sh
  cargo test
  ```
- **Profiling**:
  Enable the built-in profiler with the `profiling` feature to print per-frame
  timings:
  ```sh
  cargo run --features profiling
  ```

---

## License

This project is licensed under the MIT License.

---

## Credits

- Based on the original Barnes-Hut algorithm.
- Inspired by [DeadlockCode's Barnes-Hut implementation](https://github.com/DeadlockCode/barnes-hut.git) and the [quarkstrom](https://github.com/DeadlockCode/quarkstrom) GUI framework.

Enjoy exploring the dynamics, tweaking the parameters, and watching the electrons dance!