// body/redox.rs
// Contains charge update and redox logic for Body

use super::types::{Body, Species};
use crate::config::{
    ENABLE_ELECTRON_SEA_PROTECTION, FOIL_NEUTRAL_ELECTRONS, LITHIUM_METAL_NEUTRAL_ELECTRONS,
    BASELINE_POTENTIAL, POTENTIAL_PER_CHARGE,
};

/// Calculate local electrochemical potential from charge
/// Negative charge (excess electrons) → lower potential (more reducing)
/// Positive charge (electron deficit) → higher potential (more oxidizing)
pub fn local_potential_from_charge(charge: f32) -> f32 {
    BASELINE_POTENTIAL + charge * POTENTIAL_PER_CHARGE
}

// The four methods below (equilibrium_potential, local_potential, donation_overpotential,
// can_donate_electron) implement the Phase 2 thermodynamic donation gating from
// docs/ELECTRODE_MATERIALS_PLAN.md. Currently unused because ENABLE_DONATION_GATING is
// disabled (the gating logic has known bugs documented in the plan). Kept in place for
// when Phase 2 is revisited; allow(dead_code) suppresses the warning meanwhile.
#[allow(dead_code)]
impl Body {
    /// Get the equilibrium potential (V vs Li/Li⁺) for this species
    /// This is the potential at which the species is in equilibrium
    pub fn equilibrium_potential(&self) -> f32 {
        match self.species {
            Species::LithiumMetal | Species::LithiumIon => 0.0, // Li⁺/Li reference
            Species::FoilMetal => 0.0, // Current collector, same as Li reference
            Species::Graphite => 0.1,   // Graphite intercalation
            Species::HardCarbon => 0.2, // Hard carbon
            Species::SiliconOxide => 0.4, // SiOx
            Species::LTO => 1.55,       // Li₄Ti₅O₁₂
            Species::LFP => 3.4,        // LiFePO₄
            Species::LMFP => 3.5,       // LiMnFePO₄  
            Species::NMC => 3.7,        // LiNiMnCoO₂ (average)
            Species::NCA => 3.7,        // LiNiCoAlO₂
            Species::SEI => 0.8,        // SEI formation potential
            // Electrolyte/solvent species - not directly involved in redox
            Species::ElectrolyteAnion | Species::EC | Species::DMC | 
            Species::VC | Species::FEC | Species::EMC => 0.8, // EC reduction ~0.8V
            Species::LLZO | Species::LLZT | Species::S40B => 0.0, // Solid electrolytes
        }
    }
    
    /// Get the local electrochemical potential based on this body's charge
    pub fn local_potential(&self) -> f32 {
        local_potential_from_charge(self.charge)
    }
    
    /// Get the overpotential required for electron donation (species-specific kinetics)
    /// This represents the kinetic barrier for electron transfer from the material.
    /// Fast kinetics → low overpotential, slow kinetics → high overpotential
    pub fn donation_overpotential(&self) -> f32 {
        match self.species {
            Species::LithiumMetal => 0.05,   // Fast kinetics
            Species::FoilMetal => 0.0,       // Current collector, no barrier
            Species::Graphite => 0.1,        // Moderate kinetics
            Species::HardCarbon => 0.15,     // Slower than graphite
            Species::SiliconOxide => 0.12,   // Moderate
            Species::LTO => 0.05,            // Fast kinetics (spinel structure)
            Species::LFP => 0.1,             // Moderate (olivine)
            Species::LMFP => 0.12,           // Slightly slower than LFP
            Species::NMC => 0.08,            // Fast (layered)
            Species::NCA => 0.08,            // Fast (layered)
            _ => 0.1,                        // Default
        }
    }
    
    /// Check if electron donation is thermodynamically favorable
    /// Electrons can only be donated when local_potential < equilibrium_potential + donation_overpotential
    pub fn can_donate_electron(&self) -> bool {
        self.local_potential() < self.equilibrium_potential() + self.donation_overpotential()
    }
    
