# Common Development Tasks

## Adding a GUI Control

To add a new parameter control in the GUI:

1. **Add field to SimConfig** in `src/config.rs`:
   ```rust
   #[serde(default = "default_my_param")]
   pub my_param: f32,
   
   fn default_my_param() -> f32 { 1.0 }
   ```

2. **Add SimCommand variant** in `src/renderer/state.rs`:
   ```rust
   pub enum SimCommand {
       // ...
       UpdateMyParam(f32),
   }
   ```

3. **Handle command** in `src/app/command_loop.rs`:
   ```rust
   SimCommand::UpdateMyParam(value) => {
       simulation.config.my_param = value;
   }
   ```

4. **Add GUI widget** in appropriate tab under `src/renderer/gui/`:
   ```rust
   ui.horizontal(|ui| {
       ui.label("My Param:");
       if ui.add(egui::Slider::new(&mut state.my_param, 0.0..=10.0)).changed() {
           state.send_command(SimCommand::UpdateMyParam(state.my_param));
       }
   });
   ```

## Adding a New Species

1. Add to `Species` enum in `src/body/types.rs`
2. Add properties in `src/species.rs` SPECIES_PROPERTIES map
3. Add neutral electron count in `src/config.rs`
4. Update `Body::update_species()` if it should auto-convert
5. Update electron hopping rules in `src/simulation/electron_hopping.rs`
6. Add spawn logic in `src/app/spawn.rs` if needed

## Adding a New Force Type

1. Create function in `src/simulation/forces.rs`:
   ```rust
   pub fn apply_my_force(sim: &mut Simulation) {
       profile_scope!("forces_my_force");
       // Implementation
   }
   ```

2. Call from simulation loop in `src/simulation/simulation.rs` or `src/app/simulation_loop.rs`

3. Add enable/disable config if needed in `src/config.rs`

## Debugging Physics

- Enable `profiling` feature for timing: `cargo run --release --features profiling`
- Enable `command_debug` for SimCommand logging
- Use debug binaries in `debug/` directory (require `debug_bins` feature)
- Check `logs/` directory for thermostat and force traces

## Running DOE (Design of Experiments)

```bash
cargo run --release --features doe --bin doe_runner
```

DOE configurations are TOML files specifying parameter sweeps. Results go to `doe_results/`.

## Saving/Loading State

The simulation can save/load compressed state files:
- Binary format with bincode
- Compressed with flate2
- State includes all body positions, velocities, charges
