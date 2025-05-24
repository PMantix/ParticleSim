// Centralized configuration for simulation parameters

// ====================
// Electron Parameters
// ====================
pub const ELECTRON_SPRING_K: f32 = 0.05;                // Spring constant for electron drift
pub const ELECTRON_DRIFT_RADIUS_FACTOR: f32 = 1.2;      // Max drift radius as a factor of body radius
pub const ELECTRON_MAX_SPEED_FACTOR: f32 = 1.2;         // Max electron speed as a factor of body radius per dt

// ====================
// Species/Body Parameters
// ====================
pub const LITHIUM_ION_THRESHOLD: f32 = 0.5;             // Charge threshold for lithium ion/metal transition

// ====================
// Simulation Parameters
// ====================
pub const DEFAULT_DT: f32 = 0.0025;                     // Default simulation timestep
pub const DEFAULT_PARTICLE_COUNT: usize = 50000;        // Default number of particles
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
pub const CLUMP_SIZE: usize = 1000;                     // Number of particles per clump
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