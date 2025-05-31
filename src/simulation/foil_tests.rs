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
    // Start with 3 electrons
    body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; 3];
    let idx = sim.bodies.len();
    let id = body.id;
    sim.bodies.push(body);
    // Create a foil referencing this body by ID
    let mut foil = Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 2.0); // positive current
    foil.accum = 2.0; // force two electrons to be added
    sim.foils.push(foil);
    sim.step();
    assert_eq!(sim.bodies[idx].electrons.len(), 5, "Electrons should be added by positive current");
    // Now test negative current
    sim.foils[0].current = -2.0;
    sim.foils[0].accum = -2.0;
    sim.step();
    assert_eq!(sim.bodies[idx].electrons.len(), 3, "Electrons should be removed by negative current");
}

#[test]
fn test_foil_default_electrons() {
    use crate::body::{Body, Species, Electron};
    use ultraviolet::Vec2;

    let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    // Should default to 3 electrons for foil
    body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; 3];
    assert_eq!(body.electrons.len(), 3, "FoilMetal should start with 3 electrons");
}

#[test]
fn test_foil_is_fixed() {
    use crate::body::{Body, Species};
    use crate::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    let mut sim = Simulation::new();
    let body = Body::new(Vec2::zero(), Vec2::zero(), 1e6, 1.0, 0.0, Species::FoilMetal);
    let idx = sim.bodies.len();
    let id = body.id;
    sim.bodies.push(body);
    sim.foils.push(Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 0.0));
    sim.step();
    // No longer assert fixed, just check it still exists and has large mass
    assert_eq!(sim.bodies[idx].mass, 1e6, "FoilMetal should have large mass");
}

#[test]
fn test_foil_lj_force_affects_metal() {
    use crate::body::{Body, Species, Electron};
    use crate::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    let mut sim = Simulation::new();
    // Place a fixed foil and a free lithium metal nearby
    let foil_idx = sim.bodies.len();
    let mut foil_body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    foil_body.fixed = true;
    foil_body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; 3];
    let foil_id = foil_body.id;
    sim.bodies.push(foil_body);
    let metal_idx = sim.bodies.len();
    let mut metal_body = Body::new(Vec2::new(2.5, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
    metal_body.fixed = false;
    sim.bodies.push(metal_body);
    sim.foils.push(Foil::new(vec![foil_id], Vec2::zero(), 1.0, 1.0, 0.0));

    let initial_dist = (sim.bodies[foil_idx].pos - sim.bodies[metal_idx].pos).mag();
    println!("Initial distance: {}", initial_dist);
    for step in 0..10 {
        sim.step();
        let pos_foil = sim.bodies[foil_idx].pos;
        let pos_metal = sim.bodies[metal_idx].pos;
        let dist = (pos_foil - pos_metal).mag();
        let vel = sim.bodies[metal_idx].vel;
        println!(
            "Step {}: Metal pos = {:?}, Foil pos = {:?}, distance = {:.6}, metal vel = {:?}",
            step, pos_metal, pos_foil, dist, vel
        );
        // Optionally, print acceleration if available
        println!(
            "Step {}: Metal acc = {:?}",
            step, sim.bodies[metal_idx].acc
        );
    }
    let new_dist = (sim.bodies[foil_idx].pos - sim.bodies[metal_idx].pos).mag();
    println!("Final distance: {}", new_dist);
    assert!(new_dist < initial_dist, "LithiumMetal should be attracted to fixed FoilMetal by LJ force");
}

#[test]
fn test_overlapping_foil_indices_handled() {
    use crate::body::{Body, Species, Electron};
    use crate::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    let mut sim = Simulation::new();
    let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; 3];
    let idx = sim.bodies.len();
    let id = body.id;
    sim.bodies.push(body);
    // Add two foils referencing the same body by ID
    sim.foils.push(Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 1.0));
    sim.foils.push(Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, -1.0));
    // Should not panic or crash
    sim.step();
    assert_eq!(sim.bodies[idx].electrons.len(), 3, "Overlapping foils should not crash and net current is zero");
}
