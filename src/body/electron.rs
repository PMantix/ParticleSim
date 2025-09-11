// body/electron.rs
// Contains the Electron struct and electron-related methods for Body

use ultraviolet::Vec2;
use crate::config;
use crate::profile_scope;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Electron {
    pub rel_pos: Vec2,
    pub vel: Vec2,
}

use super::types::Body;
use crate::quadtree::Quadtree;

impl Body {
    pub fn update_electrons(
        &mut self,
        bodies: &[Body],
        quadtree: &Quadtree,
        background_field: Vec2,
        dt: f32,
        coulomb_constant: f32,
    ) {
        profile_scope!("electron_updates");
        let k = config::electron_spring_k(self.species);
        for e in &mut self.electrons {
            let electron_pos = self.pos + e.rel_pos;
            let local_field =
                quadtree.field_at_point(bodies, electron_pos, coulomb_constant)
                    + background_field;
            let acc = -local_field * k;
            e.vel += acc * dt;
            let speed = e.vel.mag();
            let max_speed = config::ELECTRON_MAX_SPEED_FACTOR * self.radius / dt;
            if speed > max_speed {
                e.vel = e.vel / speed * max_speed;
            }
            e.rel_pos += e.vel * dt;
            let max_dist = self.species.polar_offset() * self.radius;
            if e.rel_pos.mag() > max_dist {
                e.rel_pos = e.rel_pos.normalized() * max_dist;
            }
        }
    }
    pub fn _set_electron_count(&mut self) {
        if self.species == super::types::Species::LithiumMetal {
            let desired = 1 + (-self.charge).round() as usize;
            while self.electrons.len() < desired {
                let angle = fastrand::f32() * std::f32::consts::TAU;
                let rel_pos = Vec2::new(angle.cos(), angle.sin()) * self.radius * self.species.polar_offset();
                self.electrons.push(Electron { rel_pos, vel: Vec2::zero() });
            }
            while self.electrons.len() > desired {
                self.electrons.pop();
            }
        } else {
            self.electrons.clear();
        }
    }
}
