// Centralized configuration for simulation parameters

// ====================
// Electron Parameters
// ====================
pub const ELECTRON_SPRING_K: f32 = 0.05;                // Spring constant for electron drift
pub const ELECTRON_DRIFT_RADIUS_FACTOR: f32 = 1.2;      // Max drift radius as a factor of body radius
pub const ELECTRON_MAX_SPEED_FACTOR: f32 = 1.2;         // Max electron speed as a factor of body radius per dt
pub const _HOP_CHARGE_THRESHOLD: f32 = 0.2;                     // Charge threshold for hopping
pub const HOP_RADIUS_FACTOR: f32 = 2.1;                      // Hopping radius as a factor of body radius
pub const HOP_RATE_K0: f32 = 1.0;            /// Base hop‐rate constant (per unit time) at zero overpotential
pub const HOP_TRANSFER_COEFF: f32 = 0.5;            /// Transfer coefficient α (unitless, ~0.5)   
pub const HOP_ACTIVATION_ENERGY: f32 = 0.025;      /// Thermal energy k_BT (in your same charge‐units)
pub const _CLUSTER_DISTANCE: f32 = 1.2;                // Distance threshold for clustering

// ====================
// LJ Force Parameters
// ====================
pub const LJ_FORCE_EPSILON: f32 = 500.0;                  // Lennard-Jones epsilon parameter
pub const LJ_FORCE_SIGMA: f32 = 1.1;                    // Lennard-Jones sigma parameter
pub const LJ_FORCE_CUTOFF: f32 = 2.5;                  // Lennard-Jones cutoff distance
pub const LJ_FORCE_MAX: f32 = 33.33;                   // Max Lennard-Jones force magnitude

// ====================
// Species/Body Parameters
// ====================
pub const LITHIUM_ION_THRESHOLD: f32 = 0.5;             // Charge threshold for lithium ion/metal transition

// ====================
// Simulation Parameters
// ====================
pub const DEFAULT_DT: f32 = 0.0025;                     // Default simulation timestep
pub const _DEFAULT_PARTICLE_COUNT: usize = 50000;        // Default number of particles
pub const COLLISION_PASSES: usize = 3;                  // Number of collision resolution passes

// ====================
// Quadtree Parameters
// ====================
pub const QUADTREE_THETA: f32 = 1.0;                    // Barnes-Hut opening angle
pub const QUADTREE_EPSILON: f32 = 2.0;                  // Softening parameter
pub const QUADTREE_LEAF_CAPACITY: usize = 1;            // Max bodies per quadtree leaf
pub const QUADTREE_THREAD_CAPACITY: usize = 1024;       // Max bodies per thread chunk

// ====================
// Initialization/Clumping
// ====================
pub const CLUMP_RADIUS: f32 = 20.0;                     // Radius of each clump
pub const DOMAIN_BOUNDS: f32 = 350.0;                   // Simulation domain boundary

// ====================
// Threading/Parallelism
// ====================
pub const MIN_THREADS: usize = 3;                       // Minimum number of threads to use
pub const THREADS_LEAVE_FREE: usize = 2;                // Number of logical cores to leave free

// ====================
// Window/Rendering
// ====================
pub const WINDOW_WIDTH: u32 = 900;                      // Window width in pixels
pub const WINDOW_HEIGHT: u32 = 900;                     // Window height in pixels

// ====================
// DISPLAY/GUI Parameters
// ====================
pub const SHOW_FIELD_ISOLINES: bool = false;        /// Show electric field isolines/// Show electric-field isolines
pub const SHOW_VELOCITY_VECTORS: bool = false;      /// Show velocity vectors
pub const SHOW_ELECTRON_DENSITY: bool = false;      /// Show electron-density heatmap
pub const SHOW_FIELD_VECTORS: bool = false; // Show electric field vectors

#[derive(Clone, Debug)]
pub struct SimConfig {
    pub hop_rate_k0: f32,
    pub hop_transfer_coeff: f32,
    pub hop_activation_energy: f32,
    pub hop_radius_factor: f32,
    pub show_field_isolines: bool,
    pub show_velocity_vectors: bool,
    pub show_electron_density: bool,
    pub show_field_vectors: bool, // NEW: show field vectors
    // Add other parameters as needed
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            hop_rate_k0: HOP_RATE_K0,
            hop_transfer_coeff: HOP_TRANSFER_COEFF,
            hop_activation_energy: HOP_ACTIVATION_ENERGY,
            hop_radius_factor: HOP_RADIUS_FACTOR,
            show_field_isolines: SHOW_FIELD_ISOLINES,
            show_velocity_vectors: SHOW_VELOCITY_VECTORS,
            show_electron_density: SHOW_ELECTRON_DENSITY,
            show_field_vectors: SHOW_FIELD_VECTORS, // NEW
        }
    }
}