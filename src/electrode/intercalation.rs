// electrode/intercalation.rs
// Physics and logic for Li⁺ intercalation/deintercalation at electrode surfaces

use super::region::ActiveMaterialRegion;
use super::material::{MaterialType, StorageMechanism};

/// Configuration for intercalation physics
#[derive(Clone, Debug)]
pub struct IntercalationConfig {
    /// Enable intercalation reactions
    pub enabled: bool,
    
    /// Thermal energy kT in eV (0.026 eV at 300K)
    pub thermal_energy_ev: f32,
    
    /// Base rate constant for intercalation (1/fs)
    pub base_rate: f32,
    
    /// Butler-Volmer transfer coefficient (typically 0.5)
    pub transfer_coeff: f32,
    
    /// Maximum distance from surface for intercalation (Å)
    pub surface_distance: f32,
    
    /// Probability multiplier for tuning rates
    pub rate_multiplier: f32,
}

impl Default for IntercalationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            thermal_energy_ev: 0.026,  // ~300K
            base_rate: 1e-4,           // Base hopping rate
            transfer_coeff: 0.5,
            surface_distance: 5.0,     // Å
            rate_multiplier: 1.0,
        }
    }
}

/// Result of attempting an intercalation event
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IntercalationResult {
    /// Li⁺ successfully intercalated (absorbed into electrode)
    Success,
    /// Electrode is at full capacity
    AtCapacity,
    /// Li⁺ is too far from surface
    TooFar,
    /// Desolvation barrier not overcome this timestep  
    BarrierNotOvercome,
    /// Wrong species (not Li⁺)
    WrongSpecies,
    /// No electrons available for reduction
    NoElectrons,
    /// Intercalation disabled for this material
    Disabled,
}

/// Result of attempting a deintercalation event
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeintercalationResult {
    /// Li successfully released as Li⁺
    Success,
    /// Electrode is empty (no Li to release)
    Empty,
    /// Rate check failed this timestep
    RateNotMet,
    /// Wrong overpotential direction
    WrongPotential,
    /// Deintercalation disabled
    Disabled,
}

/// Calculate desolvation probability for Li⁺ approaching electrode
/// 
/// The desolvation barrier depends on:
/// - Material-specific activation energy
/// - Number of coordinating solvent molecules
/// - Temperature
/// 
/// Returns probability per timestep
pub fn desolvation_probability(
    material: MaterialType,
    solvation_number: usize,
    config: &IntercalationConfig,
    dt: f32,
) -> f32 {
    // Get base barrier for this material
    let base_barrier = material.desolvation_barrier();
    
    // Scale barrier with solvation - more solvent = harder to desolvate
    // Typical Li⁺ coordination number is 4
    let solvation_factor = (solvation_number as f32 / 4.0).max(0.5);
    let barrier = base_barrier * solvation_factor;
    
    // Arrhenius rate: k = k0 * exp(-Ea/kT)
    let rate = config.base_rate * (-barrier / config.thermal_energy_ev).exp();
    
    // Convert rate to probability for this timestep
    let prob = rate * dt * config.rate_multiplier;
    prob.clamp(0.0, 1.0)
}

/// Calculate Butler-Volmer rate for charge transfer
/// 
/// Rate = i0 * [exp(α*F*η/RT) - exp(-(1-α)*F*η/RT)]
/// 
/// For intercalation, we use overpotential relative to material OCV
pub fn butler_volmer_rate(
    material: MaterialType,
    soc: f32,
    applied_potential: f32, // V vs Li/Li⁺
    config: &IntercalationConfig,
    dt: f32,
) -> f32 {
    let ocv = material.open_circuit_voltage(soc);
    let overpotential = applied_potential - ocv;
    
    let i0 = material.exchange_current();
    let alpha = config.transfer_coeff;
    let f_over_rt = 1.0 / config.thermal_energy_ev; // F/RT in 1/V
    
    // Butler-Volmer equation (simplified)
    let forward = (alpha * f_over_rt * overpotential).exp();
    let backward = (-(1.0 - alpha) * f_over_rt * overpotential).exp();
    let rate = i0 * (forward - backward);
    
    // Convert to probability
    let prob = rate.abs() * config.base_rate * dt * config.rate_multiplier;
    prob.clamp(0.0, 1.0)
}

