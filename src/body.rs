//! Defines the `Body` struct and related types for representing particles (lithium ions and metals) in the simulation.
//!
//! Each `Body` tracks its position, velocity, charge, species (ion/metal), and associated electrons.
//! The module provides methods for updating physical state, handling redox transitions, and simulating electron dynamics.

use ultraviolet::Vec2;
use crate::config;

/// The chemical species of a particle: either a lithium ion or lithium metal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    /// A lithium ion (Li+), typically with charge +1 and no electrons.
    LithiumIon,
    /// A lithium metal atom (Li), typically with charge 0 and one valence electron.
    LithiumMetal,
}

/// Represents a particle in the simulation (either a lithium ion or metal atom).
///
/// - `pos`, `vel`, `acc`: Physical state in 2D space.
/// - `mass`, `radius`: Physical properties.
/// - `charge`: Electric charge, always derived from electron count and species.
/// - `species`: Ion or metal.
/// - `electrons`: List of valence electrons (usually 0 or 1 for Li).
/// - `e_field`: Local electric field at the particle's position.
#[derive(Clone)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    pub acc: Vec2,
    pub mass: f32,
    pub radius: f32,
    pub charge: f32,
    pub id: u64,
    pub species: Species,
    pub electrons: Vec<Electron>,
    pub e_field: Vec2,
}

/// Represents a valence electron bound to a metal atom.
#[derive(Clone, Debug)]
pub struct Electron {
    /// Position relative to the parent atom.
    pub rel_pos: Vec2,
    /// Velocity of the electron (for simulating drift).
    pub vel: Vec2,
}

