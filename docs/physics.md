# Physics

## Units

The simulation operates in a physical unit system with the following base units:

- **Length:** angstrom (Å) = `1.0e-10` meters
- **Time:** femtosecond (fs) = `1.0e-15` seconds
- **Charge:** elementary charge (e) = `1.602176634e-19` coulombs
- **Mass:** atomic mass unit (amu) = `1.66053906660e-27` kilograms

Derived quantities are computed from these bases. One simulation energy unit
corresponds to `amu·Å²/fs² ≈ 1.66e-17 J ≈ 103.6 eV`. The Coulomb constant is
`0.1389` in simulation units.

To convert back to SI units multiply simulation values by the factors above
(e.g. multiply a length measured in Å by `1e-10` to obtain meters).

## Particle Species Parameters

| Species           | Mass (amu) | Radius (Å) |
|-------------------|-----------:|-----------:|
| Lithium Ion (Li⁺) | 6.94       | 0.76       |
| Lithium Metal     | 6.94       | 1.52       |
| Foil Metal        | 1.0e6      | 1.52       |
| Electrolyte Anion | 145.0      | 2.0        |
| EC                | 88.06      | 2.5        |
| DMC               | 90.08      | 2.5        |

These masses and radii are the defaults used when spawning particles of each
species. Additional parameters such as damping and Lennard‑Jones coefficients
are defined in `src/species.rs`.

## Lennard-Jones Interactions

Each species optionally enables Lennard‑Jones (LJ) forces that model short-range
interactions. LJ parameters are specified in physical units:

- **ε (epsilon):** well depth in electronvolts (eV)
- **σ (sigma):** distance at which the potential is zero in angstroms (Å)
- **Cutoff:** interaction range in angstroms (Å)

These values are converted to simulation units via the definitions in
`src/config.rs`. Disable LJ forces for species that should behave as a simple
charged fluid (e.g. solvent molecules).
