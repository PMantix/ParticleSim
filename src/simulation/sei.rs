use super::simulation::Simulation;
use crate::body::{Body, Species};
use crate::profile_scope;
use rand::random;
use rayon::prelude::*;

impl Simulation {
    /// Perform SEI formation for eligible bodies in the simulation.
    /// Applies kinetic charge thresholds, consumes excess electrons from nearby
    /// metals, and converts the solvent into an enlarged SEI particle.
    pub fn perform_sei_formation(&mut self) {
        if !self.config.sei_formation_enabled {
            return;
        }
        profile_scope!("sei_formation");

        let dt = self.dt;
        let prob_base = self.config.sei_formation_probability;
        let bias = self.config.sei_formation_bias;
        #[derive(Clone, Copy)]
        struct SeiCandidate {
            solvent_idx: usize,
            metal_idx: usize,
            solvent_radius: f32,
            solvent_species: Species,
        }

        let config_snapshot = self.config.clone();
        let required_electrons = config_snapshot
            .sei_electrons_per_event
            .max(1) as usize;

        let conversions: Vec<SeiCandidate> = self
            .bodies
            .par_iter()
            .enumerate()
            .filter_map(|(i, body)| {
                let species = match body.species {
                    Species::EC
                    | Species::DMC
                    | Species::VC
                    | Species::FEC
                    | Species::EMC => body.species,
                    _ => return None,
                };

                let search_radius = body.radius * 2.5;
                let neighbors = self
                    .quadtree
                    .find_neighbors_within(&self.bodies, i, search_radius);

                for &neighbor_idx in &neighbors {
                    if neighbor_idx == i || neighbor_idx >= self.bodies.len() {
                        continue;
                    }

                    let neighbor = &self.bodies[neighbor_idx];
                    if !matches!(neighbor.species, Species::LithiumMetal | Species::FoilMetal) {
                        continue;
                    }

                    if neighbor.charge >= -0.01 {
                        continue;
                    }

                    let available_charge = (-neighbor.charge).max(0.0);
                    if available_charge < required_electrons as f32 {
                        continue;
                    }

                    let threshold =
                        sei_charge_threshold_for_species(species, &config_snapshot);
                    if available_charge < threshold {
                        continue;
                    }

                    // Increase probability as the local charge exceeds the kinetic threshold
                    let drive = if threshold > 0.0 {
                        1.0
                            + (available_charge - threshold).max(0.0)
                                / (threshold.max(0.25) + 1e-6)
                    } else {
                        1.0 + available_charge
                    };

                    let prob = prob_base * bias * drive * dt;
                    if prob > 0.0 && random::<f32>() < prob {
                        return Some(SeiCandidate {
                            solvent_idx: i,
                            metal_idx: neighbor_idx,
                            solvent_radius: body.radius,
                            solvent_species: species,
                        });
                    }
                }

                None
            })
            .collect();

        let radius_scale = self.config.sei_radius_scale.max(0.1);

        for candidate in conversions {
            if candidate.solvent_idx >= self.bodies.len()
                || candidate.metal_idx >= self.bodies.len()
            {
                continue;
            }

            let (metal_idx, solvent_idx) = (candidate.metal_idx, candidate.solvent_idx);
            if let Some((metal, solvent)) =
                two_body_indices_mut(&mut self.bodies, metal_idx, solvent_idx)
            {
                let threshold =
                    sei_charge_threshold_for_species(candidate.solvent_species, &self.config);
                let available_charge = (-metal.charge).max(0.0);
                if available_charge < required_electrons as f32 || available_charge < threshold {
                    continue;
                }

                if !consume_extra_electrons(metal, required_electrons) {
                    continue;
                }

                solvent.species = Species::SEI;
                solvent.mass = Species::SEI.mass();
                let base_radius = Species::SEI.radius();
                let scaled_radius = (candidate.solvent_radius * radius_scale).max(base_radius);
                solvent.radius = scaled_radius;
                solvent.electrons.clear();
                solvent.update_charge_from_electrons();
                solvent.vel *= 0.1;
                solvent.vz *= 0.1;
            }
        }
    }
}

fn sei_charge_threshold_for_species(species: Species, config: &crate::config::SimConfig) -> f32 {
    match species {
        Species::VC => config.sei_charge_threshold_vc,
        Species::FEC => config.sei_charge_threshold_fec,
        Species::EC => config.sei_charge_threshold_ec,
        Species::EMC => config.sei_charge_threshold_emc,
        Species::DMC => config.sei_charge_threshold_dmc,
        _ => 0.0,
    }
}

