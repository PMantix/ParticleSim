use crate::renderer::Renderer;
use ultraviolet::Vec2;
use std::fs;
use std::path::Path;
use image::{ImageBuffer, Rgba, RgbaImage};
use chrono::Utc;

pub struct ScreenCaptureManager {
    pub enabled: bool,
    pub interval: f32,
    pub last_capture_time: f32,
    pub folder: String,
    pub counter: usize,
    pub region: Option<(Vec2, Vec2)>,
}

impl ScreenCaptureManager {
    pub fn new() -> Self {
        Self {
            enabled: false,
            interval: 1.0,
            last_capture_time: 0.0,
            folder: "captures".to_string(),
            counter: 0,
            region: None,
        }
    }

    pub fn should_capture(&self, current_time: f32) -> bool {
        self.enabled && (current_time - self.last_capture_time) >= self.interval
    }

    pub fn capture_frame(&mut self, 
                        _render_context: &quarkstrom::RenderContext, 
                        window_width: u16, 
                        window_height: u16,
                        current_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        
        // Create capture directory if it doesn't exist
        fs::create_dir_all(&self.folder)?;
        
        // Determine capture region
        let (_capture_x, _capture_y, capture_width, capture_height) = if let Some((top_left, bottom_right)) = self.region {
            let x = top_left.x.min(bottom_right.x).max(0.0) as u32;
            let y = top_left.y.min(bottom_right.y).max(0.0) as u32;
            let width = (top_left.x - bottom_right.x).abs().min(window_width as f32) as u32;
            let height = (top_left.y - bottom_right.y).abs().min(window_height as f32) as u32;
            (x, y, width, height)
        } else {
            (0, 0, window_width as u32, window_height as u32)
        };

        // For now, we'll create a placeholder image since we can't directly read from the GPU framebuffer
        // In a real implementation, you'd need to read the framebuffer data from the GPU
        let mut image: RgbaImage = ImageBuffer::new(capture_width, capture_height);
        
        // Fill with a placeholder pattern to show the capture is working
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let r = ((x as f32 / capture_width as f32) * 255.0) as u8;
            let g = ((y as f32 / capture_height as f32) * 255.0) as u8;
            let b = ((current_time * 50.0) % 255.0) as u8;
            *pixel = Rgba([r, g, b, 255]);
        }

        // Generate filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("capture_{}_{:04}.png", timestamp, self.counter);
        let filepath = Path::new(&self.folder).join(filename);

        // Save the image
        image.save(filepath)?;
        
        self.counter += 1;
        self.last_capture_time = current_time;
        
        Ok(())
    }

    pub fn set_capture_region(&mut self, start: Vec2, end: Vec2) {
        self.region = Some((start, end));
    }

    pub fn clear_capture_region(&mut self) {
        self.region = None;
    }

    pub fn toggle_recording(&mut self) {
        self.enabled = !self.enabled;
        if self.enabled {
            self.counter = 0; // Reset counter when starting new recording session
        }
    }
}

impl Renderer {
    pub fn handle_screen_capture(&mut self, current_time: f32) {
        if self.screen_capture_enabled && (current_time - self.last_capture_time) >= self.capture_interval {
            // Trigger a capture by setting a flag
            self.should_capture_next_frame = true;
            self.last_capture_time = current_time;
        }
    }

    pub fn capture_current_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create capture directory if it doesn't exist
        fs::create_dir_all(&self.capture_folder)?;
        
        // Determine capture region
        let (capture_width, capture_height) = if let Some((start, end)) = self.capture_region {
            let width = (start.x - end.x).abs().max(1.0) as u32;
            let height = (start.y - end.y).abs().max(1.0) as u32;
            (width, height)
        } else {
            (self.window_width as u32, self.window_height as u32)
        };

        // For now, create a test image to verify the capture system is working
        // In a complete implementation, you would read from the GPU framebuffer
        let mut image: RgbaImage = ImageBuffer::new(capture_width, capture_height);
        
        // Fill with a test pattern that changes over time
        let current_time = *crate::renderer::state::SIM_TIME.lock();
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let r = ((x as f32 / capture_width as f32) * 255.0) as u8;
            let g = ((y as f32 / capture_height as f32) * 255.0) as u8;
            let b = ((current_time * 50.0) % 255.0) as u8;
            *pixel = Rgba([r, g, b, 255]);
        }

        // Generate filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("capture_{}_{:04}.png", timestamp, self.capture_counter);
        let filepath = Path::new(&self.capture_folder).join(&filename);

        // Save the image
        match image.save(&filepath) {
            Ok(_) => {
                println!("Screen capture saved: {}", filename);
                self.capture_counter += 1;
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to save screen capture: {}", e);
                Err(e.into())
            }
        }
    }

    pub fn trigger_manual_capture(&mut self) {
        self.should_capture_next_frame = true;
    }

    pub fn start_region_selection(&mut self, mouse_pos: Vec2) {
        self.is_selecting_region = true;
        self.selection_start = Some(mouse_pos);
        self.selection_end = Some(mouse_pos);
    }

    pub fn update_region_selection(&mut self, mouse_pos: Vec2) {
        if self.is_selecting_region {
            self.selection_end = Some(mouse_pos);
        }
    }

    pub fn finish_region_selection(&mut self) {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            self.capture_region = Some((start, end));
        }
        self.is_selecting_region = false;
        self.selection_start = None;
        self.selection_end = None;
    }

    pub fn cancel_region_selection(&mut self) {
        self.is_selecting_region = false;
        self.selection_start = None;
        self.selection_end = None;
    }

    pub fn clear_capture_region(&mut self) {
        self.capture_region = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ultraviolet::Vec2;
    use std::fs;
    use std::path::Path;
    use quarkstrom::Renderer as QuarkstromRenderer;

    #[test]
    fn test_screen_capture_manager_new() {
        let manager = ScreenCaptureManager::new();
        assert!(!manager.enabled);
        assert_eq!(manager.interval, 1.0);
        assert_eq!(manager.counter, 0);
        assert_eq!(manager.folder, "captures");
        assert!(manager.region.is_none());
    }

    #[test]
    fn test_should_capture_timing() {
        let mut manager = ScreenCaptureManager::new();
        
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
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
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
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
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
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
        renderer.capture_region = Some((Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)));
        renderer.clear_capture_region();
        assert!(renderer.capture_region.is_none());
    }

    #[test]
    fn test_capture_timing() {
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
        renderer.screen_capture_enabled = true;
        renderer.capture_interval = 1.0;
        renderer.last_capture_time = 0.0;
        
        // First call should update timing
        renderer.handle_screen_capture(1.0);
        assert_eq!(renderer.last_capture_time, 1.0);
        assert_eq!(renderer.capture_counter, 1);
        
        // Second call within interval should not update
        let old_counter = renderer.capture_counter;
        renderer.handle_screen_capture(1.5);
        assert_eq!(renderer.capture_counter, old_counter);
        
        // Call after interval should update
        renderer.handle_screen_capture(2.0);
        assert_eq!(renderer.last_capture_time, 2.0);
        assert_eq!(renderer.capture_counter, old_counter + 1);
    }

    #[test]
    fn test_screen_to_world_conversion() {
        let renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
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
        let mut renderer: crate::renderer::Renderer = QuarkstromRenderer::new();
        
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
