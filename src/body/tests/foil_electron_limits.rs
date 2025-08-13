// Tests for foil electron limits and default electron count
#[cfg(test)]
mod foil_electron_limits {
    use crate::body::{Body, Species, Electron};
    use crate::body::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;
    use smallvec::smallvec;

    #[test]
    fn foil_does_not_drop_below_zero_electrons() {
        let mut sim = Simulation::new();
        
        // Create two foils: one to lose electrons and one to gain them for charge conservation
        let mut body1 = Body::new(Vec2::new(0.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body1.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let id1 = body1.id;
        sim.bodies.push(body1);
        
        let mut body2 = Body::new(Vec2::new(10.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body2.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let id2 = body2.id;
        sim.bodies.push(body2);
        
        // Foil 1 wants to lose electrons (negative accumulation)
        let mut foil1 = Foil::new(vec![id1], Vec2::new(0.0, 0.0), 1.0, 1.0, -10.0, 0.0);
        foil1.accum = -100.0; // Very negative - wants to lose many electrons
        sim.foils.push(foil1);
        
        // Foil 2 wants to gain electrons (positive accumulation) for charge conservation
        let mut foil2 = Foil::new(vec![id2], Vec2::new(10.0, 0.0), 1.0, 1.0, 10.0, 0.0);
        foil2.accum = 100.0; // Very positive - wants to gain many electrons
        sim.foils.push(foil2);
        
        sim.step();
        assert_eq!(sim.bodies[0].electrons.len(), 0, "Foil should be able to go down to 0 electrons");
        assert_eq!(sim.bodies[1].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS + 1, "Charge conservation foil should gain the transferred electron");
    }

    #[test]
    fn foil_current_adds_and_removes_electrons_within_limits() {
        let mut sim = Simulation::new();
        
        // Create two foils for charge conservation
        let mut body1 = Body::new(Vec2::new(0.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body1.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let id1 = body1.id;
        sim.bodies.push(body1);
        
        let mut body2 = Body::new(Vec2::new(10.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body2.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let id2 = body2.id;
        sim.bodies.push(body2);
        
        // Test adding electrons first
        let mut foil1 = Foil::new(vec![id1], Vec2::new(0.0, 0.0), 1.0, 1.1, 2.0, 0.0);
        foil1.accum = (crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32; // Wants to gain 1 electron
        sim.foils.push(foil1);
        
        // Charge conservation foil that loses electrons
        let mut foil2 = Foil::new(vec![id2], Vec2::new(10.0, 0.0), 1.0, 1.0, -2.0, 0.0);
        foil2.accum = -((crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32); // Wants to lose 1 electron
        sim.foils.push(foil2);
        
        sim.step();
        assert_eq!(sim.bodies[0].electrons.len(), crate::config::FOIL_MAX_ELECTRONS, "Electrons should be added up to FOIL_MAX_ELECTRONS");
        assert_eq!(sim.bodies[1].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS - 1, "Charge conservation foil should lose one electron");
        
        // Test removing electrons (reverse the accumulations)
        sim.foils[0].dc_current = -2.0;
        sim.foils[0].accum = -((crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32); // Now wants to lose electrons
        sim.foils[1].dc_current = 2.0;
        sim.foils[1].accum = (crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32; // Now wants to gain electrons
        
        sim.step();
        assert_eq!(sim.bodies[0].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Electrons should be removed back to neutral level");
        assert_eq!(sim.bodies[1].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Charge conservation foil should gain back the electron");
    }

    #[test]
    fn foil_default_electrons() {
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        assert_eq!(body.electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "FoilMetal should start with correct number of electrons");
    }

    #[test]
    fn linked_foils_stop_when_one_cannot_transfer() {
        use crate::body::foil::LinkMode;
        let mut sim = Simulation::new();

        // Place bodies far apart to prevent electron hopping
        let mut body_a = Body::new(Vec2::new(-1000.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body_a.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let id_a = body_a.id;
        sim.bodies.push(body_a);

        // Body B starts with neutral electrons but will have them removed before linking
        let mut body_b = Body::new(Vec2::new(1000.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body_b.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let id_b = body_b.id;
        sim.bodies.push(body_b);

        // Remove all electrons from body B first to set up the test condition
        let body_b_idx = sim.bodies.len() - 1;
        sim.bodies[body_b_idx].electrons.clear();

        let mut foil_a = Foil::new(vec![id_a], Vec2::new(-1000.0, 0.0), 1.0, 1.0, 1.0, 0.0);
        let mut foil_b = Foil::new(vec![id_b], Vec2::new(1000.0, 0.0), 1.0, 1.0, -1.0, 0.0);
        foil_a.link_id = Some(foil_b.id);
        foil_b.link_id = Some(foil_a.id);
        foil_a.mode = LinkMode::Opposite;
        foil_b.mode = LinkMode::Opposite;
        foil_a.accum = 1.0;
        foil_b.accum = -1.0;
        sim.foils.push(foil_a);
        sim.foils.push(foil_b);

        sim.step();

        assert_eq!(sim.bodies[0].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Foil A should not gain electrons when Foil B cannot lose any");
        assert_eq!(sim.bodies[1].electrons.len(), 0, "Foil B should remain at 0 electrons since it cannot lose any more");
    }
}
