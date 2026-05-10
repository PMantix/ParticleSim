# Simulation Physics Notes

## Li⁺ excluded from thermostat
Ion velocity is Coulomb-driven. Only solvents + anions are thermostated (`thermal.rs`, `utils.rs`). This prevents the thermostat from fighting field-driven drift.

## Reduction snap
When Li⁺ reduces to Li metal via electron hop, the new metal is placed adjacent to the donor surface atom. Lost momentum distributed to nearby liquid particles (`electron_hopping.rs`).

## Species lock
`Body.species_lock_until` (default 10 fs) prevents ping-pong oscillation between metal↔ion states. All three gates needed: apply_redox, hop source, hop destination.

## Foil current signs
`+current` adds electrons to foil (deposition side), `-current` removes electrons (stripping side). Paired transfers only fire when one foil has surplus and another has deficit.

## Electrode η measurement
Use `calculate_foil_electron_ratio()` (BFS from foil through connected cluster), not foil-body charge alone. The foil-only measurement misses charge that hopped into the metal cluster.

## Potentiostatic > galvanostatic for DCR
Galvanostatic at particle scale produces ~10¹¹ C-rates. Use overpotential mode (`foil.enable_overpotential_mode(ratio)`) for impedance measurements.
