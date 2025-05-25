// body/redox.rs
// Contains charge update and redox logic for Body

use super::types::{Body, Species};

impl Body {
    pub fn update_charge_from_electrons(&mut self) {
        match self.species {
            Species::LithiumMetal => {
                self.charge = -(self.electrons.len() as f32 - 1.0);
            }
            Species::LithiumIon => {
                self.charge = 1.0 - self.electrons.len() as f32;
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
        }
    }
}
