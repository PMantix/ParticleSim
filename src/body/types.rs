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
        // Validate inputs to prevent NaN/infinite values
        let safe_pos = Vec2::new(
            if pos.x.is_finite() { pos.x } else { 0.0 },
            if pos.y.is_finite() { pos.y } else { 0.0 }
        );
        let safe_vel = Vec2::new(
            if vel.x.is_finite() { vel.x } else { 0.0 },
            if vel.y.is_finite() { vel.y } else { 0.0 }
        );
        let safe_mass = if mass.is_finite() && mass > 0.0 { mass } else { 1.0 };
        let safe_radius = if radius.is_finite() && radius > 0.0 { radius } else { 1.0 };
        let safe_charge = if charge.is_finite() { charge } else { 0.0 };
        
        Self {
            pos: safe_pos,
            z: 0.0,
            vel: safe_vel,
            vz: 0.0,
            acc: Vec2::zero(),
            az: 0.0,
            mass: safe_mass,
            radius: safe_radius,
            charge: safe_charge,
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
        let area = (2.0 * cell_list.domain_width) * (2.0 * cell_list.domain_height);
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

    /// Reset all out-of-plane state to zero, returning the body to the main plane
    pub fn reset_z(&mut self) {
        self.z = 0.0;
        self.vz = 0.0;
        self.az = 0.0;
    }

    /// Clamp the body's z-position within +/- `max_z`, zeroing velocity if clipped
    pub fn clamp_z(&mut self, max_z: f32) {
        if self.z > max_z {
            self.z = max_z;
            self.vz = 0.0;
        } else if self.z < -max_z {
            self.z = -max_z;
            self.vz = 0.0;
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

    /// Surface affinity - how strongly this species is attracted to electrode surfaces
    pub fn surface_affinity(&self) -> f32 {
        use Species::*;
        match self {
            LithiumIon => 2.0,        // Strong attraction to cathode
            ElectrolyteAnion => 1.5,  // Moderate attraction to anode
            EC | DMC => 0.5,          // Weak surface interaction (neutral solvents)
            LithiumMetal => 0.0,      // Already on surface
            FoilMetal => 0.0,         // Already on surface
        }
    }

    /// Preferred z-separation between this species and another
    pub fn preferred_z_separation(&self, other: &Species) -> f32 {
        use Species::*;
        match (self, other) {
            // Ion-solvent solvation shells
            (LithiumIon, EC) | (EC, LithiumIon) => 0.3,
            (LithiumIon, DMC) | (DMC, LithiumIon) => 0.25,
            (ElectrolyteAnion, EC) | (EC, ElectrolyteAnion) => 0.4,
            (ElectrolyteAnion, DMC) | (DMC, ElectrolyteAnion) => 0.35,
            
            // Like-like interactions (layer separation)
            (LithiumIon, LithiumIon) => 0.8,
            (ElectrolyteAnion, ElectrolyteAnion) => 0.9,
            (EC, EC) => 0.6,
            (DMC, DMC) => 0.5,
            
            // Metal particles stay at surface
            (_, LithiumMetal) | (LithiumMetal, _) => 0.0,
            (_, FoilMetal) | (FoilMetal, _) => 0.0,
            
            // Default for other combinations
            _ => 0.4,
        }
    }

    /// Strength of solvation interactions
    pub fn solvation_strength(&self, other: &Species) -> f32 {
        use Species::*;
        match (self, other) {
            // Strong ion-solvent interactions
            (LithiumIon, EC) | (EC, LithiumIon) => 1.5,
            (LithiumIon, DMC) | (DMC, LithiumIon) => 1.2,
            (ElectrolyteAnion, EC) | (EC, ElectrolyteAnion) => 1.0,
            (ElectrolyteAnion, DMC) | (DMC, ElectrolyteAnion) => 0.8,
            
            // Weaker solvent-solvent interactions
            (EC, DMC) | (DMC, EC) => 0.3,
            (EC, EC) => 0.4,
            (DMC, DMC) => 0.3,
            
            // Ion-ion repulsion in z (want separation)
            (LithiumIon, ElectrolyteAnion) | (ElectrolyteAnion, LithiumIon) => 0.2,
            (LithiumIon, LithiumIon) => 0.1,
            (ElectrolyteAnion, ElectrolyteAnion) => 0.1,
            
            // Metals don't participate in solvation
            (_, LithiumMetal) | (LithiumMetal, _) => 0.0,
            (_, FoilMetal) | (FoilMetal, _) => 0.0,
        }
    }
}
