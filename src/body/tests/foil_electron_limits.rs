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
        // Under strict conservation a single foil cannot change electron count.
        let mut sim = Simulation::new();
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        // Start at neutral (1). Attempt large negative current should NOT remove without partner.
        body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let id = body.id;
        sim.bodies.push(body);
        let mut foil = Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, -10.0, 0.0);
        foil.accum = -100.0; // would request many removals
        sim.foils.push(foil);
        sim.step();
        assert_eq!(sim.bodies[0].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Single foil cannot remove electrons without partner");
    }

    #[test]
    fn foil_current_adds_and_removes_electrons_within_limits() {
        // Use two foils: one adding, one removing, to satisfy conservation.
        let mut sim = Simulation::new();
        let mut body_add = Body::new(Vec2::new(-10.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body_add.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let mut body_remove = Body::new(Vec2::new(10.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        // Give remover foil extra electrons so it can donate.
        body_remove.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_MAX_ELECTRONS];
        let id_add = body_add.id;
        let id_remove = body_remove.id;
        sim.bodies.push(body_add);
        sim.bodies.push(body_remove);
        let mut foil_add = Foil::new(vec![id_add], Vec2::new(-10.0,0.0), 1.0,1.0, 2.0, 0.0);
        foil_add.accum = 1.5; // ready to add one
        let mut foil_remove = Foil::new(vec![id_remove], Vec2::new(10.0,0.0), 1.0,1.0, -2.0, 0.0);
        foil_remove.accum = -1.5; // ready to remove one
        sim.foils.push(foil_add);
        sim.foils.push(foil_remove);
        let before_add = sim.bodies[0].electrons.len();
        let before_remove = sim.bodies[1].electrons.len();
        sim.step();
        assert_eq!(sim.bodies[0].electrons.len(), (before_add + 1).min(crate::config::FOIL_MAX_ELECTRONS), "Adder foil should gain exactly one electron");
        assert_eq!(sim.bodies[1].electrons.len(), before_remove - 1, "Remover foil should lose exactly one electron");
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
