use std::ops::Range;
use ultraviolet::Vec2;
use super::quad::Quad;

#[derive(Clone)]
pub struct Node {
    pub children: usize,
    pub next: usize,
    pub pos: Vec2,
    pub mass: f32,
    pub quad: Quad,
    pub bodies: Range<usize>,
    pub charge: f32,
}

impl Node {
    pub const ZEROED: Self = Self {
        children: 0,
        next: 0,
        pos: Vec2 { x: 0.0, y: 0.0 },
        mass: 0.0,
        quad: Quad {
            center: Vec2 { x: 0.0, y: 0.0 },
            size: 0.0,
        },
        bodies: 0..0,
        charge: 0.0,
    };

    pub fn new(next: usize, quad: Quad, bodies: Range<usize>) -> Self {
        Self {
            children: 0,
            next,
            pos: Vec2::zero(),
            mass: 0.0,
            quad,
            bodies,
            charge: 0.0,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children == 0
    }

    pub fn is_branch(&self) -> bool {
        self.children != 0
    }

    pub fn is_empty(&self) -> bool {
        self.mass == 0.0
    }
}