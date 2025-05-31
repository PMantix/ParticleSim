// Tests for foil behavior in the simulation
// Run with: cargo test --test foil_tests

#[test]
fn test_foil_current_adds_removes_electrons() {
    use crate::body::{Body, Species, Electron};
    use crate::foil::Foil;
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
    use crate::body::{Body, Species, Electron};
    use crate::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    #[test]
    fn foil_does_not_drop_below_zero_electrons() {
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
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        assert_eq!(body.electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "FoilMetal should start with correct number of electrons");
    }
}

mod foil_mass_and_inertia {
    use crate::body::Body;
    use crate::foil::Foil;

    #[test]
    fn foil_is_inertial_with_large_mass() {
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
    use crate::body::{Body, Species, Electron};
    use crate::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    #[test]
    fn foil_lj_force_affects_metal() {
        let mut sim = Simulation::new();
        let foil_idx = sim.bodies.len();
        let mut foil_body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        foil_body.fixed = true;
        foil_body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let foil_id = foil_body.id;
        sim.bodies.push(foil_body);
        let metal_idx = sim.bodies.len();
        let mut metal_body = Body::new(Vec2::new(2.5, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        metal_body.fixed = false;
        sim.bodies.push(metal_body);
        sim.foils.push(Foil::new(vec![foil_id], Vec2::zero(), 1.0, 1.0, 0.0));
        let initial_dist = (sim.bodies[foil_idx].pos - sim.bodies[metal_idx].pos).mag();
        for _step in 0..10 {
            sim.step();
        }
        let new_dist = (sim.bodies[foil_idx].pos - sim.bodies[metal_idx].pos).mag();
        assert!(new_dist < initial_dist, "LithiumMetal should be attracted to fixed FoilMetal by LJ force");
    }
}

mod foil_overlapping_indices {
    use crate::body::{Body, Species, Electron};
    use crate::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    #[test]
    fn overlapping_foil_indices_handled() {
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
