use crate::body::Species;

#[derive(Clone, Copy, Debug)]
pub struct SpeciesProperties {
    pub damping: f32,
}

pub fn get(species: Species) -> SpeciesProperties {
    match species {
        Species::LithiumIon => SpeciesProperties { damping: 0.97 },
        Species::LithiumMetal => SpeciesProperties { damping: 0.98 },
        Species::FoilMetal => SpeciesProperties { damping: 0.98 },
        Species::ElectrolyteAnion => SpeciesProperties { damping: 0.96 },
    }
}

impl Species {
    pub fn damping(&self) -> f32 {
        get(*self).damping
    }
}
