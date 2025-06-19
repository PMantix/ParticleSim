use ultraviolet::Vec2;

/// Collection of fixed lithium metal particles representing a foil.
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Foil {
    /// Unique identifier for this foil.
    pub id: u64,
    /// Unique IDs of bodies that belong to this foil within `Simulation::bodies`.
    pub body_ids: Vec<u64>,
    /// Current in electrons per second (positive = source, negative = sink).
    pub current: f32,
    /// Internal accumulator used to emit/remove fractional electrons per step.
    pub accum: f32,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Foil {
    pub fn new(body_ids: Vec<u64>, _origin: Vec2, _width: f32, _height: f32, current: f32) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            body_ids,
            current,
            accum: 0.0,
        }
    }
}
