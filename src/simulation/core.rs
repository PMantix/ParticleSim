// simulation/core.rs
// Contains the Simulation struct and main methods (new, step)

use crate::{body::{Body, Species}, quadtree::Quadtree, utils};
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES};
use ultraviolet::Vec2;
use super::forces;
use super::collision;
use crate::config;
use crate::config::{HOP_RADIUS_FACTOR, HOP_CHARGE_THRESHOLD};

/// The main simulation state and logic for the particle system.
/// 
/// Holds all particles (bodies), manages the simulation step, and handles physics such as
/// force calculation, electron hopping, redox reactions, and integration.
pub struct Simulation {
    /// Simulation timestep (seconds per frame)
    pub dt: f32,
    /// Current frame number
    pub frame: usize,
    /// All particles in the simulation
    pub bodies: Vec<Body>,
    /// Quadtree for spatial partitioning (used for force calculations)
    pub quadtree: Quadtree,
    /// Half-size of the simulation bounding box
    pub bounds: f32,
    /// Flags for rewinding (used for undo/rewind features)
    pub rewound_flags: Vec<bool>,
    /// Uniform background electric field (set by GUI)
    pub background_e_field: Vec2,
}

impl Simulation {
    /// Create a new simulation with default parameters and initial particle configuration.
    pub fn new() -> Self {
        let dt = config::DEFAULT_DT;
        let n = config::DEFAULT_PARTICLE_COUNT;
        let theta = config::QUADTREE_THETA;
        let epsilon = config::QUADTREE_EPSILON;
        let leaf_capacity = config::QUADTREE_LEAF_CAPACITY;
        let thread_capacity = config::QUADTREE_THREAD_CAPACITY;
        let clump_size = config::CLUMP_SIZE;
        let clump_radius = config::CLUMP_RADIUS;
        let bounds = config::DOMAIN_BOUNDS;
        let bodies = utils::two_lithium_clumps_with_ions(n, clump_size, clump_radius, bounds);
        let quadtree = Quadtree::new(theta, epsilon, leaf_capacity, thread_capacity);
        let rewound_flags = vec![false; bodies.len()];
        Self {
            dt,
            frame: 0,
            bodies,
            quadtree,
            bounds,
            rewound_flags,
            background_e_field: Vec2::zero(),
        }
    }

    /// Advance the simulation by one timestep.
    ///
    /// This updates the electric field, resets flags, computes forces, integrates motion,
    /// handles collisions, updates electron states, and performs electron hopping.
    pub fn step(&mut self) {
        // Update uniform E-field from sliders
        {
            let mag = *FIELD_MAGNITUDE.lock();
            let theta = (*FIELD_DIRECTION.lock()).to_radians();
            self.background_e_field = Vec2::new(theta.cos(), theta.sin()) * mag;
        }
        // Reset rewound flags
        for flag in &mut self.rewound_flags {
            *flag = false;
        }
        self.dt = *TIMESTEP.lock();
        // Reset all accelerations
        for body in &mut self.bodies {
            body.acc = Vec2::zero();
        }
        // Compute forces
        forces::attract(self);
        forces::apply_lj_forces(self);
        // Integrate equations of motion
        self.iterate();
        // Collision passes
        let num_passes = *COLLISION_PASSES.lock();
        for _ in 1..num_passes {
            collision::collide(self);
        }
        // Update electrons for each Li metal atom
        for body in &mut self.bodies {
            //body.set_electron_count();
            body.update_electrons(body.e_field, self.dt);
            body.update_charge_from_electrons();
        }

        // Perform electron hopping pass
        self.perform_electron_hopping();

        self.frame += 1;
    }

    /// Integrate equations of motion for all bodies (velocity Verlet with damping).
    /// Handles wall reflections.
    pub fn iterate(&mut self) {
        let damping = 0.999;
        for body in &mut self.bodies {
            body.vel += body.acc * self.dt;
            body.vel *= damping;
            body.pos += body.vel * self.dt;
            // Reflect from walls
            for axis in 0..2 {
                let pos = if axis == 0 { &mut body.pos.x } else { &mut body.pos.y };
                let vel = if axis == 0 { &mut body.vel.x } else { &mut body.vel.y };
                if *pos < -self.bounds {
                    *pos = -self.bounds;
                    *vel = -(*vel);
                } else if *pos > self.bounds {
                    *pos = self.bounds;
                    *vel = -(*vel);
                }
            }
        }
    }

