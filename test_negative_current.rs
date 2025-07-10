// Quick test to verify negative DC current works
use particle_sim::simulation::Simulation;
use particle_sim::body::{Body, Species, Electron};
use particle_sim::body::foil::Foil;
use ultraviolet::Vec2;
use smallvec::smallvec;

fn main() {
    let mut sim = Simulation::new();
    
    // Create a foil body with 2 electrons
    let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    body.electrons = smallvec![
        Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() },
        Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }
    ];
    let body_id = body.id;
    sim.bodies.push(body);
    
    // Create a foil with negative DC current to remove electrons
    let mut foil = Foil::new(vec![body_id], Vec2::zero(), 1.0, 1.0, 0.0, 0.0);
    foil.dc_current = -10.0; // Strong negative current to remove electrons
    sim.foils.push(foil);
    
    println!("Before step: {} electrons", sim.bodies[0].electrons.len());
    
    // Run a few simulation steps
    for i in 0..5 {
        sim.step();
        println!("After step {}: {} electrons", i+1, sim.bodies[0].electrons.len());
    }
}
