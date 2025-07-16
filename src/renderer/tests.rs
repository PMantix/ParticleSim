#[cfg(test)]
mod tests {
    use crate::renderer::state::TIMESTEP;
    use crate::body::foil::Foil;
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
        assert!(r.foil_wave_history.contains_key(&1), "No wave history created for foil");
        
        // Check that history has entries
        let history = r.foil_wave_history.get(&1).unwrap();
        assert!(!history.is_empty(), "Wave history is empty");
        
        // For constant current, we should have consistent current values
        let first_current = history[0].1;
        let last_current = history[history.len() - 1].1;
        assert!((first_current - last_current).abs() < 0.001, "Current values should be consistent for constant current");
    }

    #[test]
    fn start_recording_schedules_next_capture() {
        let mut r = Renderer::new();
        r.capture_interval = 0.5;
        *crate::renderer::state::SIM_TIME.lock() = 1.0;
        r.start_recording();
        assert!((r.next_capture_time - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn save_image_creates_file() {
        let mut r = Renderer::new();
        let dir = std::env::temp_dir().join("psim_test_out");
        std::fs::create_dir_all(&dir).unwrap();
        let data = vec![255u8, 0, 0, 255];
        r.output_dir = dir.clone();
        r.save_rgba_png(&data, 1, 1, 0.0);
        assert!(dir.join("frame_0_00.png").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
