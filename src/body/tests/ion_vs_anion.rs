#[cfg(test)]
mod ion_vs_anion {
    use crate::body::{Body, Species, Electron};
    use crate::simulation::Simulation;
    use ultraviolet::Vec2;

    #[test]
    fn ions_and_anions_repulsion_symmetry() {
        let mut sim = Simulation::new();
        let n = 2;
        let spacing = 1.5;
        // Place ions in a line
        for i in 0..n {
            let mut ion = Body::new(
                Vec2::new(i as f32 * spacing, 0.0),
                Vec2::zero(),
                1.0,
                1.0,
                1.0,
                Species::LithiumIon,
            );
            // Remove electrons to ensure +1 charge
            ion.electrons.clear();
            ion.update_charge_from_electrons();
            sim.bodies.push(ion);
        }
        // Place anions in a line above
        for i in 0..n {
            let mut anion = Body::new(
                Vec2::new(i as f32 * spacing, 2.0),
                Vec2::zero(),
                1.0,
                1.0,
                -1.0,
                Species::ElectrolyteAnion,
            );
            // Add one electron to ensure -1 charge
            println!("---Adding electron to anion at position: {:?}", anion.pos);
            println!("Anion charge before: {}", anion.charge);
            anion.electrons = smallvec::smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }];
            anion.update_charge_from_electrons();
            println!("Anion charge after: {}", anion.charge);   
            

            sim.bodies.push(anion);
        }
        // Step simulation a few times
        for _ in 0..10 {
            sim.step();
        }
        // Check that all ions and all anions are not clumped (distance between any pair > 0.1)
        let mut max_ion_closeness = 0.0;
        let mut max_anion_closeness = 0.0;
        // Ions: indices 0..n
        for i in 0..n {
            for j in (i+1)..n {
                let d = (sim.bodies[i].pos - sim.bodies[j].pos).mag();
                if d < 0.1 { panic!("Ions clumped: d = {}", d); }
                if d > max_ion_closeness { max_ion_closeness = d; }
            }
        }
        // Anions: indices n..2n
        for i in n..2*n {
            for j in (i+1)..2*n {
                let d = (sim.bodies[i].pos - sim.bodies[j].pos).mag();
                if d < 0.1 { panic!("Anions clumped: d = {}", d); }
                if d > max_anion_closeness { max_anion_closeness = d; }
            }
        }
        // Print for manual inspection
        println!("Max ion separation: {}", max_ion_closeness);
        println!("Max anion separation: {}", max_anion_closeness);

    }
}
