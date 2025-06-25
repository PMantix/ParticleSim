// body/redox.rs
// Contains charge update and redox logic for Body

use super::types::{Body, Species};
use crate::config::{
    FOIL_NEUTRAL_ELECTRONS,
    LITHIUM_METAL_NEUTRAL_ELECTRONS,
    IONIZATION_FIELD_THRESHOLD,
};
use ultraviolet::Vec2;
use crate::quadtree::Quadtree;
use crate::cell_list::CellList;

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
    pub fn apply_redox(
        &mut self,
        bodies: &[Body],
        quadtree: &Quadtree,
        background_field: Vec2,
        _cell_list: &CellList,
        _density_threshold: f32,
    ) {
        let local_field =
            quadtree.field_at_point(bodies, self.pos, crate::simulation::forces::K_E)
            + background_field;
        let field_mag = local_field.mag();
        match self.species {
            Species::LithiumIon => {
                if !self.electrons.is_empty() && field_mag > IONIZATION_FIELD_THRESHOLD {
                    self.species = Species::LithiumMetal;
                    self.update_charge_from_electrons();
                }
            }
            Species::LithiumMetal => {
                if self.electrons.is_empty() && field_mag > IONIZATION_FIELD_THRESHOLD {
                    self.species = Species::LithiumIon;
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
