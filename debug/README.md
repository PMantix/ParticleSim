# Debug Tools

This folder contains debugging and analysis tools created during development.

## Files

### Coulomb Constant Analysis
- `check_coulomb.rs/.exe` - Verifies the theoretical Coulomb constant calculation
- `check_new_coulomb.rs/.exe` - Tests alternative Coulomb constant formulations
- `debug_forces.rs/.exe` - Compares current vs correct force calculation formulas

### Timestep Analysis
- `timestep_analysis.rs/.exe` - Calculates expected scaling factors after timestep changes

### Graphics and Visualization Debug
- `debug_graphics.rs` - Graphics system debugging and testing
- `debug_out_of_plane.rs` - Out-of-plane (Z-direction) physics debugging

### Configuration and Testing
- `verify_config.rs` - Configuration verification tool
- `test_config.rs` - Configuration testing utilities
- `test_negative_current.rs` - Negative current testing
- `minimal_test.rs` - Minimal simulation test cases

## Usage

To compile and run any of these tools:
```bash
rustc filename.rs
./filename.exe    # Windows
./filename        # Linux/Mac
```

## Purpose

These tools were created to:
1. Verify that the Coulomb constant has correct units and magnitude
2. Identify why empirical scaling was needed (28,800x factor)
3. Analyze the impact of timestep changes on simulation dynamics
4. Debug force calculation formulas in the physics engine

## Results

Key findings:
- Theoretical Coulomb constant (0.139) is physically correct
- Original timestep (0.015 fs) was extremely small, requiring massive force scaling
- New timestep (1.0 fs) should reduce needed scaling to ~6x instead of ~28,800x
- Force calculation uses softened denominator: `(r² + ε²) * r` instead of correct `(r² + ε²)^(3/2)`
