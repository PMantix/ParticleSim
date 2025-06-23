pub mod state;
pub mod input;
pub mod gui;
pub mod draw;

use crate::body::{Body, Species, foil::Foil};
use crate::config::SimConfig;
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
    foils: Vec<Foil>,
    selected_particle_id: Option<u64>,
    //foils: Vec<crate::body::foil::Foil>,
    selected_foil_ids: Vec<u64>,
    selected_particle_ids: Vec<u64>,
    sim_config: SimConfig,
    // Scenario controls
    scenario_radius: f32,
    scenario_x: f32,
    scenario_y: f32,
    scenario_species: Species,
    scenario_particle_radius: f32, 
    scenario_width: f32,           
    scenario_height: f32,
    //pub scenario_charge: i32,
    pub velocity_vector_scale: f32,
    scenario_current: f32,
    pub window_width: u16,
    pub window_height: u16,
}

impl quarkstrom::Renderer for Renderer {
    fn new() -> Self {
        Self {
            pos: Vec2::zero(),
            scale: 500.0,
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
            foils: Vec::new(),
            selected_particle_id: None,
            //foils: Vec::new(),
            selected_foil_ids: Vec::new(),
            selected_particle_ids: Vec::new(),
            sim_config: crate::config::LJ_CONFIG.lock().clone(),
            scenario_radius: 1.0,
            scenario_x: 0.0,
            scenario_y: 0.0,
            scenario_species: Species::LithiumIon,
            scenario_particle_radius: 1.0, 
            scenario_width: 5.0,           
            scenario_height: 5.0,
            //scenario_charge: 0,
            velocity_vector_scale: 0.1,
            scenario_current: 0.0,
            window_width: 800, // default value, can be changed
            window_height: 600, // default value, can be changed
        }
    }

    fn input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        self.window_width = width;
        self.window_height = height;
        self.handle_input(input, width, height);
    }
    fn render(&mut self, ctx: &mut quarkstrom::RenderContext) {
        self.draw(ctx, self.window_width, self.window_height);
    }
    fn gui(&mut self, ctx: &quarkstrom::egui::Context) {
        self.show_gui(ctx);
        // After GUI update, write changes to global config
        *crate::config::LJ_CONFIG.lock() = self.sim_config.clone();
    }
}

