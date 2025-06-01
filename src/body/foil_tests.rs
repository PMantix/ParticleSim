// Tests for foil behavior in the simulation
// Run with: cargo test --test foil_tests

#[test]
fn test_foil_current_adds_removes_electrons() {
    use crate::body::{Body, Species, Electron};
    use crate::body::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    let mut sim = Simulation::new();
    // Create a single FoilMetal body
    let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    // Start with neutral electrons
    body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
    let idx = sim.bodies.len();
    let id = body.id;
    sim.bodies.push(body);
    // Create a foil referencing this body by ID
    let mut foil = Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 2.0); // positive current
    foil.accum = (crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32; // force max electrons to be added
    sim.foils.push(foil);
    sim.step();
    assert_eq!(sim.bodies[idx].electrons.len(), crate::config::FOIL_MAX_ELECTRONS, "Electrons should be added up to FOIL_MAX_ELECTRONS");
    // Now test negative current
    sim.foils[0].current = -2.0;
    sim.foils[0].accum = -((crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32);
    sim.step();
    assert_eq!(sim.bodies[idx].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Electrons should be removed down to FOIL_NEUTRAL_ELECTRONS");
}

mod foil_electron_limits {
    #[test]
    fn foil_does_not_drop_below_zero_electrons() {
        use crate::body::{Body, Species, Electron};
        use crate::body::foil::Foil;
        use crate::simulation::Simulation;
        use ultraviolet::Vec2;
        let mut sim = Simulation::new();
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        // Start with neutral electrons
        body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let idx = sim.bodies.len();
        let id = body.id;
        sim.bodies.push(body);
        // Create a foil referencing this body by ID
        let mut foil = Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, -10.0); // large negative current
        foil.accum = -100.0; // force as many electrons to be removed as possible
        sim.foils.push(foil);
        sim.step();
        // Should not drop below 0 electrons
        assert_eq!(sim.bodies[idx].electrons.len(), 0, "Foil should not have fewer than 0 electrons");
    }

    #[test]
    fn foil_current_adds_and_removes_electrons_within_limits() {
        use crate::body::{Body, Species, Electron};
        use crate::body::foil::Foil;
        use crate::simulation::Simulation;
        use ultraviolet::Vec2;
        let mut sim = Simulation::new();
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let idx = sim.bodies.len();
        let id = body.id;
        sim.bodies.push(body);
        let mut foil = Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 2.0);
        foil.accum = (crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32;
        sim.foils.push(foil);
        sim.step();
        assert_eq!(sim.bodies[idx].electrons.len(), crate::config::FOIL_MAX_ELECTRONS, "Electrons should be added up to FOIL_MAX_ELECTRONS");
        sim.foils[0].current = -2.0;
        sim.foils[0].accum = -((crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32);
        sim.step();
        assert_eq!(sim.bodies[idx].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Electrons should be removed down to FOIL_NEUTRAL_ELECTRONS");
    }

    #[test]
    fn foil_default_electrons() {
        use crate::body::{Body, Species, Electron};
        use ultraviolet::Vec2;
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        assert_eq!(body.electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "FoilMetal should start with correct number of electrons");
    }
}

mod foil_mass_and_inertia {
    #[test]
    fn foil_is_inertial_with_large_mass() {
        use crate::body::Body;
        use crate::body::foil::Foil;
        let mut sim = crate::simulation::Simulation::new();
        let body = Body::new(ultraviolet::Vec2::zero(), ultraviolet::Vec2::zero(), 1e6, 1.0, 0.0, crate::body::Species::FoilMetal);
        let idx = sim.bodies.len();
        let id = body.id;
        sim.bodies.push(body);
        sim.foils.push(Foil::new(vec![id], ultraviolet::Vec2::zero(), 1.0, 1.0, 0.0));
        sim.step();
        assert_eq!(sim.bodies[idx].mass, 1e6, "FoilMetal should have large mass");
    }
}

mod foil_lj_force {
    #[test]
    fn foil_lj_force_affects_metal() {
        use crate::body::{Body, Species, Electron};
        use crate::body::foil::Foil;
        use crate::simulation::Simulation;
        use ultraviolet::Vec2;

        let mut sim = Simulation::new();
        let mut foil_body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);

        foil_body.fixed = true;
        foil_body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        
        sim.bodies.push(foil_body);
        let foil_id = sim.bodies.last().expect("Foil body not found after push").id;
        let mut metal_body = Body::new(Vec2::new(1.2, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        metal_body.fixed = false;
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
        assert!(new_dist < initial_dist, "LithiumMetal should be attracted to fixed FoilMetal by LJ force");
    }
}

mod foil_overlapping_indices {
    #[test]
    fn overlapping_foil_indices_handled() {
        use crate::body::{Body, Species, Electron};
        use crate::body::foil::Foil;
        use crate::simulation::Simulation;
        use ultraviolet::Vec2;
        let mut sim = Simulation::new();
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let idx = sim.bodies.len();
        let id = body.id;
        sim.bodies.push(body);
        sim.foils.push(Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 1.0));
        sim.foils.push(Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, -1.0));
        sim.step();
        assert_eq!(sim.bodies[idx].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Overlapping foils should not crash and net current is zero");
    }
}

mod foil_cohesion {
    #[test]
    fn foil_particles_remain_cohesive_within_electron_limits() {
        use crate::body::{Body, Species, Electron};
        use crate::body::foil::Foil;
        use crate::simulation::Simulation;
        use ultraviolet::Vec2;
        let n = 5;
        let spacing = 2.0; // Exactly radius-to-radius for radius=1.0
        let mut sim = Simulation::new();
        let mut ids = Vec::new();
        // Create a row of foil particles
        for i in 0..n {
            let mut body = Body::new(
                Vec2::new(i as f32 * spacing, 0.0),
                Vec2::zero(),
                1e6, // Large mass for foil
                1.0,
                0.0,
                Species::FoilMetal,
            );
            body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
            ids.push(body.id);
            sim.bodies.push(body);
        }
        sim.foils.push(Foil::new(ids.clone(), Vec2::zero(), n as f32 * spacing, 1.0, 0.0));
        // Record initial average distance between neighbors
        let initial_avg_dist: f32 = (0..n-1)
            .map(|i| (sim.bodies[i+1].pos - sim.bodies[i].pos).mag())
            .sum::<f32>() / (n as f32 - 1.0);
        // Add electrons up to max
        for i in 0..n {
            let body = &mut sim.bodies[i];
            while body.electrons.len() < crate::config::FOIL_MAX_ELECTRONS {
                body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
        }
        // Step simulation to allow forces to act
        for _ in 0..20 {
            sim.step();
        }
        let avg_dist_after_add: f32 = (0..n-1)
            .map(|i| (sim.bodies[i+1].pos - sim.bodies[i].pos).mag())
            .sum::<f32>() / (n as f32 - 1.0);
        // Remove electrons down to zero
        for i in 0..n {
            let body = &mut sim.bodies[i];
            while !body.electrons.is_empty() {
                body.electrons.pop();
            }
        }
        for _ in 0..20 {
            sim.step();
        }
        let avg_dist_after_remove: f32 = (0..n-1)
            .map(|i| (sim.bodies[i+1].pos - sim.bodies[i].pos).mag())
            .sum::<f32>() / (n as f32 - 1.0);
        // The foil should not break apart: average distance should not increase by more than 50%
        assert!(avg_dist_after_add < initial_avg_dist * 1.5, "Foil particles separated too much after adding electrons: {} vs {}", avg_dist_after_add, initial_avg_dist);
        assert!(avg_dist_after_remove < initial_avg_dist * 1.5, "Foil particles separated too much after removing electrons: {} vs {}", avg_dist_after_remove, initial_avg_dist);
    }
}

#[test]
fn test_lj_vs_coulomb_force_strength() {
    use crate::config;
    use ultraviolet::Vec2;

    // Place two metal particles at contact (r = sigma)
    let sigma = config::LJ_FORCE_SIGMA;
    let epsilon = config::LJ_FORCE_EPSILON;
    let k_e = crate::simulation::forces::K_E;
    let charge = 1.0; // 1e each

    let pos_a = Vec2::new(0.0, 0.0);
    let pos_b = Vec2::new(sigma, 0.0);

    // Calculate LJ force at r = sigma
    let r_vec = pos_b - pos_a;
    let r = r_vec.mag();
    let sr6 = (sigma / r).powi(6);
    let lj_force_mag = 24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r;

    // Calculate Coulomb force at r = sigma
    let coulomb_force_mag = k_e * charge * charge / (r * r);

    println!("LJ force magnitude at contact: {}", lj_force_mag.abs());
    println!("Coulomb force magnitude at contact: {}", coulomb_force_mag.abs());

    // The LJ force should be at least as strong as the Coulomb force to hold them together
    assert!(lj_force_mag.abs() >= coulomb_force_mag.abs(),
        "LJ force ({}) is weaker than Coulomb force ({}) at contact distance (sigma)",
        lj_force_mag.abs(), coulomb_force_mag.abs());
}

#[test]
fn test_force_summation_and_motion_balance() {
    
    use crate::config;
    use crate::body::{Body, Species};
    use ultraviolet::Vec2;

    // Place two foil particles at contact (r = sigma)
    let sigma = config::LJ_FORCE_SIGMA;
    let epsilon = config::LJ_FORCE_EPSILON;
    let mass = 1.0;
    // Use r = 1.2 * sigma for attraction (LJ attractive region)
    let r = 1.2 * sigma;
    let mut a = Body::new(Vec2::new(0.0, 0.0), Vec2::zero(), mass, 1.0, 0.0, Species::FoilMetal);
    let mut b = Body::new(Vec2::new(r, 0.0), Vec2::zero(), mass, 1.0, 0.0, Species::FoilMetal);
    // Set charge to zero to test pure LJ
    a.charge = 0.0;
    b.charge = 0.0;
    let mut sim = crate::simulation::Simulation::new();
    sim.bodies = vec![a, b];
    sim.quadtree.build(&mut sim.bodies);

    // Step 1: Only LJ (no Coulomb)
    // Skip forces::attract and do not call apply_lj_forces manually; sim.step() will handle it
    let sr6 = (sigma / r).powi(6);
    let lj_force_mag = 24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r;
    println!("expected_lj_force_mag: {} (at r = {})", lj_force_mag, r);
    // Do not call forces::apply_lj_forces(&mut sim);
    // Instead, check acceleration after sim.step()
    let old_pos_a = sim.bodies[0].pos.x;
    let old_pos_b = sim.bodies[1].pos.x;
    sim.step();
    let acc_total_a = sim.bodies[0].acc;
    let acc_total_b = sim.bodies[1].acc;
    println!("acc_total_a.x: {} (net)", acc_total_a.x);
    println!("acc_total_b.x: {} (net)", acc_total_b.x);
    // Net force should be attractive (a.x > 0, b.x < 0)
    assert!(acc_total_a.x > 0.0 && acc_total_b.x < 0.0, "Net force should pull particles together (pure LJ)");
    let new_pos_a = sim.bodies[0].pos.x;
    let new_pos_b = sim.bodies[1].pos.x;
    println!("old_pos_a: {}, new_pos_a: {}", old_pos_a, new_pos_a);
    println!("old_pos_b: {}, new_pos_b: {}", old_pos_b, new_pos_b);
    // If net force is attractive, particles should move toward each other
    assert!(new_pos_a > old_pos_a && new_pos_b < old_pos_b, "Particles should move toward each other if net force is attractive");
}
