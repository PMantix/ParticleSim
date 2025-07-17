# Particle Species Parameters

The simulation relies on a `species_properties` map to store physical parameters for each particle species. Every entry specifies the default mass, radius and damping factor used when spawning new particles of that species.

```rust
struct SpeciesProperties {
    mass: f32,
    radius: f32,
    damping: f32,
}

let mut species_properties: HashMap<Species, SpeciesProperties> = HashMap::new();
```

### Example values

- **Lithium Ion (Li⁺)**
  - `mass`: `1.0`
  - `radius`: `1.0`
  - `damping`: `0.98`
- **Electrolyte Anion**
  - `mass`: `1.0`
  - `radius`: `1.0`
  - `damping`: `0.98`
- **Foil Metal**
  - `mass`: `1e6` (large mass keeps foil bodies effectively fixed)
  - `radius`: defined by the spawning code
  - `damping`: `0.98`
- **EC / DMC (Solvent molecules)**
  - `mass`: `~90.0`
  - `radius`: `1.7`
  - `damping`: `1.0`
  - `lj_enabled`: `false` (no Lennard-Jones attraction)

### Adding a new species

1. Add a new variant to the `Species` enum in `src/body/types.rs`.
2. Insert default parameters into the `species_properties` map.
3. Optionally extend spawning helpers and GUI controls to expose the new species.

These properties determine the initial mass and size of each body as well as how quickly its velocity decays each frame. Adjusting the map allows rapid experimentation with different particle types and behaviors.

## Lennard-Jones Interactions

Each species also defines Lennard-Jones (LJ) parameters controlling short-range
attraction and repulsion between like materials. The LJ force can be enabled or
disabled per species to approximate different phases:

- **Metal-like**: LJ enabled to model cohesive metallic bonding.
- **Liquid-like**: LJ disabled so only electrostatic forces act.

Example configuration snippet:

```rust
species_properties.insert(Species::FoilMetal, SpeciesProperties {
    mass: 1e6,
    radius: 1.0,
    damping: 0.98,
    lj_enabled: true,
    lj_epsilon: 2000.0,
    lj_sigma: 1.7,
});

// Electrolyte acts liquid-like (no LJ attraction)
species_properties.insert(Species::ElectrolyteAnion, SpeciesProperties {
    mass: 40.0,
    radius: 1.5,
    damping: 0.96,
    lj_enabled: false,
    lj_epsilon: 0.0,
    lj_sigma: 1.7,
});
```

With this setup the foil behaves as a solid metal while the electrolyte remains
liquid. Adjust each species' `lj_enabled` flag and parameters to explore other
scenarios.

EC and DMC are modeled as neutral but polar molecules. Each carries a single
bound electron that drifts within the molecule to form an electric dipole.
During force calculation a polarization force is applied based on the
difference between the electric field at the electron and at the molecular
center. This allows charged particles to interact with the induced dipole so
solvent shells form naturally without any Lennard-Jones attraction.
