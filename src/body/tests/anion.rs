#[cfg(test)]
mod electrolyte_anion {
    use crate::body::{Body, Species, Electron};
    use ultraviolet::Vec2;

    #[test]
    fn anion_charge_updates_correctly() {
        let mut anion = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::ElectrolyteAnion);
        // starts with one electron for negative charge
        anion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        anion.update_charge_from_electrons();
        assert_eq!(anion.charge, -1.0);
        // remove the electron -> should become neutral
        anion.electrons.clear();
        anion.update_charge_from_electrons();
        assert_eq!(anion.charge, 0.0);
    }

    #[test]
    fn apply_redox_does_not_change_species() {
        let mut anion = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, -1.0, Species::ElectrolyteAnion);
        anion.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        anion.update_charge_from_electrons();
        anion.apply_redox(0, 1.0);
        assert_eq!(anion.species, Species::ElectrolyteAnion);
    }
}
