//! Utility functions for generating initial conditions and particle distributions for the simulation.
//!
//! Provides helpers to create uniform discs and clustered lithium/ion arrangements for testing and visualization.

use crate::body::{Body, Species, Electron};
use ultraviolet::Vec2;
use smallvec::smallvec;

/// Generate a uniform disc of `n` bodies (ions/metals) with random positions and velocities.
///
/// - Positions are distributed in a disc of radius proportional to sqrt(n).
/// - Each body is assigned a random charge and species based on its x-position.
/// - Bodies are sorted by distance from the center.
/// - Velocities are scaled by mass and distance.
///
/// Note: This is mainly for legacy or test/demo purposes.
pub fn _uniform_disc(n: usize) -> Vec<Body> {
    fastrand::seed(0);
    let inner_radius = 0.0;
    let outer_radius = (n as f32).sqrt() * 1.50;

    let mut bodies: Vec<Body> = Vec::with_capacity(n);

    while bodies.len() < n {
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let t = inner_radius / outer_radius;
        let r = fastrand::f32() * (1.0 - t * t) + t * t;
        let pos = Vec2::new(cos, sin) * outer_radius * r.sqrt();

        let vel = Vec2::new(0.0, 0.0);
        let mass = 0.5f32;
        let charge = if pos.x < 0.0 { 1.0 } else { -1.0 };

        let species = if charge > 0.5 {
            Species::LithiumCation
        } else if charge <= 0.0 {
            Species::LithiumMetal
        } else {
            Species::LithiumCation
        };

        let radius = species.radius();

        bodies.push(Body::new(pos, vel, mass, radius, charge, species));
    }

    bodies.sort_by(|a, b| a.pos.mag_sq().total_cmp(&b.pos.mag_sq()));
    let mut mass = 0.0;
    for i in 0..n {
        mass += bodies[i].mass;
        if bodies[i].pos == Vec2::zero() {
            continue;
        }

        let v = (mass / bodies[i].pos.mag()).sqrt();
        bodies[i].vel *= v;
    }

    bodies
}

/// Generate two lithium metal clumps (left/right) and a central clump of ions.
///
/// - `clump_size`: Number of metals in each clump.
/// - `clump_radius`: Radius of each clump.
/// - `domain_half_width`: Controls spacing between clumps.
/// - Each metal is initialized with a valence electron.
/// - Ions are placed in the center with no electrons.
///
/// Returns a vector of initialized `Body` objects.
pub fn _two_lithium_clumps_with_ions(
    n: usize,
    clump_size: usize,
    clump_radius: f32,
    domain_half_width: f32,
) -> Vec<Body> {
    fastrand::seed(0);
    let mut bodies = Vec::with_capacity(n);

    // Left clump center
    let left_center = Vec2::new(-domain_half_width * 0.6, 0.0);
    // Right clump center
    let right_center = Vec2::new(domain_half_width * 0.6, 0.0);
    // Middle clump center
    let center = Vec2::new(domain_half_width * 0.0, 0.0);

    // Helper to generate a random point in a disc
    let random_in_disc = |center: Vec2| {
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let r = fastrand::f32().sqrt() * clump_radius;
        center + Vec2::new(cos, sin) * r
    };

    // Generate left clump
    for _ in 0..clump_size {
        let pos = random_in_disc(left_center);
        let vel = Vec2::zero();
        let mass = Species::LithiumMetal.mass();
        let radius = Species::LithiumMetal.radius();
        let mut body = Body::new(pos, vel, mass, radius, 0.0, Species::LithiumMetal);
        body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }];
        body.update_charge_from_electrons();
        bodies.push(body);
    }

    // Generate right clump
    for _ in 0..clump_size {
        let pos = random_in_disc(right_center);
        let vel = Vec2::zero();
        let mass = Species::LithiumMetal.mass();
        let radius = Species::LithiumMetal.radius();
        let mut body = Body::new(pos, vel, mass, radius, 0.0, Species::LithiumMetal);
        body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }];
        body.update_charge_from_electrons();
        bodies.push(body);
    }

    // Generate middle lump (ions)
    for _ in 0..clump_size*2 {
        let pos = random_in_disc(center);
        let vel = Vec2::zero();
        let mass = Species::LithiumCation.mass();
        let radius = Species::LithiumCation.radius();
        let mut body = Body::new(pos, vel, mass, radius, 1.0, Species::LithiumCation);
        body.electrons.clear();
        body.update_charge_from_electrons();
        bodies.push(body);
    }


    /*// Fill the rest with ions, randomly distributed
    let ions_to_add = n.saturating_sub(2 * clump_size);
    for _ in 0..ions_to_add {
        let x = fastrand::f32() * 2.0 * domain_half_width - domain_half_width;
        let y = (fastrand::f32() - 0.5) * 2.0 * domain_half_width;
        let pos = Vec2::new(x, y);
        let vel = Vec2::zero();
        let mass:f32 = 0.5;
        let radius = 1.0 * mass.cbrt();
        let charge = 1.0;
        bodies.push(Body::new(pos, vel, mass, radius, charge, Species::LithiumCation));
    }*/

    bodies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_disc_creates_ions_with_correct_radius() {
        let bodies = _uniform_disc(10);
        
        for body in &bodies {
            let expected_radius = body.species.radius();
            assert_eq!(body.radius, expected_radius, 
                "Body with species {:?} should have radius {} but got {}", 
                body.species, expected_radius, body.radius);
        }
        
        // Check that we have both LithiumCation and LithiumMetal particles with different radii
        let ions: Vec<_> = bodies.iter().filter(|b| b.species == Species::LithiumCation).collect();
        let metals: Vec<_> = bodies.iter().filter(|b| b.species == Species::LithiumMetal).collect();
        
        assert!(!ions.is_empty(), "Should have some ions");
        assert!(!metals.is_empty(), "Should have some metals");
        
        // Verify ions have different radius than metals
        if !ions.is_empty() && !metals.is_empty() {
            assert_ne!(ions[0].radius, metals[0].radius, 
                "Ion radius ({}) should be different from metal radius ({})", 
                ions[0].radius, metals[0].radius);
        }
    }

    #[test] 
    fn two_lithium_clumps_with_ions_creates_correct_radii() {
        let bodies = _two_lithium_clumps_with_ions(20, 5, 10.0, 50.0);
        
        for body in &bodies {
            let expected_radius = body.species.radius();
            assert_eq!(body.radius, expected_radius,
                "Body with species {:?} should have radius {} but got {}",
                body.species, expected_radius, body.radius);
        }
        
        // Check that we have both species with different radii
        let ions: Vec<_> = bodies.iter().filter(|b| b.species == Species::LithiumCation).collect();
        let metals: Vec<_> = bodies.iter().filter(|b| b.species == Species::LithiumMetal).collect();
        
        assert!(!ions.is_empty(), "Should have some ions");
        assert!(!metals.is_empty(), "Should have some metals");
        
        // Verify different radii
        if !ions.is_empty() && !metals.is_empty() {
            assert_ne!(ions[0].radius, metals[0].radius,
                "Ion radius ({}) should be different from metal radius ({})",
                ions[0].radius, metals[0].radius);
        }
    }
}
