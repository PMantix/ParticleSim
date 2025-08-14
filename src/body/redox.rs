// body/redox.rs
// Contains charge update and redox logic for Body

use super::types::{Body, Species};
use crate::config::{
    FOIL_NEUTRAL_ELECTRONS,
    LITHIUM_METAL_NEUTRAL_ELECTRONS,
    ENABLE_ELECTRON_SEA_PROTECTION,
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
        let old_species = self.species;
        
        match self.species {
            Species::LithiumIon => {
                if !self.electrons.is_empty() {
                    self.species = Species::LithiumMetal;
                    self.update_charge_from_electrons();
                }
            }
            Species::LithiumMetal => {
                // Determine escape direction from local electric field
                let escape_dir = if self.e_field.mag() > 1e-6 {
                    Some(self.e_field.normalized())
                } else {
                    None
                };

                let can_oxidize = if ENABLE_ELECTRON_SEA_PROTECTION {
                    !self.surrounded_by_metal
                } else {
                    true
                };

                let mut allow = false;

                if let Some(dir) = escape_dir {
                    // Allow oxidation if no electron aligns with escape direction
                    let threshold = 0.5; // ~60 degrees
                    let aligned = self.electrons.iter().any(|e| {
                        let rel = e.rel_pos;
                        rel.mag() > 1e-6 && rel.normalized().dot(dir) > threshold
                    });
                    if !aligned { allow = true; }
                } else if self.electrons.is_empty() {
                    // Fallback to original behavior when no escape direction
                    allow = true;
                }

                if can_oxidize && allow {
                    self.electrons.clear();
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
        
        // Update radius if species changed
        if old_species != self.species {
            self.radius = self.species.radius();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::body::{Body, Species, Electron};
    use ultraviolet::Vec2;

    #[test]
    fn apply_redox_updates_radius_on_species_change() {
        let ion_radius = Species::LithiumIon.radius();
        let metal_radius = Species::LithiumMetal.radius();
        
        // Test ion -> metal
        let mut ion = Body::new(Vec2::zero(), Vec2::zero(), 1.0, ion_radius, 1.0, Species::LithiumIon);
        assert_eq!(ion.radius, ion_radius);
        assert_eq!(ion.species, Species::LithiumIon);
        
        // Add electron to make it become metal
        ion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        ion.apply_redox();
        
        assert_eq!(ion.species, Species::LithiumMetal);
        assert_eq!(ion.radius, metal_radius);
        
        // Test metal -> ion
        let mut metal = Body::new(Vec2::zero(), Vec2::zero(), 1.0, metal_radius, 0.0, Species::LithiumMetal);
        assert_eq!(metal.radius, metal_radius);
        assert_eq!(metal.species, Species::LithiumMetal);
        
        // Remove all electrons to make it become ion
        metal.electrons.clear();
        metal.apply_redox();
        
        assert_eq!(metal.species, Species::LithiumIon);
        assert_eq!(metal.radius, ion_radius);
    }

    #[test]
    fn apply_redox_respects_electron_sea_protection() {
        let ion_radius = Species::LithiumIon.radius();
        let metal_radius = Species::LithiumMetal.radius();
        
        // Test that surrounded metal resists oxidation
        let mut surrounded_metal = Body::new(Vec2::zero(), Vec2::zero(), 1.0, metal_radius, 0.0, Species::LithiumMetal);
        surrounded_metal.surrounded_by_metal = true; // Simulate being in a metal cluster
        assert_eq!(surrounded_metal.species, Species::LithiumMetal);
        assert_eq!(surrounded_metal.electrons.len(), 0); // No electrons, would normally oxidize
        
        // Apply redox - should NOT convert to ion due to electron sea protection
        surrounded_metal.apply_redox();
        
        if crate::config::ENABLE_ELECTRON_SEA_PROTECTION {
            assert_eq!(surrounded_metal.species, Species::LithiumMetal, 
                       "Surrounded metal should resist oxidation due to electron sea");
            assert_eq!(surrounded_metal.radius, metal_radius);
        }
        
        // Test that isolated metal still oxidizes normally
        let mut isolated_metal = Body::new(Vec2::zero(), Vec2::zero(), 1.0, metal_radius, 0.0, Species::LithiumMetal);
        isolated_metal.surrounded_by_metal = false; // Not surrounded
        assert_eq!(isolated_metal.electrons.len(), 0); // No electrons
        
        // Apply redox - should convert to ion
        isolated_metal.apply_redox();
        
        assert_eq!(isolated_metal.species, Species::LithiumIon, 
                   "Isolated metal with no electrons should oxidize to ion");
        assert_eq!(isolated_metal.radius, ion_radius);
    }
}