use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Body {
    /// Create a new particle (ion or metal) with the given properties.
    /// Note: electrons should be initialized separately for metals.
    pub fn new(pos: Vec2, vel: Vec2, mass: f32, radius: f32, charge: f32, species: Species) -> Self {
        Self {
            pos,
            vel,
            acc: Vec2::zero(),
            mass,
            radius,
            charge,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            species,
            electrons: Vec::new(),
            e_field: Vec2::zero(),
        }
    }

    /// Update the species (ion/metal) based on the current charge.
    /// Used for legacy compatibility; prefer using `apply_redox` for physical accuracy.
    pub fn update_species(&mut self) {
        if self.charge > config::LITHIUM_ION_THRESHOLD {
            self.species = Species::LithiumIon;
        } else if self.charge <= 0.0 {
            self.species = Species::LithiumMetal;
        }
    }

    /// Update the positions and velocities of all valence electrons under the given net field.
    pub fn update_electrons(&mut self, net_field: Vec2, dt: f32) {
        let k = config::ELECTRON_SPRING_K;

        for e in &mut self.electrons {
            let acc = -net_field * k;
            e.vel += acc * dt;
            let speed = e.vel.mag();
            let max_speed = config::ELECTRON_MAX_SPEED_FACTOR * self.radius / dt;
            if speed > max_speed {
                e.vel = e.vel / speed * max_speed;
            }
            e.rel_pos += e.vel * dt;
            let max_dist = config::ELECTRON_DRIFT_RADIUS_FACTOR * self.radius;
            if e.rel_pos.mag() > max_dist {
                e.rel_pos = e.rel_pos.normalized() * max_dist;
            }
        }
    }

    /// Set the number of valence electrons for a metal atom based on its charge.
    /// For Li metal: 1 electron is neutral, >1 is anionic, <1 is cationic.
    pub fn _set_electron_count(&mut self) {
        if self.species == Species::LithiumMetal {
            let desired = 1 + (-self.charge).round() as usize;
            while self.electrons.len() < desired {
                let angle = fastrand::f32() * std::f32::consts::TAU;
                let rel_pos = Vec2::new(angle.cos(), angle.sin()) * self.radius * config::ELECTRON_DRIFT_RADIUS_FACTOR;
                self.electrons.push(Electron { rel_pos, vel: Vec2::zero() });
            }
            while self.electrons.len() > desired {
                self.electrons.pop();
            }
        } else {
            self.electrons.clear();
        }
    }

    /// Update the charge based on the number of electrons and species.
    /// - For Li metal: charge = -(n_electrons - 1)
    /// - For Li ion: charge = 1 - n_electrons
    pub fn update_charge_from_electrons(&mut self) {
        match self.species {
            Species::LithiumMetal => {
                self.charge = -(self.electrons.len() as f32 - 1.0);
            }
            Species::LithiumIon => {
                self.charge = 1.0 - self.electrons.len() as f32;
            }
        }
    }

    /// Apply redox transitions based on electron count:
    /// - Ion with ≥1 electron becomes metal (no electron is consumed; the electron causes reduction).
    /// - Metal with 0 electrons becomes ion.
    /// Always updates charge after transition.
    pub fn apply_redox(&mut self) {
        match self.species {
            Species::LithiumIon => {
                if !self.electrons.is_empty() {
                    // Ion with at least one electron becomes metal
                    self.species = Species::LithiumMetal;
                    self.update_charge_from_electrons();
                }
            }
            Species::LithiumMetal => {
                if self.electrons.is_empty() {
                    // Metal with no electrons becomes ion
                    self.species = Species::LithiumIon;
                    self.update_charge_from_electrons();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Simulation; // Only imported for tests
    use crate::quadtree::Quadtree; // Only imported for tests

    #[test]
    fn ion_becomes_metal_when_charge_high() {
        let mut b = Body {
            pos: Vec2::zero(),
            vel: Vec2::zero(),
            acc: Vec2::zero(),
            mass: 1.0,
            radius: 1.0,
            charge: 0.00, //above the threshold to become "lithium metal"
            id: 0,
            species: Species::LithiumIon,
            electrons: Vec::new(),
            e_field: Vec2::zero(),
        };
        b.update_species();
        assert_eq!(b.species, Species::LithiumMetal);
    }

    #[test]
    fn metal_becomes_ion_when_charge_low() {
        let mut b = Body {
                       pos: Vec2::zero(),
            vel: Vec2::zero(),
            acc: Vec2::zero(),
            mass: 1.0,
            radius: 1.0,
            charge: 1.0,                     // below your ion‐threshold (0.0)
            id: 0,
            species: Species::LithiumMetal,
            electrons: Vec::new(),
            e_field: Vec2::zero(),
        };
        b.update_species();
        assert_eq!(b.species, Species::LithiumIon);
    }

    #[cfg(test)]
    mod electron_tests {
        use super::*;

        #[test]
        fn electron_moves_under_field() {
            let mut b = Body::new(
                Vec2::zero(),
                Vec2::zero(),
                1.0,1.0,
                0.0,
                Species::LithiumMetal,
            );
            //exactly one electrode at center
            b.electrons=vec![Electron {rel_pos:Vec2::zero(),vel:Vec2::zero()}];

            //apply a rightward field
            let field = Vec2::new(1.0, 0.0);
            b.update_electrons(field, 0.1);

            // the electron should have moved positively in x
            assert!(b.electrons[0].rel_pos.x < 0.0, 
                "Expected electrion to drift left (x < 0), but rel_pos.x = {}", b.electrons[0].rel_pos.x);
        }
    }

    #[cfg(test)]
    mod hopping_tests {
        use super::*;

        #[test]
        fn electron_hops_to_lower_potential_metal() {
            //Create two metals side by side
            let mut a = Body::new(
                Vec2::new(0.0, 0.0),
                Vec2::zero(),
                1.0, 1.0,
                1.0,
                Species::LithiumMetal,
            );  
            let mut b = Body::new(
                Vec2::new(1.0, 0.0),
                Vec2::zero(),
                1.0, 1.0,
                -2.0,
                Species::LithiumMetal,
            );

            a.update_charge_from_electrons();    

            for _e in 0..3 {
                b.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
            b.update_charge_from_electrons();

            let mut sim = Simulation {
                bodies: vec![a, b],
                dt: 0.1,
                background_e_field: Vec2::zero(),
                bounds: 100.0,
                frame: 0,
                quadtree: Quadtree::new(
                    config::QUADTREE_THETA,
                    config::QUADTREE_EPSILON,
                    config::QUADTREE_LEAF_CAPACITY,
                    config::QUADTREE_THREAD_CAPACITY,
                ),
                rewound_flags: vec![false; 2],
                // Add any other required fields with dummy/test values as needed
            };

            // Before hop, A should have 0 electrons, B should have 3
            assert_eq!(sim.bodies[0].electrons.len(), 0);
            assert_eq!(sim.bodies[1].electrons.len(), 3);
            
            // Run a single sim step (you may need to call only the hop logic)
            sim.perform_electron_hopping();

            // After hop, A should have 1 electrons, B should have 2
            assert_eq!(sim.bodies[0].electrons.len(), 1);
            assert_eq!(sim.bodies[1].electrons.len(), 2);
        }
    }
}