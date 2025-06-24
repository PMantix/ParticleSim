// body/types.rs
// Contains the Species enum, Body struct, and related methods (except electron and redox logic)

use ultraviolet::Vec2;
use crate::config;
use super::electron::Electron;
use smallvec::SmallVec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    LithiumIon,
    LithiumMetal,
    FoilMetal, // NEW
    ElectrolyteAnion,
}

#[derive(Clone)]
#[derive(Debug)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    pub acc: Vec2,
    pub mass: f32,
    pub radius: f32,
    pub charge: f32,
    pub id: u64,
    pub species: Species,
    pub electrons: SmallVec<[Electron; 2]>,
    pub e_field: Vec2,
}

use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Body {
    pub fn new(pos: Vec2, vel: Vec2, mass: f32, radius: f32, charge: f32, species: Species) -> Self {
        // Remove automatic fixed for FoilMetal
        Self {
            pos,
            vel,
            acc: Vec2::zero(),
            mass,
            radius,
            charge,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            species,
            electrons: SmallVec::new(),
            e_field: Vec2::zero(),
        }
    }
    pub fn update_species(&mut self) {
        if self.species == Species::FoilMetal || self.species == Species::ElectrolyteAnion {
            // Don't auto-convert FoilMetal or ElectrolyteAnion to other species
            return;
        }
        if self.charge > config::LITHIUM_ION_THRESHOLD {
            self.species = Species::LithiumIon;
        } else if self.charge <= 0.0 {
            self.species = Species::LithiumMetal;
        }
    }

    pub fn neutral_electron_count(&self) -> usize {
        match self.species {
            Species::LithiumMetal => crate::config::LITHIUM_METAL_NEUTRAL_ELECTRONS,
            Species::FoilMetal => crate::config::FOIL_NEUTRAL_ELECTRONS,
            Species::ElectrolyteAnion => crate::config::ELECTROLYTE_ANION_NEUTRAL_ELECTRONS,
            _ => 0, // Ions and others have 0 neutral electrons
        }
    }

    /// Count nearby metal neighbors (LithiumMetal or FoilMetal) within `radius`
    /// using the provided quadtree.
    pub fn metal_neighbor_count(
        &self,
        bodies: &[Body],
        quadtree: &crate::quadtree::Quadtree,
        radius: f32,
        buffer: &mut Vec<usize>,
    ) -> usize {
        let idx = bodies.iter().position(|b| b.id == self.id);
        if let Some(i) = idx {
            quadtree.find_neighbors_within(bodies, i, radius, buffer);
            buffer
                .iter()
                .filter(|&&n| matches!(bodies[n].species, Species::LithiumMetal | Species::FoilMetal))
                .count()
        } else {
            0
        }
    }
}
