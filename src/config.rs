// Centralized configuration for simulation parameters

// ====================
// Lithium-salt molarity (deprecated constant)
// ====================
// Note: Molarity is now a runtime UI control in the Scenario tab (electrolyte controls)
// and this constant is not used by the simulation. Kept only to avoid breaking imports.
pub const LITHIUM_SALT_MOLARITY: f32 = 1.0;

// ====================
// Electron Parameters
// ====================
pub const ELECTRON_SPRING_K: f32 = 5.0; // Spring constant for electron drift
pub const ELECTRON_SPRING_K_METAL: f32 = ELECTRON_SPRING_K; // Metal-specific spring constant
pub const ELECTRON_SPRING_K_EC: f32 = ELECTRON_SPRING_K; // EC-specific spring constant
pub const ELECTRON_SPRING_K_DMC: f32 = ELECTRON_SPRING_K; // DMC-specific spring constant

// Effective polarization charge (in units of e) for solvent molecules
pub const POLAR_CHARGE_EC: f32 = 0.40;
pub const POLAR_CHARGE_DMC: f32 = 0.11; //0.054;
pub const POLAR_CHARGE_DEFAULT: f32 = 1.0;

use crate::body::Species;
use crate::units;

/// Get the electron spring constant for a given species
pub fn electron_spring_k(species: Species) -> f32 {
    use Species::*;
    match species {
        LithiumMetal | FoilMetal => ELECTRON_SPRING_K_METAL,
        EC => ELECTRON_SPRING_K_EC,
        DMC => ELECTRON_SPRING_K_DMC,
        _ => ELECTRON_SPRING_K,
    }
}
pub const ELECTRON_DRIFT_RADIUS_FACTOR_EC: f32 = 1.0; // Max electron speed as a factor of body radius per
pub const ELECTRON_DRIFT_RADIUS_FACTOR_DMC: f32 = 0.73; // DMC-specific drift radius factor
pub const ELECTRON_DRIFT_RADIUS_FACTOR_METAL: f32 = 1.0; // Metal-specific drift radius factor
pub const ELECTRON_MAX_SPEED_FACTOR: f32 = 10.2; // Max electron speed as a factor of body radius per dt
pub const HOP_RADIUS_FACTOR: f32 = 2.1; // Hopping radius as a factor of body radius
pub const HOP_RATE_K0: f32 = 1.0;
/// Base hop‐rate constant (per unit time) at zero overpotential
pub const HOP_TRANSFER_COEFF: f32 = 0.5;
/// Transfer coefficient α (unitless, ~0.5)
pub const HOP_ACTIVATION_ENERGY: f32 = 0.025;
/// Thermal energy k_BT (in your same charge‐units)
pub fn default_hop_alignment_bias() -> f32 {
    0.01
}


/// Get the effective polarization charge for a given species
/*
    use Species::*;
    match species {
        EC => POLAR_CHARGE_EC,
        DMC => POLAR_CHARGE_DMC,
        _ => POLAR_CHARGE_DEFAULT,
    }
}*/
// ====================
// Butler-Volmer Parameters
// ====================
/// Enable Butler-Volmer kinetics for inter-species electron transfer
pub const BV_ENABLED: bool = true;
/// Exchange current density i0 used in the Butler-Volmer expression
pub const BV_EXCHANGE_CURRENT: f32 = 0.1;
/// Transfer coefficient alpha used in the Butler-Volmer expression
pub const BV_TRANSFER_COEFF: f32 = 0.5;
/// Scale factor corresponding to RT/(nF) for the overpotential term
pub const BV_OVERPOTENTIAL_SCALE: f32 = 0.025;

// ====================
// LJ Force Parameters
// ====================
/// Lennard-Jones epsilon in electronvolts.
pub const LJ_EPSILON_EV: f32 = 0.0103;
/// Lennard-Jones sigma in angstroms.
pub const LJ_SIGMA_A: f32 = 1.80;
/// Lennard-Jones cutoff distance in angstroms.
pub const LJ_CUTOFF_A: f32 = 2.2;
/// Lennard-Jones epsilon converted to simulation energy units.
pub const LJ_FORCE_EPSILON: f32 = (LJ_EPSILON_EV as f64 * units::EV_TO_SIM) as f32;
/// Lennard-Jones sigma in simulation length units (angstroms).
pub const LJ_FORCE_SIGMA: f32 = LJ_SIGMA_A;
/// Lennard-Jones cutoff in simulation length units (angstroms).
pub const LJ_FORCE_CUTOFF: f32 = LJ_CUTOFF_A;
/// Max Lennard-Jones force magnitude (simulation units).
pub const LJ_FORCE_MAX: f32 = 200.0;
/// Density above which the cell list is used for LJ interactions
pub const LJ_CELL_DENSITY_THRESHOLD: f32 = 0.001;

