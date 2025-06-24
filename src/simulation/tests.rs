// Redox-related tests for the Simulation and Body

use super::simulation::Simulation;
use crate::body::{Body, Electron, Species};
use crate::quadtree::Quadtree;
use crate::config;
use crate::config::SimConfig;
use ultraviolet::Vec2;
use smallvec::{SmallVec, smallvec};

#[cfg(test)]
mod reactions {
    use crate::cell_list::CellList;
    use std::collections::HashMap;
    use super::*;

    #[test]
    fn ion_reduces_to_metal_on_electron_arrival() {
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
            cell_list: CellList::new(10.0, 1.0),
            body_to_foil: HashMap::new(),
        };
        sim.quadtree.build(&mut sim.bodies);
        let bodies_snapshot = sim.bodies.clone();
        let mut buf = Vec::new();
        let b = &mut sim.bodies[0];
        b.apply_redox(&bodies_snapshot, &sim.quadtree, &mut buf);
        assert_eq!(b.species, Species::LithiumMetal, "Ion with electron should become metal");
        assert_eq!(b.electrons.len(), 1, "Should have one valence electron");
        assert_eq!(b.charge, 0.0, "Neutral metal should have charge 0");
    }

    #[test]
    fn metal_oxidizes_to_ion_when_bare() {
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
            cell_list: CellList::new(10.0, 1.0),
            body_to_foil: HashMap::new(),
        };
        sim.quadtree.build(&mut sim.bodies);
        let bodies_snapshot = sim.bodies.clone();
        let mut buf = Vec::new();
        let b = &mut sim.bodies[0];
        b.apply_redox(&bodies_snapshot, &sim.quadtree, &mut buf);
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
        let mut bodies = vec![ion];
        let mut qt = Quadtree::new(
            config::QUADTREE_THETA,
            config::QUADTREE_EPSILON,
            config::QUADTREE_LEAF_CAPACITY,
            config::QUADTREE_THREAD_CAPACITY,
        );
        qt.build(&mut bodies);
        let bodies_snapshot = bodies.clone();
        let mut buf = Vec::new();
        bodies[0].apply_redox(&bodies_snapshot, &qt, &mut buf);
        assert_eq!(bodies[0].species, Species::LithiumMetal);
        assert_eq!(bodies[0].electrons.len(), 2);
    }

    #[test]
    fn repeated_redox_transitions_cycle_species() {
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
        let mut bodies = vec![body];
        let mut qt = Quadtree::new(
            config::QUADTREE_THETA,
            config::QUADTREE_EPSILON,
            config::QUADTREE_LEAF_CAPACITY,
            config::QUADTREE_THREAD_CAPACITY,
        );
        qt.build(&mut bodies);
        let bodies_snapshot = bodies.clone();
        let mut buf = Vec::new();
        bodies[0].apply_redox(&bodies_snapshot, &qt, &mut buf);
        assert_eq!(bodies[0].species, Species::LithiumMetal);
        bodies[0].electrons.clear();
        bodies[0].update_charge_from_electrons();
        qt.build(&mut bodies);
        let bodies_snapshot = bodies.clone();
        let mut buf2 = Vec::new();
        bodies[0].apply_redox(&bodies_snapshot, &qt, &mut buf2);
        assert_eq!(bodies[0].species, Species::LithiumIon);
    }

    #[test]
    fn electron_hop_between_metals_conserves_electrons_and_charge() {
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
            cell_list: CellList::new(10.0, 1.0),
            body_to_foil: HashMap::new(),
        };
        sim.quadtree.build(&mut sim.bodies);

        let exclude = vec![false; sim.bodies.len()];
        sim.perform_electron_hopping_with_exclusions(&exclude);
        let a = &sim.bodies[0];
        let b = &sim.bodies[1];
        assert_eq!(a.electrons.len() + b.electrons.len(), total_electrons);
        assert!((a.charge + b.charge - total_charge).abs() < 1e-6);
    }

    #[test]
    fn electrons_conserved_after_multiple_hops_and_redox() {
        let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        let b = Body::new(Vec2::new(1.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        let ion = Body::new(Vec2::new(2.0, 0.0), Vec2::zero(), 1.0, 1.0, 1.0, Species::LithiumIon);
        for _ in 0..crate::config::FOIL_NEUTRAL_ELECTRONS { a.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }); }
        a.update_charge_from_electrons();
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
            cell_list: CellList::new(10.0, 1.0),
            body_to_foil: HashMap::new(),
        };
        sim.quadtree.build(&mut sim.bodies);

        let exclude = vec![false; sim.bodies.len()];
        sim.perform_electron_hopping_with_exclusions(&exclude);
        sim.perform_electron_hopping_with_exclusions(&exclude);
        sim.quadtree.build(&mut sim.bodies);
        let bodies_ptr = &sim.bodies as *const Vec<Body>;
        let qt_ptr = &sim.quadtree as *const Quadtree;
        for b in &mut sim.bodies {
            let bodies = unsafe { &*bodies_ptr };
            let qt = unsafe { &*qt_ptr };
            b.apply_redox(bodies, qt);
        }
        let sum_electrons = sim.bodies.iter().map(|b| b.electrons.len()).sum::<usize>();
        assert_eq!(sum_electrons, total_electrons);
    }

    #[test]
    fn metal_with_many_neighbors_does_not_ionize() {
        let mut center = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        center.electrons.clear();
        center.update_charge_from_electrons();
        let mut bodies = vec![center];
        // Surround with exactly threshold number of neighbors
        for i in 0..crate::config::IONIZATION_NEIGHBOR_THRESHOLD {
            let angle = i as f32 * std::f32::consts::TAU / crate::config::IONIZATION_NEIGHBOR_THRESHOLD as f32;
            let mut nb = Body::new(
                Vec2::new(angle.cos(), angle.sin()),
                Vec2::zero(),
                1.0,
                1.0,
                0.0,
                Species::LithiumMetal,
            );
            nb.electrons = smallvec![Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::LITHIUM_METAL_NEUTRAL_ELECTRONS];
            nb.update_charge_from_electrons();
            bodies.push(nb);
        }
        let mut qt = Quadtree::new(
            config::QUADTREE_THETA,
            config::QUADTREE_EPSILON,
            config::QUADTREE_LEAF_CAPACITY,
            config::QUADTREE_THREAD_CAPACITY,
        );
        qt.build(&mut bodies);
        let bodies_snapshot = bodies.clone();
        let mut buf = Vec::new();
        bodies[0].apply_redox(&bodies_snapshot, &qt, &mut buf);
        assert_eq!(bodies[0].species, Species::LithiumMetal);
    }

    #[test]
    fn ion_with_neighbors_does_not_reduce() {
        let mut ion = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumIon);
        ion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        ion.update_charge_from_electrons();
        let mut bodies = vec![ion];
        for i in 0..crate::config::IONIZATION_NEIGHBOR_THRESHOLD {
            let angle = i as f32 * std::f32::consts::TAU / crate::config::IONIZATION_NEIGHBOR_THRESHOLD as f32;
            let nb = Body::new(
                Vec2::new(angle.cos(), angle.sin()),
                Vec2::zero(),
                1.0,
                1.0,
                0.0,
                Species::LithiumMetal,
            );
            bodies.push(nb);
        }
        let mut qt = Quadtree::new(
            config::QUADTREE_THETA,
            config::QUADTREE_EPSILON,
            config::QUADTREE_LEAF_CAPACITY,
            config::QUADTREE_THREAD_CAPACITY,
        );
        qt.build(&mut bodies);
        let bodies_snapshot = bodies.clone();
        let mut buf = Vec::new();
        bodies[0].apply_redox(&bodies_snapshot, &qt, &mut buf);
        assert_eq!(bodies[0].species, Species::LithiumIon);
    }

    mod hopping_kinetics_tests {
        use super::*;
        use crate::body::{Body, Electron};
        use ultraviolet::Vec2;
        use std::collections::HashMap;

        #[test]
        fn always_hop_when_activation_nearly_0() {
            let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::LithiumMetal);
            let mut b = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0,  0.0, Species::LithiumMetal);
            a.electrons = smallvec! [
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
            ];
            b.electrons = SmallVec::new();

            println!("Testing near-zero activation energy");
            println!("Initial electrons in a: {}", a.electrons.len());
            println!("Initial electrons in b: {}", b.electrons.len());

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
                cell_list: CellList::new(10.0, 1.0),
                body_to_foil: HashMap::new(),
            };
            sim.quadtree.build(&mut sim.bodies);
            let exclude = vec![false; sim.bodies.len()];
            sim.perform_electron_hopping_with_exclusions(&exclude);

            println!("After hopping:");
            println!("Electrons in a: {}", sim.bodies[0].electrons.len());
            println!("Electrons in b: {}", sim.bodies[1].electrons.len());

            sim.bodies[0].update_charge_from_electrons();   
            sim.bodies[1].update_charge_from_electrons();   
            assert_eq!(sim.bodies[0].electrons.len(), 0);
            assert_eq!(sim.bodies[1].electrons.len(), 1);
        }

        #[test]
        fn never_hop_when_activation_infinite() {
            let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::LithiumMetal);
            let mut b = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0,  0.0, Species::LithiumMetal);
            a.electrons = smallvec! [
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
            ];
            b.electrons = SmallVec::new();

            println!("Testing infinite activation energy");
            println!("Initial electrons in a: {}", a.electrons.len());
            println!("Initial electrons in b: {}", b.electrons.len());

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
                cell_list: CellList::new(10.0, 1.0),
                body_to_foil: HashMap::new(),
            };
            sim.quadtree.build(&mut sim.bodies);

            let exclude = vec![false; sim.bodies.len()];
            sim.perform_electron_hopping_with_exclusions(&exclude);
            sim.bodies[0].update_charge_from_electrons();   
            sim.bodies[1].update_charge_from_electrons();   

            println!("After hopping:");
            println!("Electrons in a: {}", sim.bodies[0].electrons.len());
            println!("Electrons in b: {}", sim.bodies[1].electrons.len());

            // after hopping, a and b should have unchanged electrons
            assert_eq!(sim.bodies[0].electrons.len(), 2);
            assert_eq!(sim.bodies[1].electrons.len(), 0);
        }

        #[test]
        fn never_hop_when_rate_zero() {
            let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::LithiumMetal);
            let mut b = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0,  0.0, Species::LithiumMetal);
            a.electrons = smallvec! [
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
            ];
            b.electrons = SmallVec::new();
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
                cell_list: CellList::new(10.0, 1.0),
                body_to_foil: HashMap::new(),
            };
            sim.quadtree.build(&mut sim.bodies);

            let exclude = vec![false; sim.bodies.len()];
            sim.perform_electron_hopping_with_exclusions(&exclude);
            assert_eq!(sim.bodies[0].electrons.len(), 2);
            assert_eq!(sim.bodies[1].electrons.len(), 0);
        }

        #[test]
        fn always_hop_when_rate_very_high() {
            let mut a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::LithiumMetal);
            let mut b = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0,  0.0, Species::LithiumMetal);
            a.electrons = smallvec! [
                Electron{ rel_pos: Vec2::zero(), vel: Vec2::zero() },
            ];
            b.electrons = SmallVec::new();

            println!("Testing high hop rate");
            println!("Initial electrons in a: {}", a.electrons.len());
            println!("Initial electrons in b: {}", b.electrons.len());

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
                cell_list: CellList::new(10.0, 1.0),
                body_to_foil: HashMap::new(),
            };
            sim.quadtree.build(&mut sim.bodies);
            let exclude = vec![false; sim.bodies.len()];
            sim.perform_electron_hopping_with_exclusions(&exclude);

            println!("After hopping:");
            println!("Electrons in a: {}", sim.bodies[0].electrons.len());
            println!("Electrons in b: {}", sim.bodies[1].electrons.len());

            assert_eq!(sim.bodies[0].electrons.len(), 0);
            assert_eq!(sim.bodies[1].electrons.len(), 1);
        }

        #[test]
        fn butler_volmer_inter_species_hop() {
            let mut metal = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
            // One extra electron to donate
            metal.electrons = smallvec![
                Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() };
                crate::config::LITHIUM_METAL_NEUTRAL_ELECTRONS + 1
            ];
            metal.update_charge_from_electrons();

            let ion = Body::new(Vec2::new(1.0,0.0), Vec2::zero(), 1.0, 1.0, 1.0, Species::LithiumIon);

            let mut sim = Simulation {
                dt: 0.1,
                frame: 0,
                bodies: vec![metal, ion],
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
                    use_butler_volmer: true,
                    bv_exchange_current: 1e6_f32,
                    bv_overpotential_scale: 1e-8_f32,
                    ..Default::default()
                },
                foils: Vec::new(),
                cell_list: CellList::new(10.0, 1.0),
                body_to_foil: HashMap::new(),
            };
            sim.quadtree.build(&mut sim.bodies);
            let exclude = vec![false; sim.bodies.len()];
            sim.perform_electron_hopping_with_exclusions(&exclude);

            assert_eq!(sim.bodies[0].electrons.len(), crate::config::LITHIUM_METAL_NEUTRAL_ELECTRONS);
            assert_eq!(sim.bodies[1].electrons.len(), 1);
        }

        #[test]
        fn test_field_centered_and_symmetric_direct() {
            let body = Body {
                pos: Vec2::zero(),
                vel: Vec2::zero(),
                acc: Vec2::zero(),
                mass: 1.0,
                radius: 1.0,
                charge: 1.0,
                species: Species::LithiumIon,
                electrons: SmallVec::new(),
                id: 0,
                e_field: Vec2::zero(),
            };
            let bodies = vec![body];
            let config = crate::config::SimConfig::default();

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

                match i {
                    0 => {
                        assert!(field.x > 0.0, "Field x should be positive");
                        assert!(field.y.abs() < 1e-6, "Field y should be ~0");
                    }
                    1 => {
                        assert!(field.y > 0.0, "Field y should be positive");
                        assert!(field.x.abs() < 1e-6, "Field x should be ~0");
                    }
                    2 => {
                        assert!(field.x < 0.0, "Field x should be negative");
                        assert!(field.y.abs() < 1e-6, "Field y should be ~0");
                    }
                    3 => {
                        assert!(field.y < 0.0, "Field y should be negative");
                        assert!(field.x.abs() < 1e-6, "Field x should be ~0");
                    }
                    _ => {}
                }
                magnitudes.push(field.mag());
            }

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
                electrons: SmallVec::new(),
                id: 0,
                e_field: Vec2::zero(),
            };
            
            let mut bodies = vec![body];

            let mut quadtree = Quadtree::new(
                config::QUADTREE_THETA,
                config::QUADTREE_EPSILON,
                config::QUADTREE_LEAF_CAPACITY,
                config::QUADTREE_THREAD_CAPACITY,
            );
            quadtree.build(&mut bodies);

            fn field_at(quadtree: &Quadtree, bodies: &[Body], pos: Vec2, k_e: f32) -> Vec2 {
                quadtree.field_at_point(bodies, pos, k_e)
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