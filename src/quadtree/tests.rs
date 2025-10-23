#[cfg(test)]
mod tests {
    use crate::body::{Body, Species};
    use crate::quadtree::Quadtree;
    use crate::units::COULOMB_CONSTANT;
    use smallvec::SmallVec;
    use ultraviolet::Vec2;

    #[test]
    fn test_quadtree_field_centered_on_body() {
        // Create a single charged body at the origin
        let body = Body {
            pos: Vec2::zero(),
            vel: Vec2::zero(),
            acc: Vec2::zero(),
            mass: 1.0,
            radius: 1.0,
            charge: 1.0,
            species: Species::LithiumIon,
            electrons: SmallVec::new(),
            id: 0,
            e_field: Vec2::zero(),
            last_surround_frame: 0,
            last_surround_pos: Vec2::zero(),
            surrounded_by_metal: false,
            z: 0.0,
            vz: 0.0,
            az: 0.0,
        };
        let mut bodies = vec![body];

        // Build the quadtree and compute fields
        let mut quadtree = Quadtree::new(
            0.5,  // theta
            1e-6, // epsilon
            8,    // leaf_capacity
            32,   // thread_capacity
        );
        quadtree.build(&mut bodies);

        // We'll manually call the field function at various points
        let test_positions = [
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, -1.0),
        ];

        let mut magnitudes = Vec::new();

        for pos in &test_positions {
            // Use the same logic as in the quadtree's field function
            let field = quadtree.acc_pos(*pos, 1.0, 0.0, &bodies, COULOMB_CONSTANT);
            let expected_dir = pos.normalized();
            let field_dir = field.normalized();
            let dot = field_dir.dot(expected_dir);
            assert!(
                (dot - 1.0).abs() < 1e-5,
                "Field at {:?} not pointing radially out: dot={}",
                pos,
                dot
            );
            magnitudes.push(field.mag());
        }

        // All magnitudes should be (nearly) equal
        let avg_mag = magnitudes.iter().sum::<f32>() / magnitudes.len() as f32;
        for (i, mag) in magnitudes.iter().enumerate() {
            assert!(
                (mag - avg_mag).abs() < 1e-5,
                "Field magnitude at direction {} differs: {} vs avg {}",
                i,
                mag,
                avg_mag
            );
        }
    }

    #[test]
    fn overlapping_particles_produce_finite_force() {
        // Two bodies occupying nearly the same position
        let mut bodies = vec![
            Body {
                pos: Vec2::zero(),
                vel: Vec2::zero(),
                acc: Vec2::zero(),
                mass: 1.0,
                radius: 1.0,
                charge: 1.0,
                species: Species::LithiumIon,
                electrons: SmallVec::new(),
                id: 0,
                e_field: Vec2::zero(),
                last_surround_frame: 0,
                last_surround_pos: Vec2::zero(),
                surrounded_by_metal: false,
                z: 0.0,
                vz: 0.0,
                az: 0.0,
            },
            Body {
                pos: Vec2::new(0.5, 0.0), // overlapping radii
                vel: Vec2::zero(),
                acc: Vec2::zero(),
                mass: 1.0,
                radius: 1.0,
                charge: -1.0,
                species: Species::LithiumIon,
                electrons: SmallVec::new(),
                id: 1,
                e_field: Vec2::zero(),
                last_surround_frame: 0,
                last_surround_pos: Vec2::zero(),
                surrounded_by_metal: false,
                z: 0.0,
                vz: 0.0,
                az: 0.0,
            },
        ];

        let mut quadtree = Quadtree::new(1.0, 2.0, 8, 32);
        quadtree.build(&mut bodies);

        let field = quadtree.acc_pos(
            bodies[0].pos,
            bodies[0].charge,
            bodies[0].radius,
            &bodies,
            COULOMB_CONSTANT,
        );
        assert!(
            field.x.is_finite() && field.y.is_finite(),
            "Field should be finite for overlapping bodies"
        );
    }
}
