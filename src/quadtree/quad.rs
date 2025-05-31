use ultraviolet::Vec2;
use crate::body::Body;

#[derive(Clone, Copy)]
pub struct Quad {
    pub center: Vec2,
    pub size: f32,
}

impl Quad {
    pub fn new_containing(bodies: &[Body]) -> Self {
        if bodies.is_empty() {
            return Self { center: Vec2::zero(), size: 1.0 };
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for body in bodies {
            min_x = min_x.min(body.pos.x);
            min_y = min_y.min(body.pos.y);
            max_x = max_x.max(body.pos.x);
            max_y = max_y.max(body.pos.y);
        }

        let center = Vec2::new(min_x + max_x, min_y + max_y) * 0.5;
        let size = (max_x - min_x).max(max_y - min_y);

        Self { center, size }
    }

    pub fn into_quadrant(mut self, quadrant: usize) -> Self {
        self.size *= 0.5;
        self.center.x += ((quadrant & 1) as f32 - 0.5) * self.size;
        self.center.y += ((quadrant >> 1) as f32 - 0.5) * self.size;
        self
    }

    pub fn subdivide(&self) -> [Quad; 4] {
        [0, 1, 2, 3].map(|i| self.into_quadrant(i))
    }
}