// ====================
// Species/Body Parameters
// ====================
pub const LITHIUM_ION_THRESHOLD: f32 = 0.5; // Charge threshold for lithium ion/metal transition
pub const FOIL_NEUTRAL_ELECTRONS: usize = 1;
pub const LITHIUM_METAL_NEUTRAL_ELECTRONS: usize = 1;
pub const ELECTROLYTE_ANION_NEUTRAL_ELECTRONS: usize = 0;
pub const EC_NEUTRAL_ELECTRONS: usize = 1;
pub const DMC_NEUTRAL_ELECTRONS: usize = 1;
pub const FOIL_MAX_ELECTRONS: usize = 2; // Max electrons for foil metal
pub const LITHIUM_METAL_MAX_ELECTRONS: usize = 3; // Max electrons for lithium metal
/// Maximum number of nearby metallic neighbors allowed before ionization is inhibited
//pub const IONIZATION_NEIGHBOR_THRESHOLD: usize = 4;
/// Minimum local electric-field magnitude required for ionization/reduction
//pub const IONIZATION_FIELD_THRESHOLD: f32 = 1.0e3;
/// Enable electron sea protection: metals surrounded by other metals resist oxidation
pub const ENABLE_ELECTRON_SEA_PROTECTION: bool = true;
/// Radius factor (times body radius) for determining metal surroundings
pub const SURROUND_RADIUS_FACTOR: f32 = 3.5;
/// Neighbor count threshold for considering a body "surrounded" by metal
pub const SURROUND_NEIGHBOR_THRESHOLD: usize = 4;
/// Minimum displacement before recomputing `surrounded_by_metal`
pub const SURROUND_MOVE_THRESHOLD: f32 = 0.5;
/// Maximum number of frames between surround checks
pub const SURROUND_CHECK_INTERVAL: usize = 10;

// ====================
// History/Playback Performance
// ====================
/// History capture interval - capture every N frames instead of every frame
/// This prevents O(N) per-frame overhead from killing performance with many particles
/// Value of 5 = capture every 5th frame = 5x less history overhead, playback still smooth

/// Number of frames of history preserved for playback controls
/// Simple ring buffer approach - much faster than compressed deltas
pub const PLAYBACK_HISTORY_FRAMES: usize = 10000;

// ====================
// Simulation Parameters
// ====================
/// Default timestep in femtoseconds.
/// Typical MD timesteps: 0.5-2.0 fs. Old value was 0.015 fs (too small).
pub const DEFAULT_DT_FS: f32 = 5.0;
pub const COLLISION_PASSES: usize = 7; // Number of collision resolution passes
/// Number of frames of history preserved for playback controls
/// Memory usage: ~115KB per 1000 particles per frame
/// 5000 frames ≈ 576MB for small sims, 2.9GB for medium sims
// Configuration constants

// ====================
// Quadtree Parameters
// ====================
pub const QUADTREE_THETA: f32 = 1.0; // Barnes-Hut opening angle
pub const QUADTREE_EPSILON: f32 = 2.0; // Softening parameter
pub const QUADTREE_LEAF_CAPACITY: usize = 1; // Max bodies per quadtree leaf
pub const QUADTREE_THREAD_CAPACITY: usize = 1024; // Max bodies per thread chunk

// ====================
// Initialization/Clumping
// ====================
pub const CLUMP_RADIUS: f32 = 20.0; // Radius of each clump
pub const DOMAIN_BOUNDS: f32 = 350.0; // Simulation domain boundary
/// Half-depth of the simulation domain for quasi-3D motion
pub const DOMAIN_DEPTH: f32 = 1.0;
pub const OUT_OF_PLANE_ENABLED: bool = false;
pub const Z_STIFFNESS: f32 = 1.0;
pub const Z_DAMPING: f32 = 0.5;
pub const MAX_Z: f32 = DOMAIN_DEPTH;
// Z-axis constraint parameters removed (simplified approach)

