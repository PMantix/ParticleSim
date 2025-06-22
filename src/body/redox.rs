// body/redox.rs
// Contains charge update and redox logic for Body

use super::types::{Body, Species};
use crate::config::{
    FOIL_NEUTRAL_ELECTRONS,
    LITHIUM_METAL_NEUTRAL_ELECTRONS,
};

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
            Species::ElectrolyteAnion => {
                self.charge = -(self.electrons.len() as f32);
            }
        }
    }

    // Add the missing method here
    pub fn redox_probabilities(&self, _dt: f32) -> (f32, f32) {
        // Placeholder implementation, replace with actual logic as needed
        (0.5, 0.5)
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
            Species::ElectrolyteAnion => {
                // Electrolyte anions remain the same species
            }
        }
    }
}
