// body/tests.rs
// Contains all tests for Body, including electron and hopping tests

#[cfg(test)]
mod tests {
    //use super::*;
    use crate::body::{Body, Electron, Species};
    use ultraviolet::Vec2;
    use crate::quadtree::Quadtree;
    //use crate::cell_list::CellList;
    //use crate::config;

    #[test]
    fn test_body_charge_update() {
        let mut body = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            1.0,
            Species::LithiumIon,
        );
        assert_eq!(body.charge, 1.0);
        body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        body.update_charge_from_electrons();
        assert_eq!(body.charge, 0.0);
    }

    #[test]
    fn test_body_electron_add_remove() {
        let mut body = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumMetal,
        );
        assert_eq!(body.electrons.len(), 0);
        body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        assert_eq!(body.electrons.len(), 1);
        body.electrons.pop();
        assert_eq!(body.electrons.len(), 0);
    }

    #[test]
    fn test_species_switch_on_redox() {
        let mut body = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            1.0,
            Species::LithiumIon,
        );
        body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        body.update_charge_from_electrons();
        let mut bodies = vec![body.clone()];
        let mut qt = Quadtree::new(
            crate::config::QUADTREE_THETA,
            crate::config::QUADTREE_EPSILON,
            crate::config::QUADTREE_LEAF_CAPACITY,
            crate::config::QUADTREE_THREAD_CAPACITY,
        );
        qt.build(&mut bodies);
        //let bodies_snapshot = bodies.clone();
        bodies[0].apply_redox();
        assert_eq!(bodies[0].species, Species::LithiumMetal);
        bodies[0].electrons.clear();
        bodies[0].update_charge_from_electrons();
        qt.build(&mut bodies);
        //let bodies_snapshot = bodies.clone();
        bodies[0].apply_redox();
        assert_eq!(bodies[0].species, Species::LithiumIon);
    }

    #[test]
    fn ec_and_dmc_remain_neutral() {
        for species in [Species::EC, Species::DMC] {
            let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, species);
            body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            body.update_charge_from_electrons();
            assert_eq!(body.charge, 0.0);
            let old_species = body.species;
            body.apply_redox();
            assert_eq!(body.species, old_species);
        }
    }

    mod physics {
        use std::collections::HashMap;

        use crate::body::{Body, Species, Electron};
        use crate::simulation::Simulation;
        use crate::quadtree::Quadtree;
        use ultraviolet::Vec2;
        use crate::config;
        use crate::config::SimConfig;
        use smallvec::smallvec;
        use crate::cell_list::CellList;

        #[test]
        fn ion_becomes_metal_when_charge_high() {
            let mut b = Body::new(
                Vec2::zero(),
                Vec2::zero(),
                1.0,
                1.0,
                0.0,
                Species::LithiumIon,
            );
            b.update_species();
            assert_eq!(b.species, Species::LithiumMetal);
        }

        #[test]
        fn metal_becomes_ion_when_charge_low() {
            let mut b = Body::new(
                Vec2::zero(),
                Vec2::zero(),
                1.0,
                1.0,
                1.0,
                Species::LithiumMetal,
            );
            b.update_species();
            assert_eq!(b.species, Species::LithiumIon);
        }

        #[test]
        fn electron_moves_under_field() {
            println!("Starting electron movement test");
            let mut b = Body::new(
                Vec2::zero(),
                Vec2::zero(),
                1.0,1.0,
                0.0,
                Species::LithiumMetal,
            );
            println!("Created body: {:?}", b);
            b.electrons=smallvec![Electron {rel_pos:Vec2::zero(),vel:Vec2::zero()}];
            println!("Added electron: {:?}", b.electrons);
            let field = Vec2::new(1.0, 0.0);
            println!("Field set to: {:?}", field);
            let mut bodies = vec![b.clone()];
            let mut qt = Quadtree::new(0.5, 0.01, 1, 1);
            println!("Before qt.build");
            qt.build(&mut bodies);
            println!("After qt.build");
            let bodies_clone = bodies.clone();
            println!("Before update: rel_pos = {:?}, vel = {:?}", bodies[0].electrons[0].rel_pos, bodies[0].electrons[0].vel);
            bodies[0].update_electrons(&bodies_clone, &qt, field, 0.1);
            println!("After update: rel_pos = {:?}, vel = {:?}", bodies[0].electrons[0].rel_pos, bodies[0].electrons[0].vel);
            assert!(bodies[0].electrons[0].rel_pos.x < 0.0,
                "Expected electron to drift left (x < 0), but rel_pos.x = {}", bodies[0].electrons[0].rel_pos.x);
        }

        // Update all test calls to use the new exclusion-aware hopping function
        #[test]
        fn electron_hops_to_lower_potential_metal() {
            let mut a = Body::new(
                Vec2::new(0.0, 0.0),
                Vec2::zero(),
                1.0, 1.0,
                1.0,
                Species::LithiumMetal,
            );
            let mut b = Body::new(
                Vec2::new(1.0, 0.0),
                Vec2::zero(),
                1.0, 1.0,
                -2.0,
                Species::LithiumMetal,
            );
            a.update_charge_from_electrons();
            for _e in 0..(crate::config::FOIL_NEUTRAL_ELECTRONS + 1) {
                b.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
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
                config: SimConfig { ..Default::default() },
                foils: Vec::new(),
                cell_list: CellList::new(10.0, 1.0),
                body_to_foil: HashMap::new(),
            };
            assert_eq!(sim.bodies[0].electrons.len(), 0);
            assert_eq!(sim.bodies[1].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS + 1);
            sim.quadtree.build(&mut sim.bodies);
            let exclude = vec![false; sim.bodies.len()];
            sim.perform_electron_hopping_with_exclusions(&exclude);
            assert_eq!(sim.bodies[0].electrons.len(), 1);
            assert_eq!(sim.bodies[1].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS);
        }
        
        #[test]
        fn electron_hops_from_neutral_metal_to_deficient_foil() {
            use crate::body::{Body, Species, Electron};
            use crate::simulation::Simulation;
            use crate::quadtree::Quadtree;
            use crate::config;
            use crate::config::SimConfig;
            use ultraviolet::Vec2;
        
            // Create a neutral metal (LithiumMetal) with 1 electron.
            let mut metal = Body::new(
                Vec2::new(0.0, 0.0),
                Vec2::zero(),
                1.0,
                1.0,
                0.0,
                Species::LithiumMetal,
            );
            metal.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            metal.update_charge_from_electrons();
        
            // Create a deficient foil (FoilMetal) with 0 electrons.
            let mut foil = Body::new(
                Vec2::new(1.0, 0.0),
                Vec2::zero(),
                1.0,
                1.0,
                0.0,
                Species::FoilMetal,
            );
            // Leave foil.electrons empty to simulate electron deficiency.
            foil.update_charge_from_electrons();
        
            let mut sim = Simulation {
                dt: 0.1,
                frame: 0,
                bodies: vec![metal, foil],
                quadtree: Quadtree::new(
                    config::QUADTREE_THETA,
                    config::QUADTREE_EPSILON,
                    config::QUADTREE_LEAF_CAPACITY,
                    config::QUADTREE_THREAD_CAPACITY,
                ),
                bounds: 10.0,
                rewound_flags: vec![false; 2],
                background_e_field: Vec2::zero(),
                config: SimConfig { ..Default::default() },
                foils: Vec::new(),
                cell_list: CellList::new(10.0, 1.0),
                body_to_foil: HashMap::new(),
            };
        
            // Build quadtree before hopping.
            sim.quadtree.build(&mut sim.bodies);
            let exclude = vec![false; sim.bodies.len()];
            // Perform electron hopping.
            sim.perform_electron_hopping_with_exclusions(&exclude);
        
            // After hopping, we expect the metal to lose an electron and the foil to gain one.
            assert_eq!(sim.bodies[0].electrons.len(), 0, "Metal should lose its electron after hopping");
            assert_eq!(sim.bodies[1].electrons.len(), 1, "Deficient foil should receive an electron after hopping");
        }

        #[test]
        fn foil_current_accumulation_does_not_cause_neighbor_hopping() {
            use crate::body::{Body, Species, Electron};
            use crate::simulation::Simulation;
            use crate::quadtree::Quadtree;
            use crate::config;
            use crate::config::SimConfig;
            use crate::body::foil::Foil;
            use ultraviolet::Vec2;

            // Three foils in a row
            let mut foil1 = Body::new(Vec2::new(0.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
            let mut foil2 = Body::new(Vec2::new(2.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
            let mut foil3 = Body::new(Vec2::new(4.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
            // All start with neutral electron count
            for foil in [&mut foil1, &mut foil2, &mut foil3] {
                for _ in 0..crate::config::FOIL_NEUTRAL_ELECTRONS {
                    foil.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                }
                foil.update_charge_from_electrons();
            }

            let foil2_id = foil2.id; // Save the ID before moving foil2

            let mut sim = Simulation {
                dt: 0.1,
                frame: 0,
                bodies: vec![foil1, foil2, foil3],
                quadtree: Quadtree::new(
                    config::QUADTREE_THETA,
                    config::QUADTREE_EPSILON,
                    config::QUADTREE_LEAF_CAPACITY,
                    config::QUADTREE_THREAD_CAPACITY,
                ),
                bounds: 10.0,
                rewound_flags: vec![false; 3],
                background_e_field: Vec2::zero(),
                config: SimConfig { ..Default::default() },
                foils: vec![Foil {
                    id: 42, // Unique ID for the foil
                    link_id: None,
                    body_ids: vec![foil2_id], // Use the saved ID
                    dc_current: 10.0,
                    ac_current: 0.0,
                    accum: 1.5,
                    switch_hz: 0.0,
                    mode: crate::body::foil::LinkMode::Parallel,
                }],
                cell_list: CellList::new(10.0, 1.0),
                body_to_foil: HashMap::new(),
            };

            // Build quadtree before step
            sim.quadtree.build(&mut sim.bodies);
            // Step the simulation (should add 1 electron to foil2 only)
            sim.step();
            // Check electron counts

            let n1 = sim.bodies[0].electrons.len();
            let n2 = sim.bodies[1].electrons.len();
            let n3 = sim.bodies[2].electrons.len();
            assert_eq!(n1, crate::config::FOIL_NEUTRAL_ELECTRONS, "Foil 1 should not lose or gain electrons");
            assert_eq!(n2, crate::config::FOIL_NEUTRAL_ELECTRONS + 1, "Foil 2 should gain exactly one electron");
            assert_eq!(n3, crate::config::FOIL_NEUTRAL_ELECTRONS, "Foil 3 should not lose or gain electrons");
        }
    }
}
