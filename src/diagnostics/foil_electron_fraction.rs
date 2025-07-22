use std::collections::{HashMap, HashSet, VecDeque};

use crate::body::{Body, Species, foil::Foil};

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

    /// Recompute electron fractions for all foils.
    pub fn calculate(&mut self, bodies: &[Body], foils: &[Foil]) {
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

            for id in &foil.body_ids {
                if let Some(&idx) = id_to_index.get(id) {
                    queue.push_back(idx);
                    visited.insert(idx);
                }
            }

            while let Some(idx) = queue.pop_front() {
                let body = &bodies[idx];
                total_electrons += body.electrons.len();
                total_neutral += body.neutral_electron_count();
                for (j, other) in bodies.iter().enumerate() {
                    if visited.contains(&j) {
                        continue;
                    }
                    if !matches!(other.species, Species::LithiumMetal | Species::FoilMetal) {
                        continue;
                    }
                    let threshold = (body.radius + other.radius) * 1.1;
                    if (body.pos - other.pos).mag() <= threshold {
                        visited.insert(j);
                        queue.push_back(j);
                    }
                }
            }

            if total_neutral > 0 {
                self.fractions
                    .insert(foil.id, total_electrons as f32 / total_neutral as f32);
            }
        }
    }
}

