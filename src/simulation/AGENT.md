# Module: simulation

Core physics engine and stepping logic.

Files:
- `forces.rs` – force calculations (electrostatics, etc.).
- `collision.rs` – particle collision resolution.
- `simulation.rs` – main `Simulation` struct and step function.
- `utils.rs` – small helpers for integrators or statistics.
- `tests.rs` – unit tests (may not run under Codex).
- `mod.rs` – re-exports module contents.
