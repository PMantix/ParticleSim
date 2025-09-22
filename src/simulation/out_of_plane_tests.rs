use super::*;
use crate::body::{Body, Species};
use crate::simulation::out_of_plane::apply_out_of_plane;
use ultraviolet::Vec2;

#[cfg(test)]
mod out_of_plane_tests {
    use super::*;

    fn create_test_simulation() -> Simulation {
        let mut sim = Simulation::new();
        
        // Enable out-of-plane motion with reasonable parameters
        sim.config.enable_out_of_plane = true;
        sim.config.max_z = 5.0;
        sim.config.z_stiffness = 1.0;
        sim.config.z_damping = 0.1;
        // z_frustration_strength removed in favor of simple Li+ collision softness
        sim.domain_depth = 5.0;
        
        // Add a test particle
        let mut body = Body::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            1.0,    // mass
            1.0,    // radius 
            0.0,    // charge
            Species::LithiumIon,
        );
        body.z = 1.0; // Start at z=1
        body.vz = 0.0;
        body.az = 0.0;
        sim.bodies.push(body);
        
        sim
    }

    #[test]
    fn test_out_of_plane_disabling_resets_z() {
        let mut sim = create_test_simulation();
        
        // Particle should start at z=1
        assert_eq!(sim.bodies[0].z, 1.0);
        
        // Disable out-of-plane motion
        sim.config.enable_out_of_plane = false;
        
        // Apply out-of-plane function
        apply_out_of_plane(&mut sim);
        
        // Particle should be reset to z=0
        assert_eq!(sim.bodies[0].z, 0.0);
        assert_eq!(sim.bodies[0].vz, 0.0);
        assert_eq!(sim.bodies[0].az, 0.0);
    }

    #[test]
    fn test_out_of_plane_forces_applied() {
        let mut sim = create_test_simulation();
        
        // Set particle at z=2 (away from equilibrium)
        sim.bodies[0].z = 2.0;
        sim.bodies[0].vz = 0.0;
        sim.bodies[0].az = 0.0;
        sim.bodies[0].acc = Vec2::new(1.0, 0.0); // Some in-plane acceleration
        
        // Apply out-of-plane forces
        apply_out_of_plane(&mut sim);
        
        // Should have negative z-acceleration (spring force pulling back to z=0)
        assert!(sim.bodies[0].az < 0.0, "Should have negative z-acceleration due to spring force");
    }

    #[test]
    fn test_invalid_parameters_safety() {
        let mut sim = create_test_simulation();
        
        // Set invalid parameters
        sim.config.z_stiffness = f32::NAN;
        sim.config.max_z = 0.0;
        
        // This should not crash or cause issues
        apply_out_of_plane(&mut sim);
        
        // Particle should remain safe
        assert!(sim.bodies[0].z.is_finite());
        assert!(sim.bodies[0].vz.is_finite());
        assert!(sim.bodies[0].az.is_finite());
    }

    #[test]
    fn test_metal_particles_stay_fixed() {
        let mut sim = create_test_simulation();
        
        // Add a metal particle at non-zero z
        let mut metal_body = Body::new(
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 0.0),
            1.0,    // mass
            1.0,    // radius
            0.0,    // charge
            Species::LithiumMetal,
        );
        metal_body.z = 3.0; // Should be forced to 0
        metal_body.vz = 1.0; // Should be forced to 0
        metal_body.az = 1.0; // Should be forced to 0
        sim.bodies.push(metal_body);
        
        // Apply out-of-plane forces
        apply_out_of_plane(&mut sim);
        
        // Metal particle should be fixed at z=0
        assert_eq!(sim.bodies[1].z, 0.0);
        assert_eq!(sim.bodies[1].vz, 0.0);
        assert_eq!(sim.bodies[1].az, 0.0);
    }

    #[test]
    fn test_boundary_enforcement_safety() {
        let mut sim = create_test_simulation();
        
        // Add particle with invalid position
        let mut invalid_body = Body::new(
            Vec2::new(f32::NAN, f32::INFINITY),
            Vec2::new(0.0, 0.0),
            1.0,    // mass
            1.0,    // radius
            0.0,    // charge
            Species::LithiumIon,
        );
        invalid_body.z = f32::NAN;
        invalid_body.vz = f32::INFINITY;
        sim.bodies.push(invalid_body);
        
        // This should not crash
        apply_out_of_plane(&mut sim);
        
        // Invalid particle should be handled safely
        // (either reset or left in a safe state)
        assert!(sim.bodies.len() == 2); // Both particles should still exist
    }
}
