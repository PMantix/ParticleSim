# Module: commands

Processes simulation commands issued from the GUI.

Files:
- `dispatcher.rs` – routes `SimCommand` variants to handlers.
- `particle.rs` – command handlers related to particles and domain size.
- `foil.rs` – command handlers related to foil currents and linking.
- `state.rs` – commands for saving/loading or stepping the simulation.
- `mod.rs` – re-exports `process_command`.
