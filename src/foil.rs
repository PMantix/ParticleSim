use ultraviolet::Vec2;

/// Collection of fixed lithium metal particles representing a foil.
pub struct Foil {
    /// Indices of bodies that belong to this foil within `Simulation::bodies`.
    pub body_indices: Vec<usize>,
    /// Current in electrons per second (positive = source, negative = sink).
    pub current: f32,
    /// Internal accumulator used to emit/remove fractional electrons per step.
    pub accum: f32,
    /// Foil dimensions for reference.
    pub width: f32,
    pub height: f32,
    pub origin: Vec2,
}

impl Foil {
    pub fn new(body_indices: Vec<usize>, origin: Vec2, width: f32, height: f32, current: f32) -> Self {
        Self {
            body_indices,
            current,
            accum: 0.0,
            width,
            height,
            origin,
        }
    }
}
