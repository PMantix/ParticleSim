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
        let metal_body = Body::new(Vec2::new(1.8, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        sim.bodies.push(metal_body);
        let metal_id = sim.bodies.last().expect("Metal body not found after push").id;
        sim.foils.push(Foil::new(vec![foil_id], Vec2::zero(), 1.0, 1.0, 0.0));
        sim.quadtree.build(&mut sim.bodies);
        let foil = sim.bodies.iter().find(|b| b.id == foil_id).expect("Foil not found");
        let metal = sim.bodies.iter().find(|b| b.id == metal_id).expect("Metal not found");
        let initial_dist = (foil.pos - metal.pos).mag();
        println!("Initial metal position: {:?}", metal.pos);
        for _step in 0..100 {
            sim.step();
        }
        let foil = sim.bodies.iter().find(|b| b.id == foil_id).expect("Foil not found after step");
        let metal = sim.bodies.iter().find(|b| b.id == metal_id).expect("Metal not found after step");
        println!("Final metal position: {:?}", metal.pos);
        let new_dist = (foil.pos - metal.pos).mag();
        assert!(new_dist < initial_dist, "LithiumMetal should be attracted to FoilMetal by LJ force");
    }

    #[test]
    fn foil_coulomb_force_repels_like_charges() {
        let mut sim = Simulation::new();
        // Disable LJ by setting epsilon to zero
        sim.config.lj_force_epsilon = 0.0;
        // Two bodies with positive charge
        let foil1 = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 1.0, Species::FoilMetal);
        let foil2 = Body::new(Vec2::new(1.0, 0.0), Vec2::zero(), 1.0, 1.0, 1.0, Species::FoilMetal);
        sim.bodies.push(foil1);
        sim.bodies.push(foil2);
        sim.quadtree.build(&mut sim.bodies);
        let initial_dist = (sim.bodies[0].pos - sim.bodies[1].pos).mag();
        for _ in 0..3 {
            sim.step();
        }
        let new_dist = (sim.bodies[0].pos - sim.bodies[1].pos).mag();
        assert!(new_dist > initial_dist, "Like charges should repel via Coulomb force");
    }

    #[test]
    fn foil_lj_force_attracts_at_long_range_repels_at_short_range() {
        let mut sim = Simulation::new();
        // Use LJ parameters from config.rs so test adapts to config changes
        let sigma = sim.config.lj_force_sigma;
        let cutoff = sim.config.lj_force_cutoff * sigma;
        // Attract at long range (non-overlapping, but within cutoff)
        let long_range = cutoff * 0.92; // safely within cutoff, but > sigma
        let foil1 = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        let foil2 = Body::new(Vec2::new(long_range, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        sim.bodies.push(foil1);
        sim.bodies.push(foil2);
        sim.quadtree.build(&mut sim.bodies);
        let initial_dist = (sim.bodies[0].pos - sim.bodies[1].pos).mag();
        println!("Initial positions: {:?} {:?}", sim.bodies[0].pos, sim.bodies[1].pos);
        for _ in 0..200 {
            sim.step();
        }
        let new_dist = (sim.bodies[0].pos - sim.bodies[1].pos).mag();
        println!("Final positions: {:?} {:?}", sim.bodies[0].pos, sim.bodies[1].pos);
        println!("Initial dist: {initial_dist}, New dist: {new_dist}");
        assert!(new_dist < initial_dist, "LJ force should attract at long range");
        // Repel at short range (overlapping, r < sigma)
        let mut sim = Simulation::new();
        let sigma = sim.config.lj_force_sigma;
        let short_range = sigma * 0.75; // well within repulsive regime
        let foil1 = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        let foil2 = Body::new(Vec2::new(short_range, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        sim.bodies.push(foil1);
        sim.bodies.push(foil2);
        sim.quadtree.build(&mut sim.bodies);
        let initial_dist = (sim.bodies[0].pos - sim.bodies[1].pos).mag();
        println!("Initial positions: {:?} {:?}", sim.bodies[0].pos, sim.bodies[1].pos);
        for _ in 0..200 {
            sim.step();
        }
        let new_dist = (sim.bodies[0].pos - sim.bodies[1].pos).mag();
        println!("Final positions: {:?} {:?}", sim.bodies[0].pos, sim.bodies[1].pos);
        println!("Initial dist: {initial_dist}, New dist: {new_dist}");
        assert!(new_dist > initial_dist, "LJ force should repel at short range");
    }

    #[test]
    fn foil_combined_lj_and_coulomb_force() {
        let mut sim = Simulation::new();
        // Use default LJ settings from config.rs (opposite charges)
        let foil1 = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 1.0, Species::FoilMetal);
        let foil2 = Body::new(Vec2::new(1.8, 0.0), Vec2::zero(), 1.0, 1.0, -1.0, Species::FoilMetal);
        sim.bodies.push(foil1);
        sim.bodies.push(foil2);
        sim.quadtree.build(&mut sim.bodies);
        let initial_dist = (sim.bodies[0].pos - sim.bodies[1].pos).mag();
        println!("Initial positions: {:?} {:?}", sim.bodies[0].pos, sim.bodies[1].pos);
        for _ in 0..100 {
            sim.step();
        }
        let new_dist = (sim.bodies[0].pos - sim.bodies[1].pos).mag();
        println!("Final positions: {:?} {:?}", sim.bodies[0].pos, sim.bodies[1].pos);
        println!("Initial dist: {initial_dist}, New dist: {new_dist}");
        assert!(new_dist < initial_dist, "Combined LJ and Coulomb (opposite charge) should attract");
    }
}
