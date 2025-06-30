use std::collections::HashMap;
use once_cell::sync::Lazy;

use crate::body::Species;

#[derive(Clone, Copy, Debug)]
pub struct SpeciesProps {
    /// Particle mass in mass units
    pub mass: f32,
    /// Particle radius in distance units
    pub radius: f32,
    pub damping: f32,
}

pub static SPECIES_PROPERTIES: Lazy<HashMap<Species, SpeciesProps>> = Lazy::new(|| {
    use Species::*;
    let mut m = HashMap::new();
    m.insert(LithiumIon, SpeciesProps { mass: 1.0, radius: 1.0, damping: 1.0 });
    m.insert(LithiumMetal, SpeciesProps { mass: 1.0, radius: 1.0, damping: 1.0 });
    m.insert(FoilMetal, SpeciesProps { mass: 1e6, radius: 1.0, damping: 1.0 });
    m.insert(ElectrolyteAnion, SpeciesProps { mass: 40.0, radius: 1.5, damping: 1.0 });
    m
});

