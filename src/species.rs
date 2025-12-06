use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::body::Species;

#[derive(Clone, Copy, Debug)]
pub struct SpeciesProps {
    /// Mass in atomic mass units (amu)
    pub mass: f32,
    /// Radius in angstroms (Å)
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
            mass: 6.94,   // amu
            radius: 0.76, // Å
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
            mass: 6.94,   // amu
            radius: 1.52, // Å
            damping: 0.01,
            color: [192, 192, 192, 255],
            lj_enabled: true,
            lj_epsilon: crate::config::LJ_FORCE_EPSILON,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_METAL,
            polar_charge: crate::config::POLAR_CHARGE_DEFAULT,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m.insert(
        FoilMetal,
        SpeciesProps {
            mass: 1.0e6,  // amu
            radius: 1.52, // Å
            damping: 0.1,
            color: [128, 64, 0, 255],
            lj_enabled: true,
            lj_epsilon: crate::config::LJ_FORCE_EPSILON,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_METAL,
            polar_charge: crate::config::POLAR_CHARGE_DEFAULT,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m.insert(
        ElectrolyteAnion,
        SpeciesProps {
            mass: 145.0, // amu
            radius: 2.0, // Å
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
            mass: 88.06, // amu
            radius: 2.5, // Å
            damping: 1.0,
            color: [0, 200, 0, 100],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_EC,
            polar_charge: crate::config::POLAR_CHARGE_EC,
            enable_repulsion: true,
            repulsion_strength: 5.0, // Reduced from 100.0 - represents osmotic pressure
            repulsion_cutoff: 5.0,   // Reduced from 11.0 - shorter range interaction
        },
    );
    m.insert(
        DMC,
        SpeciesProps {
            mass: 90.08, // amu
            radius: 2.5, // Å
            damping: 1.0,
            color: [0, 100, 50, 200],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_DMC,
            polar_charge: crate::config::POLAR_CHARGE_DMC,
            enable_repulsion: true,
            repulsion_strength: 5.0, // Reduced from 100.0 - represents osmotic pressure
            repulsion_cutoff: 5.0,   // Reduced from 11.0 - shorter range interaction
        },
    );
    m.insert(
        VC,
        SpeciesProps {
            mass: 86.0,
            radius: 2.4,
            damping: 1.0,
            color: [220, 180, 255, 140],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_VC,
            polar_charge: crate::config::POLAR_CHARGE_VC,
            enable_repulsion: true,
            repulsion_strength: 5.0,
            repulsion_cutoff: 5.0,
        },
    );
    m.insert(
        FEC,
        SpeciesProps {
            mass: 107.0,
            radius: 2.5,
            damping: 0.8,
            color: [140, 210, 255, 160],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_FEC,
            polar_charge: crate::config::POLAR_CHARGE_FEC,
            enable_repulsion: true,
            repulsion_strength: 6.0,
            repulsion_cutoff: 5.0,
        },
    );
    m.insert(
        EMC,
        SpeciesProps {
            mass: 104.0,
            radius: 2.6,
            damping: 1.0,
            color: [120, 200, 140, 150],
            lj_enabled: false,
            lj_epsilon: 0.0,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_EMC,
            polar_charge: crate::config::POLAR_CHARGE_EMC,
            enable_repulsion: true,
            repulsion_strength: 4.5,
            repulsion_cutoff: 5.5,
        },
    );
    m.insert(
        LLZO,
        SpeciesProps {
            mass: 840.0,
            radius: 4.5,
            damping: 0.2,
            color: [255, 215, 130, 255],
            lj_enabled: true,
            lj_epsilon: crate::config::LJ_FORCE_EPSILON,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_LLZO,
            polar_charge: crate::config::POLAR_CHARGE_LLZO,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m.insert(
        LLZT,
        SpeciesProps {
            mass: 865.0,
            radius: 4.7,
            damping: 0.2,
            color: [255, 190, 80, 255],
            lj_enabled: true,
            lj_epsilon: crate::config::LJ_FORCE_EPSILON,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_LLZT,
            polar_charge: crate::config::POLAR_CHARGE_LLZT,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m.insert(
        S40B,
        SpeciesProps {
            mass: 340.0,
            radius: 4.2,
            damping: 0.25,
            color: [255, 235, 140, 255],
            lj_enabled: true,
            lj_epsilon: crate::config::LJ_FORCE_EPSILON,
            lj_sigma: crate::config::LJ_FORCE_SIGMA,
            lj_cutoff: crate::config::LJ_FORCE_CUTOFF,
            polar_offset: crate::config::ELECTRON_DRIFT_RADIUS_FACTOR_S40B,
            polar_charge: crate::config::POLAR_CHARGE_S40B,
            enable_repulsion: false,
            repulsion_strength: 5.0,
            repulsion_cutoff: 2.0,
        },
    );
    m
});

/// Maximum LJ interaction range across all species.
pub fn max_lj_cutoff() -> f32 {
    use Species::*;
    let species_list = [
        LithiumIon,
        LithiumMetal,
        FoilMetal,
        ElectrolyteAnion,
        EC,
        DMC,
        VC,
        FEC,
        EMC,
        LLZO,
        LLZT,
        S40B,
    ];

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
    let species_list = [
        LithiumIon,
        LithiumMetal,
        FoilMetal,
        ElectrolyteAnion,
        EC,
        DMC,
        VC,
        FEC,
        EMC,
        LLZO,
        LLZT,
        S40B,
    ];

    species_list
        .iter()
        .map(|&species| get_species_props(species))
        .filter(|p| p.enable_repulsion)
        .map(|p| p.repulsion_cutoff)
        .fold(0.0_f32, f32::max)
}

/// Mutable override properties for species (used by GUI)
pub static SPECIES_PROPERTY_OVERRIDES: Lazy<Mutex<HashMap<Species, SpeciesProps>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Get species properties with GUI overrides applied
pub fn get_species_props(species: Species) -> SpeciesProps {
    // Check if there's an override first
    if let Ok(overrides) = SPECIES_PROPERTY_OVERRIDES.lock() {
        if let Some(override_props) = overrides.get(&species) {
            return *override_props;
        }
    }
    // Fall back to default properties
    SPECIES_PROPERTIES.get(&species).copied().unwrap_or({
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

pub fn calculate_solvent_particle_counts(
    parts: &[(Species, f32)],
    total_count: usize,
) -> Vec<(Species, usize)> {
    let total_parts: f32 = parts.iter().map(|(_, p)| p).sum();
    if total_parts <= 0.0 {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut current_count = 0;

    for (i, (species, part)) in parts.iter().enumerate() {
        let count = if i == parts.len() - 1 {
            total_count.saturating_sub(current_count)
        } else {
            ((part / total_parts) * total_count as f32).round() as usize
        };

        if count > 0 {
            result.push((*species, count));
            current_count += count;
        }
    }

    result
}
