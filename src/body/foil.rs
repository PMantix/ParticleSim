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
    /// Current in electrons per second (positive = source, negative = sink).
    pub current: f32,
    /// Internal accumulator used to emit/remove fractional electrons per step.
    pub accum: f32,
    /// Identifier of a linked foil, if any.
    pub link_id: Option<u64>,
    /// Link mode describing how the currents are related.
    pub mode: LinkMode,
    /// Frequency in Hz to toggle the foil current on/off.
    /// `0.0` means the foil is always on.
    pub switch_hz: f32,
}

impl Foil {
    pub fn new(
        body_ids: Vec<u64>,
        _origin: Vec2,
        _width: f32,
        _height: f32,
        current: f32,
        switch_hz: f32,
    ) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            body_ids,
            current,
            accum: 0.0,
            link_id: None,
            mode: LinkMode::Parallel,
            switch_hz,
        }
    }
}
