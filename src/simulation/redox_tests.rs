// simulation/redox_tests.rs
// Contains redox-related tests for the Simulation and Body

use super::simulation::Simulation;
use crate::body::{Body, Electron, Species};
use crate::quadtree::Quadtree;
use crate::config;
use ultraviolet::Vec2;

#[cfg(test)]
mod redox_tests {
    use super::*;

    #[test]
    fn ion_reduces_to_metal_on_electron_arrival() {
        // ...existing code...
        let mut ion = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumIon,
        );
        ion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        ion.update_charge_from_electrons();
        let mut sim = Simulation {
            dt: 0.1,
            frame: 0,
            bodies: vec![ion],
            quadtree: Quadtree::new(
                config::QUADTREE_THETA,
                config::QUADTREE_EPSILON,
                config::QUADTREE_LEAF_CAPACITY,
                config::QUADTREE_THREAD_CAPACITY,
            ),
            bounds: 1.0,
            rewound_flags: vec![false],
            background_e_field: Vec2::zero(),
        };
        let b = &mut sim.bodies[0];
        b.apply_redox();
        assert_eq!(b.species, Species::LithiumMetal, "Ion with electron should become metal");
        assert_eq!(b.electrons.len(), 1, "Should have one valence electron");
        assert_eq!(b.charge, 0.0, "Neutral metal should have charge 0");
    }

    #[test]
    fn metal_oxidizes_to_ion_when_bare() {
        // ...existing code...
        let metal = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumMetal,
        );
        let mut sim = Simulation {
            dt: 0.1,
            frame: 0,
            bodies: vec![metal],
            quadtree: Quadtree::new(
                config::QUADTREE_THETA,
                config::QUADTREE_EPSILON,
                config::QUADTREE_LEAF_CAPACITY,
                config::QUADTREE_THREAD_CAPACITY,
            ),
            bounds: 1.0,
            rewound_flags: vec![false],
            background_e_field: Vec2::zero(),
        };
        let b = &mut sim.bodies[0];
        b.apply_redox();
        let b = &sim.bodies[0];
        assert_eq!(b.species, Species::LithiumIon, "Metal with no electrons should become ion");
        assert_eq!(b.charge, 1.0, "Ion with no electrons should have charge +1");
    }

    #[test]
    fn multi_electron_ion_remains_ion_and_charge_decreases() {
        // ...existing code...
        let mut ion = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumIon,
        );
        ion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        ion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        ion.update_charge_from_electrons();
        assert_eq!(ion.species, Species::LithiumIon);
        assert_eq!(ion.charge, -1.0);
        ion.apply_redox();
        assert_eq!(ion.species, Species::LithiumMetal);
        assert_eq!(ion.electrons.len(), 2);
        assert_eq!(ion.charge, -1.0);
    }

    #[test]
    fn repeated_redox_transitions_cycle_species() {
        // ...existing code...
        let mut body = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumIon,
        );
        body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        body.update_charge_from_electrons();
        body.apply_redox();
        assert_eq!(body.species, Species::LithiumMetal);
        body.electrons.clear();
        body.update_charge_from_electrons();
        body.apply_redox();
        assert_eq!(body.species, Species::LithiumIon);
    }

    #[test]
    fn electron_hop_between_metals_conserves_electrons_and_charge() {
        // ...existing code...
        let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        let mut b = Body::new(Vec2::new(1.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        a.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        a.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        a.update_charge_from_electrons();
        b.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        b.update_charge_from_electrons();
        let total_electrons = a.electrons.len() + b.electrons.len();
        let total_charge = a.charge + b.charge;
        let mut sim = Simulation {
            dt: 0.1,
            frame: 0,
            bodies: vec![a, b],
            quadtree: Quadtree::new(
                config::QUADTREE_THETA,
                config::QUADTREE_EPSILON,
                config::QUADTREE_LEAF_CAPACITY,
                config::QUADTREE_THREAD_CAPACITY,
            ),
            bounds: 10.0,
            rewound_flags: vec![false; 2],
            background_e_field: Vec2::zero(),
        };
        sim.perform_electron_hopping();
        let a = &sim.bodies[0];
        let b = &sim.bodies[1];
        assert_eq!(a.electrons.len() + b.electrons.len(), total_electrons);
        assert!((a.charge + b.charge - total_charge).abs() < 1e-6);
    }

    #[test]
    fn electrons_conserved_after_multiple_hops_and_redox() {
        // ...existing code...
        let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        let mut b = Body::new(Vec2::new(1.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        let mut ion = Body::new(Vec2::new(2.0, 0.0), Vec2::zero(), 1.0, 1.0, 1.0, Species::LithiumIon);
        for _ in 0..3 { a.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }); }
        a.update_charge_from_electrons();
        b.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        b.update_charge_from_electrons();
        ion.electrons.clear();
        ion.update_charge_from_electrons();
        let total_electrons = a.electrons.len() + b.electrons.len() + ion.electrons.len();
        let mut sim = Simulation {
            dt: 0.1,
            frame: 0,
            bodies: vec![a, b, ion],
            quadtree: Quadtree::new(
                config::QUADTREE_THETA,
                config::QUADTREE_EPSILON,
                config::QUADTREE_LEAF_CAPACITY,
                config::QUADTREE_THREAD_CAPACITY,
            ),
            bounds: 10.0,
            rewound_flags: vec![false; 3],
            background_e_field: Vec2::zero(),
        };
        sim.perform_electron_hopping();
        sim.perform_electron_hopping();
        for b in &mut sim.bodies { b.apply_redox(); }
        let sum_electrons = sim.bodies.iter().map(|b| b.electrons.len()).sum::<usize>();
        //println!("DEBUG: electrons: {:?}", sim.bodies.iter().map(|b| b.electrons.len()).collect::<Vec<_>>());
        //println!("DEBUG: sum_electrons = {}, expected = {}", sum_electrons, total_electrons);
        assert_eq!(sum_electrons, total_electrons);
    }
}
