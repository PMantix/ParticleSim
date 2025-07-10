use ultraviolet::Vec2;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Serialize, Deserialize};

//static NEXT_FOIL_ID: AtomicU64 = AtomicU64::new(1);

/// Mode describing how currents are linked between foils.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkMode {
    /// Currents have the same sign and magnitude.
    Parallel,
    /// Currents have opposite sign but equal magnitude.
    Opposite,
}

/// Collection of fixed lithium metal particles representing a foil.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Foil {
    /// Unique identifier for this foil.
    pub id: u64,
    /// Unique IDs of bodies that belong to this foil within `Simulation::bodies`.
    pub body_ids: Vec<u64>,
    /// DC component of current (constant base current).
    pub dc_current: f32,
    /// AC component of current (amplitude of oscillating current).
    pub ac_current: f32,
    /// Internal accumulator used to emit/remove fractional electrons per step.
    pub accum: f32,
    /// Frequency in Hz for toggling the current on/off. `0.0` disables switching.
    pub switch_hz: f32,
    /// Identifier of a linked foil, if any.
    pub link_id: Option<u64>,
    /// Link mode describing how the currents are related.
    pub mode: LinkMode,
}

impl Foil {
    pub fn new(
        body_ids: Vec<u64>,
        _origin: Vec2,
        _width: f32,
        _height: f32,
        current: f32,
        _switch_hz: f32,
    ) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            body_ids,
            dc_current: current, // Initialize DC current to the provided current
            ac_current: 0.0,     // No AC component by default
            accum: 0.0,
            switch_hz: 0.0,
            link_id: None,
            mode: LinkMode::Parallel,
        }
    }
}
