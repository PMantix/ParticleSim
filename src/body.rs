// Defines the body struct (position, velocity, acceleration, mass, radius, charge) and its methods
// for updating position and velocity. The charge is used to calculate the electric field and force on the body.

use ultraviolet::Vec2;

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
        if self.charge > 0.5 {
            self.species = Species::LithiumIon;
            //println!("Species: LithiumIon");
        } else if self.charge <= 0.0 {
            self.species = Species::LithiumMetal;
            //println!("Species: LithiumMetal");
        }
    }

    /*pub fn update_electrons_OLD(&mut self, net_field: Vec2, _dt: f32) {

        let net_force = -1.0 * net_field; // electron charge = -1

        let stiffness = 0.05; // Tune this value for how "stiff" the response is
        let max_dist = self.radius * 1.2;

        let offset = net_force * stiffness;
        let offset_mag = offset.mag().min(max_dist);

        let direction = if offset.mag() > 1e-8 {
            offset.normalized()
        } else {
            Vec2::zero()
        };

        for electron in &mut self.electrons {
            electron.rel_pos = direction * offset_mag;
            electron.vel = Vec2::zero(); // No velocity
        }

    }*/

    pub fn update_electrons(&mut self, net_field: Vec2, dt: f32) {
    // Spring‐constant–style stiffness for electron response
    let k = 0.05;

    for e in &mut self.electrons {
       // 1) Compute acceleration = −k × field
       let acc = -net_field * k;

       // 2) Integrate velocity, clamping its magnitude
       e.vel += acc * dt;
       let speed     = e.vel.mag();
       let max_speed = 1.2 * self.radius / dt;
       if speed > max_speed {
           e.vel = e.vel / speed * max_speed;
       }

       // 3) Move electron by its velocity
       e.rel_pos += e.vel * dt;

       // 4) Clamp electron within drift radius (1.2× body radius)
       let max_dist = 1.2 * self.radius;
       if e.rel_pos.mag() > max_dist {
           e.rel_pos = e.rel_pos.normalized() * max_dist;
       }
    }
}
    

    pub fn set_electron_count(&mut self) {
        // For Li metal: 1 electron for charge 0, 2 for -1, 3 for -2, etc.
        if self.species == Species::LithiumMetal {
            let desired = 1 + (-self.charge).round() as usize;
            while self.electrons.len() < desired {
                // Spawn at random angle near parent
                let angle = fastrand::f32() * std::f32::consts::TAU;
                let rel_pos = Vec2::new(angle.cos(), angle.sin()) * self.radius * 1.2;
                self.electrons.push(Electron { rel_pos, vel: Vec2::zero() });
            }
            while self.electrons.len() > desired {
                self.electrons.pop();
            }
        } else {
            self.electrons.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}