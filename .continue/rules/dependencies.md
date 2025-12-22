# External Dependencies

## Quarkstrom (Local Dependency)

ParticleSim uses a local rendering framework called **Quarkstrom** located at `../quarkstrom/quarkstrom`.

Quarkstrom provides:
- WebGPU-based GPU rendering
- Particle visualization with instanced rendering
- Window management (winit integration)
- egui integration for immediate-mode GUI
- Input handling (mouse, keyboard)

Key Quarkstrom types used:
- `Renderer` - Main rendering context
- `Particle` - GPU particle representation
- Window events and input state

## Cargo Dependencies

### Core Libraries
- `ultraviolet` - Fast linear algebra (Vec2, etc.) with serde support
- `fastrand` - Fast random number generation
- `broccoli` - Spatial tree for collision detection
- `broccoli-rayon` - Parallel collision detection

### Concurrency
- `parking_lot` - Fast mutex implementation
- `once_cell` - Lazy static initialization
- `rayon` - Data parallelism
- `crossbeam` - Concurrent data structures

### Serialization
- `serde` - Serialization framework
- `serde_json` - JSON format
- `toml` - TOML config files
- `bincode` - Binary serialization for state saves
- `flate2` - Compression for state files

### Utilities
- `palette` - Color manipulation
- `rand` / `rand_distr` - Random distributions
- `smallvec` - Stack-allocated small vectors

## Documentation Links

When implementing battery simulation features, reference:

- Ultraviolet Vec2: https://docs.rs/ultraviolet/latest/ultraviolet/
- Broccoli spatial trees: https://docs.rs/broccoli/latest/broccoli/
- Rayon parallelism: https://docs.rs/rayon/latest/rayon/
- Serde derive: https://serde.rs/derive.html

For electrochemistry concepts:
- Butler-Volmer kinetics
- Lennard-Jones potential
- Barnes-Hut algorithm
