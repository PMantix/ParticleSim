// body/types.rs
// Contains the Species enum, Body struct, and related methods (except electron and redox logic)

use ultraviolet::Vec2;
use crate::config;
use super::electron::Electron;
use crate::species::SpeciesProps;
use smallvec::SmallVec;
use serde::{Serialize, Deserialize};
use std::hash::Hash;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum Species {
    LithiumIon,
    LithiumMetal,
    FoilMetal, // NEW
    ElectrolyteAnion,
    EC,
    DMC,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Body {
    pub pos: Vec2,
    pub z: f32,
    pub vel: Vec2,
    pub vz: f32,
    pub acc: Vec2,
    pub az: f32,
    pub mass: f32,
    pub radius: f32,
    pub charge: f32,
    pub id: u64,
    pub species: Species,
    pub electrons: SmallVec<[Electron; 2]>,
    pub e_field: Vec2,
    pub surrounded_by_metal: bool,
    pub last_surround_pos: Vec2,
    pub last_surround_frame: usize,
}

use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Body {
    pub fn new(pos: Vec2, vel: Vec2, mass: f32, radius: f32, charge: f32, species: Species) -> Self {
        // Remove automatic fixed for FoilMetal
        Self {
            pos,
            z: 0.0,
            vel,
            vz: 0.0,
            acc: Vec2::zero(),
            az: 0.0,
            mass,
            radius,
            charge,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            species,
            electrons: SmallVec::new(),
            e_field: Vec2::zero(),
            surrounded_by_metal: false,
            last_surround_pos: pos,
            last_surround_frame: 0,
        }
    }

    #[allow(dead_code)]
    /// Create a new Body using species properties for mass and radius
    pub fn new_from_species(pos: Vec2, vel: Vec2, charge: f32, species: Species) -> Self {
        Self::new(
            pos,
            vel,
            species.mass(),
            species.radius(),
            charge,
            species
        )
    }
    pub fn update_species(&mut self) {
        if matches!(
            self.species,
            Species::FoilMetal | Species::ElectrolyteAnion | Species::EC | Species::DMC
        ) {
            // Don't auto-convert FoilMetal, Anions, or solvent molecules
            return;
        }
        
        let old_species = self.species;
        
        if self.charge > config::LITHIUM_ION_THRESHOLD {
            self.species = Species::LithiumIon;
        } else if self.charge <= 0.0 {
            self.species = Species::LithiumMetal;
        }
        
        // Update radius if species changed
        if old_species != self.species {
            self.radius = self.species.radius();
        }
    }

    pub fn neutral_electron_count(&self) -> usize {
        match self.species {
            Species::LithiumMetal | Species::LithiumIon => crate::config::LITHIUM_METAL_NEUTRAL_ELECTRONS,
            Species::FoilMetal => crate::config::FOIL_NEUTRAL_ELECTRONS,
            Species::ElectrolyteAnion => crate::config::ELECTROLYTE_ANION_NEUTRAL_ELECTRONS,
            Species::EC => crate::config::EC_NEUTRAL_ELECTRONS,
            Species::DMC => crate::config::DMC_NEUTRAL_ELECTRONS,
        }
    }

    /// Count nearby metal neighbors (LithiumMetal or FoilMetal) within
    /// `radius`. Uses a cell list for dense systems and the quadtree
    /// otherwise, mirroring the Lennard-Jones force logic.
    #[allow(dead_code)]
    pub fn metal_neighbor_count(
        &self,
        bodies: &[Body],
        quadtree: &crate::quadtree::Quadtree,
        cell_list: &crate::cell_list::CellList,
        radius: f32,
        density_threshold: f32,
    ) -> usize {
        let area = (2.0 * cell_list.bounds) * (2.0 * cell_list.bounds);
        let density = bodies.len() as f32 / area;
        let use_cell = density > density_threshold;

        let idx = bodies.iter().position(|b| b.id == self.id);
        if let Some(i) = idx {
            if use_cell {
                cell_list.metal_neighbor_count(bodies, i, radius)
            } else {
                quadtree
                    .find_neighbors_within(bodies, i, radius)
                    .into_iter()
                    .filter(|&n| matches!(bodies[n].species, Species::LithiumMetal | Species::FoilMetal))
                    .count()
            }
        } else {
            0
        }
    }

    /// Update the `surrounded_by_metal` flag if enough neighbors are nearby.
    /// The check is skipped unless the body moved farther than
    /// `SURROUND_MOVE_THRESHOLD` since the last update or more than
    /// `SURROUND_CHECK_INTERVAL` frames elapsed.
    pub fn maybe_update_surrounded(
        &mut self,
        index: usize,
        bodies: &[Body],
        quadtree: &crate::quadtree::Quadtree,
        cell_list: &crate::cell_list::CellList,
        use_cell: bool,
        frame: usize,
    ) {
        let moved = (self.pos - self.last_surround_pos).mag()
            > config::SURROUND_MOVE_THRESHOLD * self.radius;
        if moved || frame - self.last_surround_frame >= config::SURROUND_CHECK_INTERVAL {
            let radius = self.radius * config::SURROUND_RADIUS_FACTOR;
            let count = if use_cell {
                cell_list.metal_neighbor_count(bodies, index, radius)
            } else {
                quadtree
                    .find_neighbors_within(bodies, index, radius)
                    .into_iter()
                    .filter(|&n| matches!(bodies[n].species, Species::LithiumMetal | Species::FoilMetal))
                    .count()
            };
            self.surrounded_by_metal = count >= config::SURROUND_NEIGHBOR_THRESHOLD;
            self.last_surround_pos = self.pos;
            self.last_surround_frame = frame;
        }
    }
}

impl Species {
    fn props(&self) -> SpeciesProps {
        crate::species::get_species_props(*self)
    }

    pub fn mass(&self) -> f32 {
        self.props().mass
    }

    pub fn radius(&self) -> f32 {
        self.props().radius
    }

    pub fn damping(&self) -> f32 {
        self.props().damping
    }

    pub fn color(&self) -> [u8; 4] {
        self.props().color
    }

    pub fn lj_enabled(&self) -> bool {
        self.props().lj_enabled
    }

    pub fn lj_epsilon(&self) -> f32 {
        self.props().lj_epsilon
    }

    pub fn lj_sigma(&self) -> f32 {
        self.props().lj_sigma
    }

    pub fn lj_cutoff(&self) -> f32 {
        self.props().lj_cutoff
    }

    pub fn polar_offset(&self) -> f32 {
        self.props().polar_offset
    }

    pub fn polar_charge(&self) -> f32 {
        self.props().polar_charge
    }

    pub fn repulsion_enabled(&self) -> bool {
        self.props().enable_repulsion
    }

    pub fn repulsion_strength(&self) -> f32 {
        self.props().repulsion_strength
    }

    pub fn repulsion_cutoff(&self) -> f32 {
        self.props().repulsion_cutoff
    }
}