    pub fn update_charge_from_electrons(&mut self) {
        match self.species {
            Species::FoilMetal => {
                self.charge = -(self.electrons.len() as f32 - FOIL_NEUTRAL_ELECTRONS as f32);
            }
            Species::LithiumMetal => {
                self.charge =
                    -(self.electrons.len() as f32 - LITHIUM_METAL_NEUTRAL_ELECTRONS as f32);
            }
            Species::LithiumIon => {
                self.charge = 1.0 - (self.electrons.len() as f32);
            }
            Species::ElectrolyteAnion
            | Species::EC
            | Species::DMC
            | Species::VC
            | Species::FEC
            | Species::EMC => {
                self.charge = -(self.electrons.len() as f32 - self.neutral_electron_count() as f32);
            }
            Species::LLZO | Species::LLZT | Species::S40B => {
                self.charge = -(self.electrons.len() as f32 - self.neutral_electron_count() as f32);
            }
            Species::SEI => {
                self.charge = 0.0; // SEI is always neutral
            }
            // Intercalation electrode materials - charge based on electron excess
            // neutral_electron_count() returns 0, so charge = -electrons.len()
            // This allows electron hopping to work properly between electrode particles
            Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO
            | Species::LFP | Species::LMFP | Species::NMC | Species::NCA => {
                self.charge = -(self.electrons.len() as f32 - self.neutral_electron_count() as f32);
            }
        }
    }
    /// Run redox conversion logic for this body.
    ///
    /// `current_time` is the current sim_time in fs. `lock_duration` is the
    /// minimum gap (in fs) between two species transitions on the same body —
    /// after a transition fires, this body refuses further species changes
    /// until `current_time >= species_lock_until`. This breaks the
    /// oxidize/reduce ping-pong that would otherwise occur on consecutive
    /// timesteps when a freshly-formed ion is still in hop range of a
    /// neighboring metal-with-electron. `lock_duration = 0` disables the
    /// lock and matches pre-lock behavior (used in unit tests).
    pub fn apply_redox(&mut self, current_time: f32, lock_duration: f32) {
        // If still locked, no species change permitted this call.
        if current_time < self.species_lock_until {
            return;
        }
        let old_species = self.species;

        match self.species {
            Species::LithiumIon => {
                if !self.electrons.is_empty() {
                    // Always convert Li⁺ with an electron to Li⁰.
                    // A neutral LithiumIon (charge=0) is physically invalid, so we must
                    // convert regardless of potential gating. Potential gating belongs at
                    // the electron-transfer decision in electron_hopping.rs, not here.
                    self.species = Species::LithiumMetal;
                    self.update_charge_from_electrons();
                }
            }
            Species::LithiumMetal => {
                if self.electrons.is_empty() {
                    // Check if this metal is surrounded by other metals (electron sea)
                    // If surrounded, resist oxidation as electrons are delocalized
                    let can_oxidize = if ENABLE_ELECTRON_SEA_PROTECTION {
                        !self.surrounded_by_metal
                    } else {
                        true
                    };

                    if can_oxidize {
                        self.species = Species::LithiumIon;
                        self.update_charge_from_electrons();
                    }
                }
            }
            Species::FoilMetal => {
                // FoilMetal never changes species
            }
            Species::ElectrolyteAnion
            | Species::EC
            | Species::DMC
            | Species::VC
            | Species::FEC
            | Species::EMC => {
                // Electrolyte anions and solvent molecules remain the same species
            }
            Species::LLZO | Species::LLZT | Species::S40B => {
                // Solid electrolyte grains never change species
            }
            Species::SEI => {
                // SEI never changes species (irreversible formation)
            }
            // Intercalation electrode materials - never change species (Li storage is tracked separately)
            Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO
            | Species::LFP | Species::LMFP | Species::NMC | Species::NCA => {
                // Electrode materials don't undergo redox - Li intercalation is handled separately
            }
        }

        // Update radius if species changed, and arm the lock so the next
        // transition can't fire until lock_duration fs have elapsed.
        if old_species != self.species {
            self.radius = self.species.radius();
            self.species_lock_until = current_time + lock_duration;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::body::{Body, Electron, Species};
    use ultraviolet::Vec2;

    #[test]
    fn apply_redox_updates_radius_on_species_change() {
        let ion_radius = Species::LithiumIon.radius();
        let metal_radius = Species::LithiumMetal.radius();

        // Test ion -> metal
        let mut ion = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            ion_radius,
            1.0,
            Species::LithiumIon,
        );
        assert_eq!(ion.radius, ion_radius);
        assert_eq!(ion.species, Species::LithiumIon);

        // Add electron to make it become metal
        ion.electrons.push(Electron {
            rel_pos: Vec2::zero(),
            vel: Vec2::zero(),
        });
        ion.apply_redox(0.0, 0.0);

        assert_eq!(ion.species, Species::LithiumMetal);
        assert_eq!(ion.radius, metal_radius);

        // Test metal -> ion
        let mut metal = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            metal_radius,
            0.0,
            Species::LithiumMetal,
        );
        assert_eq!(metal.radius, metal_radius);
        assert_eq!(metal.species, Species::LithiumMetal);

        // Remove all electrons to make it become ion
        metal.electrons.clear();
        metal.apply_redox(0.0, 0.0);

        assert_eq!(metal.species, Species::LithiumIon);
        assert_eq!(metal.radius, ion_radius);
    }

    #[test]
    fn apply_redox_respects_electron_sea_protection() {
        let ion_radius = Species::LithiumIon.radius();
        let metal_radius = Species::LithiumMetal.radius();

        // Test that surrounded metal resists oxidation
        let mut surrounded_metal = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            metal_radius,
            0.0,
            Species::LithiumMetal,
        );
        surrounded_metal.surrounded_by_metal = true; // Simulate being in a metal cluster
        assert_eq!(surrounded_metal.species, Species::LithiumMetal);
        assert_eq!(surrounded_metal.electrons.len(), 0); // No electrons, would normally oxidize

        // Apply redox - should NOT convert to ion due to electron sea protection
        surrounded_metal.apply_redox(0.0, 0.0);

        if crate::config::ENABLE_ELECTRON_SEA_PROTECTION {
            assert_eq!(
                surrounded_metal.species,
                Species::LithiumMetal,
                "Surrounded metal should resist oxidation due to electron sea"
            );
            assert_eq!(surrounded_metal.radius, metal_radius);
        }

        // Test that isolated metal still oxidizes normally
        let mut isolated_metal = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            metal_radius,
            0.0,
            Species::LithiumMetal,
        );
        isolated_metal.surrounded_by_metal = false; // Not surrounded
        assert_eq!(isolated_metal.electrons.len(), 0); // No electrons

        // Apply redox - should convert to ion
        isolated_metal.apply_redox(0.0, 0.0);

        assert_eq!(
            isolated_metal.species,
            Species::LithiumIon,
            "Isolated metal with no electrons should oxidize to ion"
        );
        assert_eq!(isolated_metal.radius, ion_radius);
    }
}
