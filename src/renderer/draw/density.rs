use crate::renderer::Renderer;
use crate::body::Species;
use ultraviolet::Vec2;
use rayon::prelude::*;

impl Renderer {
    /// Calculate 2D domain density for selected species across the entire domain.
    /// Returns a grid of density values and the maximum density for normalization.
    pub fn calculate_2d_domain_density(&self, grid_spacing: f32, smoothing: f32) -> (Vec<f32>, f32, usize, usize, Vec2, Vec2) {
        // Use the full domain bounds for the density calculation
        let domain_width = self.domain_width;
        let domain_height = self.domain_height;
        
        // Calculate grid from center outward to cover full domain
        let min = Vec2::new(-domain_width, -domain_height);
        let max = Vec2::new(domain_width, domain_height);

        let nx = ((max.x - min.x) / grid_spacing).ceil() as usize + 1;
        let ny = ((max.y - min.y) / grid_spacing).ceil() as usize + 1;

        let mut samples = vec![0.0f32; nx * ny];
        let max_density = samples
            .par_iter_mut()
            .enumerate()
            .map(|(i, sample)| {
                let ix = i % nx;
                let iy = i / nx;
                let x = min.x + (ix as f32 + 0.5) * grid_spacing;
                let y = min.y + (iy as f32 + 0.5) * grid_spacing;
                let pos = Vec2::new(x, y);
                let mut density = 0.0f32;
                
                // Filter bodies based on selected species
                for body in &self.bodies {
                    if !self.is_species_selected_for_density(&body.species) {
                        continue;
                    }
                    
                    let r = pos - body.pos;
                    let dist2 = r.mag_sq();
                    // Use Gaussian smoothing kernel for density calculation
                    let weight = (-dist2 / (smoothing * smoothing)).exp();
                    density += weight;
                }
                
                *sample = density;
                density
            })
            .reduce(|| 0.0f32, f32::max);

        (samples, max_density, nx, ny, min, max)
    }

    /// Calculate numerical average density for selected species.
    /// Returns (average_density, selected_particle_count, effective_area).
    pub fn calculate_numerical_density(&self) -> (f32, usize, f32) {
        // Count particles of selected species
        let selected_particles: Vec<&crate::body::Body> = self.bodies
            .iter()
            .filter(|body| self.is_species_selected_for_density(&body.species))
            .collect();
        
        let particle_count = selected_particles.len();
        
        if particle_count == 0 {
            return (0.0, 0, 0.0);
        }
        
        if particle_count == 1 {
            // For a single particle, use its cross-sectional area
            let particle = selected_particles[0];
            let area = std::f32::consts::PI * particle.radius * particle.radius;
            return (1.0 / area, 1, area);
        }
        
        // Calculate the effective area occupied by selected species
        // Method 1: Bounding rectangle (simpler, more conservative)
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        
        // Find bounding rectangle of selected particles
        for body in &selected_particles {
            min_x = min_x.min(body.pos.x - body.radius);
            max_x = max_x.max(body.pos.x + body.radius);
            min_y = min_y.min(body.pos.y - body.radius);
            max_y = max_y.max(body.pos.y + body.radius);
        }
        
        // Calculate bounding rectangle area
        let width = max_x - min_x;
        let height = max_y - min_y;
        let bounding_area = width * height;
        
        // Method 2: Convex hull area (more accurate but complex)
        // For now, we'll use a simplified approach: sum of particle cross-sections + buffer
        let total_particle_area: f32 = selected_particles.iter()
            .map(|body| std::f32::consts::PI * body.radius * body.radius)
            .sum();
        
        // Choose the more appropriate area measure
        let effective_area = if bounding_area > total_particle_area * 2.0 {
            // If bounding area is much larger, particles are spread out - use bounding area
            bounding_area
        } else {
            // If particles are densely packed, use a buffer around total particle area
            total_particle_area * 1.5  // 50% buffer for spacing between particles
        };
        
        // Avoid division by zero
        let effective_area = if effective_area > 0.0 { effective_area } else { 1.0 };
        
        // Calculate average density (particles per unit area)
        let average_density = particle_count as f32 / effective_area;
        
        (average_density, particle_count, effective_area)
    }

    /// Check if a species is selected for density calculation based on UI checkboxes.
    pub fn is_species_selected_for_density(&self, species: &Species) -> bool {
        match species {
            Species::LithiumCation => self.density_calc_lithium_cation,
            Species::LithiumMetal => self.density_calc_lithium_metal,
            Species::FoilMetal => self.density_calc_foil_metal,
            Species::Pf6Anion => self.density_calc_pf6_anion,
            Species::EC => self.density_calc_ec,
            Species::DMC => self.density_calc_dmc,
        }
    }

    /// Draw the 2D domain density heatmap.
    pub fn draw_2d_domain_density(&self, ctx: &mut quarkstrom::RenderContext) {
        let grid_spacing = 8.0;  // Slightly larger grid for domain-wide view
        let smoothing = 8.0;     // Slightly more smoothing for cleaner visualization

        let (samples, max_density, nx, ny, min, _max) = self.calculate_2d_domain_density(grid_spacing, smoothing);
        
        // Avoid division by zero
        let max_density = max_density.max(1e-6);

        // Render density grid
        for ix in 0..nx - 1 {
            for iy in 0..ny - 1 {
                let density = samples[iy * nx + ix];
                let normalized_density = (density / max_density).clamp(0.0, 1.0);
                
                // Use a blue-to-red gradient for density visualization
                let r = (normalized_density * 255.0) as u8;
                let g = ((1.0 - normalized_density) * normalized_density * 4.0 * 255.0).min(255.0) as u8; // Peak at middle density
                let b = ((1.0 - normalized_density) * 255.0) as u8;
                let alpha = (normalized_density * 120.0 + 20.0) as u8; // Minimum alpha for visibility
                
                let color = [r, g, b, alpha];

                let rect_min = Vec2::new(
                    min.x + ix as f32 * grid_spacing,
                    min.y + iy as f32 * grid_spacing,
                );
                let rect_max = rect_min + Vec2::new(grid_spacing, grid_spacing);
                ctx.draw_rect(rect_min, rect_max, color);
            }
        }
    }
}