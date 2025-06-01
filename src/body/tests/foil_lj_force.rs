// Tests for foil LJ force and attraction
#[cfg(test)]
mod foil_lj_force {
    use crate::body::{Body, Species, Electron};
    use crate::body::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    #[test]
    fn foil_lj_force_affects_metal() {
        let mut sim = Simulation::new();
        let mut foil_body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        foil_body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        sim.bodies.push(foil_body);
        let foil_id = sim.bodies.last().expect("Foil body not found after push").id;
        let metal_body = Body::new(Vec2::new(1.2, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        sim.bodies.push(metal_body);
        let metal_id = sim.bodies.last().expect("Metal body not found after push").id;
        sim.foils.push(Foil::new(vec![foil_id], Vec2::zero(), 1.0, 1.0, 0.0));
        sim.quadtree.build(&mut sim.bodies);
        let foil = sim.bodies.iter().find(|b| b.id == foil_id).expect("Foil not found");
        let metal = sim.bodies.iter().find(|b| b.id == metal_id).expect("Metal not found");
        let initial_dist = (foil.pos - metal.pos).mag();
        println!("Initial metal position: {:?}", metal.pos);
        for _step in 0..3 {
            sim.step();
        }
        let foil = sim.bodies.iter().find(|b| b.id == foil_id).expect("Foil not found after step");
        let metal = sim.bodies.iter().find(|b| b.id == metal_id).expect("Metal not found after step");
        println!("Final metal position: {:?}", metal.pos);
        let new_dist = (foil.pos - metal.pos).mag();
        assert!(new_dist < initial_dist, "LithiumMetal should be attracted to FoilMetal by LJ force");
    }
}
