// Configuration types for compressed history
// Separated to avoid recompiling complex logic when config changes

/// Configuration for change detection thresholds
/// Optimized based on simulation physics and precision requirements
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub struct ChangeThresholds {
    /// Minimum position change to record (in Angstroms)
    /// Based on thermal motion scale: ~0.01 Å is typical atomic vibration amplitude
    pub position_epsilon: f32,
    /// Minimum velocity change to record (Å/fs)
    /// Based on thermal velocity scale: ~0.1 Å/fs is typical atomic velocity
    pub velocity_epsilon: f32, 
    /// Minimum charge change to record (elementary charges)
    /// Small fractional charges from polarization effects
    pub charge_epsilon: f32,
    /// Minimum electron position change to record (Å)
    /// Electrons can drift significantly within atoms
    pub electron_epsilon: f32,
    /// Minimum z-position change to record (Å)
    /// Out-of-plane motion threshold
    pub z_epsilon: f32,
    /// Threshold for considering a body "moved significantly"
    /// Used for electron surroundings recalculation
    pub significant_move_epsilon: f32,
}

impl Default for ChangeThresholds {
    fn default() -> Self {
        Self {
            // Position: 0.01 Å ≈ 1% of typical atomic radius
            // This catches meaningful atomic displacements while ignoring noise
            position_epsilon: 0.01,
            
            // Velocity: 0.1 Å/fs ≈ 100 m/s at atomic scale
            // Catches significant momentum changes
            velocity_epsilon: 0.1,
            
            // Charge: 0.01e ≈ 1% of elementary charge
            // Captures polarization and fractional charge effects
            charge_epsilon: 0.01,
            
            // Electron position: 0.1 Å ≈ 10% of typical atomic radius
            // Electrons are mobile within atoms
            electron_epsilon: 0.1,
            
            // Z-position: 0.05 Å for out-of-plane motion detection
            // Smaller threshold since z-motion is typically constrained
            z_epsilon: 0.05,
            
            // Significant movement: 0.5 Å for triggering electron recalculation
            // Large enough to avoid noise but small enough for accuracy
            significant_move_epsilon: 0.5,
        }
    }
}

/// Types of simulations with different precision requirements
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Will be used in GUI integration
pub enum SimulationType {
    /// Initial equilibration phase - can use loose thresholds
    Equilibration,
    /// Production run - balanced precision/performance
    Production,
    /// Detailed analysis - maximum precision
    Analysis,
}

#[allow(dead_code)] // Will be used in GUI integration
impl ChangeThresholds {
    /// Configuration optimized for memory efficiency
    /// Uses larger thresholds to reduce stored deltas
    pub fn memory_optimized() -> Self {
        Self {
            position_epsilon: 0.05,
            velocity_epsilon: 0.2, 
            charge_epsilon: 0.05,
            electron_epsilon: 0.2,
            z_epsilon: 0.1,
            significant_move_epsilon: 1.0,
        }
    }
    
    /// Configuration optimized for maximum precision
    /// Uses smaller thresholds to capture more detail
    pub fn high_precision() -> Self {
        Self {
            position_epsilon: 0.005,
            velocity_epsilon: 0.05,
            charge_epsilon: 0.005,
            electron_epsilon: 0.05,
            z_epsilon: 0.02,
            significant_move_epsilon: 0.2,
        }
    }
    
    /// Adaptive configuration based on simulation type
    pub fn for_simulation_type(sim_type: SimulationType) -> Self {
        match sim_type {
            SimulationType::Equilibration => Self::memory_optimized(),
            SimulationType::Production => Self::default(),
            SimulationType::Analysis => Self::high_precision(),
        }
    }
}

/// Configuration for the compressed history system
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub struct CompressionConfig {
    /// Interval between keyframes (in frame count)
    pub keyframe_interval: usize,
    /// Maximum number of keyframes to keep
    pub max_keyframes: usize,
    /// Maximum number of delta frames to keep
    pub max_deltas: usize,
    /// Change detection thresholds
    pub thresholds: ChangeThresholds,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            keyframe_interval: 100,     // Keyframe every 100 frames
            max_keyframes: 50,          // Keep 50 keyframes (5000 frames span)
            max_deltas: 5000,           // Keep 5000 delta frames
            thresholds: ChangeThresholds::default(),
        }
    }
}

#[allow(dead_code)] // Will be used in GUI integration
impl CompressionConfig {
    /// Create configuration optimized for memory efficiency
    /// Fewer keyframes, larger intervals, loose thresholds
    pub fn memory_optimized() -> Self {
        Self {
            keyframe_interval: 200,     // Keyframe every 200 frames
            max_keyframes: 25,          // Keep 25 keyframes (5000 frames span)
            max_deltas: 5000,           // Keep 5000 delta frames
            thresholds: ChangeThresholds::memory_optimized(),
        }
    }
    
    /// Create configuration optimized for precision
    /// More keyframes, smaller intervals, tight thresholds
    pub fn high_precision() -> Self {
        Self {
            keyframe_interval: 50,      // Keyframe every 50 frames
            max_keyframes: 100,         // Keep 100 keyframes (5000 frames span)
            max_deltas: 5000,           // Keep 5000 delta frames
            thresholds: ChangeThresholds::high_precision(),
        }
    }
    
    /// Create configuration for a specific simulation type
    pub fn for_simulation_type(sim_type: SimulationType) -> Self {
        match sim_type {
            SimulationType::Equilibration => Self::memory_optimized(),
            SimulationType::Production => Self::default(),
            SimulationType::Analysis => Self::high_precision(),
        }
    }
    
    /// Create configuration for a specific memory budget (in MB)
    pub fn for_memory_budget(memory_mb: f64) -> Self {
        if memory_mb < 100.0 {
            Self::memory_optimized()
        } else if memory_mb > 500.0 {
            Self::high_precision()
        } else {
            Self::default()
        }
    }
}