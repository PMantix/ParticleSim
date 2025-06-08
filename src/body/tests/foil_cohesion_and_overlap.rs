// Tests for foil overlapping indices and cohesion
#[cfg(test)]
mod foil_cohesion_and_overlap {
    use crate::body::{Body, Species, Electron};
    use crate::body::foil::Foil;
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;
    use smallvec::smallvec;

    #[test]
    fn overlapping_foil_indices_handled() {
        let mut sim = Simulation::new();
        let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
        let idx = sim.bodies.len();
        let id = body.id;
        sim.bodies.push(body);
        sim.foils.push(Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, 1.0));
        sim.foils.push(Foil::new(vec![id], Vec2::zero(), 1.0, 1.0, -1.0));
        sim.step();
        assert_eq!(sim.bodies[idx].electrons.len(), crate::config::FOIL_NEUTRAL_ELECTRONS, "Overlapping foils should not crash and net current is zero");
    }

    #[test]
    fn foil_particles_remain_cohesive_within_electron_limits() {
        let n = 5;
        let spacing = 2.0;
        let mut sim = Simulation::new();
        let mut ids = Vec::new();
        for i in 0..n {
            let mut body = Body::new(
                Vec2::new(i as f32 * spacing, 0.0),
                Vec2::zero(),
                1e6,
                1.0,
                0.0,
                Species::FoilMetal,
            );
            body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
            ids.push(body.id);
            sim.bodies.push(body);
        }
        sim.foils.push(Foil::new(ids.clone(), Vec2::zero(), n as f32 * spacing, 1.0, 0.0));
        let initial_avg_dist: f32 = (0..n-1)
            .map(|i| (sim.bodies[i+1].pos - sim.bodies[i].pos).mag())
            .sum::<f32>() / (n as f32 - 1.0);
        for i in 0..n {
            let body = &mut sim.bodies[i];
            while body.electrons.len() < crate::config::FOIL_MAX_ELECTRONS {
                body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
        }
        for _ in 0..20 {
            sim.step();
        }
        let avg_dist_after_add: f32 = (0..n-1)
            .map(|i| (sim.bodies[i+1].pos - sim.bodies[i].pos).mag())
            .sum::<f32>() / (n as f32 - 1.0);
        for i in 0..n {
            let body = &mut sim.bodies[i];
            while !body.electrons.is_empty() {
                body.electrons.pop();
            }
        }
        for _ in 0..20 {
            sim.step();
        }
        let avg_dist_after_remove: f32 = (0..n-1)
            .map(|i| (sim.bodies[i+1].pos - sim.bodies[i].pos).mag())
            .sum::<f32>() / (n as f32 - 1.0);
        assert!(avg_dist_after_add < initial_avg_dist * 1.5, "Foil particles separated too much after adding electrons: {} vs {}", avg_dist_after_add, initial_avg_dist);
        assert!(avg_dist_after_remove < initial_avg_dist * 1.5, "Foil particles separated too much after removing electrons: {} vs {}", avg_dist_after_remove, initial_avg_dist);
    }
}
