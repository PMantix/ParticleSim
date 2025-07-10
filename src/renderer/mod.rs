pub mod state;
pub mod input;
pub mod gui;
pub mod draw;

use crate::body::{Body, Species, foil::Foil};
use crate::config::{SimConfig, DOMAIN_BOUNDS};
use crate::quadtree::Node;
use crate::plotting::{PlottingSystem, PlotType, Quantity, SamplingMode};
use ultraviolet::Vec2;
use quarkstrom::winit_input_helper::WinitInputHelper;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum DeleteOption {
    AllSpecies,
    LithiumIon,
    LithiumMetal,
    FoilMetal,
    ElectrolyteAnion,
}

pub struct Renderer {
    pos: Vec2,
    scale: f32,
    scale_factor: f32,
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
    /// Local copy of the simulation frame for time-based visualizations
    frame: usize,
    /// History of on/off states for selected foils
    foil_wave_history: HashMap<u64, Vec<(f32, f32)>>,
    // Scenario controls
    scenario_radius: f32,
    scenario_x: f32,
    scenario_y: f32,
    scenario_species: Species,
    scenario_particle_radius: f32, 
    scenario_width: f32,           
    scenario_height: f32,
    scenario_random_count: usize,
    //pub scenario_charge: i32,
    pub velocity_vector_scale: f32,
    //scenario_current: f32,
    pub window_width: u16,
    pub window_height: u16,
    pub show_electron_deficiency: bool,
    // State saving/loading UI
    pub save_state_name: String,
    pub load_state_selected: Option<String>,
    // Plotting system
    plotting_system: PlottingSystem,
    // Plotting UI state
    show_plotting_window: bool,
    new_plot_type: PlotType,
    new_plot_quantity: Quantity,
    new_plot_sampling_mode: SamplingMode,
    new_plot_title: String,
    new_plot_spatial_bins: usize,
    new_plot_time_window: f32,
    new_plot_update_frequency: f32,
    // Domain size controls
    pub domain_width: f32,
    pub domain_height: f32,
    // LJ species selection
    pub selected_lj_species: Species,
    // Delete species selection
    pub selected_delete_option: DeleteOption,
}

impl quarkstrom::Renderer for Renderer {
    fn new() -> Self {
        Self {
            pos: Vec2::zero(),
            scale: 500.0,
            scale_factor: 1.0,
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
            frame: 0,
            foil_wave_history: HashMap::new(),
            scenario_radius: 1.0,
            scenario_x: 0.0,
            scenario_y: 0.0,
            scenario_species: Species::LithiumIon,
            scenario_particle_radius: 1.0, 
            scenario_width: 5.0,           
            scenario_height: 5.0,
            scenario_random_count: 1,
            //scenario_charge: 0,
            velocity_vector_scale: 0.1,
            //scenario_current: 0.0,
            window_width: 800, // default value, can be changed
            window_height: 600, // default value, can be changed
            show_electron_deficiency: true,
            save_state_name: String::new(),
            load_state_selected: None,
            // Initialize plotting system with simulation bounds using domain bounds
            plotting_system: PlottingSystem::new(DOMAIN_BOUNDS),
            // Plotting UI defaults
            show_plotting_window: false,
            new_plot_type: PlotType::TimeSeries,
            new_plot_quantity: Quantity::TotalSpeciesCount(Species::LithiumIon),
            new_plot_sampling_mode: SamplingMode::Continuous,
            new_plot_title: "New Plot".to_string(),
            new_plot_spatial_bins: 50,
            new_plot_time_window: 10.0,
            new_plot_update_frequency: 5.0,
            domain_width: 300.0,  // Default domain size
            domain_height: 300.0,
            selected_lj_species: Species::LithiumMetal, // Default to LithiumMetal for LJ editing
            selected_delete_option: DeleteOption::AllSpecies, // Default to All Species
        }
    }

    fn input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        self.window_width = width;
        self.window_height = height;
        self.scale_factor = input.scale_factor().unwrap_or(1.0) as f32;
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

#[cfg(test)]
mod tests;

