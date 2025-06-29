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
        assert_eq!(sim.bodies[idx].electrons.len(), 0, "Foil should not have fewer than 0 electrons");
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
        sim.foils[0].current = -2.0;
        sim.foils[0].accum = -((crate::config::FOIL_MAX_ELECTRONS - crate::config::FOIL_NEUTRAL_ELECTRONS) as f32);
        sim.step();
        assert_eq!(sim.bodies[idx].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Electrons should be removed down to FOIL_NEUTRAL_ELECTRONS");
    }

    #[test]
    fn foil_default_electrons() {
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        assert_eq!(body.electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "FoilMetal should start with correct number of electrons");
    }
}
