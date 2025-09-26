# Codex Agent Instructions

This repository is a Rust project that simulates electrochemical particle systems.

## Thermostat & Temperature Notes
- "Liquid temperature" now includes LithiumIon, ElectrolyteAnion, EC, and DMC (COM drift removed).
- Maxwell–Boltzmann thermostat rescales only these liquid species (metals excluded) every configured interval.
- Detailed thermostat diagnostics are gated behind the cargo feature `thermostat_debug`.
	- Enable via: `cargo run --features thermostat_debug --release`
	- Provides scaling factors, per-frame liquid KE, and sampling of representative particles.

## Programmatic Checks
- Run `cargo check` before committing changes to verify the project compiles.
- The `cargo test` suite relies on features that do not function in the Codex environment, so it is unnecessary to run `cargo test` here.

## Module Guides
Each major module under `src/` has its own `AGENT.md` file describing the module and key files.
