// body/electron.rs
// Contains the Electron struct and electron-related methods for Body

use ultraviolet::Vec2;
use crate::config;

#[derive(Clone, Debug)]
pub struct Electron {
    pub rel_pos: Vec2,
    pub vel: Vec2,
}

use super::types::Body;

impl Body {
    pub fn update_electrons<F>(&mut self, field_at: F, dt: f32)
    where
        F: Fn(Vec2) -> Vec2,
    {
        let k = config::ELECTRON_SPRING_K;
        for e in &mut self.electrons {
            let electron_pos = self.pos + e.rel_pos;
            let local_field = field_at(electron_pos);
            let acc = -local_field * k;
            e.vel += acc * dt;
            let speed = e.vel.mag();
            let max_speed = config::ELECTRON_MAX_SPEED_FACTOR * self.radius / dt;
            if speed > max_speed {
                e.vel = e.vel / speed * max_speed;
            }
            e.rel_pos += e.vel * dt;
            let max_dist = config::ELECTRON_DRIFT_RADIUS_FACTOR * self.radius;
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
                let rel_pos = Vec2::new(angle.cos(), angle.sin()) * self.radius * config::ELECTRON_DRIFT_RADIUS_FACTOR;
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