/// Direction of net current based on overpotential
pub fn reaction_direction(
    material: MaterialType,
    soc: f32,
    applied_potential: f32,
) -> ReactionDirection {
    let ocv = material.open_circuit_voltage(soc);
    let overpotential = applied_potential - ocv;
    
    match material.mechanism() {
        StorageMechanism::Plating => {
            // For Li metal: negative overpotential → plating (reduction)
            if overpotential < -0.01 {
                ReactionDirection::Reduction // Li⁺ + e⁻ → Li⁰
            } else if overpotential > 0.01 {
                ReactionDirection::Oxidation  // Li⁰ → Li⁺ + e⁻
            } else {
                ReactionDirection::Equilibrium
            }
        }
        StorageMechanism::Intercalation | StorageMechanism::Alloying => {
            // For intercalation: depends on anode vs cathode
            match material.role() {
                super::material::ElectrodeRole::Anode => {
                    // Anode: lower potential → intercalation (charging)
                    if overpotential < -0.01 {
                        ReactionDirection::Reduction  // Li⁺ intercalates
                    } else if overpotential > 0.01 {
                        ReactionDirection::Oxidation  // Li deintercalates
                    } else {
                        ReactionDirection::Equilibrium
                    }
                }
                super::material::ElectrodeRole::Cathode => {
                    // Cathode: higher potential → deintercalation (charging)
                    if overpotential > 0.01 {
                        ReactionDirection::Oxidation  // Li deintercalates
                    } else if overpotential < -0.01 {
                        ReactionDirection::Reduction  // Li⁺ intercalates
                    } else {
                        ReactionDirection::Equilibrium
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReactionDirection {
    /// Reduction: Li⁺ + e⁻ → Li (intercalation/plating)
    Reduction,
    /// Oxidation: Li → Li⁺ + e⁻ (deintercalation/stripping)  
    Oxidation,
    /// At equilibrium (within threshold)
    Equilibrium,
}

/// Statistics for intercalation events over a time window
#[derive(Clone, Debug, Default)]
pub struct IntercalationStats {
    pub attempted_intercalations: usize,
    pub successful_intercalations: usize,
    pub rejected_at_capacity: usize,
    pub rejected_barrier: usize,
    pub attempted_deintercalations: usize,
    pub successful_deintercalations: usize,
    pub rejected_empty: usize,
}

impl IntercalationStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    
    pub fn intercalation_success_rate(&self) -> f32 {
        if self.attempted_intercalations == 0 {
            0.0
        } else {
            self.successful_intercalations as f32 / self.attempted_intercalations as f32
        }
    }
    
    pub fn deintercalation_success_rate(&self) -> f32 {
        if self.attempted_deintercalations == 0 {
            0.0
        } else {
            self.successful_deintercalations as f32 / self.attempted_deintercalations as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desolvation_probability() {
        let config = IntercalationConfig::default();
        let dt = 1.0; // 1 fs
        
        // Should be low probability per step
        let prob = desolvation_probability(MaterialType::Graphite, 4, &config, dt);
        assert!(prob > 0.0 && prob < 0.1);
        
        // Less solvation = higher probability
        let prob_less = desolvation_probability(MaterialType::Graphite, 2, &config, dt);
        assert!(prob_less > prob);
    }

    #[test]
    fn test_reaction_direction() {
        // Graphite anode at 50% SOC, OCV ~0.1V
        let material = MaterialType::Graphite;
        let soc = 0.5;
        let ocv = material.open_circuit_voltage(soc);
        
        // Below OCV → reduction (intercalation)
        assert_eq!(
            reaction_direction(material, soc, ocv - 0.1),
            ReactionDirection::Reduction
        );
        
        // Above OCV → oxidation (deintercalation)
        assert_eq!(
            reaction_direction(material, soc, ocv + 0.1),
            ReactionDirection::Oxidation
        );
        
        // At OCV → equilibrium
        assert_eq!(
            reaction_direction(material, soc, ocv),
            ReactionDirection::Equilibrium
        );
    }
}
