// Defines the body struct (position, velocity, acceleration, mass, radius, charge) and its methods
// for updating position and velocity. The charge is used to calculate the electric field and force on the body.

use ultraviolet::Vec2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    LithiumIon,
    LithiumMetal,
    Electron,
}

#[derive(Clone, Copy)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    pub acc: Vec2,
    pub mass: f32,
    pub radius: f32,
	pub charge: f32, 	// electric charge
    pub id: u64,
    pub species: Species,
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
        }
    }

    /*pub fn update(&mut self, dt: f32) {
        self.vel += self.acc * dt;
        self.pos += self.vel * dt;
    }*/

    pub fn update_species(&mut self) {
        if self.charge > 0.5 {
            self.species = Species::LithiumIon;
            println!("Species: LithiumIon");
        } else if self.charge == 0.0 {
            self.species = Species::LithiumMetal;
            println!("Species: LithiumMetal");
        } else if self.charge < 0.0 {
            self.species = Species::Electron;
            println!("Species: Electron");
        }
    }
}
