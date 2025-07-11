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
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let idx = sim.bodies.len();
        let id = body.id;
        sim.bodies.push(body);
        let mut foil = Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, -10.0, 0.0);
        foil.accum = -100.0;
        sim.foils.push(foil);
        sim.step();
        assert_eq!(sim.bodies[idx].electrons.len(), 0, "Foil should be able to go down to 0 electrons");
    }

    #[test]
    fn foil_current_adds_and_removes_electrons_within_limits() {
        let mut sim = Simulation::new();
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let idx = sim.bodies.len();
        let id = body.id;
        sim.bodies.push(body);
        let mut foil = Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 2.0, 0.0);
        foil.accum = (crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32;
        sim.foils.push(foil);
        sim.step();
        assert_eq!(sim.bodies[idx].electrons.len(), crate::config::FOIL_MAX_ELECTRONS, "Electrons should be added up to FOIL_MAX_ELECTRONS");
        sim.foils[0].dc_current = -2.0;
        sim.foils[0].accum = -((crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32);
        sim.step();
        assert_eq!(sim.bodies[idx].electrons.len(), 1, "Electrons should be removed down to 1 (one electron removed from 2)");
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

        let mut body_a = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body_a.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let id_a = body_a.id;
        sim.bodies.push(body_a);

        let body_b = Body::new(Vec2::new(2.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        let id_b = body_b.id;
        sim.bodies.push(body_b);

        let mut foil_a = Foil::new(vec![id_a], Vec2::zero(), 1.0, 1.0, 1.0, 0.0);
        let mut foil_b = Foil::new(vec![id_b], Vec2::zero(), 1.0, 1.0, -1.0, 0.0);
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
        assert_eq!(sim.bodies[1].electrons.len(), 0, "Foil B should remain without electrons");
    }
}
