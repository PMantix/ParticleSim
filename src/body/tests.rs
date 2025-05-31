// body/tests.rs
// Contains all tests for Body, including electron and hopping tests

#[cfg(test)]
mod physics {
    use crate::body::{Body, Species, Electron};
    use crate::Simulation;
    use crate::quadtree::Quadtree;
    use ultraviolet::Vec2;
    use crate::config;
    use crate::config::SimConfig;

    #[test]
    fn ion_becomes_metal_when_charge_high() {
        let mut b = Body {
            pos: Vec2::zero(),
            vel: Vec2::zero(),
            acc: Vec2::zero(),
            mass: 1.0,
            radius: 1.0,
            charge: 0.00,
            id: 0,
            species: Species::LithiumIon,
            electrons: Vec::new(),
            e_field: Vec2::zero(),
            fixed: false,
        };
        b.update_species();
        assert_eq!(b.species, Species::LithiumMetal);
    }

    #[test]
    fn metal_becomes_ion_when_charge_low() {
        let mut b = Body {
            pos: Vec2::zero(),
            vel: Vec2::zero(),
            acc: Vec2::zero(),
            mass: 1.0,
            radius: 1.0,
            charge: 1.0,
            id: 0,
            species: Species::LithiumMetal,
            electrons: Vec::new(),
            e_field: Vec2::zero(),
            fixed: false,
        };
        b.update_species();
        assert_eq!(b.species, Species::LithiumIon);
    }

    #[test]
    fn electron_moves_under_field() {
        let mut b = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,1.0,
            0.0,
            Species::LithiumMetal,
        );
        b.electrons=vec![Electron {rel_pos:Vec2::zero(),vel:Vec2::zero()}];
        let field = Vec2::new(1.0, 0.0);
        b.update_electrons(|_pos| field, 0.1);
        assert!(b.electrons[0].rel_pos.x < 0.0,
            "Expected electron to drift left (x < 0), but rel_pos.x = {}", b.electrons[0].rel_pos.x);
    }

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
        for _e in 0..crate::config::FOIL_NEUTRAL_ELECTRONS {
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
            config: SimConfig {..Default::default()},
            foils: Vec::new(),
        };
        assert_eq!(sim.bodies[0].electrons.len(), 0);
        assert_eq!(sim.bodies[1].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS);
        sim.quadtree.build(&mut sim.bodies);
        sim.perform_electron_hopping();
        assert_eq!(sim.bodies[0].electrons.len(), 1);
        assert_eq!(sim.bodies[1].electrons.len(), 2);
    }
}
