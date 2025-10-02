# Code Cleanup Checklist

This document lists minor code quality improvements identified during the October 2025 codebase review. All items are **optional** and low-priority—the codebase is already in good shape.

---

## Priority 1: Debug Artifact Cleanup

### A. Remove Unused Debug Variables
**File**: `src/simulation/thermal.rs` (lines 32-36)

**Current**:
```rust
let mut _ec_count = 0; // debug only
let mut _dmc_count = 0; // debug only
let mut _li_count = 0; // debug only
let mut _anion_count = 0; // debug only
```

**Options**:
1. **Keep under feature flag** (recommended if occasionally useful):
   ```rust
   #[cfg(feature = "thermostat_debug")]
   let mut _ec_count = 0;
   // ... etc
   ```

2. **Remove entirely** if no longer needed for diagnostics

---

## Priority 2: Variable Naming Improvements

### A. Improve Temporary Variable Names in Collision Code
**File**: `src/simulation/collision.rs` (lines ~196-198)

**Current**:
```rust
let tmpx = d_xy.x * corr;
let tmpy = d_xy.y * corr;
let tmpz = dz * corr;
```

**Suggested**:
```rust
let sep_x = d_xy.x * corr;
let sep_y = d_xy.y * corr;
let sep_z = dz * corr;
```

**Rationale**: Makes it clear these are separation components

---

### B. Clarify Switch-Charging Variable Names (Optional)
**File**: `src/simulation/simulation.rs` (lines ~225, 319-320)

**Current**:
```rust
let (active_sp, inactive_sp) = ...
let pos_step_ids: Vec<u64> = ...
```

**Optional Improvements**:
- `active_sp` → `active_setpoint` (if verbosity is acceptable)
- `pos_step_ids` → `positive_foil_ids` (clearer in context)

**Note**: Current names are fine; only change if team prefers explicit verbosity

---

## Priority 3: Comment Streamlining

### A. Remove Obvious Comments
**File**: Various

**Examples**:
```rust
// GOOD: Descriptive comments that add context
// Sanitize non-finite position/velocity values before collision resolution

// REMOVE: Comments that restate the code
let dt = config::DEFAULT_DT_FS;  // No comment needed—obvious from name
```

**Guideline**: Keep comments that explain *why*, remove comments that explain *what* if the code is self-explanatory.

---

### B. Expand Terse Comments
**File**: `src/simulation/collision.rs`, `src/simulation/forces.rs`

**Current**:
```rust
// Check inputs
```

**Suggested**:
```rust
// Validate position and velocity fields are finite before collision resolution
```

---

## Priority 4: Code Consistency

### A. Consistent Collection Pre-allocation
**File**: `src/simulation/simulation.rs` and others

**Pattern to Establish**:
```rust
// PREFER: Pre-allocate with capacity hint where size is known
let mut results = Vec::with_capacity(estimated_size);

// AVOID: Default allocation if we know the final size
let mut results = Vec::new();
```

**Files to Review**:
- `src/simulation/simulation.rs` (applied_inactive sets, role vectors)
- `src/simulation/compressed_history.rs` (delta generation loops)

---

## Priority 5: Performance Annotations (Optional)

### A. Add #[inline] Hints for Hot Path Helpers
**Files**: `src/simulation/forces.rs`, `src/simulation/collision.rs`

**Example**:
```rust
#[inline]
fn apply_li_collision_softness(...) -> Vec2 {
    // ... small helper called in tight loop
}
```

**Rationale**: Help compiler optimize hot paths (though LLVM usually does this already)

---

## Priority 6: Documentation Comments

### A. Add Module-Level Doc Comments
**Files**: `src/switch_charging/mod.rs`, `src/simulation/collision.rs`

**Example**:
```rust
//! Switch-charging system for cyclic electrode control.
//!
//! Provides 4-step switching between foil pairs with configurable
//! active/inactive setpoints in Current or Overpotential modes.
//! Supports global configuration or per-step customization.

pub mod mod;  // Module exports
```

---

### B. Document Public API Functions
**Files**: Various public interfaces

**Pattern**:
```rust
/// Applies soft collision correction to a separation vector.
///
/// Returns the scaled separation based on species toggles and softness factor.
/// If neither body is a softened species, returns the input unchanged.
///
/// # Arguments
/// * `sim` - Simulation context for config access
/// * `body_i_idx` - Index of first body
/// * `body_j_idx` - Index of second body
/// * `vec_xy` - Input 2D separation vector
///
/// # Returns
/// Scaled separation vector after applying softness factor
fn apply_li_collision_softness(...) -> Vec2 {
    // Implementation
}
```

---

## Implementation Strategy

### Quick Win (30 minutes)
1. Remove unused `_count` debug variables (Priority 1A)
2. Rename `tmpx/tmpy/tmpz` in collision.rs (Priority 2A)
3. Remove obvious comments (Priority 3A)

### Medium Effort (2 hours)
4. Add module-level doc comments (Priority 6A)
5. Expand terse comments (Priority 3B)
6. Pre-allocate collections in hot paths (Priority 4A)

### Low Priority (Future)
7. Add `#[inline]` hints to hot helpers (Priority 5A)
8. Comprehensive function documentation (Priority 6B)

---

## Testing After Changes

1. **Compile Check**: `cargo check`
2. **Full Build**: `cargo build --release`
3. **Run Simulation**: Verify no regressions in behavior
4. **Profile (Optional)**: Confirm performance improvements if optimizing

---

## Notes

- All suggestions are **non-breaking** changes
- Current code is production-ready; these are polish items
- Focus on Priority 1 and 2 for immediate readability gains
- Defer Priority 5 and 6 unless working on those modules extensively

---

**Last Updated**: October 1, 2025  
**Reviewer**: Automated codebase analysis
