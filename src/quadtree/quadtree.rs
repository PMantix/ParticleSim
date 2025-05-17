use super::node::Node;
use super::quad::Quad;
use crate::body::Body;
use ultraviolet::Vec2;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ops::Range;
use rayon::prelude::*;
use crate::partition::Partition;

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

        let mut split = [range.start, 0, 0, 0, range.end];

        let predicate = |body: &Body| body.pos.y < center.y;
        split[2] = split[0] + bodies[split[0]..split[4]].partition(predicate);

        let predicate = |body: &Body| body.pos.x < center.x;
        split[1] = split[0] + bodies[split[0]..split[2]].partition(predicate);
        split[3] = split[2] + bodies[split[2]..split[4]].partition(predicate);

        let len = self.atomic_len.fetch_add(1, Ordering::Relaxed);
        let children = len * 4 + 1;
        self.parents[len] = node;
        self.nodes[node].children = children;

        let nexts = [
            children + 1,
            children + 2,
            children + 3,
            self.nodes[node].next,
        ];
        let quads = self.nodes[node].quad.subdivide();
        for i in 0..4 {
            let bodies = split[i]..split[i + 1];
            self.nodes[children + i] = Node::new(nexts[i], quads[i], bodies);
        }

        children
    }

    pub fn propagate(&mut self) {
        let len = self.atomic_len.load(Ordering::Relaxed);
        for &node in self.parents[..len].iter().rev() {
            let i = self.nodes[node].children;

            self.nodes[node].pos = self.nodes[i].pos
                + self.nodes[i + 1].pos
                + self.nodes[i + 2].pos
                + self.nodes[i + 3].pos;

            self.nodes[node].mass = self.nodes[i].mass
                + self.nodes[i + 1].mass
                + self.nodes[i + 2].mass
                + self.nodes[i + 3].mass;

            self.nodes[node].charge = self.nodes[i].charge
                + self.nodes[i + 1].charge
                + self.nodes[i + 2].charge
                + self.nodes[i + 3].charge;
        }
        self.nodes[0..len * 4 + 1].par_iter_mut().for_each(|node| {
            node.pos /= node.mass.max(f32::MIN_POSITIVE);
        });
    }

    pub fn build(&mut self, bodies: &mut [Body]) {
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
                                weighted_pos += body.pos * body.mass;
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

        self.propagate();
    }

    pub fn acc_pos(&self, pos: Vec2, q: f32, bodies: &[Body], k_e: f32) -> Vec2 {
        let mut acc = Vec2::zero();

        let mut node = Self::ROOT;
        loop {
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

    pub fn acc(&self, bodies: &mut Vec<Body>, k_e: f32) {
        let bodies_ptr = std::ptr::addr_of_mut!(*bodies) as usize;

        bodies.par_iter_mut().for_each(|body| {
            let bodies = unsafe { &*(bodies_ptr as *const Vec<Body>) };
            body.acc = self.acc_pos(body.pos, body.charge, bodies, k_e);
        });
    }
}