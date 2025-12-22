# Coding Standards

## Language & Toolchain

- **Language**: Rust (Edition 2021)
- **Build**: `cargo build --release` for optimized builds
- **Linting**: Use `cargo clippy` before commits
- **Formatting**: Use `cargo fmt` for consistent style

## Naming Conventions

- **Files**: `snake_case.rs`
- **Modules**: `snake_case`
- **Types/Structs/Enums**: `PascalCase` (e.g., `Species`, `Body`, `Simulation`)
- **Functions/Methods**: `snake_case` (e.g., `find_neighbors_within`, `update_species`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_DT_FS`, `LJ_FORCE_EPSILON`)
- **Feature flags**: `snake_case` (e.g., `debug_bins`, `profiling`)

## Code Organization

1. **Imports order**:
   - `std` library imports
   - External crate imports
   - Local crate imports (`use crate::...`)
   - Super/self imports

2. **Module structure**: Each major module has `mod.rs` that re-exports public items

3. **Configuration**: Put physics constants in `src/config.rs`, species properties in `src/species.rs`

## Error Handling

- Use `Result<T, E>` for fallible operations
- Use `Option<T>` for optional values
- Prefer `eprintln!` for debug output rather than panics
- Validate numeric inputs to prevent NaN/infinity (see `Body::new()`)

## Performance Patterns

- Use `#[inline]` for small, frequently-called functions
- Prefer `SmallVec` over `Vec` for small, fixed-size collections
- Use `parking_lot::Mutex` instead of `std::sync::Mutex` for better performance
- Profile with `profile_scope!` macro when `profiling` feature is enabled

## GUI/Simulation Communication

Commands flow from GUI to simulation via the `SimCommand` enum in `src/renderer/state.rs`. To add a new control:

1. Add variant to `SimCommand`
2. Handle in `src/app/command_loop.rs`
3. Add GUI control in appropriate tab under `src/renderer/gui/`

## Testing

- Unit tests go in the same file or `tests/` subfolders
- Integration tests in `debug/` directory (require `debug_bins` feature)
- Run with: `cargo test --features unit_tests`

## Documentation

- Use `///` doc comments for public APIs
- Use `//!` for module-level documentation
- Add inline `//` comments for physics explanations
