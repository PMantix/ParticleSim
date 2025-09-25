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

        // Calculate current temperature of liquid particles (Li+, Anion, EC, DMC)
        let mut liquid_ke = 0.0;
        let mut liquid_count = 0;
        for body in &self.bodies {
            match body.species {
                Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                    liquid_ke += 0.5 * body.mass * body.vel.mag_sq();
                    liquid_count += 1;
                }
                _ => {}
            }
        }
        if liquid_count == 0 { return; }
        let avg_kinetic_energy = liquid_ke / liquid_count as f32;
        let current_temp = avg_kinetic_energy / BOLTZMANN_CONSTANT;
        if current_temp <= 0.0 { return; }
    let scale = (target_temp / current_temp).sqrt();
    *crate::renderer::state::LAST_THERMOSTAT_SCALE.lock() = scale;
        // Scale velocities for liquid species only
        for body in &mut self.bodies {
            match body.species {
                Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                    body.vel *= scale;
                }
                _ => {}
            }
        }
        if (self.frame % 500) == 0 {
            eprintln!("[thermostat] frame={} current_liquid_T={:.2}K target_T={:.2}K scale={:.4}", self.frame, current_temp, target_temp, scale);
        }
        // Store last scale factor in a debug field? Could add instrumentation later.
    }
}