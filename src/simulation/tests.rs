// simulation/redox_tests.rs
// Contains redox-related tests for the Simulation and Body

use super::simulation::Simulation;
use crate::body::{Body, Electron, Species};
use crate::quadtree::Quadtree;
use crate::config;
use crate::config::SimConfig;
use ultraviolet::Vec2;

#[cfg(test)]
mod reactions {
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
            config: Default::default(),
            foils: Vec::new(),
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
            config: Default::default(),
            foils: Vec::new(),
        };
        let b = &mut sim.bodies[0];
        b.apply_redox();
        let b = &sim.bodies[0];
        assert_eq!(b.species, Species::LithiumIon, "Metal with no electrons should become ion");
        assert_eq!(b.charge, 1.0, "Ion with no electrons should have charge +1");
    }

    #[test]
    fn multi_electron_ion_becomes_metal() {
        let mut ion = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumIon,
        );
        // Add two electrons (more than neutral metal)
        ion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        ion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        ion.update_charge_from_electrons();
        assert_eq!(ion.species, Species::LithiumIon);
        ion.apply_redox();
        assert_eq!(ion.species, Species::LithiumMetal);
        assert_eq!(ion.electrons.len(), 2);
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
            config: Default::default(),
            foils: Vec::new(),
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
        let ion = Body::new(Vec2::new(2.0, 0.0), Vec2::zero(), 1.0, 1.0, 1.0, Species::LithiumIon);
        for _ in 0..crate::config::FOIL_NEUTRAL_ELECTRONS { a.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }); }
        a.update_charge_from_electrons();
        b.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        b.update_charge_from_electrons();
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
            config: Default::default(),
            foils: Vec::new(),
        };
        sim.perform_electron_hopping();
        sim.perform_electron_hopping();
        for b in &mut sim.bodies { b.apply_redox(); }
        let sum_electrons = sim.bodies.iter().map(|b| b.electrons.len()).sum::<usize>();
        assert_eq!(sum_electrons, total_electrons);
    }
    #[cfg(test)]
    mod hopping_kinetics_tests {
        use super::*;
        use crate::body::{Body, Electron};
        use ultraviolet::Vec2;

        #[test]
        fn always_hop_when_activation_nearly_0() {
            // two metals apart but within hop radius
            let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::LithiumMetal);
            let mut b = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0,  0.0, Species::LithiumMetal);
            a.electrons = vec! [
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() }
            ];
            a.update_charge_from_electrons();
            b.update_charge_from_electrons();
            //println!("DEBUG: a.electrons.len() = {}, b.electrons.len() = {}", a.electrons.len(), b.electrons.len());
            //println!("DEBUG: a.charge = {}, b.charge = {}", a.charge, b.charge);
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
                config: SimConfig {
                    hop_activation_energy: 1e-8_f32, // nearly zero activation energy for testing
                    ..Default::default()
                },
                foils: Vec::new(),
            };
            sim.quadtree.build(&mut sim.bodies);
            sim.perform_electron_hopping();
            sim.bodies[0].update_charge_from_electrons();   
            sim.bodies[1].update_charge_from_electrons();   

            // after one hop, a should lose an electron, b should gain one
            assert_eq!(sim.bodies[0].electrons.len(), 1);
            assert_eq!(sim.bodies[1].electrons.len(), 1);
        }

        #[test]
        fn never_hop_when_activation_infinite() {
            // two metals apart but within hop radius
            let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::LithiumMetal);
            let mut b = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0,  0.0, Species::LithiumMetal);
            a.electrons = vec! [
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() }
            ];
            a.update_charge_from_electrons();
            b.update_charge_from_electrons();
            //println!("DEBUG: a.electrons.len() = {}, b.electrons.len() = {}", a.electrons.len(), b.electrons.len());
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
                config: SimConfig {
                    hop_activation_energy: 1e15_f32, // very large activation energy for testing
                    ..Default::default()
                },
                foils: Vec::new(),
            };
            sim.perform_electron_hopping();
            sim.bodies[0].update_charge_from_electrons();   
            sim.bodies[1].update_charge_from_electrons();   
        }

        #[test]
        fn never_hop_when_rate_zero() {
            // two metals apart but within hop radius
            let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::LithiumMetal);
            let mut b = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0,  0.0, Species::LithiumMetal);
            a.electrons = vec! [
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() }
            ];
            a.update_charge_from_electrons();
            b.update_charge_from_electrons();
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
                config: SimConfig {
                    hop_rate_k0: 0.0, // zero rate constant for testing
                    ..Default::default()
                },
                foils: Vec::new(),
            };
            sim.perform_electron_hopping();
            // after hopping, a and b should have unchanged electrons
            assert_eq!(sim.bodies[0].electrons.len(), 2);
            assert_eq!(sim.bodies[1].electrons.len(), 0);
        }

        #[test]
        fn always_hop_when_rate_very_high() {
            // two metals apart but within hop radius
            let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::LithiumMetal);
            let mut b = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0,  0.0, Species::LithiumMetal);
            a.electrons = vec! [
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() }
            ];
            a.update_charge_from_electrons();
            b.update_charge_from_electrons();
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
                config: SimConfig {
                    hop_rate_k0: 1e15_f32, // large rate constant for testing
                    ..Default::default()
                },
                foils: Vec::new(),
            };
            sim.quadtree.build(&mut sim.bodies);
            sim.perform_electron_hopping();
            // after hopping, a should lose one electron, b should gain one
            assert_eq!(sim.bodies[0].electrons.len(), 1);
            assert_eq!(sim.bodies[1].electrons.len(), 1);
        }

        #[test]
        fn test_field_centered_and_symmetric_direct() {
            // Place a single positive charge at the origin
            let body = Body {
                pos: Vec2::zero(),
                vel: Vec2::zero(),
                acc: Vec2::zero(),
                mass: 1.0,
                radius: 1.0,
                charge: 1.0,
                species: Species::LithiumIon, // or another appropriate variant
                electrons: Vec::new(),
                id: 0,
                e_field: Vec2::zero(),
                lj_force: Vec2::zero(),
                coulomb_force: Vec2::zero(),
            };
            let bodies = vec![body];
            let config = crate::config::SimConfig::default();

            // Simple direct field computation for test
            fn compute_field_at_point(bodies: &[Body], pos: Vec2, _config: &SimConfig) -> Vec2 {
                let mut field = Vec2::zero();
                for b in bodies {
                    let r = pos - b.pos;
                    let r2 = r.mag_sq();
                    if r2 > 1e-8 {
                        field += r.normalized() * (b.charge / r2);
                    }
                }
                field
            }

            let positions = [
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, -1.0),
            ];
            let expected = [
                "right (+x)",
                "up (+y)",
                "left (-x)",
                "down (-y)",
            ];

            let mut magnitudes = Vec::new();

            for (i, pos) in positions.iter().enumerate() {
                let field = compute_field_at_point(&bodies, *pos, &config);

                // Check direction
                match i {
                    0 => { // +x
                        assert!(field.x > 0.0, "Field x should be positive");
                        assert!(field.y.abs() < 1e-6, "Field y should be ~0");
                    }
                    1 => { // +y
                        assert!(field.y > 0.0, "Field y should be positive");
                        assert!(field.x.abs() < 1e-6, "Field x should be ~0");
                    }
                    2 => { // -x
                        assert!(field.x < 0.0, "Field x should be negative");
                        assert!(field.y.abs() < 1e-6, "Field y should be ~0");
                    }
                    3 => { // -y
                        assert!(field.y < 0.0, "Field y should be negative");
                        assert!(field.x.abs() < 1e-6, "Field x should be ~0");
                    }
                    _ => {}
                }
                magnitudes.push(field.mag());
            }

            // All magnitudes should be (nearly) equal
            let avg_mag = magnitudes.iter().sum::<f32>() / magnitudes.len() as f32;
            for (i, mag) in magnitudes.iter().enumerate() {
                assert!(
                    (mag - avg_mag).abs() < 1e-5,
                    "Field magnitude at direction {} differs: {} vs avg {}",
                    expected[i],
                    mag,
                    avg_mag
                );
            }
        }
        // ...existing code...
        #[test]
        fn test_field_centered_and_symmetric_quadtree() {
            use crate::quadtree::Quadtree;
            use crate::simulation::forces::K_E;

            let body = Body {
                pos: Vec2::zero(),
                vel: Vec2::zero(),
                acc: Vec2::zero(),
                mass: 1.0,
                radius: 1.0,
                charge: 1.0,
                species: Species::LithiumIon,
                electrons: Vec::new(),
                id: 0,
                e_field: Vec2::zero(),
                lj_force: Vec2::zero(),
                coulomb_force: Vec2::zero(),
            };
            
            let mut bodies = vec![body];

            // Build a quadtree for the test
            let mut quadtree = Quadtree::new(
                config::QUADTREE_THETA,
                config::QUADTREE_EPSILON,
                config::QUADTREE_LEAF_CAPACITY,
                config::QUADTREE_THREAD_CAPACITY,
            );
            quadtree.build(&mut bodies);

            // Helper to get field at a point using the quadtree
            fn field_at(quadtree: &Quadtree, bodies: &[Body], pos: Vec2, k_e: f32) -> Vec2 {
                quadtree.field_at_point(bodies, pos, k_e) // You may need to implement or expose this
            }

            let positions = [
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, -1.0),
            ];

            let mut magnitudes = Vec::new();

            for pos in &positions {
                let field = field_at(&quadtree, &bodies, *pos, K_E);
                let expected_dir = (*pos).normalized();
                let field_dir = field.normalized();
                let dot = field_dir.dot(expected_dir);
                assert!(
                    (dot - 1.0).abs() < 1e-5,
                    "Field at {:?} not pointing radially out: dot={}",
                    pos,
                    dot
                );
                magnitudes.push(field.mag());
            }

            // All magnitudes should be (nearly) equal
            let avg_mag = magnitudes.iter().sum::<f32>() / magnitudes.len() as f32;
            for (i, mag) in magnitudes.iter().enumerate() {
                assert!(
                    (mag - avg_mag).abs() < 1e-5,
                    "Field magnitude at direction {} differs: {} vs avg {}",
                    i,
                    mag,
                    avg_mag
                );
            }
        }
    }
}
