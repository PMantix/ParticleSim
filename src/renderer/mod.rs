pub mod state;
pub mod input;
pub mod gui;
pub mod draw;

use crate::body::Body;
use crate::quadtree::Node;
use ultraviolet::Vec2;
use quarkstrom::winit_input_helper::WinitInputHelper;

pub struct Renderer {
    pos: Vec2,
    scale: f32,

    settings_window_open: bool,

    show_bodies: bool,
    show_quadtree: bool,

    depth_range: (usize, usize),

    spawn_body: Option<Body>,
    angle: Option<f32>,
    total: Option<f32>,

    confirmed_bodies: Option<Body>,

    bodies: Vec<Body>,
    quadtree: Vec<Node>,
}

impl quarkstrom::Renderer for Renderer {
    fn new() -> Self {
        Self {
            pos: Vec2::zero(),
            scale: 3600.0,
            settings_window_open: false,
            show_bodies: true,
            show_quadtree: false,
            depth_range: (0, 0),
            spawn_body: None,
            angle: None,
            total: None,
            confirmed_bodies: None,
            bodies: Vec::new(),
            quadtree: Vec::new(),
        }
    }

    fn input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        self.handle_input(input, width, height);
    }
    fn render(&mut self, ctx: &mut quarkstrom::RenderContext) {
        self.draw(ctx);
    }
    fn gui(&mut self, ctx: &quarkstrom::egui::Context) {
        self.show_gui(ctx);
    }
}
