# Barnes-Hut N-Body Simulation (Rust)

This repository contains a modular, parallelized, and extensible implementation of the Barnes-Hut algorithm for simulating large-scale N-body systems in 2D, written in Rust.  
It is designed for high performance, clarity, and ease of extension, with a focus on modern Rust best practices and code organization.
End goal is to develop a particle based simulator of electrochemical charging and discharging processes, **now with explicit electron polarization and field-driven electron dynamics**.

---

## Features

- **Barnes-Hut Quadtree**: Efficient spatial partitioning for O(N log N) force calculation.
- **Parallel Simulation**: Uses Rayon for multi-threaded computation.
- **Explicit Electron Polarization**: Lithium metal particles now have explicit valence electrons that polarize in response to the net electric field (background + all other charges), visualizing local charge separation and field effects.
- **Physically Accurate Field Handling**: Electron polarization is based on the true net field at each lithium metal atom, not just background or local random motion.
- **Modular Codebase**: Clean separation of simulation, quadtree, rendering, and state management.
- **Interactive GUI**: Real-time visualization and controls via [quarkstrom](https://github.com/DeadlockCode/quarkstrom).
- **Particle Selection & Editing**: Select particles with Shift+Left Click and adjust their charge using keyboard shortcuts (`+`/`-`).
- **Live Particle Editing**: Changes to particle properties (e.g., charge) are applied robustly during simulation via a command system.
- **Configurable Parameters**: Easily adjust simulation size, physics constants, and visualization options.
- **Extensible**: Well-structured for adding new physics, force laws, or visualization features.
- **Visual Debugging**: See selected particles highlighted with a halo and view live charge values.
- **Velocity Vector Overlay**: Toggle drawing of velocity vectors to visualize particle motion.
- **Metal Foils**: Fixed rectangular foils can act as electron sources or sinks with adjustable current.
- Draws heavily from original source: https://github.com/DeadlockCode/barnes-hut.git

---

## Getting Started

1. **Install [Rust](https://www.rust-lang.org/tools/install)**
2. Clone this repository:
3. Build and run:
   ```sh
   cargo run --release
   ```
   > **Note:** On non-Windows platforms, see [this issue](https://github.com/DeadlockCode/n-body/issues/1) for dependencies.

---

## Controls

- **Scroll**: Zoom in/out
- **Middle Mouse Button**: Pan the view
- **Shift + Left Click**: Select a particle
- **+ / -**: Increase/decrease charge of selected particle
- **Escape**: Deselect particle
- **Right Mouse Button**: Spawn a new body
- **Move Mouse (while holding right click)**: Adjust mass of spawned body
- **Space**: Pause/resume simulation
- **E**: Open settings menu (toggle quadtree visualization, etc.)
- Enable "Show Field Isolines" in the settings to visualize the electric potential field

---

## Scenario Controls (New!)

The GUI now features a **Scenario** section for rapid simulation setup and prototyping:

- **Delete All Particles**: Instantly clear the simulation space.
- **Add Circle**: Spawn a ring of particles at any position, with configurable radius, species (Metal or Ion), and (optionally) count.
    - Set the desired radius, X/Y position, and species using the GUI controls.
    - Click "Add" to create a circle of particles in the simulation.
- **Add Rectangle**: Spawn a rectangular block of particles.
- **Add Foil**: Create a fixed rectangular metal foil that can source or sink electrons at a configurable current.

This enables quick experimentation with different initial conditions and system configurations, all from the GUI.

---

## Project Structure

- `src/renderer/` — Rendering and GUI logic (modularized)
- `src/quadtree/` — Quadtree spatial partitioning (split into node, quad, traits, and main logic)
- `src/simulation.rs` — Simulation step logic and physics
- `src/body.rs` — Body struct and related methods (now includes explicit electrons and polarization)
- `src/partition.rs` — Utilities for partitioning and parallelization
- `src/main.rs` — Entry point, threading, and main loop

---

## Extending

- Add new force laws or physics in `src/simulation.rs` or as new modules.
- Extend the GUI in `src/renderer/gui.rs`.
- Implement new spatial partitioning strategies in `src/quadtree/`.

---

## License

MIT License

---

## Credits

- Based on the original Barnes-Hut algorithm and heavily based on: https://github.com/DeadlockCode/barnes-hut.git [DeadlockCode's video](https://youtu.be/nZHjD3cI-EU).
- Uses [quarkstrom](https://github.com/DeadlockCode/quarkstrom) for rendering and GUI.

---

## Physical Model: Electrons, Redox, and Charge Conservation

This simulation models lithium ions (Li⁺) and lithium metal (Li) with explicit valence electrons:

- **Redox Transitions**: Ions become metals if they gain ≥1 electron (the electron causes reduction; no electron is consumed in the transition). Metals become ions if they lose all electrons. Charge is always recalculated from the electron count and species.
- **Electron Hopping**: Electrons can hop between metals and ions, but only down the potential gradient and only if the destination has fewer electrons. Electron conservation is strictly enforced.
- **Physically Accurate Charge**: For Li metal, charge = -(n_electrons - 1). For Li ion, charge = 1 - n_electrons. This ensures correct redox and charge behavior.
- **Debugging**: The code includes debug print statements to trace electron hopping and redox transitions.

---

## Documentation & Testing

- All major modules and functions are documented with clear doc comments.
- The codebase is organized for maintainability and extensibility.
- Tests cover redox transitions, electron hopping, and edge cases. To run tests:

```powershell
cargo test
```

---