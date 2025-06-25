// body/redox.rs
// Contains charge update and redox logic for Body

use super::types::{Body, Species};
use super::electron::Electron;
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
        config: &crate::config::SimConfig,
        dt: f32,
    ) {
        let local_field =
            quadtree.field_at_point(bodies, self.pos, crate::simulation::forces::K_E)
            + background_field;
        let field_mag = local_field.mag();
        match self.species {
            Species::LithiumIon => {
                if field_mag > IONIZATION_FIELD_THRESHOLD {
                    let rate = if config.use_butler_volmer {
                        let alpha = config.bv_transfer_coeff;
                        let scale = config.bv_overpotential_scale;
                        let i0 = config.bv_exchange_current;
                        let forward = (alpha * field_mag / scale).exp();
                        let backward = (-(1.0 - alpha) * field_mag / scale).exp();
                        (i0 * (forward - backward)).abs()
                    } else {
                        config.hop_rate_k0
                            * (config.hop_transfer_coeff * field_mag
                                / config.hop_activation_energy)
                                .exp()
                    };
                    let p = 1.0 - (-rate * dt).exp();
                    if rand::random::<f32>() < p {
                        self.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                        self.species = Species::LithiumMetal;
                        self.update_charge_from_electrons();
                    }
                }
            }
            Species::LithiumMetal => {
                if !self.electrons.is_empty() && field_mag > IONIZATION_FIELD_THRESHOLD {
                    let rate = if config.use_butler_volmer {
                        let alpha = config.bv_transfer_coeff;
                        let scale = config.bv_overpotential_scale;
                        let i0 = config.bv_exchange_current;
                        let forward = (alpha * field_mag / scale).exp();
                        let backward = (-(1.0 - alpha) * field_mag / scale).exp();
                        (i0 * (forward - backward)).abs()
                    } else {
                        config.hop_rate_k0
                            * (config.hop_transfer_coeff * field_mag
                                / config.hop_activation_energy)
                                .exp()
                    };
                    let p = 1.0 - (-rate * dt).exp();
                    if rand::random::<f32>() < p {
                        self.electrons.pop();
                        self.update_charge_from_electrons();
                        if self.electrons.is_empty() {
                            self.species = Species::LithiumIon;
                        }
                    }
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
