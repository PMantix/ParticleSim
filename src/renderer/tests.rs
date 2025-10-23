#[cfg(test)]
mod tests {
    use crate::body::foil::Foil;
    use crate::renderer::state::TIMESTEP;
    use crate::renderer::Renderer;
    use quarkstrom::Renderer as QuarkstromRenderer;

    #[test]
    fn constant_current_produces_lines() {
        *TIMESTEP.lock() = 0.001;
        let mut r = Renderer::new();
        r.foils.push(Foil {
            id: 1,
            body_ids: vec![],
            dc_current: 1.0,
            ac_current: 0.0,
            accum: 0.0,
            switch_hz: 0.0,
            link_id: None,
            mode: crate::body::foil::LinkMode::Parallel,
            charging_mode: crate::body::foil::ChargingMode::Current,
            overpotential_controller: None,
            slave_overpotential_current: 0.0,
            electron_delta_since_measure: 0,
        });
        r.selected_foil_ids.push(1);

        // Simulate some frames to build up history
        for f in 0..1000 {
            r.frame = f;
            // Set simulation time to simulate passage of time
            *crate::renderer::state::SIM_TIME.lock() = f as f32 * 0.001;
            r.update_foil_wave_history();
        }

        // Check that history was created for the foil
        assert!(
            r.foil_wave_history.contains_key(&1),
            "No wave history created for foil"
        );

        // Check that history has entries
        let history = r.foil_wave_history.get(&1).unwrap();
        assert!(!history.is_empty(), "Wave history is empty");

        // For constant current, we should have consistent current values
        let first_current = history[0].1;
        let last_current = history[history.len() - 1].1;
        assert!(
            (first_current - last_current).abs() < 0.001,
            "Current values should be consistent for constant current"
        );
    }
}