fn two_body_indices_mut(
    bodies: &mut [Body],
    first: usize,
    second: usize,
) -> Option<(&mut Body, &mut Body)> {
    if first == second || first >= bodies.len() || second >= bodies.len() {
        return None;
    }

    if first < second {
        let (left, right) = bodies.split_at_mut(second);
        Some((&mut left[first], &mut right[0]))
    } else {
        let (left, right) = bodies.split_at_mut(first);
        Some((&mut right[0], &mut left[second]))
    }
}

fn consume_extra_electrons(body: &mut Body, count: usize) -> bool {
    let neutral = body.neutral_electron_count();
    if body.electrons.len() < neutral + count {
        return false;
    }

    for _ in 0..count {
        body.electrons.pop();
    }
    body.update_charge_from_electrons();
    true
}

#[cfg(test)]
mod sei_tests {
    use super::super::simulation::Simulation;
    use crate::body::{Body, Electron, Species};
    use ultraviolet::Vec2;

    fn base_sim() -> Simulation {
        let mut sim = Simulation::new();
        sim.bodies.clear();
        sim.config.sei_formation_enabled = true;
        sim.config.sei_formation_probability = 5.0; // Combined with dt ensures prob>=1 when allowed
        sim.config.sei_formation_bias = 1.0;
        sim.config.sei_electrons_per_event = 1;
        sim.config.sei_radius_scale = 1.0;
        sim.dt = 1.0;
        sim
    }

    fn populate_metal(extra_electrons: usize) -> Body {
        let mut metal = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            Species::LithiumMetal.radius(),
            0.0,
            Species::LithiumMetal,
        );
        let neutral = crate::config::LITHIUM_METAL_NEUTRAL_ELECTRONS;
        let target = neutral + extra_electrons;
        while metal.electrons.len() < target {
            metal.electrons.push(Electron {
                rel_pos: Vec2::zero(),
                vel: Vec2::zero(),
            });
        }
        metal.update_charge_from_electrons();
        metal
    }

    fn populate_solvent(species: Species) -> Body {
        Body::new(
            Vec2::new(0.5, 0.0),
            Vec2::zero(),
            species.mass(),
            species.radius(),
            0.0,
            species,
        )
    }

    fn simulate_once(species: Species, extra_electrons: usize) -> (bool, usize) {
        let mut sim = base_sim();
        let metal = populate_metal(extra_electrons);
        let solvent = populate_solvent(species);
        sim.bodies.push(metal);
        sim.bodies.push(solvent);
        sim.quadtree.build(&mut sim.bodies);

        sim.perform_sei_formation();

        let neutral = crate::config::LITHIUM_METAL_NEUTRAL_ELECTRONS;
        let remaining_extra = sim.bodies[0]
            .electrons
            .len()
            .saturating_sub(neutral);
        (sim.bodies[1].species == Species::SEI, remaining_extra)
    }

    fn required_extra_electrons(species: Species, config: &crate::config::SimConfig) -> usize {
        let threshold = sei_charge_threshold_for_species(species, config);
        threshold.max(1.0).ceil() as usize
    }

    #[test]
    fn sei_forms_at_threshold_for_each_solvent() {
        let config = crate::config::SimConfig::default();
        let species_cases = [Species::VC, Species::FEC, Species::EC, Species::EMC, Species::DMC];
        for &species in &species_cases {
            let required = required_extra_electrons(species, &config);
            let (converted, remaining_extra) = simulate_once(species, required);
            assert!(converted, "{species:?} should convert with {required} extra e⁻");
            assert_eq!(
                remaining_extra,
                required.saturating_sub(1),
                "Metal should lose exactly one electron when {species:?} converts"
            );
        }
    }

    #[test]
    fn sei_blocked_below_threshold_for_each_solvent() {
        let config = crate::config::SimConfig::default();
        let species_cases = [Species::VC, Species::FEC, Species::EC, Species::EMC, Species::DMC];
        for &species in &species_cases {
            let required = required_extra_electrons(species, &config);
            let insufficient = required.saturating_sub(1);
            let (converted, remaining_extra) = simulate_once(species, insufficient);
            assert!(
                !converted,
                "{species:?} should NOT convert with only {insufficient} extra e⁻"
            );
            assert_eq!(
                remaining_extra,
                insufficient,
                "Metal electron count should remain unchanged for {species:?}"
            );
        }
    }
}
