pub mod state;
pub mod input;
pub mod gui;
pub mod draw;
pub mod screen_capture;

use crate::body::{Body, Species, foil::Foil};
use crate::config::SimConfig;
use crate::quadtree::Node;
use crate::plotting::{PlottingSystem, PlotType, Quantity, SamplingMode};
use crate::diagnostics::{TransferenceNumberDiagnostic, FoilElectronFractionDiagnostic};
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
    EC,
    DMC,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuiTab {
    Simulation,
    Visualization,
    Species,
    Physics,
    Scenario,
    Foils,
    Analysis,
    Debug,
    Diagnostics,
    ScreenCapture,
}

impl Default for GuiTab {
    fn default() -> Self {
        GuiTab::Simulation
    }
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
    scenario_width: f32,           
    scenario_height: f32,
    scenario_random_count: usize,
    //pub scenario_charge: i32,
    pub velocity_vector_scale: f32,
    //scenario_current: f32,
    pub window_width: u16,
    pub window_height: u16,
    pub show_foil_electron_deficiency: bool,
    pub show_metal_electron_deficiency: bool,
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
    // Current GUI tab
    pub current_tab: GuiTab,
    pub transference_number_diagnostic: Option<TransferenceNumberDiagnostic>,
    pub foil_electron_fraction_diagnostic: Option<FoilElectronFractionDiagnostic>,
    pub solvation_diagnostic: Option<crate::diagnostics::SolvationDiagnostic>,
    
    // Solvation visualization flags
    pub show_cip_ions: bool,
    pub show_sip_ions: bool,
    pub show_s2ip_ions: bool,
    pub show_fd_ions: bool,
    
    // View mode toggle
    pub side_view_mode: bool,  // false = X-Y (top-down), true = X-Z (side view)
    
    // Electrolyte solution controls
    pub electrolyte_molarity: f32,
    pub electrolyte_total_particles: usize,
    
    // Screen capture functionality
    pub screen_capture_enabled: bool,
    pub capture_interval: f32,  // seconds between captures
    pub last_capture_time: f32,
    pub capture_folder: String,
    pub selection_start: Option<Vec2>,  // for drag selection
    pub selection_end: Option<Vec2>,
    pub capture_region: Option<(Vec2, Vec2)>,  // (top_left, bottom_right) in world space
    pub capture_region_ratio: Option<(Vec2, Vec2)>,
    pub is_selecting_region: bool,
    pub capture_counter: usize,
    pub should_capture_next_frame: bool,
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
            scenario_width: 5.0,           
            scenario_height: 5.0,
            scenario_random_count: 1,
            //scenario_charge: 0,
            velocity_vector_scale: 0.1,
            //scenario_current: 0.0,
            window_width: 800, // default value, can be changed
            window_height: 600, // default value, can be changed
            show_foil_electron_deficiency: true,
            show_metal_electron_deficiency: false,
            save_state_name: String::new(),
            load_state_selected: None,
            // Initialize plotting system with simulation bounds using domain bounds
            plotting_system: PlottingSystem::new(),
            // Plotting UI defaults
            show_plotting_window: false,
            new_plot_type: PlotType::TimeSeries,
            new_plot_quantity: Quantity::TotalSpeciesCount(Species::LithiumIon),
            new_plot_sampling_mode: SamplingMode::Continuous,
            new_plot_title: "New Plot".to_string(),
            new_plot_spatial_bins: 50,
            new_plot_time_window: 10.0,
            new_plot_update_frequency: 5.0,
            domain_width: *crate::renderer::state::DOMAIN_WIDTH.lock(),  // Initialize from shared state
            domain_height: *crate::renderer::state::DOMAIN_HEIGHT.lock(), // Initialize from shared state
            selected_lj_species: Species::LithiumMetal, // Default to LithiumMetal for LJ editing
            selected_delete_option: DeleteOption::AllSpecies, // Default to All Species
            current_tab: GuiTab::default(), // Default to Simulation tab
            transference_number_diagnostic: Some(TransferenceNumberDiagnostic::new()),
            foil_electron_fraction_diagnostic: Some(FoilElectronFractionDiagnostic::new()),
            solvation_diagnostic: Some(crate::diagnostics::SolvationDiagnostic::new()),
            
            // Solvation visualization flags - default to false
            show_cip_ions: false,
            show_sip_ions: false,
            show_s2ip_ions: false,
            show_fd_ions: false,
            
            // View mode - default to top-down (X-Y)
            side_view_mode: false,
            
            // Electrolyte solution controls
            electrolyte_molarity: 1.0,        // 1M default
            electrolyte_total_particles: 1000, // 1000 particles default
            
            // Screen capture defaults
            screen_capture_enabled: false,
            capture_interval: 1.0,  // 1 second between captures
            last_capture_time: 0.0,
            capture_folder: "captures".to_string(),
            selection_start: None,
            selection_end: None,
            capture_region: None,
            capture_region_ratio: None,
            is_selecting_region: false,
            capture_counter: 0,
            should_capture_next_frame: false,
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

