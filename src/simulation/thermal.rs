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
        if self.bodies.is_empty() { return; }
        if target_temp <= 0.0 {
            eprintln!("[thermostat-skip] non-positive target");
            return;
        }

        // Calculate current temperature of liquid particles (Li+, Anion, EC, DMC)
        let mut liquid_ke = 0.0;
        let mut liquid_count = 0;
    let mut _total_ke = 0.0f32; // retained for potential future use
        for body in &self.bodies {
            _total_ke += 0.5 * body.mass * body.vel.mag_sq();
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
        if current_temp <= 1e-8 {
            // Bootstrap: assign random velocities at target temperature then return (will log next interval)
            crate::simulation::utils::initialize_liquid_velocities_to_temperature(&mut self.bodies, target_temp);
            self.thermostat_bootstrapped = true;
            return;
        }
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
        // Recompute liquid temperature after scaling for debug
        let mut new_liquid_ke = 0.0f32;
        for body in &self.bodies {
            match body.species {
                Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                    new_liquid_ke += 0.5 * body.mass * body.vel.mag_sq();
                }
                _ => {}
            }
        }
        let new_liquid_temp = (new_liquid_ke / liquid_count as f32) / BOLTZMANN_CONSTANT;
        if self.frame % 1000 == 0 { // occasional summary
            eprintln!("[thermostat-summary] frame={} N_liq={} T_liq={:.2}K target={:.2}K scale={:.4}", self.frame, liquid_count, new_liquid_temp, target_temp, scale);
        }
        // Store last scale factor in a debug field? Could add instrumentation later.
    }
}