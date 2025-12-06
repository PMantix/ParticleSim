// thermal.rs
// Temperature control and thermostat functionality

use crate::body::Species;
use crate::units::BOLTZMANN_CONSTANT;

#[cfg(feature = "thermostat_debug")]
macro_rules! tdbg { ($($arg:tt)*) => { eprintln!($($arg)*); } }
#[cfg(not(feature = "thermostat_debug"))]
macro_rules! tdbg {
    ($($arg:tt)*) => {};
}
use super::Simulation;

impl Simulation {
    /// Apply Maxwell-Boltzmann thermostat to maintain target temperature
    /// Applies only to liquid particles (Li+, anion, EC, DMC); excludes metals
    pub fn apply_thermostat(&mut self) {
        let target_temp = self.config.temperature;
        if self.bodies.is_empty() {
            tdbg!("[thermostat-skip] empty bodies");
            return;
        }
        if target_temp <= 0.0 {
            tdbg!("[thermostat-skip] non-positive target: {}", target_temp);
            return;
        }
        tdbg!(
            "[thermostat-enter] frame={} target_temp={} bodies_count={}",
            self.frame,
            target_temp,
            self.bodies.len()
        );

        // Calculate current temperature of liquid particles (Li+, anion, EC, DMC)
        // Exclude metals which may be constrained / collective
        let mut liquid_ke = 0.0;
        let mut liquid_count = 0;
        for body in &self.bodies {
            match body.species {
                Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                    liquid_ke += 0.5 * body.mass * body.vel.mag_sq();
                    liquid_count += 1;
                    if body.species == Species::EC {
                        _ec_count += 1;
                    }
                    if body.species == Species::DMC {
                        _dmc_count += 1;
                    }
                    if body.species == Species::LithiumIon {
                        _li_count += 1;
                    }
                    if body.species == Species::ElectrolyteAnion {
                        _anion_count += 1;
                    }
                }
                _ => {} // Skip metals (LithiumMetal, FoilMetal)
            }
        }
        tdbg!(
            "[thermostat-debug] Li+: {} Anion: {} EC: {} DMC: {} Total bodies: {}",
            _li_count,
            _anion_count,
            _ec_count,
            _dmc_count,
            self.bodies.len()
        );
        if liquid_count == 0 {
            tdbg!("[thermostat-debug] No liquid particles found");
            return;
        }
        // Compute current liquid KE per particle and convert to Kelvin.
        // KE_per_particle (2D) = k_B * T  =>  T = KE_per_particle / k_B
        let ke_per_particle = liquid_ke / liquid_count as f32;
        let current_temp = ke_per_particle / BOLTZMANN_CONSTANT; // Kelvin

        // Debug: Check first few particles to see what's happening with low temperature
        #[cfg(feature = "thermostat_debug")]
        if current_temp < 10.0 {
            let mut debug_count = 0;
            let mut total_vel_mag = 0.0;
            let mut total_mass = 0.0;
            for body in &self.bodies {
                if matches!(
                    body.species,
                    Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC
                ) {
                    if debug_count < 5 {
                        let vel_mag = body.vel.mag();
                        let ke = 0.5 * body.mass * body.vel.mag_sq();
                        tdbg!("[thermostat-sample] particle={} species={:?} mass={:.6} vel_mag={:.6} ke={:.6}", 
                    debug_count, body.species, body.mass, vel_mag, ke);
                    }
                    total_vel_mag += body.vel.mag();
                    total_mass += body.mass;
                    debug_count += 1;
                    if debug_count >= 5 && debug_count >= liquid_count {
                        break;
                    }
                }
            }
            let avg_vel_mag = if debug_count > 0 {
                total_vel_mag / debug_count as f32
            } else {
                0.0
            };
            let avg_mass = if debug_count > 0 {
                total_mass / debug_count as f32
            } else {
                0.0
            };
            tdbg!(
                "[thermostat-avg] avg_vel_mag={:.6} avg_mass={:.6} sampled={}",
                avg_vel_mag,
                avg_mass,
                debug_count
            );
        }
        tdbg!("[thermostat-debug] frame={} liquid_count={} liquid_ke={:.6} ke_pp={:.6} current_temp={:.6}K", self.frame, liquid_count, liquid_ke, ke_per_particle, current_temp);
        tdbg!(
            "[thermostat-active] Thermostat will run: current={:.6}K target={:.2}K",
            current_temp,
            target_temp
        );
        if current_temp <= 1e-3 {
            // effectively zero Kelvin
            // Bootstrap: assign random velocities to all liquid particles (Li+, anion, EC, DMC)
            crate::simulation::utils::initialize_liquid_velocities_to_temperature(
                &mut self.bodies,
                target_temp,
            );
            tdbg!(
                "[thermostat-bootstrap] frame={} bootstrapped velocities",
                self.frame
            );
            self.thermostat_bootstrapped = true;
            return;
        }
        let scale = (target_temp / current_temp).sqrt(); // dimensionless
        let safe_scale = scale.clamp(0.1, 10.0); // Prevent extreme scaling
        tdbg!("[thermostat-scale] frame={} current_temp={:.6} target_temp={:.2} scale={:.4} safe_scale={:.4}", 
            self.frame, current_temp, target_temp, scale, safe_scale);
        *crate::renderer::state::LAST_THERMOSTAT_SCALE.lock() = safe_scale;
        // Scale velocities for liquid particles only using the safe scale
        for body in &mut self.bodies {
            match body.species {
                Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                    body.vel *= safe_scale;
                }
                _ => {} // Don't thermostat ions - they're electrostatically constrained
            }
        }
        // Recompute liquid temperature after scaling for debug
        let mut _new_liquid_ke = 0.0f32; // debug only
        for body in &self.bodies {
            match body.species {
                Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                    _new_liquid_ke += 0.5 * body.mass * body.vel.mag_sq();
                }
                _ => {}
            }
        }
        #[cfg(feature = "thermostat_debug")]
        {
            let new_ke_per_particle = _new_liquid_ke / liquid_count as f32;
            let new_liquid_temp = new_ke_per_particle / BOLTZMANN_CONSTANT;
            if self.frame % 1000 == 0 {
                tdbg!("[thermostat-summary] frame={} N_liquid={} T_liquid={:.2}K target={:.2}K scale={:.4}", self.frame, liquid_count, new_liquid_temp, target_temp, scale);
            }
        }
        // Store last scale factor in a debug field? Could add instrumentation later.
    }
}
