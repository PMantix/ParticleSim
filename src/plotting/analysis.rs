// plotting/analysis.rs
// Data analysis functions for the plotting system

use crate::body::{Body, Species};
use ultraviolet::Vec2;
use std::collections::HashMap;

/// Calculate concentration map of particles in a 2D grid
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
    
    for body in bodies {
        let position = if axis_is_x { body.pos.x } else { body.pos.y };
        // Fix binning calculation with proper bounds checking
        let normalized_pos = (position + bounds) / (2.0 * bounds);
        let bin_idx_f = normalized_pos * bins as f32;
        
        // Clamp to valid range and convert to usize
        if bin_idx_f >= 0.0 && bin_idx_f < bins as f32 {
            let bin_idx = bin_idx_f.floor() as usize;
            if bin_idx < bins {
                bin_charges[bin_idx] += body.charge;
            }
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
        // Fix binning calculation with proper bounds checking
        let normalized_pos = (position + bounds) / (2.0 * bounds);
        let bin_idx_f = normalized_pos * bins as f32;
        
        // Clamp to valid range and convert to usize
        if bin_idx_f >= 0.0 && bin_idx_f < bins as f32 {
            let bin_idx = bin_idx_f.floor() as usize;
            if bin_idx < bins {
                bin_velocities[bin_idx] += velocity;
                bin_counts[bin_idx] += 1;
            }
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
#[allow(dead_code)]
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

/// Calculate field strength distribution along an axis
pub fn calculate_field_strength_distribution(bodies: &[Body], axis_is_x: bool, bounds: f32, bins: usize) -> Vec<f32> {
    let mut field_strengths = vec![0.0; bins];
    let bin_size = (2.0 * bounds) / bins as f32;
    
    for i in 0..bins {
        let pos_along_axis = -bounds + (i as f32 + 0.5) * bin_size;
        
        // Sample field strength at several points along the perpendicular axis
        let mut avg_field = 0.0;
        let sample_points = 5;
        
        for j in 0..sample_points {
            let pos_perp = -bounds + (j as f32 / (sample_points - 1) as f32) * (2.0 * bounds);
            
            let sample_pos = if axis_is_x {
                Vec2::new(pos_along_axis, pos_perp)
            } else {
                Vec2::new(pos_perp, pos_along_axis)
            };
            
            avg_field += calculate_local_field_strength(sample_pos, bodies);
        }
        
        field_strengths[i] = avg_field / sample_points as f32;
    }
    
    field_strengths
}
