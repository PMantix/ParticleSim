// body/redox.rs
// Contains charge update and redox logic for Body

use super::types::{Body, Species};
use crate::config::{
    FOIL_NEUTRAL_ELECTRONS,
    LITHIUM_METAL_NEUTRAL_ELECTRONS,
    REDOX_I0, REDOX_ALPHA_A, REDOX_ALPHA_C, REDOX_NF_RT,
};
use rand::random;

impl Body {
    pub fn update_charge_from_electrons(&mut self) {
        match self.species {
            Species::FoilMetal => {
                self.charge = -(self.electrons.len() as f32 - FOIL_NEUTRAL_ELECTRONS as f32);
            }
            Species::LithiumMetal => {
                self.charge = -(self.electrons.len() as f32 - LITHIUM_METAL_NEUTRAL_ELECTRONS as f32);
            }
            Species::LithiumIon => {
                self.charge = 1.0 - (self.electrons.len() as f32);
            }
        }
    }
    /// Compute Butler-Volmer redox probabilities based on local overpotential.
    pub fn redox_probabilities(&self, dt: f32) -> (f32, f32) {
        // Approximate overpotential using the x-component of the local field
        // across the body radius to retain a sign.
        let eta = self.e_field.x * self.radius;
        let k_red = REDOX_I0 * (-REDOX_ALPHA_C * eta * REDOX_NF_RT).exp();
        let k_ox = REDOX_I0 * (REDOX_ALPHA_A * eta * REDOX_NF_RT).exp();
        let p_red = 1.0 - (-k_red * dt).exp();
        let p_ox = 1.0 - (-k_ox * dt).exp();
        (p_red, p_ox)
    }

    pub fn apply_redox(&mut self, dt: f32) {
        match self.species {
            Species::LithiumIon => {
                if !self.electrons.is_empty() {
                    let (p_red, _) = self.redox_probabilities(dt);
                    if rand::random::<f32>() < p_red {
                        self.species = Species::LithiumMetal;
                    }
                    self.update_charge_from_electrons();
                }
            }
            Species::LithiumMetal => {
                if self.electrons.is_empty() {
                    let (_, p_ox) = self.redox_probabilities(dt);
                    if rand::random::<f32>() < p_ox {
                        self.species = Species::LithiumIon;
                    }
                    self.update_charge_from_electrons();
                }
            }
            Species::FoilMetal => {
                // FoilMetal never changes species
            }
        }
    }
}
