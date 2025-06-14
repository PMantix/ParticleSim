use super::node::Node;
use super::quad::Quad;
use crate::body::Body;
use ultraviolet::Vec2;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ops::Range;
use rayon::prelude::*;
use crate::partition::Partition;
use crate::profile_scope;

pub struct Quadtree {
    pub t_sq: f32,
    pub e_sq: f32,
    pub leaf_capacity: usize,
    pub thread_capacity: usize,
    pub atomic_len: AtomicUsize,
    pub nodes: Vec<Node>,
    pub parents: Vec<usize>,
}

impl Quadtree {
    pub const ROOT: usize = 0;

    pub fn new(theta: f32, epsilon: f32, leaf_capacity: usize, thread_capacity: usize) -> Self {
        Self {
            t_sq: theta * theta,
            e_sq: epsilon * epsilon,
            leaf_capacity,
            thread_capacity,
            atomic_len: 0.into(),
            nodes: Vec::new(),
            parents: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.atomic_len.store(0, Ordering::Relaxed);
    }

    pub fn subdivide(&mut self, node: usize, bodies: &mut [Body], range: Range<usize>) -> usize {
        let center = self.nodes[node].quad.center;

        // Prevent infinite subdivision: if all bodies are at (nearly) the same position or quad is too small, treat as leaf
        let all_same_pos = bodies[range.clone()]
            .windows(2)
            .all(|w| (w[0].pos - w[1].pos).mag_sq() < 1e-12);
        if all_same_pos || self.nodes[node].quad.size < 1e-6 {
            return node;
        }

        let mut split = [range.start, 0, 0, 0, range.end];

        let predicate = |body: &Body| body.pos.y < center.y;
        split[2] = split[0] + bodies[split[0]..split[4]].partition(predicate);

        let predicate = |body: &Body| body.pos.x < center.x;
        split[1] = split[0] + bodies[split[0]..split[2]].partition(predicate);
        split[3] = split[2] + bodies[split[2]..split[4]].partition(predicate);

        let len = self.atomic_len.fetch_add(1, Ordering::Relaxed);
        let children = len * 4 + 1;

        // Ensure enough space for all children nodes and parents
        if self.parents.len() <= len {
            self.parents.resize(len + 1, 0);
        }
        self.parents[len] = node;
        self.nodes[node].children = children;

        if self.nodes.len() <= children + 3 {
            self.nodes.resize(children + 4, Node::ZEROED);
        }

        let nexts = [
            children + 1,
            children + 2,
            children + 3,
            self.nodes[node].next,
        ];
        let quads = self.nodes[node].quad.subdivide();
        for i in 0..4 {
            let bodies_range = split[i]..split[i + 1];
            self.nodes[children + i] = Node::new(nexts[i], quads[i], bodies_range);
        }

        children
    }

    pub fn propagate(&mut self, bodies: &[Body]) {
        let len = self.atomic_len.load(Ordering::Relaxed);
        for &node in self.parents[..len].iter().rev() {
            let i = self.nodes[node].children;

            // Compute charge-weighted, mass-weighted, or geometric center for node position
            let range = self.nodes[node].bodies.clone();
            let total_mass = bodies[range.clone()].iter().map(|b| b.mass).sum::<f32>();
            let total_charge = bodies[range.clone()].iter().map(|b| b.charge).sum::<f32>();

            let weighted_pos = if total_charge.abs() > 1e-6 {
                bodies[range.clone()].iter().fold(Vec2::zero(), |acc, b| acc + b.pos * b.charge) / total_charge
            } else if total_mass > 1e-6 {
                bodies[range.clone()].iter().fold(Vec2::zero(), |acc, b| acc + b.pos * b.mass) / total_mass
            } else if range.len() > 0 {
                bodies[range.clone()].iter().fold(Vec2::zero(), |acc, b| acc + b.pos) / (range.len() as f32)
            } else {
                Vec2::zero()
            };

            self.nodes[node].pos = weighted_pos;
            self.nodes[node].mass = self.nodes[i].mass
                + self.nodes[i + 1].mass
                + self.nodes[i + 2].mass
                + self.nodes[i + 3].mass;
            self.nodes[node].charge = self.nodes[i].charge
                + self.nodes[i + 1].charge
                + self.nodes[i + 2].charge
                + self.nodes[i + 3].charge;
        }
    }

    pub fn build(&mut self, bodies: &mut [Body]) {
        profile_scope!("quadtree_build");
        if bodies.is_empty() {
            self.clear();
            return;
        }

        self.clear();

        let new_len = 4 * bodies.len() + 1024;
        self.nodes.resize(new_len, Node::ZEROED);
        self.parents.resize(new_len / 4, 0);

        let quad = Quad::new_containing(bodies);
        self.nodes[Self::ROOT] = Node::new(0, quad, 0..bodies.len());

        let (tx, rx) = crossbeam::channel::unbounded();
        tx.send(Self::ROOT).unwrap();

        let quadtree_ptr = self as *mut Quadtree as usize;
        let bodies_ptr = bodies.as_ptr() as usize;
        let bodies_len = bodies.len();

        let counter = AtomicUsize::new(0);
        rayon::broadcast(|_| {
            let mut stack = Vec::new();
            let quadtree = unsafe { &mut *(quadtree_ptr as *mut Quadtree) };
            let bodies =
                unsafe { std::slice::from_raw_parts_mut(bodies_ptr as *mut Body, bodies_len) };

            while counter.load(Ordering::Relaxed) != bodies.len() {
                while let Ok(node) = rx.try_recv() {
                    let range = quadtree.nodes[node].bodies.clone();
                    let len = quadtree.nodes[node].bodies.len();

                    if range.len() >= quadtree.thread_capacity {
                        let children = quadtree.subdivide(node, bodies, range);
                        for i in 0..4 {
                            if !self.nodes[children + i].bodies.is_empty() {
                                tx.send(children + i).unwrap();
                            }
                        }
                        continue;
                    }

                    counter.fetch_add(len, Ordering::Relaxed);

                    stack.push(node);
                    while let Some(node) = stack.pop() {
                        let range = quadtree.nodes[node].bodies.clone();
                        if range.len() <= quadtree.leaf_capacity {
                            let mut total_mass = 0.0;
                            let mut weighted_pos = Vec2::zero();
                            let mut total_charge = 0.0;

                            for body in &bodies[range.clone()] {
                                total_mass += body.mass;
                                weighted_pos += body.pos * body.charge; // charge-weighted
                                total_charge += body.charge;
                            }

                            quadtree.nodes[node].mass = total_mass;
                            quadtree.nodes[node].pos = weighted_pos;
                            quadtree.nodes[node].charge = total_charge;
                            continue;
                        }
                        let children = quadtree.subdivide(node, bodies, range);
                        for i in 0..4 {
                            if !self.nodes[children + i].bodies.is_empty() {
                                stack.push(children + i);
                            }
                        }
                    }
                }
            }
        });

        self.propagate(bodies);
    }

    pub fn acc_pos(&self, pos: Vec2, q: f32, bodies: &[Body], k_e: f32) -> Vec2 {
        let mut acc = Vec2::zero();
        let mut node = Self::ROOT;
        let mut _iter_count = 0;
        loop {
            _iter_count += 1;
            if node >= self.nodes.len() {
                break;
            }
            let n = self.nodes[node].clone();

            let d = pos - n.pos;
            let d_sq = d.mag_sq();

            if n.quad.size * n.quad.size < d_sq * self.t_sq {
                let denom = (d_sq + self.e_sq) * d_sq.sqrt();
                acc += d * (k_e * q * n.charge / denom);

                if n.next == 0 {
                    break;
                }
                node = n.next;
            } else if n.is_leaf() {
                for i in n.bodies {
                    let body = &bodies[i];

                    if (body.pos - pos).mag_sq() < 1e-6 {
                        continue;
                    }

                    let d = pos - body.pos;
                    let d_sq = d.mag_sq();
                    let denom = (d_sq + self.e_sq) * d_sq.sqrt();
                    acc += d * (k_e * q * body.charge / denom).min(f32::MAX);
                }

                if n.next == 0 {
                    break;
                }
                node = n.next;
            } else {
                node = n.children;
            }
        }

        acc
    }

    pub fn _acc(&self, bodies: &mut Vec<Body>, k_e: f32) {
        let bodies_ptr = std::ptr::addr_of_mut!(*bodies) as usize;

        bodies.par_iter_mut().for_each(|body| {
            let bodies = unsafe { &*(bodies_ptr as *const Vec<Body>) };
            body.acc = self.acc_pos(body.pos, body.charge, bodies, k_e);
        });
    }

    pub fn field(&self, bodies: &mut Vec<Body>, k_e: f32) {
        profile_scope!("quadtree_field");
        let bodies_ptr = std::ptr::addr_of_mut!(*bodies) as usize;

        bodies.par_iter_mut().for_each(|body| {
            let bodies = unsafe { &*(bodies_ptr as *const Vec<Body>) };
            // Use test charge q = 1.0 to get the field
            body.e_field = self.acc_pos(body.pos, 1.0, bodies, k_e);
        });
    }

    /// Find indices of bodies within `cutoff` distance of body at index `i` (excluding `i` itself)
    pub fn find_neighbors_within(&self, bodies: &[Body], i: usize, cutoff: f32) -> Vec<usize> {
        profile_scope!("quadtree_neighbors");
        let mut neighbors = Vec::new();
        let pos = bodies[i].pos;
        let cutoff_sq = cutoff * cutoff;

        let mut stack = vec![Self::ROOT];
        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];
            // Compute min squared distance from pos to node's quad
            let quad = &node.quad;
            let half = quad.size * 0.5;
            let min = quad.center - Vec2::one() * half;
            let max = quad.center + Vec2::one() * half;
            let mut d2 = 0.0;
            for k in 0..2 {
                let p = if k == 0 { pos.x } else { pos.y };
                let mn = if k == 0 { min.x } else { min.y };
                let mx = if k == 0 { max.x } else { max.y };
                if p < mn { d2 += (mn - p).powi(2); }
                else if p > mx { d2 += (p - mx).powi(2); }
            }
            if d2 > cutoff_sq {
                continue;
            }
            if node.is_leaf() {
                for idx in node.bodies.clone() {
                    if idx != i && (bodies[idx].pos - pos).mag_sq() < cutoff_sq {
                        neighbors.push(idx);
                    }
                }
            } else {
                for c in 0..4 {
                    stack.push(node.children + c);
                }
            }
        }
        neighbors
    }

    /// Compute the electric field at an arbitrary point using the quadtree (Barnes-Hut).
    pub fn field_at_point(&self, bodies: &[Body], pos: Vec2, k_e: f32) -> Vec2 {
        // This should use the same logic as acc_pos, but with test charge q=1.0
        self.acc_pos(pos, 1.0, bodies, k_e)
    }
}