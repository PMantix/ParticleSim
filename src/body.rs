// Defines the body struct (position, velocity, acceleration, mass, radius, charge) and its methods
// for updating position and velocity. The charge is used to calculate the electric field and force on the body.

use ultraviolet::Vec2;
use crate::config; // <-- Add this line

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    LithiumIon,
    LithiumMetal,
    // Electron, // Not constructed, so commented out to avoid dead_code warning
}

#[derive(Clone)]
pub struct Body { //Body is a struct that represents a particle in the simulation, which is either a lithium ion, lithium metal
    pub pos: Vec2,
    pub vel: Vec2,
    pub acc: Vec2,
    pub mass: f32,
    pub radius: f32,
    pub charge: f32, 	// electric charge
    pub id: u64,
    pub species: Species,
    pub electrons: Vec<Electron>,
    pub e_field: Vec2,
}

#[derive(Clone, Debug)]
pub struct Electron {
    pub rel_pos: Vec2,
    pub vel: Vec2,
}

use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Body {
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

    pub fn update_species(&mut self) {
        if self.charge > config::LITHIUM_ION_THRESHOLD {
            self.species = Species::LithiumIon;
        } else if self.charge <= 0.0 {
            self.species = Species::LithiumMetal;
        }
    }

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

    pub fn update_charge_from_electrons(&mut self) {
        if self.species == Species::LithiumMetal {
            self.charge = -(self.electrons.len() as f32 - 1.0);
        } else {
            self.charge = 0.0; // or whatever is appropriate for ions
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