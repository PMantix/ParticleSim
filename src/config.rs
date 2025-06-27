// Centralized configuration for simulation parameters

// ====================
// Electron Parameters
// ====================
pub const ELECTRON_SPRING_K: f32 = 0.05;                // Spring constant for electron drift
pub const ELECTRON_DRIFT_RADIUS_FACTOR: f32 = 1.2;      // Max drift radius as a factor of body radius
pub const ELECTRON_MAX_SPEED_FACTOR: f32 = 1.2;         // Max electron speed as a factor of body radius per dt
pub const HOP_RADIUS_FACTOR: f32 = 2.1;                      // Hopping radius as a factor of body radius
pub const HOP_RATE_K0: f32 = 1.0;            /// Base hop‐rate constant (per unit time) at zero overpotential
pub const HOP_TRANSFER_COEFF: f32 = 0.5;            /// Transfer coefficient α (unitless, ~0.5)   
pub const HOP_ACTIVATION_ENERGY: f32 = 0.025;      /// Thermal energy k_BT (in your same charge‐units)

// ====================
// Butler-Volmer Parameters
// ====================
/// Enable Butler-Volmer kinetics for inter-species electron transfer
pub const BV_ENABLED: bool = false;
/// Exchange current density i0 used in the Butler-Volmer expression
pub const BV_EXCHANGE_CURRENT: f32 = 1.0;
/// Transfer coefficient alpha used in the Butler-Volmer expression
pub const BV_TRANSFER_COEFF: f32 = 0.5;
/// Scale factor corresponding to RT/(nF) for the overpotential term
pub const BV_OVERPOTENTIAL_SCALE: f32 = 0.025;

// ====================
// LJ Force Parameters
// ====================
pub const LJ_FORCE_EPSILON: f32 = 2000.0;                  // Lennard-Jones epsilon parameter
pub const LJ_FORCE_SIGMA: f32 = 1.7;                    // Lennard-Jones sigma parameter
pub const LJ_FORCE_CUTOFF: f32 = 3.5;                  // Lennard-Jones cutoff distance
pub const LJ_FORCE_MAX: f32 = 1000.0;                   // Max Lennard-Jones force magnitude
/// Density above which the cell list is used for LJ interactions
pub const LJ_CELL_DENSITY_THRESHOLD: f32 = 0.001;

// ====================
// Species/Body Parameters
// ====================
pub const LITHIUM_ION_THRESHOLD: f32 = 0.5;             // Charge threshold for lithium ion/metal transition
pub const FOIL_NEUTRAL_ELECTRONS: usize = 1;
pub const LITHIUM_METAL_NEUTRAL_ELECTRONS: usize = 1;
pub const ELECTROLYTE_ANION_NEUTRAL_ELECTRONS: usize = 1;
pub const FOIL_MAX_ELECTRONS: usize = 2;           // Max electrons for foil metal
/// Maximum number of nearby metallic neighbors allowed before ionization is inhibited
//pub const IONIZATION_NEIGHBOR_THRESHOLD: usize = 4;
/// Minimum local electric-field magnitude required for ionization/reduction
//pub const IONIZATION_FIELD_THRESHOLD: f32 = 1.0e3;
/// Radius factor (times body radius) for determining metal surroundings
pub const SURROUND_RADIUS_FACTOR: f32 = 3.5;
/// Neighbor count threshold for considering a body "surrounded" by metal
pub const SURROUND_NEIGHBOR_THRESHOLD: usize = 4;
/// Minimum displacement before recomputing `surrounded_by_metal`
pub const SURROUND_MOVE_THRESHOLD: f32 = 0.5;
/// Maximum number of frames between surround checks
pub const SURROUND_CHECK_INTERVAL: usize = 10;

// ====================
// Simulation Parameters
// ====================
pub const DEFAULT_DT: f32 = 0.005;                     // Reduced minimum simulation timestep for better stability
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
pub const WINDOW_WIDTH: u32 = 1500;                      // Window width in pixels
pub const WINDOW_HEIGHT: u32 = 1200;                     // Window height in pixels

// ====================
// DISPLAY/GUI Parameters
// ====================
pub const SHOW_FIELD_ISOLINES: bool = false;        /// Show electric field isolines/// Show electric-field isolines
pub const SHOW_VELOCITY_VECTORS: bool = false;      /// Show velocity vectors
pub const SHOW_CHARGE_DENSITY: bool = false;      /// Show charge-density heatmap
pub const SHOW_FIELD_VECTORS: bool = false; // Show electric field vectors

use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolineFieldMode {
    Total,
    ExternalOnly,
    BodyOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub hop_rate_k0: f32,
    pub hop_transfer_coeff: f32,
    pub hop_activation_energy: f32,
    pub hop_radius_factor: f32,
    /// Enable Butler-Volmer kinetics for inter-species hops
    pub use_butler_volmer: bool,
    /// Exchange current density i0 for Butler-Volmer
    pub bv_exchange_current: f32,
    /// Transfer coefficient alpha for Butler-Volmer
    pub bv_transfer_coeff: f32,
    /// Overpotential scale factor RT/(nF) for Butler-Volmer
    pub bv_overpotential_scale: f32,
    pub show_field_isolines: bool,
    pub show_velocity_vectors: bool,
    pub show_charge_density: bool,
    pub show_field_vectors: bool, // NEW: show field vectors
    pub isoline_field_mode: IsolineFieldMode,
    pub damping_base: f32, // Add base damping factor
    // --- LJ parameters for runtime tuning ---
    pub lj_force_epsilon: f32,
    pub lj_force_sigma: f32,
    pub lj_force_cutoff: f32,
    pub show_lj_vs_coulomb_ratio: bool, // Show LJ/Coulomb force ratio debug overlay
    pub cell_list_density_threshold: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            hop_rate_k0: HOP_RATE_K0,
            hop_transfer_coeff: HOP_TRANSFER_COEFF,
            hop_activation_energy: HOP_ACTIVATION_ENERGY,
            hop_radius_factor: HOP_RADIUS_FACTOR,
            use_butler_volmer: BV_ENABLED,
            bv_exchange_current: BV_EXCHANGE_CURRENT,
            bv_transfer_coeff: BV_TRANSFER_COEFF,
            bv_overpotential_scale: BV_OVERPOTENTIAL_SCALE,
            show_field_isolines: SHOW_FIELD_ISOLINES,
            show_velocity_vectors: SHOW_VELOCITY_VECTORS,
            show_charge_density: SHOW_CHARGE_DENSITY,
            show_field_vectors: SHOW_FIELD_VECTORS, // NEW
            isoline_field_mode: IsolineFieldMode::Total,
            damping_base: 0.98, // Default base damping
            lj_force_epsilon: LJ_FORCE_EPSILON,
            lj_force_sigma: LJ_FORCE_SIGMA,
            lj_force_cutoff: LJ_FORCE_CUTOFF,
            show_lj_vs_coulomb_ratio: false, // Default off
            cell_list_density_threshold: LJ_CELL_DENSITY_THRESHOLD,
        }
    }
}

use once_cell::sync::Lazy;
use parking_lot::Mutex;

pub static LJ_CONFIG: Lazy<Mutex<SimConfig>> = Lazy::new(|| Mutex::new(SimConfig::default()));