# Module: simulation

Core physics engine and stepping logic.

## Thermostat
The simulation applies a Maxwell–Boltzmann style velocity rescaling to the "liquid" set:
`LithiumIon | ElectrolyteAnion | EC | DMC`.

Implementation details:
- Temperature is computed from KE per particle with center-of-mass velocity removed for those species.
- Metals (LithiumMetal, FoilMetal) are excluded from scaling to preserve electrode dynamics.
// Bootstrap: when liquid KE is effectively zero, all liquid species (Li+, anion, EC, DMC) are initialized with Maxwellian velocities.
- Enable verbose diagnostics with cargo feature `thermostat_debug`.

Files:
- `forces.rs` – force calculations (electrostatics, etc.).
- `collision.rs` – particle collision resolution.
- `simulation.rs` – main `Simulation` struct and step function.
- `utils.rs` – small helpers for integrators or statistics.
- `tests.rs` – unit tests (may not run under Codex).
- `mod.rs` – re-exports module contents.
