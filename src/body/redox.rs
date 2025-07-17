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
            Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                self.charge = -(self.electrons.len() as f32 - self.neutral_electron_count() as f32);
            }
        }
    }
    pub fn apply_redox(&mut self) {
        match self.species {
            Species::LithiumIon => {
                if !self.electrons.is_empty() {
                    self.species = Species::LithiumMetal;
                    self.update_charge_from_electrons();
                }
            }
            Species::LithiumMetal => {
                if self.electrons.is_empty() {
                    self.species = Species::LithiumIon;
                    self.update_charge_from_electrons();
                }
            }
            Species::FoilMetal => {
                // FoilMetal never changes species
            }
            Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                // Electrolyte anions and solvent molecules remain the same species
            }
        }
    }
}
