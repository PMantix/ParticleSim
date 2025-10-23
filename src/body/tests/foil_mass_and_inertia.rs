// Tests for foil mass and inertia
#![cfg(any(test, feature = "unit_tests"))]
#[cfg(test)]
mod foil_mass_and_inertia {
    use crate::body::foil::Foil;
    use crate::body::Body;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    #[test]
    fn foil_is_inertial_with_large_mass() {
        let mut sim = Simulation::new();
        let body = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1e6,
            1.0,
            0.0,
            crate::body::Species::FoilMetal,
        );
        let idx = sim.bodies.len();
        let id = body.id;
        sim.bodies.push(body);
        sim.foils
            .push(Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 0.0, 0.0));
        sim.step();
        assert_eq!(
            sim.bodies[idx].mass, 1e6,
            "FoilMetal should have large mass"
        );
    }
}