    /// Perform electron hopping between eligible lithium metal and ion particles.
    ///
    /// This function finds pairs of particles where an electron can hop from a metal atom
    /// (with more than one electron) to a neighbor (metal with fewer electrons or an ion),
    /// within a certain radius and charge threshold. After hopping, redox state is updated.
    pub fn perform_electron_hopping(&mut self) {
        let n = self.bodies.len();
        let mut hops: Vec<(usize, usize)> = vec![];

        // Identify all valid electron hops for this step
        for src_idx in 0..n {
            let src_body = &self.bodies[src_idx];
            // Only lithium metal atoms with more than one electron can donate
            if src_body.species != Species::LithiumMetal || src_body.electrons.len() <= 1 {
                continue;
            }
            let hop_radius = HOP_RADIUS_FACTOR * src_body.radius;
            // Find a neighbor with higher charge (less negative)
            if let Some(dst_idx) = self.bodies
                .iter()
                .enumerate()
                .filter(|&(j, b)| {
                    j != src_idx &&
                    (
                        (b.species == Species::LithiumMetal && b.electrons.len() < src_body.electrons.len() && b.charge > src_body.charge)
                        ||
                        (b.species == Species::LithiumIon)
                    )
                })
                .filter(|(_, b)| (b.pos - src_body.pos).mag() <= hop_radius)
                .filter(|(_, b)| {
                    // Only hop if destination has fewer electrons or is at higher potential
                    (b.charge > src_body.charge) && (b.electrons.len() < src_body.electrons.len())
                })
                .min_by(|(_, a), (_, b)| {
                    let da = a.charge - src_body.charge;
                    let db = b.charge - src_body.charge;
                    da.partial_cmp(&db).unwrap()
                })
                .map(|(j, _)| j)
            {
                let dst_body = &self.bodies[dst_idx];
                if dst_body.charge - src_body.charge >= HOP_CHARGE_THRESHOLD {
                    hops.push((src_idx, dst_idx));
                }
            }
        }

        // Apply all hops (transfer electrons and update redox state)
        for (src_idx, dst_idx) in hops {
            // To avoid double mutable borrow, split the borrow using split_at_mut
            let (first, second) = self.bodies.split_at_mut(std::cmp::max(src_idx, dst_idx));
            let (src, dst) = if src_idx < dst_idx {
                (&mut first[src_idx], &mut second[0])
            } else {
                (&mut second[0], &mut first[dst_idx])
            };
            // Redox: transfer electron and update redox state
            if src.electrons.len() > 1 {
                if let Some(e) = src.electrons.pop() {
                    // Debug output for tracing electron hops
                    println!("-------------------");
                    println!("Electron hopping: src={} (charge={}), dst={} (charge={})", src_idx, src.charge, dst_idx, dst.charge);
                    println!("Electron hopping: src species={:?}, dst species={:?}", src.species, dst.species);
                    println!("Electron hopping: src electrons={}, dst electrons={}", src.electrons.len(), dst.electrons.len());
                    println!("-------------------");
                    dst.electrons.push(e);
                    src.apply_redox();
                    dst.apply_redox();
                }
            }
        }
    }

}

#[cfg(test)]
mod redox_tests {
    use super::*; // for Simulation, Species
    use crate::body::{Body, Electron};
    use ultraviolet::Vec2;

    #[test]
    fn ion_reduces_to_metal_on_electron_arrival() {
        // Setup: one ion with one electron attached
        let mut ion = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumIon,
        );
        //ion.update_charge_from_electrons();
        //println!("Start --- Ion charge: {}, Ion electrons: {}", ion.charge, ion.electrons.len());
        ion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        ion.update_charge_from_electrons();
        //println!("After Charge --- Ion charge: {}, Ion electrons: {}", ion.charge, ion.electrons.len());

        let mut sim = Simulation {
            dt: 0.1,
            frame: 0,
            bodies: vec![ion],
            quadtree: Quadtree::new(
                config::QUADTREE_THETA,
                config::QUADTREE_EPSILON,
                config::QUADTREE_LEAF_CAPACITY,
                config::QUADTREE_THREAD_CAPACITY,
            ),
            bounds: 1.0,
            rewound_flags: vec![false],
            background_e_field: Vec2::zero(),
        };

        
        //sim.perform_redox();
        let b = &mut sim.bodies[0];
        b.apply_redox();
        //println!("After Redox --- Ion charge: {}, Ion electrons: {}", b.charge, b.electrons.len());

        assert_eq!(b.species, Species::LithiumMetal, "Ion with electron should become metal");
        assert_eq!(b.electrons.len(), 1, "Should have one valence electron");
        assert_eq!(b.charge, 0.0, "Neutral metal should have charge 0");
    }

    #[test]
    fn metal_oxidizes_to_ion_when_bare() {
        // Setup: one metal with zero electrons
        let metal = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumMetal,
        );
        let mut sim = Simulation {
            dt: 0.1,
            frame: 0,
            bodies: vec![metal],
            quadtree: Quadtree::new(
                config::QUADTREE_THETA,
                config::QUADTREE_EPSILON,
                config::QUADTREE_LEAF_CAPACITY,
                config::QUADTREE_THREAD_CAPACITY,
            ),
            bounds: 1.0,
            rewound_flags: vec![false],
            background_e_field: Vec2::zero(),
        };

        //println!("Start --- Metal charge: {}, Metal electrons: {}", sim.bodies[0].charge, sim.bodies[0].electrons.len());
        //println!("Metal species: {:?}", sim.bodies[0].species);
        //sim.perform_redox();
        let b = &mut sim.bodies[0];
        b.apply_redox();
        //println!("After Redox --- Metal charge: {}, Metal electrons: {}", b.charge, b.electrons.len());
        //println!("Metal species: {:?}", b.species);

        let b = &sim.bodies[0];
        assert_eq!(b.species, Species::LithiumIon, "Metal with no electrons should become ion");
        assert_eq!(b.charge, 1.0, "Ion with no electrons should have charge +1");
    }
}
