# Module: body

Defines particle structures and behaviors.

Key files:
- `electron.rs` – electron representation.
- `foil.rs` – foil body behavior.
- `redox.rs` – species transitions based on electron count.
- `types.rs` – core `Body`, `Electron`, and species definitions.
- `tests/` and `tests.rs` – unit tests (these require features that may not run in Codex).

## Species Groups
- Liquid (thermostatted): `LithiumIon`, `ElectrolyteAnion`, `EC`, `DMC`
- Metals (not thermostatted): `LithiumMetal`, `FoilMetal`
Only liquid species contribute to the liquid temperature metric and are candidates for velocity rescaling.
