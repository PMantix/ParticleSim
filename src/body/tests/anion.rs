#[cfg(test)]
mod pf6_anion {
    use crate::body::{Body, Species, Electron};
    use crate::quadtree::Quadtree;
    //use crate::cell_list::CellList;
    use ultraviolet::Vec2;

    #[test]
    fn pf6_anion_charge_updates_correctly() {
        let mut anion = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::Pf6Anion);
        // PF6 anion starts with one electron for negative charge
        anion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        anion.update_charge_from_electrons();
        assert_eq!(anion.charge, -1.0);
        // Remove the electron -> should become neutral
        anion.electrons.clear();
        anion.update_charge_from_electrons();
        assert_eq!(anion.charge, 0.0);
    }

    #[test]
    fn pf6_anion_apply_redox_does_not_change_species() {
        let mut anion = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::Pf6Anion);
        anion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        anion.update_charge_from_electrons();
        let mut bodies = vec![anion];
        let mut qt = Quadtree::new(0.5, 0.01, 1, 1);
        qt.build(&mut bodies);
        {
            //let bodies_clone = bodies.clone();
            let (first, _rest) = bodies.split_at_mut(1);
            first[0].apply_redox();
        }
        assert_eq!(bodies[0].species, Species::Pf6Anion);
    }
}
