# Domain Size GUI Synchronization Fix

## Problem
When users clicked "Add Random" particles in the GUI, the particles were being placed within the GUI's displayed domain limits (default 300x300) rather than the actual simulation domain limits loaded from the configuration file (600x400). This caused particles to be placed in a smaller area than intended.

## Root Cause
The renderer's GUI domain size fields (`domain_width`, `domain_height`) were initialized with hardcoded default values (300x300) and were not being updated when the simulation loaded the actual domain size from `init_config.toml`.

## Solution
1. **Added Shared State Variables**: Added `DOMAIN_WIDTH` and `DOMAIN_HEIGHT` to the shared state in `src/renderer/state.rs` to allow communication between simulation and renderer threads.

2. **Initialize Shared State Early**: Modified `src/app/mod.rs` to set the shared state domain values immediately after loading the config, before the renderer is created.

3. **Updated Renderer Initialization**: Modified the renderer constructor in `src/renderer/mod.rs` to initialize from shared state instead of hardcoded defaults.

4. **Updated Command Handler**: Modified `handle_set_domain_size()` in `src/commands/particle.rs` to update the shared state variables when the simulation domain size is set.

5. **Added GUI Synchronization**: Added domain size synchronization in `show_gui()` in `src/renderer/gui/mod.rs` to ensure GUI fields stay synchronized during runtime.

## Implementation Details

### Added to `src/renderer/state.rs`:
```rust
pub static DOMAIN_WIDTH: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(300.0));
pub static DOMAIN_HEIGHT: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(300.0));
```

### Modified `src/app/mod.rs`:
```rust
// Determine the domain size from configuration
let (global_width, global_height) = if let Some(ref sim_config) = init_config.simulation {
    let (width, height) = sim_config.domain_size();
    println!("Setting domain size to {}x{}", width, height);
    
    // Initialize shared state with correct domain size before renderer is created
    *crate::renderer::state::DOMAIN_WIDTH.lock() = width;
    *crate::renderer::state::DOMAIN_HEIGHT.lock() = height;
    
    tx.send(SimCommand::SetDomainSize { width, height }).unwrap();
    (width, height)
} else {
    // ... handle default case ...
};
```

### Modified `src/renderer/mod.rs`:
```rust
domain_width: *crate::renderer::state::DOMAIN_WIDTH.lock(),  // Initialize from shared state
domain_height: *crate::renderer::state::DOMAIN_HEIGHT.lock(), // Initialize from shared state
```

### Modified `src/renderer/gui/mod.rs`:
```rust
pub fn show_gui(&mut self, ctx: &quarkstrom::egui::Context) {
    // Sync domain size from shared state (updated by simulation)
    self.domain_width = *crate::renderer::state::DOMAIN_WIDTH.lock();
    self.domain_height = *crate::renderer::state::DOMAIN_HEIGHT.lock();
    // ... rest of GUI code ...
}
```

## Result
Now when users create random particles through the GUI, they are correctly placed within the full domain boundaries as specified in the configuration file (600x400), not the old default (300x300).

## Testing
The fix was verified by:
1. Running the simulation with the default `init_config.toml` (600x400 domain)
2. Confirming the console output shows "Setting domain size to 600x400" 
3. Verifying that GUI controls now use the correct domain dimensions for particle placement
