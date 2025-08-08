use std::collections::HashMap;
use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::body::Species;

#[derive(Clone, Copy, Debug)]
pub struct SpeciesProps {
    pub mass: f32,
    pub radius: f32,
    pub damping: f32,
    pub color: [u8; 4],
    pub lj_enabled: bool,
    pub lj_epsilon: f32,
    pub lj_sigma: f32,
    pub lj_cutoff: f32,
    pub polar_offset: f32,
    pub polar_charge: f32,
    pub enable_repulsion: bool,
    pub repulsion_strength: f32,
    pub repulsion_cutoff: f32,
}

pub static SPECIES_PROPERTIES: Lazy<HashMap<Species, SpeciesProps>> = Lazy::new(|| {
    use Species::*;
    let mut m = HashMap::new();
    m.insert(
        LithiumIon,
        SpeciesProps {
            mass: 1.0,
            radius: 0.6667,
            damping: 1.0,
            color: [255, 255, 0, 255],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: 0.0,
            polar_charge: crate::config::POLAR_CHARGE_DEFAULT,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m.insert(
        LithiumMetal,
        SpeciesProps {
            mass: 10.0,
            radius: 1.0,
            damping: 0.01,
            color: [192, 192, 192, 255],
            lj_enabled: true,
            lj_epsilon: crate::config::LJ_FORCE_EPSILON,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: 0.0,
            polar_charge: crate::config::POLAR_CHARGE_DEFAULT,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m.insert(
        FoilMetal,
        SpeciesProps {
            mass: 1e6,
            radius: 1.0,
            damping: 0.1,
            color: [128, 64, 0, 255],
            lj_enabled: true,
            lj_epsilon: crate::config::LJ_FORCE_EPSILON,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: 0.0,
            polar_charge: crate::config::POLAR_CHARGE_DEFAULT,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m.insert(
        ElectrolyteAnion,
        SpeciesProps {
            mass: 20.88,
            radius: 1.6667,
            damping: 1.0,
            color: [0, 128, 255, 255],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: 0.3,
            polar_charge: crate::config::POLAR_CHARGE_DEFAULT,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m.insert(
        EC,
        SpeciesProps {
            mass: 12.69,
            radius: 1.5,
            damping: 1.0,
            color: [0, 200, 0, 100],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_EC,
            polar_charge: crate::config::POLAR_CHARGE_EC,
            enable_repulsion: true,
            repulsion_strength: 100.0,
            repulsion_cutoff: 11.0,
        },
    );
    m.insert(
        DMC,
        SpeciesProps {
            mass: 12.98,
            radius: 1.5,
            damping: 1.0,
            color: [0, 100, 50, 200],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_DMC,
            polar_charge: crate::config::POLAR_CHARGE_DMC,
            enable_repulsion: true,
            repulsion_strength: 100.0,
            repulsion_cutoff: 11.0,
        },
    );
    m
});

/// Maximum LJ interaction range across all species.
pub fn max_lj_cutoff() -> f32 {
    use Species::*;
    let species_list = [LithiumIon, LithiumMetal, FoilMetal, ElectrolyteAnion, EC, DMC];
    
    species_list
        .iter()
        .map(|&species| get_species_props(species))
        .filter(|p| p.lj_enabled)
        .map(|p| p.lj_cutoff * p.lj_sigma)
        .fold(0.0_f32, f32::max)
}

/// Maximum repulsion cutoff across all species.
pub fn max_repulsion_cutoff() -> f32 {
    use Species::*;
    let species_list = [LithiumIon, LithiumMetal, FoilMetal, ElectrolyteAnion, EC, DMC];

    species_list
        .iter()
        .map(|&species| get_species_props(species))
        .filter(|p| p.enable_repulsion)
        .map(|p| p.repulsion_cutoff)
        .fold(0.0_f32, f32::max)
}

/// Mutable override properties for species (used by GUI)
pub static SPECIES_PROPERTY_OVERRIDES: Lazy<Mutex<HashMap<Species, SpeciesProps>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// Get species properties with GUI overrides applied
pub fn get_species_props(species: Species) -> SpeciesProps {
    // Check if there's an override first
    if let Ok(overrides) = SPECIES_PROPERTY_OVERRIDES.lock() {
        if let Some(override_props) = overrides.get(&species) {
            return *override_props;
        }
    }
    // Fall back to default properties
    SPECIES_PROPERTIES.get(&species).copied().unwrap_or_else(|| {
        // Fallback if species not found
        SpeciesProps {
            mass: 1.0,
            radius: 1.0,
            damping: 1.0,
            color: [255, 255, 255, 255],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: 0.0,
            polar_charge: crate::config::POLAR_CHARGE_DEFAULT,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        }
    })
}

/// Update species properties (used by GUI)
pub fn update_species_props(species: Species, props: SpeciesProps) {
    if let Ok(mut overrides) = SPECIES_PROPERTY_OVERRIDES.lock() {
        overrides.insert(species, props);
    }
}
