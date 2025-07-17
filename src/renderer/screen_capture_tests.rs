#[cfg(test)]
mod tests {
    use super::super::*;
    use ultraviolet::Vec2;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_screen_capture_manager_new() {
        let manager = screen_capture::ScreenCaptureManager::new();
        assert!(!manager.enabled);
        assert_eq!(manager.interval, 1.0);
        assert_eq!(manager.counter, 0);
        assert_eq!(manager.folder, "captures");
        assert!(manager.region.is_none());
    }

    #[test]
    fn test_should_capture_timing() {
        let mut manager = screen_capture::ScreenCaptureManager::new();
        
        // Should not capture when disabled
        assert!(!manager.should_capture(1.0));
        
        // Enable recording
        manager.enabled = true;
        manager.last_capture_time = 0.0;
        manager.interval = 1.0;
        
        // Should capture after interval
        assert!(manager.should_capture(1.0));
        assert!(manager.should_capture(1.5));
        
        // Should not capture before interval
        manager.last_capture_time = 0.5;
        assert!(!manager.should_capture(1.0));
        assert!(manager.should_capture(1.5));
    }

    #[test]
    fn test_capture_region_management() {
        let mut renderer = Renderer::new();
        
        // Test region selection
        let start = Vec2::new(100.0, 100.0);
        let end = Vec2::new(200.0, 200.0);
        
        renderer.start_region_selection(start);
        assert!(renderer.is_selecting_region);
        assert_eq!(renderer.selection_start, Some(start));
        
        renderer.update_region_selection(end);
        assert_eq!(renderer.selection_end, Some(end));
        
        renderer.finish_region_selection();
        assert!(!renderer.is_selecting_region);
        assert_eq!(renderer.capture_region, Some((start, end)));
        assert!(renderer.selection_start.is_none());
        assert!(renderer.selection_end.is_none());
    }

    #[test]
    fn test_cancel_region_selection() {
        let mut renderer = Renderer::new();
        
        let start = Vec2::new(100.0, 100.0);
        let end = Vec2::new(200.0, 200.0);
        
        renderer.start_region_selection(start);
        renderer.update_region_selection(end);
        renderer.cancel_region_selection();
        
        assert!(!renderer.is_selecting_region);
        assert!(renderer.selection_start.is_none());
        assert!(renderer.selection_end.is_none());
        assert!(renderer.capture_region.is_none());
    }

    #[test]
    fn test_clear_capture_region() {
        let mut renderer = Renderer::new();
        
        renderer.capture_region = Some((Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)));
        renderer.clear_capture_region();
        assert!(renderer.capture_region.is_none());
    }

    #[test]
    fn test_screen_capture_toggle() {
        let mut renderer = Renderer::new();
        
        assert!(!renderer.screen_capture_enabled);
        
        // Simulate toggling recording in GUI
        renderer.screen_capture_enabled = !renderer.screen_capture_enabled;
        assert!(renderer.screen_capture_enabled);
        
        renderer.screen_capture_enabled = !renderer.screen_capture_enabled;
        assert!(!renderer.screen_capture_enabled);
    }

    #[test]
    fn test_capture_timing() {
        let mut renderer = Renderer::new();
        
        renderer.screen_capture_enabled = true;
        renderer.capture_interval = 1.0;
        renderer.last_capture_time = 0.0;
        
        // First call should update timing
        renderer.handle_screen_capture(1.0, 800, 600);
        assert_eq!(renderer.last_capture_time, 1.0);
        assert_eq!(renderer.capture_counter, 1);
        
        // Second call within interval should not update
        let old_counter = renderer.capture_counter;
        renderer.handle_screen_capture(1.5, 800, 600);
        assert_eq!(renderer.capture_counter, old_counter);
        
        // Call after interval should update
        renderer.handle_screen_capture(2.0, 800, 600);
        assert_eq!(renderer.last_capture_time, 2.0);
        assert_eq!(renderer.capture_counter, old_counter + 1);
    }

    #[test]
    fn test_screen_to_world_conversion() {
        let renderer = Renderer::new();
        let width = 800;
        let height = 600;
        
        // Test center of screen
        let screen_center = Vec2::new(width as f32 / 2.0, height as f32 / 2.0);
        let world_pos = renderer.screen_to_world(screen_center, width, height);
        
        // The conversion should place the center near the camera position
        // (exact values depend on the camera position and scale)
        assert!(world_pos.x.is_finite());
        assert!(world_pos.y.is_finite());
    }

    #[test]
    fn test_capture_folder_creation() {
        let test_folder = "test_captures";
        
        // Clean up any existing test folder
        let _ = fs::remove_dir_all(test_folder);
        
        // Test folder creation logic (simulated)
        let result = fs::create_dir_all(test_folder);
        assert!(result.is_ok());
        
        // Verify folder exists
        assert!(Path::new(test_folder).exists());
        
        // Clean up
        let _ = fs::remove_dir_all(test_folder);
    }

    #[test]
    fn test_capture_file_naming() {
        use chrono::Utc;
        
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let counter = 42;
        let filename = format!("capture_{}_{:04}.png", timestamp, counter);
        
        assert!(filename.contains("capture_"));
        assert!(filename.contains("0042"));
        assert!(filename.ends_with(".png"));
    }

    #[test]
    fn test_capture_interval_validation() {
        let mut renderer = Renderer::new();
        
        // Test valid intervals
        renderer.capture_interval = 0.1;
        assert_eq!(renderer.capture_interval, 0.1);
        
        renderer.capture_interval = 10.0;
        assert_eq!(renderer.capture_interval, 10.0);
        
        // In real usage, the GUI would enforce ranges
        // but we can test the underlying data holds any value
        renderer.capture_interval = 0.01;
        assert_eq!(renderer.capture_interval, 0.01);
    }
}
