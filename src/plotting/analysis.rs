// plotting/analysis.rs
// Data analysis functions for the plotting system

use crate::body::{Body, Species};
use crate::body::foil::Foil;
use ultraviolet::Vec2;
use std::collections::HashMap;

/// Calculate concentration map of particles in a 2D grid
pub fn calculate_concentration_map(bodies: &[Body], species: Species, bounds: f32, grid_size: usize) -> Vec<Vec<f32>> {
    let mut grid = vec![vec![0.0; grid_size]; grid_size];
    let cell_size = (2.0 * bounds) / grid_size as f32;
    
    for body in bodies {
        if body.species == species {
            let x_idx = ((body.pos.x + bounds) / cell_size) as usize;
            let y_idx = ((body.pos.y + bounds) / cell_size) as usize;
            
            if x_idx < grid_size && y_idx < grid_size {
                grid[y_idx][x_idx] += 1.0;
            }
        }
    }
    
    // Normalize by cell area
    let cell_area = cell_size * cell_size;
    for row in &mut grid {
        for cell in row {
            *cell /= cell_area;
        }
    }
    
    grid
}

/// Calculate species population counts
pub fn calculate_species_populations(bodies: &[Body]) -> HashMap<Species, usize> {
    let mut populations = HashMap::new();
    
    for body in bodies {
        *populations.entry(body.species).or_insert(0) += 1;
    }
    
    populations
}

/// Calculate charge distribution along an axis
pub fn calculate_charge_distribution(bodies: &[Body], axis_is_x: bool, bounds: f32, bins: usize) -> Vec<f32> {
    let mut bin_charges = vec![0.0; bins];
    let bin_size = (2.0 * bounds) / bins as f32;
    
    for body in bodies {
        let position = if axis_is_x { body.pos.x } else { body.pos.y };
        let bin_idx = ((position + bounds) / bin_size) as usize;
        
        if bin_idx < bins {
            bin_charges[bin_idx] += body.charge;
        }
    }
    
    bin_charges
}

/// Calculate mean velocity along spatial bins
pub fn calculate_velocity_profile(bodies: &[Body], axis_is_x: bool, bounds: f32, bins: usize) -> (Vec<f32>, Vec<f32>) {
    let mut bin_velocities = vec![0.0; bins];
    let mut bin_counts = vec![0; bins];
    let bin_size = (2.0 * bounds) / bins as f32;
    
    for body in bodies {
        let position = if axis_is_x { body.pos.x } else { body.pos.y };
        let velocity = if axis_is_x { body.vel.x } else { body.vel.y };
        let bin_idx = ((position + bounds) / bin_size) as usize;
        
        if bin_idx < bins {
            bin_velocities[bin_idx] += velocity;
            bin_counts[bin_idx] += 1;
        }
    }
    
    // Calculate mean velocities
    let mut mean_velocities = vec![0.0; bins];
    let mut bin_positions = vec![0.0; bins];
    
    for i in 0..bins {
        bin_positions[i] = -bounds + (i as f32 + 0.5) * bin_size;
        mean_velocities[i] = if bin_counts[i] > 0 {
            bin_velocities[i] / bin_counts[i] as f32
        } else {
            0.0
        };
    }
    
    (bin_positions, mean_velocities)
}

/// Track electron hop rate between bodies
pub fn calculate_electron_hop_rate(bodies: &[Body], dt: f32) -> f32 {
    // This would require tracking electron transfer events
    // For now, return a placeholder based on electron counts
    let total_electrons: usize = bodies.iter().map(|b| b.electrons.len()).sum();
    total_electrons as f32 / (bodies.len().max(1) as f32 * dt)
}

/// Calculate field strength at position
pub fn calculate_local_field_strength(pos: Vec2, bodies: &[Body]) -> f32 {
    let mut field = Vec2::zero();
    const K: f32 = 8.99e9; // Coulomb constant (simplified)
    
    for body in bodies {
        let r = pos - body.pos;
        let r_mag_sq = r.mag_sq();
        if r_mag_sq > 1e-6 { // Avoid division by zero
            let field_mag = K * body.charge / r_mag_sq;
            field += r.normalized() * field_mag;
        }
    }
    
    field.mag()
}

/// Calculate current analysis comparing command vs actual electron flow
pub fn calculate_current_analysis(foils: &[Foil], bodies: &[Body], dt: f32) -> HashMap<u64, (f32, f32)> {
    let mut analysis = HashMap::new();
    
    for foil in foils {
        let command_current = foil.current;
        
        // Calculate actual electron flow (simplified)
        // This would require tracking electron movements across foil boundaries
        let actual_current = estimate_actual_current(foil, bodies, dt);
        
        analysis.insert(foil.id, (command_current, actual_current));
    }
    
    analysis
}

fn estimate_actual_current(foil: &Foil, bodies: &[Body], _dt: f32) -> f32 {
    // Simplified estimation based on foil bodies' electron content
    // Find bodies that belong to this foil
    let foil_bodies: Vec<_> = bodies.iter()
        .filter(|body| foil.body_ids.contains(&body.id))
        .collect();
    
    if foil_bodies.is_empty() {
        return 0.0;
    }
    
    // Calculate average position of foil bodies
    let foil_center = foil_bodies.iter()
        .fold(Vec2::zero(), |acc, body| acc + body.pos) / foil_bodies.len() as f32;
    
    // Use average particle radius as size estimate
    let avg_radius = foil_bodies.iter()
        .map(|body| body.radius)
        .sum::<f32>() / foil_bodies.len() as f32;
    
    let foil_size = avg_radius * 2.0;
    
    let nearby_electrons: usize = bodies.iter()
        .filter(|body| (body.pos - foil_center).mag() < foil_size)
        .map(|body| body.electrons.len())
        .sum();
    
    nearby_electrons as f32 * 0.1 // Placeholder conversion factor
}
