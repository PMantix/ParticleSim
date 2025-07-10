use std::collections::HashMap;
use once_cell::sync::Lazy;

use crate::body::Species;
use crate::config;

#[derive(Clone, Copy, Debug)]
pub struct SpeciesProps {
    pub mass: f32,
    pub radius: f32,
    pub damping: f32,
    pub lj_enabled: bool,
    pub lj_epsilon: f32,
    pub lj_sigma: f32,
    pub lj_cutoff: f32,
}

pub static SPECIES_PROPERTIES: Lazy<HashMap<Species, SpeciesProps>> = Lazy::new(|| {
    use Species::*;
    let mut m = HashMap::new();
    m.insert(LithiumIon, SpeciesProps {
        mass: 1.0,
        radius: 1.0,
        damping: 1.0,
        lj_enabled: false,
        lj_epsilon: config::LJ_FORCE_EPSILON,
        lj_sigma: config::LJ_FORCE_SIGMA,
        lj_cutoff: config::LJ_FORCE_CUTOFF,
    });
    m.insert(LithiumMetal, SpeciesProps {
        mass: 1.0,
        radius: 1.0,
        damping: 1.0,
        lj_enabled: true,
        lj_epsilon: config::LJ_FORCE_EPSILON,
        lj_sigma: config::LJ_FORCE_SIGMA,
        lj_cutoff: config::LJ_FORCE_CUTOFF,
    });
    m.insert(FoilMetal, SpeciesProps {
        mass: 1e6,
        radius: 1.0,
        damping: 1.0,
        lj_enabled: true,
        lj_epsilon: config::LJ_FORCE_EPSILON,
        lj_sigma: config::LJ_FORCE_SIGMA,
        lj_cutoff: config::LJ_FORCE_CUTOFF,
    });
    m.insert(ElectrolyteAnion, SpeciesProps {
        mass: 40.0,
        radius: 1.5,
        damping: 1.0,
        lj_enabled: false,
        lj_epsilon: config::LJ_FORCE_EPSILON,
        lj_sigma: config::LJ_FORCE_SIGMA,
        lj_cutoff: config::LJ_FORCE_CUTOFF,
    });
    m
});