// ====================
// Li+ Collision Softness (Simple)
// ====================
/// Single knob controlling how soft Li+ collisions are.
/// 0.0 = hard collisions (baseline); 1.0 = very soft (max reduction).
pub const LI_COLLISION_SOFTNESS: f32 = 0.8;
/// Size of position history buffer for movement analysis
// Position history removed (simplified approach)

// ====================
// Threading/Parallelism
// ====================
pub const MIN_THREADS: usize = 3; // Minimum number of threads to use
pub const THREADS_LEAVE_FREE: usize = 2; // Number of logical cores to leave free

// ====================
// Window/Rendering
// ====================
pub const WINDOW_WIDTH: u32 = 1500; // Window width in pixels
pub const WINDOW_HEIGHT: u32 = 1200; // Window height in pixels

// ====================
// DISPLAY/GUI Parameters
// ====================
pub const SHOW_FIELD_ISOLINES: bool = false;
/// Show electric field isolines/// Show electric-field isolines
pub const SHOW_VELOCITY_VECTORS: bool = false;
/// Show velocity vectors
pub const SHOW_CHARGE_DENSITY: bool = false;
/// Show charge-density heatmap (DISABLED FOR PERFORMANCE)
pub const SHOW_2D_DOMAIN_DENSITY: bool = false;
/// Show 2D particle density heatmap (DISABLED FOR PERFORMANCE)
pub const SHOW_FIELD_VECTORS: bool = false; // Show electric field vectors (DISABLED FOR PERFORMANCE)

// ====================
// Temperature
// ====================
/// Default simulation temperature for thermal motion (Kelvin)
pub const DEFAULT_TEMPERATURE: f32 = 300.0; // Room temperature in Kelvin

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DipoleModel {
    /// Original model: only use the field difference at nucleus vs. electron from neighbors' NET charges
    /// (i.e., do not treat neighbors as explicit ±q dipoles). Simpler and typically more stable.
    SingleOffset,
    /// Newer model: treat EC/DMC as explicit ±q_eff conjugate pairs to enable dipole–dipole interactions.
    ConjugatePair,
}

