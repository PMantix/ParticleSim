// Lightweight data structures for compressed history
// Separated to reduce compilation overhead from heavy serde derives

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ultraviolet::Vec2;

use crate::body::{Body, Species};
use crate::body::foil::Foil;
use crate::config::SimConfig;

/// Lightweight body representation that excludes computed values
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightBody {
    pub pos: Vec2,
    pub z: f32,
    pub vel: Vec2, 
    pub vz: f32,
    pub mass: f32,
    pub charge: f32,
    pub species: Species,
    pub radius: f32,
    pub electrons: Vec<crate::body::Electron>,
}

/// Lightweight foil representation that excludes PID history
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightFoil {
    pub id: u64,
    pub body_ids: Vec<u64>,
    pub dc_current: f32,
    pub ac_current: f32,
    pub switch_hz: f32,
    pub link_id: Option<u64>,
    pub mode: crate::body::foil::LinkMode,
    pub charging_mode: crate::body::foil::ChargingMode,
    pub slave_overpotential_current: f32,
}

/// Complete lightweight snapshot of simulation state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightSnapshot {
    pub frame: usize,
    pub sim_time: f32,
    pub dt: f32,
    pub last_thermostat_time: f32,
    pub bodies: Vec<LightBody>,
    pub foils: Vec<LightFoil>,
    pub body_to_foil: HashMap<u64, u64>,
    pub config: SimConfig,
    pub domain_width: f32,
    pub domain_height: f32,
    pub domain_depth: f32,
}

/// Full simulation snapshot (alias for backward compatibility)
pub type SimulationSnapshot = LightSnapshot;

/// Trait for types that can be converted from a simulation snapshot
pub trait FromSnapshot<T> {
    fn from_snapshot(snapshot: &T) -> Self;
}

/// Trait for types that can be converted to a simulation snapshot
pub trait ToSnapshot<T> {
    fn to_snapshot(&self) -> T;
}

/// Change detection for bodies
#[allow(dead_code)] // Will be used by CompressedHistorySystem
impl LightBody {
    #[inline]
    pub fn has_changed(&self, other: &LightBody, thresholds: &super::ChangeThresholds) -> bool {
        (self.pos - other.pos).mag() > thresholds.position_epsilon ||
        (self.vel - other.vel).mag() > thresholds.velocity_epsilon ||
        (self.charge - other.charge).abs() > thresholds.charge_epsilon ||
        (self.z - other.z).abs() > thresholds.z_epsilon ||
        self.electrons.len() != other.electrons.len() ||
        self.electrons.iter().zip(other.electrons.iter()).any(|(a, b)| {
            (a.rel_pos - b.rel_pos).mag() > thresholds.electron_epsilon
        })
    }

    pub fn compute_delta(&self, previous: &LightBody) -> Option<super::BodyDelta> {
        // Implementation moved to delta.rs to keep this module focused
        super::delta::compute_body_delta(self, previous)
    }
}

/// Change detection for foils
#[allow(dead_code)] // Will be used by CompressedHistorySystem
impl LightFoil {
    #[inline]
    pub fn has_changed(&self, other: &LightFoil) -> bool {
        self.dc_current != other.dc_current ||
        self.ac_current != other.ac_current ||
        self.switch_hz != other.switch_hz ||
        self.link_id != other.link_id ||
        self.slave_overpotential_current != other.slave_overpotential_current ||
        self.body_ids != other.body_ids
    }

    pub fn compute_delta(&self, previous: &LightFoil) -> Option<super::FoilDelta> {
        // Implementation moved to delta.rs to keep this module focused
        super::delta::compute_foil_delta(self, previous)
    }
}