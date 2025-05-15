pub const K_E: f32 = 8.988e2;  // Coulomb's constant
use crate::{body::Body, quadtree::Quadtree, utils};

use broccoli::aabb::Rect;
use broccoli_rayon::{build::RayonBuildPar, prelude::RayonQueryPar};
use ultraviolet::Vec2;

pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub quadtree: Quadtree,
    pub bounds: f32, // half size of the bounding box
    pub rewound_flags: Vec<bool>,
}

impl Simulation {
    pub fn new() -> Self {
        let dt = 0.01;
        let n = 50000;
        let theta = 1.0;
        let epsilon = 2.0;
        let leaf_capacity = 1;
        let thread_capacity = 1024;

        let bounds = 350.0;

        let bodies: Vec<Body> = utils::uniform_disc(n);
        let quadtree = Quadtree::new(theta, epsilon, leaf_capacity, thread_capacity);
        let rewound_flags = vec![false; bodies.len()];

        Self {
            dt,
            frame: 0,
            bodies,
            quadtree,
            bounds,
            rewound_flags,
        }
    }

    pub fn step(&mut self) {
        //reset the rewound flags, allowing particles to be "hit" again
        for flag in &mut self.rewound_flags {
            *flag = false;
        }


        self.dt = *crate::renderer::TIMESTEP.lock();

        self.iterate();
        let num_passes = *crate::renderer::COLLISION_PASSES.lock();
        for _ in 1..num_passes  {
            self.collide();
        }
        self.attract();

        self.frame += 1;
    }

    pub fn attract(&mut self) {
        self.quadtree.build(&mut self.bodies);
        self.quadtree.acc(&mut self.bodies, K_E);
    }

    pub fn iterate(&mut self) {
        for body in &mut self.bodies {
            body.update(self.dt);

            // Reflect from walls
            for axis in 0..2 {
                let pos = if axis == 0 { &mut body.pos.x } else { &mut body.pos.y };
                let vel = if axis == 0 { &mut body.vel.x } else { &mut body.vel.y };

                if *pos < -self.bounds {
                    *pos = -self.bounds;
                    *vel = -(*vel);
                } else if *pos > self.bounds {
                    *pos = self.bounds;
                    *vel = -(*vel);
                }
            }
            
        }
    }

    pub fn collide(&mut self) {
        let mut rects = self
            .bodies
            .iter()
            .enumerate()
            .map(|(index, body)| {
                let pos = body.pos;
                let radius = body.radius;
                let min = pos - Vec2::one() * radius;
                let max = pos + Vec2::one() * radius;
                (Rect::new(min.x, max.x, min.y, max.y), index)
            })
            .collect::<Vec<_>>();

        let mut broccoli = broccoli::Tree::par_new(&mut rects);

        let ptr = self as *mut Self as usize;
        
        let num_passes = *crate::renderer::COLLISION_PASSES.lock();

        broccoli.par_find_colliding_pairs(|i, j| {
            let sim = unsafe { &mut *(ptr as *mut Self) };

            let i = *i.unpack_inner();
            let j = *j.unpack_inner();

            sim.resolve(i, j, num_passes);
        });
    }

    fn resolve(&mut self, i: usize, j: usize, num_passes: usize) {
        let b1 = &self.bodies[i];
        let b2 = &self.bodies[j];

        let p1 = b1.pos;
        let p2 = b2.pos;

        let r1 = b1.radius;
        let r2 = b2.radius;

        let d = p2 - p1;
        let r = r1 + r2;

        if d.mag_sq() > r * r {
            return;
        }

        let v1 = b1.vel;
        let v2 = b2.vel;

        let v = v2 - v1;

        let d_dot_v = d.dot(v);

        let m1 = b1.mass;
        let m2 = b2.mass;

        let weight1 = m2 / (m1 + m2);
        let weight2 = m1 / (m1 + m2);

        if d_dot_v >= 0.0 && d != Vec2::zero() {
            let tmp = d * (r / d.mag() - 1.0);
            self.bodies[i].pos -= weight1 * tmp;
            self.bodies[j].pos += weight2 * tmp;
            return;
        }

        let v_sq = v.mag_sq();
        let d_sq = d.mag_sq();
        let r_sq = r * r;

        let correction_scale = 1.0 / num_passes as f32; // e.g. 0.5 if running 2 passes
        let t = correction_scale * (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt()) / v_sq;

        //let t = (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt()) / v_sq;

        self.bodies[i].pos -= v1 * t;
        self.bodies[j].pos -= v2 * t;

        let p1 = self.bodies[i].pos;
        let p2 = self.bodies[j].pos;
        let d = p2 - p1;
        let d_dot_v = d.dot(v);
        let d_sq = d.mag_sq();

        let tmp = d * (1.5 * d_dot_v / d_sq);
        let v1 = v1 + tmp * weight1;
        let v2 = v2 - tmp * weight2;

        self.bodies[i].vel = v1;
        self.bodies[j].vel = v2;
        self.bodies[i].pos += v1 * t;
        self.bodies[j].pos += v2 * t;

    }

// Soft Spring Collision Code (didn't really work all that well)
//     fn resolve(&mut self, i: usize, j: usize) {
//     let (b1, b2) = {
//         let (left, right) = self.bodies.split_at_mut(j.max(i));
//         if i < j {
//             (&mut left[i], &mut right[0])
//         } else {
//             (&mut right[0], &mut left[j])
//         }
//     };

//     let delta = b2.pos - b1.pos;
//     let dist_sq = delta.mag_sq();
//     let min_dist = b1.radius + b2.radius;

//     // Avoid computing square root unless definitely overlapping
//     if dist_sq >= min_dist * min_dist {
//         return;
//     }

//     let dist = dist_sq.sqrt().max(1e-6); // avoid divide-by-zero
//     let overlap = min_dist - dist;

//     let normal = delta / dist; // direction from b1 to b2

//     // === Spring force ===
//     let stiffness = 200.0; //ne this: how strongly they push apart
//     let force_mag = stiffness * overlap;

//     // === Damping force ===
//     let rel_vel = b1.vel - b2.vel;
//     let damping = 1.0; //tune this too
//     //let damp_mag = rel_vel.dot(normal) * damping;

//     let damping_force = damping * rel_vel.dot(normal).min(0.0); // only damp when approaching


//     //let total_force = normal * (force_mag + damp_mag);
//     let total_force = normal * (force_mag + damping_force);


//     //debug
//     // if overlap > 0.0 {
//     //     println!(
//     //         "Overlap: {:.3}, SpringForce: {:.2}, BeforeAcc: {:?}, AfterAcc: {:?}",
//     //         overlap,
//     //         total_force.mag(),
//     //         b1.acc,
//     //         b1.acc - total_force / b1.mass
//     //     );
//     // }

//     // Apply equal and opposite accelerations
//     b1.acc -= total_force / b1.mass;
//     b2.acc += total_force / b2.mass;

// }

}
