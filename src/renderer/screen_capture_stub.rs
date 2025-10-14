//! Stub implementations for screen capture-related APIs when the `screen_capture` feature is disabled.
//! This keeps other modules (draw/input/gui) compiling without pulling heavy deps.

use ultraviolet::Vec2;

use super::Renderer;

impl Renderer {
    pub fn handle_screen_capture(&mut self, _current_time: f32, _width: u16, _height: u16) {}
    pub fn screen_to_world(&self, screen_pos: Vec2, _width: u16, _height: u16) -> Vec2 { screen_pos } // identity fallback
    pub fn finish_region_selection(&mut self, _width: u16, _height: u16) {}
    pub fn cancel_region_selection(&mut self) { self.is_selecting_region = false; }
    pub fn clear_capture_region(&mut self) { self.capture_region = None; self.capture_region_ratio = None; }
    pub fn verify_capture_region_after_resize(&mut self, _width: u16, _height: u16) {}
    pub fn start_region_selection(&mut self, start: Vec2) { self.is_selecting_region = true; self.selection_start = Some(start); self.selection_end = Some(start); }
    pub fn update_region_selection(&mut self, end: Vec2) { if self.is_selecting_region { self.selection_end = Some(end); } }
}
