#[cfg(test)]
mod tests {
    use super::*;
    use ultraviolet::Vec2;
    use crate::body::{Body, Species};
    use crate::quadtree::Quadtree;
    use crate::simulation::forces::K_E;

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
            electrons: Vec::new(),
            id: 0,
            e_field: Vec2::zero(),
            // Add other fields as needed for your Body struct
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
            let field = quadtree.acc_pos(*pos, 1.0, &bodies, K_E);
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
}