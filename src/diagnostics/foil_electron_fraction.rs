use std::collections::{HashMap, HashSet, VecDeque};

use crate::body::{Body, Species, foil::Foil};
use crate::quadtree::Quadtree;

/// Diagnostic calculating the ratio of actual electrons to neutral electrons
/// for each foil and connected metal cluster.
#[derive(Default)]
pub struct FoilElectronFractionDiagnostic {
    pub fractions: HashMap<u64, f32>,
}

impl FoilElectronFractionDiagnostic {
    pub fn new() -> Self {
        Self { fractions: HashMap::new() }
    }

    /// Recompute electron fractions for all foils using quadtree for efficient neighbor search.
    pub fn calculate(&mut self, bodies: &[Body], foils: &[Foil], quadtree: &Quadtree) {
        self.fractions.clear();
        let id_to_index: HashMap<u64, usize> = bodies
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();

        for foil in foils {
            let mut queue = VecDeque::new();
            let mut visited: HashSet<usize> = HashSet::new();
            let mut total_electrons = 0usize;
            let mut total_neutral = 0usize;

            // Start BFS from all foil bodies
            for id in &foil.body_ids {
                if let Some(&idx) = id_to_index.get(id) {
                    queue.push_back(idx);
                    visited.insert(idx);
                }
            }

            // BFS to find all connected metal bodies using quadtree for neighbor search
            while let Some(idx) = queue.pop_front() {
                let body = &bodies[idx];
                total_electrons += body.electrons.len();
                total_neutral += body.neutral_electron_count();
                
                // Use quadtree to efficiently find nearby neighbors
                let search_radius = body.radius * 2.2; // Slightly larger than connection threshold
                let nearby_indices = quadtree.find_neighbors_within(bodies, idx, search_radius);
                
                for &neighbor_idx in &nearby_indices {
                    if visited.contains(&neighbor_idx) {
                        continue;
                    }
                    
                    let neighbor = &bodies[neighbor_idx];
                    if !matches!(neighbor.species, Species::LithiumMetal | Species::FoilMetal) {
                        continue;
                    }
                    
                    // Check actual connection threshold
                    let threshold = (body.radius + neighbor.radius) * 1.1;
                    if (body.pos - neighbor.pos).mag() <= threshold {
                        visited.insert(neighbor_idx);
                        queue.push_back(neighbor_idx);
                    }
                }
            }

            if total_neutral > 0 {
                self.fractions
                    .insert(foil.id, total_electrons as f32 / total_neutral as f32);
            }
        }
    }
    
    /// More efficient version that only recalculates if enough time has passed
    /// to avoid performance issues during rendering
    pub fn calculate_if_needed(&mut self, bodies: &[Body], foils: &[Foil], quadtree: &Quadtree, current_time: f32, min_interval: f32) -> bool {
        static mut LAST_CALCULATION_TIME: f32 = 0.0;
        
        let time_since_last = unsafe { 
            let elapsed = current_time - LAST_CALCULATION_TIME;
            if elapsed >= min_interval {
                LAST_CALCULATION_TIME = current_time;
                true
            } else {
                false
            }
        };
        
        if time_since_last {
            self.calculate(bodies, foils, quadtree);
            true
        } else {
            false
        }
    }
}

