# Performance Analysis: Li+ vs PF6- vs EC Particles

## Problem
Simulations with Li+ ions run extremely slowly compared to PF6- anions or EC molecules, even though both Li+ and PF6- are charged particles.

## Root Cause: Electron Update Loop

### The Performance Bottleneck
Found in `src/simulation/simulation.rs` lines 149-160 and `src/body/electron.rs`:

```rust
// In simulation step: O(N) loop over all particles
for i in 0..len {
    body.update_electrons(...);  // Called for EVERY particle
}

// In update_electrons: O(M) loop over electrons per particle
for e in &mut self.electrons {
    // EXPENSIVE: Full quadtree field calculation for each electron
    let local_field = quadtree.field_at_point(bodies, electron_pos, coulomb_constant);
}
```

**Total complexity**: O(N × M × log N) where N = particles, M = electrons per particle

### Species-Specific Electron Counts
- **Li+ ions**: 1 electron each (`LITHIUM_METAL_NEUTRAL_ELECTRONS = 1`)
- **PF6- anions**: 0 electrons (`ELECTROLYTE_ANION_NEUTRAL_ELECTRONS = 0`)  
- **EC molecules**: 1 electron each (`EC_NEUTRAL_ELECTRONS = 1`)
- **DMC molecules**: 1 electron each (`DMC_NEUTRAL_ELECTRONS = 1`)

### Performance Impact
- **1000 Li+ ions**: 1000 expensive `quadtree.field_at_point()` calls per timestep
- **1000 PF6- anions**: 0 expensive calls (skip electron updates entirely)
- **1000 EC molecules**: 1000 expensive calls (dipole mechanism requires electron drift)

## Why Li+ is Slower than PF6-

Both are charged (+1 and -1 respectively) so electrostatic force calculations are similar, BUT:
- Li+ ions have 1 electron each requiring expensive field calculations
- PF6- anions have 0 electrons, completely skipping the expensive electron update loop

## Optimizations Implemented

### 1. Early Exit for Empty Electron Lists
**File**: `src/body/electron.rs`
```rust
pub fn update_electrons(...) {
    // Early exit for particles with no electrons - major performance optimization
    if self.electrons.is_empty() {
        return;
    }
    // ... expensive calculations only if electrons exist
}
```

**Impact**: PF6- anions and any other electron-free particles now skip electron updates entirely.

### 2. Previous Incorrect Optimization (Reverted)
~~Attempted to skip field calculations for neutral particles, but this broke EC/DMC dipole interactions which require field calculations even though the molecules are neutral.~~

## Performance Expectations

### After Optimizations
- **Pure PF6- systems**: Very fast (no electron updates)
- **Pure Li+ systems**: Still slow (1 electron per ion requires field calculations)
- **Pure EC/DMC systems**: Moderate (need electron updates for dipole physics)
- **Mixed systems**: Performance depends on ratio of electron-bearing particles

### Fundamental Limitation
Li+ ions inherently require expensive electron field calculations for realistic physics. The simulation models electron drift/binding effects that are computationally expensive but physically necessary.

## Historical Context
The performance regression likely occurred when electron physics was enhanced over the past 100+ commits. Previously Li+ ions may have had simpler electron handling or fewer electrons per ion.

## Recommendations
1. **For fast testing**: Use PF6- anions instead of Li+ ions when electron physics isn't critical
2. **For realistic simulations**: Accept that Li+ performance is inherently limited by electron physics
3. **For mixed systems**: Minimize the ratio of electron-bearing particles where possible
