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

## Coding Conventions
- `parking_lot::Mutex` over `std::sync::Mutex`
- `SmallVec` for small fixed-size collections (e.g., bound electrons)
- `#[inline]` on small, hot functions
- Use `profile_scope!` macro when `profiling` feature is enabled
- Physics constants go in `config.rs`, species properties in `species.rs`
- Unit tests gated behind `#[cfg(feature = "unit_tests")]`; integration tests in `debug/` directory

## DOE Coordination

`feature/eis-amplitude-study` hosts a two-machine Claude-to-Claude DOE
coordination subsystem. Long-running headless DOE batches are driven by
append-only JSONL files in `coordination/` plus a polling controller and
optional LAN messaging server:

- `scripts/south_controller.py` — polls `coordination/north_jobs.jsonl`
  every 15s, claims un-claimed jobs, runs them via `scripts/run_job.sh`,
  records completions in `coordination/south_status.jsonl`.
- `scripts/messaging_server.py` — LAN HTTP server (port 8765,
  bearer-auth) for sub-second peer-to-peer messaging across the JSONL files.
- `scripts/_south_run.sh` — internal helper; writes `<id>.meta` on exit
  so completions are recorded even if the controller restarts.

Full protocol in `coordination/PROTOCOL.md`. If those processes are already
running at session start, prefer working with them over restarting.

## Operational Pitfalls

Things that aren't visible from reading the code but bit prior sessions:

- **Never `taskkill //F //IM bash.exe`** to clean up DOE subprocesses. It
  kills the South controller, messaging server, and Monitor as collateral.
  Target specific PIDs or specific binary names
  (`taskkill //F //IM e1_baseline_dcr.exe`).

- **Worktrees + `git pull --rebase`**: when a `git worktree` is checked out
  in a sibling directory, its fetches overwrite the shared
  `.git/FETCH_HEAD`, causing plain `git pull --rebase` (no args) elsewhere
  to fail with "Cannot rebase onto multiple branches". `south_controller.py`
  now uses explicit `git pull --rebase origin <branch>`; do the same for
  any new automation. Remove unused worktrees with `git worktree remove`.

- **`run_job.sh` auto-build triggers on missing binary, not stale binary**.
  After pulling a commit that touches a binary's source, manually
  `cargo build --release --bin <name>` before launching new jobs (and kill
  any in-flight runs that started on the old binary if their results would
  be invalidated).
