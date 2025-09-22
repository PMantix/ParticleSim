// thermal.rs
// Temperature control and thermostat functionality

use crate::body::Species;
use crate::units::BOLTZMANN_CONSTANT;
use super::Simulation;

impl Simulation {
    /// Apply Maxwell-Boltzmann thermostat to maintain target temperature
    /// Only applies to solvent particles (EC/DMC), excludes metals
    pub fn apply_thermostat(&mut self) {
        let target_temp = self.config.temperature;
        if target_temp <= 0.0 {
            return;
        }

        // Calculate current temperature of solvent particles only
        let mut solvent_ke = 0.0;
        let mut solvent_count = 0;

        for body in &self.bodies {
            match body.species {
                Species::EC | Species::DMC => {
                    solvent_ke += 0.5 * body.mass * body.vel.mag_sq();
                    solvent_count += 1;
                }
                _ => {} // Skip metals and ions
            }
        }

        if solvent_count == 0 {
            return; // No solvent particles to thermostat
        }

        // For 2D: <E> = k_B * T, so T = <E> / k_B
        let avg_kinetic_energy = solvent_ke / solvent_count as f32;
        let current_temp = avg_kinetic_energy / BOLTZMANN_CONSTANT;

        if current_temp > 0.0 {
            let scale = (target_temp / current_temp).sqrt();

            // Scale velocities of solvent particles only
            for body in &mut self.bodies {
                match body.species {
                    Species::EC | Species::DMC => {
                        body.vel *= scale;
                    }
                    _ => {} // Don't modify metals or ions
                }
            }
        }
    }
}