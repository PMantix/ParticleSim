# Headless Binaries

Measurement and validation binaries that run without the GUI. Auto-discovered by cargo (no Cargo.toml entry needed).

## Key binaries
- `e1_baseline_dcr` — Potentiostatic DCR: rest→pulse→rest. Args: `<ratio> [pulse_ps] [snap_dir]`
- `e2_repeatability` — 3 consecutive pulses on same cell with snapshots. Args: `<ratio> [snap_dir]`
- `potentiostatic_sweep` — Sweep voltage steps, measure steady-state current
- `steady_state_search` — Galvanostatic sweep across electrode sizes/currents
- `tafel_slope` — Galvanostatic Tafel sweep, recover BV transfer coefficient α
- `nernst_einstein` — Haven ratio from MSD + drift velocity
- `physics_invariants` — Phase 1 invariant tests (charge balance, energy drift, etc.)
- `bfs_debug` — Verify electrode geometry and BFS body count
- `ion_motion_diag` — Track ion kinematics step-by-step

## Spawning geometry
`SimCommand::AddRectangle` and `SimCommand::AddFoil` interpret `x, y` as **bottom-left origin**, not center. The TOML loader converts center→origin via `to_origin_coords()`. Headless binaries must do the same: `origin = center - size/2`. Verify with `bfs_debug` that electrode BFS finds the expected body count.

## Coordinate with South
South is a compute node (Ryzen 7950X3D) running batch jobs. Communication:
- **LAN**: `http://192.168.1.184:8765` (token in `.doe_token`), via `scripts/north_p2p.sh`
- **Git fallback**: `coordination/north_to_south.jsonl` on `feature/eis-amplitude-study`
- South polls origin every 15s; push jobs via either channel
- Results land in `doe_results/` subdirectories; South commits and pushes