impl Default for DipoleModel {
    fn default() -> Self {
        DipoleModel::SingleOffset
    }
}

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
    #[serde(default = "default_hop_alignment_bias")]
    pub hop_alignment_bias: f32,
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
    pub show_2d_domain_density: bool,
    pub show_field_vectors: bool, // NEW: show field vectors
    pub isoline_field_mode: IsolineFieldMode,
    /// Number of isoline levels to plot
    pub isoline_count: usize,
    /// Target samples across the shorter viewport dimension for isoline fidelity
    pub isoline_target_samples: usize,
    /// Shift isoline percentile mapping up/down the sampled range [-0.5..0.5], applied after clipping
    pub isoline_bias: f32,
    /// Clip isoline percentile range by this margin on both ends [0.0..0.49]
    pub isoline_clip_margin: f32,
    /// Enable local refinement around cells that an isoline crosses (reduces boxy artifacts)
    pub isoline_local_refine: bool,
    /// Subdivision factor for local refinement (2 = 2x2 per cell)
    pub isoline_local_refine_factor: usize,
    /// Fraction of levels to refine centered around mid-range (1.0 = all levels)
    pub isoline_local_refine_band: f32,
    /// Strength of color deviation from white for isolines [0..1]
    pub isoline_color_strength: f32,
    /// Gamma on signed magnitude mapping for isoline color (perceptual control)
    pub isoline_color_gamma: f32,
    /// Draw translucent filled isobands between levels
    pub isoline_filled: bool,
    /// Alpha for filled isobands [0..255]
    pub isoline_fill_alpha: u8,
    /// Nonlinear distribution exponent for level percentiles (1.0 = linear; >1 pushes toward extremes)
    pub isoline_distribution_gamma: f32,
    pub damping_base: f32,              // Add base damping factor
    pub show_lj_vs_coulomb_ratio: bool, // Show LJ/Coulomb force ratio debug overlay
    pub cell_list_density_threshold: f32,
    // Global LJ parameters for GUI control
    pub lj_force_epsilon: f32,
    pub lj_force_sigma: f32,
    pub lj_force_cutoff: f32,
    pub coulomb_constant: f32,
    /// Current simulation temperature
    pub temperature: f32,
    /// Interval between thermostat applications (fs)
    #[serde(alias = "thermostat_frequency")]
    pub thermostat_interval_fs: f32,
    pub enable_out_of_plane: bool,
    pub z_stiffness: f32,
    pub z_damping: f32,
    pub max_z: f32,
    /// Enable expensive many-body z-forces (solvation, density effects)
    pub enable_z_many_body_forces: bool,

    // Li+ collision softness (simple, force-independent)
    pub li_collision_softness: f32,
    /// Enable soft-collision scaling for Li+ ions
    pub soft_collision_lithium_ion: bool,
    /// Enable soft-collision scaling for electrolyte anions
    pub soft_collision_anion: bool,

    // Induced external field from foil charging
    /// Gain mapping foil drive (current or overpotential) to induced |E|
    pub induced_field_gain: f32,
    /// Exponential smoothing factor for induced field [0..1): higher = smoother
    pub induced_field_smoothing: f32,
    /// If true, use foil centroids to set field direction (neg -> pos). If false, keep UI direction.
    pub induced_field_use_direction: bool,
    /// Scale that converts overpotential ratio deviation |target-1| into an equivalent drive
    pub induced_field_overpot_scale: f32,

    /// Vacancy polarization bias gain: scales the influence of local valence-electron offset on hop selection
    pub hop_vacancy_polarization_gain: f32,

    /// Dipole interaction model for EC/DMC
    #[serde(default)]
    pub dipole_model: DipoleModel,

    /// Version number incremented whenever config changes (for clone detection)
    #[serde(skip)]
    pub config_version: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            config_version: 0,
            hop_rate_k0: HOP_RATE_K0,
            hop_transfer_coeff: HOP_TRANSFER_COEFF,
            hop_activation_energy: HOP_ACTIVATION_ENERGY,
            hop_radius_factor: HOP_RADIUS_FACTOR,
            hop_alignment_bias: default_hop_alignment_bias(),
            use_butler_volmer: BV_ENABLED,
            bv_exchange_current: BV_EXCHANGE_CURRENT,
            bv_transfer_coeff: BV_TRANSFER_COEFF,
            bv_overpotential_scale: BV_OVERPOTENTIAL_SCALE,
            show_field_isolines: SHOW_FIELD_ISOLINES,
            show_velocity_vectors: SHOW_VELOCITY_VECTORS,
            show_charge_density: SHOW_CHARGE_DENSITY,
            show_2d_domain_density: SHOW_2D_DOMAIN_DENSITY,
            show_field_vectors: SHOW_FIELD_VECTORS, // NEW
            isoline_field_mode: IsolineFieldMode::Total,
            isoline_count: 12,
            isoline_target_samples: 2,
            isoline_bias: 0.0,
            isoline_clip_margin: 0.035,
            isoline_local_refine: true,
            isoline_local_refine_factor: 4,
            isoline_local_refine_band: 1.0,
            isoline_color_strength: 0.4,
            isoline_color_gamma: 1.0,
            isoline_filled: false,
            isoline_fill_alpha: 40,
            isoline_distribution_gamma: 1.0,
            damping_base: 1.00,              // Default base damping
            show_lj_vs_coulomb_ratio: false, // Default off
            cell_list_density_threshold: LJ_CELL_DENSITY_THRESHOLD,
            lj_force_epsilon: LJ_FORCE_EPSILON,
            lj_force_sigma: LJ_FORCE_SIGMA,
            lj_force_cutoff: LJ_FORCE_CUTOFF,
            coulomb_constant: units::COULOMB_CONSTANT,
            temperature: DEFAULT_TEMPERATURE,
            thermostat_interval_fs: 1.0, // Apply thermostat every 1 fs by default
            enable_out_of_plane: OUT_OF_PLANE_ENABLED,
            z_stiffness: Z_STIFFNESS,
            z_damping: Z_DAMPING,
            max_z: MAX_Z,
            enable_z_many_body_forces: false, // Default to false for performance

            // Li+ collision softness (simple)
            li_collision_softness: LI_COLLISION_SOFTNESS,
            soft_collision_lithium_ion: true,
            soft_collision_anion: false,

            // Induced external field defaults (disabled by default via zero gain)
            induced_field_gain: 0.0,
            induced_field_smoothing: 0.9,
            induced_field_use_direction: true,
            induced_field_overpot_scale: 100.0,

            // Vacancy polarization bias (disabled by default)
            hop_vacancy_polarization_gain: 300.0,

            // Dipole model default: original SingleOffset
            dipole_model: DipoleModel::SingleOffset,
        }
    }
}

use once_cell::sync::Lazy;
use parking_lot::Mutex;

pub static LJ_CONFIG: Lazy<Mutex<SimConfig>> = Lazy::new(|| Mutex::new(SimConfig::default()));
