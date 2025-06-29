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

### Adding a new species

1. Add a new variant to the `Species` enum in `src/body/types.rs`.
2. Insert default parameters into the `species_properties` map.
3. Optionally extend spawning helpers and GUI controls to expose the new species.

These properties determine the initial mass and size of each body as well as how quickly its velocity decays each frame. Adjusting the map allows rapid experimentation with different particle types and behaviors.
