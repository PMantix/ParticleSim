use super::node::Node;
use super::quad::Quad;
use crate::body::Body;
use ultraviolet::Vec2;
use std::sync::atomic::{AtomicUsize, Ordering, AtomicU8};
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

        // Prevent infinite subdivision: if all bodies are at (nearly) the same position, quad is too small, or only one body, treat as leaf
        let all_same_pos = bodies[range.clone()]
            .windows(2)
            .all(|w| (w[0].pos - w[1].pos).mag_sq() < 1e-12);
        if all_same_pos || self.nodes[node].quad.size < 1e-6 || range.len() <= 1 {
            return node;
        }

        let mut split = [range.start, 0, 0, 0, range.end];

        let predicate = |body: &Body| body.pos.y < center.y;
        split[2] = split[0] + bodies[split[0]..split[4]].partition(predicate);

        let predicate = |body: &Body| body.pos.x < center.x;
        split[1] = split[0] + bodies[split[0]..split[2]].partition(predicate);
        split[3] = split[2] + bodies[split[2]..split[4]].partition(predicate);

        let prev_len = self.atomic_len.fetch_add(1, Ordering::Relaxed);
        let children = prev_len * 4 + 1;
    // Removed verbose diagnostics; keep function lightweight

        // Ensure enough space for all children nodes and parents with proper bounds checking
        while self.parents.len() <= prev_len {
            let new_cap = (prev_len + 1) * 2;
            self.parents.resize(new_cap, 0); // Double the size to prevent frequent resizing
        }
        self.parents[prev_len] = node;
        self.nodes[node].children = children;

        while self.nodes.len() <= children + 3 {
            let new_cap = (children + 4) * 2;
            self.nodes.resize(new_cap, Node::ZEROED); // Double the size to prevent frequent resizing
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
            //let total_charge = bodies[range.clone()].iter().map(|b| b.charge).sum::<f32>();

            // Use absolute charge for weighting to avoid cancellation issues
            let total_abs_charge = bodies[range.clone()].iter().map(|b| b.charge.abs()).sum::<f32>();
            let weighted_pos = if total_abs_charge > 1e-6 {
                bodies[range.clone()].iter().fold(Vec2::zero(), |acc, b| acc + b.pos * b.charge.abs()) / total_abs_charge
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

    // Per-node claim flags to prevent duplicate subdivision across workers.
    // 0 = unclaimed, 1 = claimed (subdivision in progress or done)
    let claims: Vec<AtomicU8> = (0..new_len).map(|_| AtomicU8::new(0)).collect();
    let claims_ptr = claims.as_ptr() as usize;

        let counter = AtomicUsize::new(0);
        let workers_started = AtomicUsize::new(0);
        let workers_done = AtomicUsize::new(0);
    let _expected_workers = rayon::current_num_threads();
    let _t0 = std::time::Instant::now();
        
        rayon::broadcast(|_| {
            let _w_id = workers_started.fetch_add(1, Ordering::Relaxed);
            let mut stack = Vec::new();
            let quadtree = unsafe { &mut *(quadtree_ptr as *mut Quadtree) };
            let bodies =
                unsafe { std::slice::from_raw_parts_mut(bodies_ptr as *mut Body, bodies_len) };
            let claims = unsafe {
                std::slice::from_raw_parts(claims_ptr as *const AtomicU8, new_len)
            };
            
            let mut idle_iterations = 0;
            const MAX_IDLE_ITERATIONS: usize = 1000; // Prevent infinite loops
            let mut _processed_nodes: usize = 0;
            
            loop {
                let current_counter = counter.load(Ordering::Relaxed);
                
                // Exit condition: all bodies processed OR timeout reached
                if current_counter >= bodies.len() || idle_iterations > MAX_IDLE_ITERATIONS {
                    break;
                }
                
                let mut work_done = false;
                
                // Try to get work from channel
                while let Ok(node) = rx.try_recv() {
                    work_done = true;
                    idle_iterations = 0; // Reset idle counter when work is found
                    
                    #[cfg(feature = "debug_quadtree")]
                    println!("Quadtree::build: processing node {}", node);
                    let range = quadtree.nodes[node].bodies.clone();
                    let len = quadtree.nodes[node].bodies.len();

                    if range.len() >= quadtree.thread_capacity {
                        #[cfg(feature = "debug_quadtree")]
                        println!("Quadtree::build: subdividing node {}", node);
                        // Try to claim this node for subdivision to prevent duplicate work
                        let claimed = claims[node]
                            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                            .is_ok();
                        if claimed {
                            let children = quadtree.subdivide(node, bodies, range.clone());
                            if children != node {
                                for i in 0..4 {
                                    if !quadtree.nodes[children + i].bodies.is_empty() {
                                        tx.send(children + i).unwrap();
                                    }
                                }
                            } else {
                                // Node is a leaf, increment counter
                                counter.fetch_add(len, Ordering::Relaxed);
                            }
                        } else {
                            // Another worker is (or has) subdivided this node; skip duplicating
                            // The winning worker will enqueue children.
                        }
                        _processed_nodes += 1;
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
                            quadtree.nodes[node].pos = if total_charge.abs() > 1e-6 {
                                weighted_pos / total_charge
                            } else {
                                weighted_pos
                            };
                            quadtree.nodes[node].charge = total_charge;
                            _processed_nodes += 1;
                            continue;
                        }
                        #[cfg(feature = "debug_quadtree")]
                        println!("Quadtree::build: subdividing node {} in stack", node);
                        // Claim before subdividing to prevent duplicate subdivision across workers
                        let claimed = claims[node]
                            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                            .is_ok();
                        if claimed {
                            let children = quadtree.subdivide(node, bodies, range);
                            for i in 0..4 {
                                if !quadtree.nodes[children + i].bodies.is_empty() {
                                    stack.push(children + i);
                                }
                            }
                        } else {
                            // Someone else subdivided or is subdividing; if already subdivided, we can follow children
                            let children = quadtree.nodes[node].children;
                            if children != 0 {
                                for i in 0..4 {
                                    if !quadtree.nodes[children + i].bodies.is_empty() {
                                        stack.push(children + i);
                                    }
                                }
                            }
                        }
                        _processed_nodes += 1;
                        // (progress logging removed)
                    }
                }
                
                // If no work was done this iteration, increment idle counter
                if !work_done {
                    idle_iterations += 1;
                    // Small yield to prevent excessive CPU usage during idle periods
                    std::thread::yield_now();
                }
            }
            let _done = workers_done.fetch_add(1, Ordering::Relaxed) + 1;
        });

        self.propagate(bodies);
    }

    pub fn acc_pos(&self, pos: Vec2, q: f32, radius: f32, bodies: &[Body], k_e: f32) -> Vec2 {
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
            let dist = d_sq.sqrt();

            // Node radius approximated as half the quad size
            let node_radius = n.quad.size * 0.5;

            // Barnes-Hut opening criterion using distance minus body radius
            let dist_adj = (dist - radius).max(0.0);

            if n.quad.size * n.quad.size < dist_adj.powi(2) * self.t_sq {
                let min_sep = radius + node_radius;
                let r_eff = dist.max(min_sep);
                let denom = (r_eff * r_eff + self.e_sq) * r_eff;
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
                    let dist = d.mag();
                    let min_sep = radius + body.radius;
                    let r_eff = dist.max(min_sep);
                    let denom = (r_eff * r_eff + self.e_sq) * r_eff;
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
            body.acc = self.acc_pos(body.pos, body.charge, body.radius, bodies, k_e);
        });
    }

    pub fn field(&self, bodies: &mut Vec<Body>, k_e: f32) {
        profile_scope!("quadtree_field");
        let bodies_ptr = std::ptr::addr_of_mut!(*bodies) as usize;

        bodies.par_iter_mut().for_each(|body| {
            let bodies = unsafe { &*(bodies_ptr as *const Vec<Body>) };
            // Use test charge q = 1.0 to get the field
            body.e_field = self.acc_pos(body.pos, 1.0, body.radius, bodies, k_e);
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
        self.acc_pos(pos, 1.0, 0.0, bodies, k_e)
    }
}