# Barnes-Hut N-Body Simulation (Rust)

This repository contains a modular, parallelized, and extensible implementation of the Barnes-Hut algorithm for simulating large-scale N-body systems in 2D, written in Rust.  
It is designed for high performance, clarity, and ease of extension, with a focus on modern Rust best practices and code organization.
End goal is to develop a particle based simulator of electrochemical charging and discharging processes

---

## Features

- **Barnes-Hut Quadtree**: Efficient spatial partitioning for O(N log N) force calculation.
- **Parallel Simulation**: Uses Rayon for multi-threaded computation.
- **Modular Codebase**: Clean separation of simulation, quadtree, rendering, and state management.
- **Interactive GUI**: Real-time visualization and controls via [quarkstrom](https://github.com/DeadlockCode/quarkstrom).
- **Particle Selection & Editing**: Select particles with Shift+Left Click and adjust their charge using keyboard shortcuts (`+`/`-`).
- **Live Particle Editing**: Changes to particle properties (e.g., charge) are applied robustly during simulation via a command system.
- **Configurable Parameters**: Easily adjust simulation size, physics constants, and visualization options.
- **Extensible**: Well-structured for adding new physics, force laws, or visualization features.
- **Visual Debugging**: See selected particles highlighted with a halo and view live charge values.
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

---

## Project Structure

- `src/renderer/` — Rendering and GUI logic (modularized)
- `src/quadtree/` — Quadtree spatial partitioning (split into node, quad, traits, and main logic)
- `src/simulation.rs` — Simulation step logic and physics
- `src/body.rs` — Body struct and related methods
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