# Realistic Z-Direction Forces - Improved Quasi-2.5D Physics

## Overview

This document describes the improved quasi-2.5D system that replaces the artificial spring force in the z-direction with more physically realistic forces while maintaining computational efficiency.

## Key Improvements

### 1. **Surface Binding Potential (Double-Well)**

**Previous:** Linear spring force `F = -k*z` that artificially pulls particles to z=0.

**New:** Double-well potential that creates natural binding sites near electrode surfaces:

```rust
// Polynomial potential: U(z) = a*z^4 - b*z^2 
// Force: F = -dU/dz = -4*a*z^3 + 2*b*z
```

**Benefits:**
- Creates realistic minima near ±max_z (electrode surfaces)
- Unstable equilibrium at z=0 (particle naturally choose a surface)
- Smooth potential with no discontinuities
- Surface-specific binding strengths based on species

### 2. **Species-Dependent Surface Affinity**

Different particle types have realistic interactions with surfaces:

- **Li⁺ ions**: Strong attraction to cathode (surface_affinity = 2.0)
- **Anions**: Moderate attraction to anode (surface_affinity = 1.5)  
- **EC/DMC solvents**: Weak surface interactions (surface_affinity = 0.5)
- **Metal particles**: Fixed at surfaces (surface_affinity = 0.0)

### 3. **Electric Field Gradient Effects**

Charged particles experience forces due to field variations in z-direction:

```rust
F_z = -charge * field_gradient * z
```

This creates realistic charge separation:
- Positive charges prefer negative electrode
- Negative charges prefer positive electrode

### 4. **Solvation Shell Forces**

Particles prefer specific z-separations based on their chemical nature:

- **Ion-solvent shells**: Li⁺ prefers 0.3Å separation from EC molecules
- **Like-like repulsion**: Ions of same type avoid same z-layer
- **Solvent organization**: EC and DMC form structured layers

### 5. **Density-Dependent Forces**

High particle density creates osmotic pressure that pushes particles out-of-plane:

- Monitors local density in z-layers
- Creates spreading force when density exceeds threshold
- Prevents unrealistic particle clustering

## Physical Realism Improvements

### **Before (Artificial Spring):**
- Single equilibrium at z=0
- No surface chemistry
- No species differentiation
- Unphysical "frustration" mechanism

### **After (Realistic Forces):**
- Natural electrode binding
- Species-specific interactions  
- Solvation shell formation
- Charge-dependent layering
- Osmotic pressure effects

## Implementation Details

### **Force Calculation Structure:**

1. **Local forces** (no inter-particle coupling):
   - Surface binding potential
   - Electric field gradients

2. **Many-body forces** (separate pass to avoid borrowing issues):
   - Solvation shell interactions
   - Density gradient forces

### **Configuration Parameters:**

The system maintains the same configuration interface but interprets parameters differently:

- `z_stiffness`: Now scales surface binding strength (not spring constant)
- `z_damping`: Still provides velocity damping
- `max_z`: Still defines electrode separation
- `z_frustration_strength`: Still redirects in-plane stress to z-motion

### **Performance:**

- Maintains quasi-2.5D efficiency
- Two-pass force calculation avoids N² scaling
- Local forces computed in parallel
- Many-body forces pre-computed to avoid borrowing conflicts

## Usage

The system is drop-in compatible with existing code. Enable with:

```rust
sim.config.enable_out_of_plane = true;
sim.config.max_z = 2.0;  // Electrode separation
```

## Expected Behaviors

1. **Layering**: Particles naturally form layers near electrode surfaces
2. **Charge separation**: Cations and anions segregate to opposite electrodes  
3. **Solvation**: Ion-solvent complexes maintain preferred geometries
4. **Pressure relief**: Dense regions spread particles out-of-plane
5. **Species sorting**: Different particle types occupy different z-regions

This system provides much more realistic electrochemical behavior while maintaining the computational advantages of the quasi-2.5D approach.
