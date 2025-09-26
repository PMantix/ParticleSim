pub mod draw;
pub mod gui;
pub mod input;
pub mod screen_capture;
pub mod state;

use crate::body::{foil::Foil, Body, Species};
use crate::config::SimConfig;
use crate::diagnostics::{FoilElectronFractionDiagnostic, TransferenceNumberDiagnostic};
use crate::plotting::{PlotType, PlottingSystem, Quantity, SamplingMode};
use crate::quadtree::Node;
use crate::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use crate::switch_charging;
use quarkstrom::egui::{self, Color32, Pos2, Vec2 as EVec2};
use quarkstrom::winit_input_helper::WinitInputHelper;
use std::collections::HashMap;
use std::fs;
use ultraviolet::Vec2;

const SPLASH_ART: &[&str] = &[
    "██████╗  █████╗ ██████╗ ████████╗██╗ ██████╗██╗     ███████╗",
    "██╔══██╗██╔══██╗██╔══██╗╚══██╔══╝██║██╔════╝██║     ██╔════╝",
    "██████╔╝███████║██████╔╝   ██║   ██║██║     ██║     █████╗  ",
    "██╔═══╝ ██╔══██║██╔══██╗   ██║   ██║██║     ██║     ██╔══╝  ",
    "██║     ██║  ██║██║  ██║   ██║   ██║╚██████╗███████╗███████╗",
    "╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═╝ ╚═════╝╚══════╝╚══════╝",
    "",
    "███████╗██╗███╗   ███╗",
    "██╔════╝██║████╗ ████║",
    "███████╗██║██╔████╔██║",
    "╚════██║██║██║╚██╔╝██║",
    "███████║██║██║ ╚═╝ ██║",
    "╚══════╝╚═╝╚═╝     ╚═╝",
];

struct PopEffect {
    pos: Pos2,
    vel: EVec2,
    char: char,
    life: f32,
}

#[derive(Clone)]
struct SplashChar {
    row: usize,
    col: usize,
    ch: char,
    color: Color32,
    original_color: Color32,
    color_timer: f32,
}

struct SplashParticle {
    pos: Pos2,
    vel: EVec2,
    radius: f32,
    stuck_timer: f32,
    last_pos: Pos2,
    charge: f32, // Positive charge for repulsion
}

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

#[derive(Clone, Debug)]
pub struct MeasurementRecord {
    pub step: usize,
    pub time_fs: f32,
    pub distance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuiTab {
    Simulation,
    Visualization,
    Species,
    Physics,
    Scenario,
    Foils,
    SwitchCharging,
    Measurement,
    Analysis,
    Debug,
    Diagnostics,
    ScreenCapture,
    SoftDynamics,
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
    species_dark_mode_enabled: bool,
    species_dark_mode_strength: f32,
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
    selected_pid_foil_id: Option<u64>, // For PID graph foil selection
    switch_ui_state: switch_charging::SwitchUiState,
    sim_config: SimConfig,
    /// Local copy of the simulation frame for time-based visualizations
    frame: usize,
    playback_cursor: usize,
    playback_speed: f32,
    playback_follow_live: bool,
    playback_auto_resume: bool,
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
    pub show_switching_role_halos: bool,
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

    // 2D Domain Density Calculation - Species Selection
    pub density_calc_lithium_ion: bool,
    pub density_calc_lithium_metal: bool,
    pub density_calc_foil_metal: bool,
    pub density_calc_electrolyte_anion: bool,
    pub density_calc_ec: bool,
    pub density_calc_dmc: bool,

    // View mode toggle
    pub side_view_mode: bool, // false = X-Y (top-down), true = X-Z (side view)

    // Electrolyte solution controls
    pub electrolyte_molarity: f32,
    pub electrolyte_total_particles: usize,

    // Screen capture functionality
    pub screen_capture_enabled: bool,
    pub capture_interval: f32, // seconds between captures
    pub last_capture_time: f32,
    pub capture_folder: String,
    pub selection_start: Option<Vec2>, // for drag selection
    pub selection_end: Option<Vec2>,
    pub capture_region: Option<(Vec2, Vec2)>, // (top_left, bottom_right) in world space
    pub capture_region_ratio: Option<(Vec2, Vec2)>,
    pub is_selecting_region: bool,
    pub capture_counter: usize,
    pub should_capture_next_frame: bool,
    pub measurement_start: Option<Vec2>,
    pub measurement_selecting_start: bool,
    pub measurement_history: Vec<MeasurementRecord>,
    pub measurement_cursor: Option<Vec2>,
    pub last_non_measurement_tab: GuiTab,
    // Splash screen state
    show_splash: bool,
    splash_chars: Vec<SplashChar>,
    splash_particles: Vec<SplashParticle>,
    pop_effects: Vec<PopEffect>,
    scenarios: Vec<String>,
    selected_scenario: usize,
    splash_art_width: usize,
    splash_art_height: usize,
    char_size: f32,
    // Mouse interaction
    last_mouse_pos: Option<EVec2>,
    mouse_velocity: EVec2,
    // Foil group linking selections
    group_a_selected: Vec<u64>,
    group_b_selected: Vec<u64>,
    // Dipole visualization
    pub show_dipoles: bool,
    pub dipole_scale: f32,
}

impl quarkstrom::Renderer for Renderer {
    fn new() -> Self {
        let char_size = 16.0;
        let splash_art_width = SPLASH_ART
            .iter()
            .map(|s| s.chars().count())
            .max()
            .unwrap_or(0);
        let splash_art_height = SPLASH_ART.len();
        let splash_chars = {
            let mut chars = Vec::new();
            for (row, line) in SPLASH_ART.iter().enumerate() {
                for (col, ch) in line.chars().enumerate() {
                    if ch != ' ' {
                        chars.push(SplashChar {
                            row,
                            col,
                            ch,
                            color: Color32::WHITE,
                            original_color: Color32::WHITE,
                            color_timer: 0.0,
                        });
                    }
                }
            }
            chars
        };
        let splash_particles = Vec::new(); // Initialize empty, will be created on first update
        Self {
            pos: Vec2::zero(),
            scale: 500.0,
            scale_factor: 1.0,
            settings_window_open: false,
            show_bodies: true,
            show_quadtree: false,
            species_dark_mode_enabled: false,
            species_dark_mode_strength: 0.5,
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
            selected_pid_foil_id: None, // Initialize PID graph foil selection to None
            switch_ui_state: switch_charging::SwitchUiState::new(),
            sim_config: crate::config::LJ_CONFIG.lock().clone(),
            frame: 0,
            playback_cursor: 0,
            playback_speed: 1.0,
            playback_follow_live: true,
            playback_auto_resume: true,
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
            window_width: 800,  // default value, can be changed
            window_height: 600, // default value, can be changed
            show_foil_electron_deficiency: true,
            show_metal_electron_deficiency: false,
            show_switching_role_halos: false,
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
            domain_width: *crate::renderer::state::DOMAIN_WIDTH.lock(), // Initialize from shared state
            domain_height: *crate::renderer::state::DOMAIN_HEIGHT.lock(), // Initialize from shared state
            selected_lj_species: Species::LithiumMetal, // Default to LithiumMetal for LJ editing
            selected_delete_option: DeleteOption::AllSpecies, // Default to All Species
            current_tab: GuiTab::default(),             // Default to Simulation tab
            transference_number_diagnostic: Some(TransferenceNumberDiagnostic::new()),
            foil_electron_fraction_diagnostic: Some(FoilElectronFractionDiagnostic::new()),
            solvation_diagnostic: Some(crate::diagnostics::SolvationDiagnostic::new()),

            // Solvation visualization flags - default to false
            show_cip_ions: false,
            show_sip_ions: false,
            show_s2ip_ions: false,
            show_fd_ions: false,

            // 2D Domain Density Calculation - Species Selection (default to Li+ only)
            density_calc_lithium_ion: true,
            density_calc_lithium_metal: false,
            density_calc_foil_metal: false,
            density_calc_electrolyte_anion: false,
            density_calc_ec: false,
            density_calc_dmc: false,

            // View mode - default to top-down (X-Y)
            side_view_mode: false,

            // Electrolyte solution controls
            electrolyte_molarity: 1.0,         // 1M default
            electrolyte_total_particles: 1000, // 1000 particles default

            // Screen capture defaults
            screen_capture_enabled: false,
            capture_interval: 1.0, // 1 second between captures
            last_capture_time: 0.0,
            capture_folder: "captures".to_string(),
            selection_start: None,
            selection_end: None,
            capture_region: None,
            capture_region_ratio: None,
            is_selecting_region: false,
            capture_counter: 0,
            should_capture_next_frame: false,
            measurement_start: None,
            measurement_selecting_start: false,
            measurement_history: Vec::new(),
            measurement_cursor: None,
            last_non_measurement_tab: GuiTab::Simulation,
            show_splash: true,
            splash_chars,
            splash_particles,
            pop_effects: Vec::new(),
            scenarios: Self::available_scenarios(),
            selected_scenario: 0,
            splash_art_width,
            splash_art_height,
            char_size,
            last_mouse_pos: None,
            mouse_velocity: EVec2::new(0.0, 0.0),
            group_a_selected: Vec::new(),
            group_b_selected: Vec::new(),
            show_dipoles: false,
            dipole_scale: 1.0,
        }
    }

    fn input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        if width == 0 || height == 0 {
            // Window is minimized; ignore input until restored
            return;
        }
        self.scale_factor = input.scale_factor().unwrap_or(1.0) as f32;
        self.handle_input(input, width, height);
    }
    fn render(&mut self, ctx: &mut quarkstrom::RenderContext) {
        if self.window_width == 0 || self.window_height == 0 {
            // Surface has zero area while minimized, skip drawing
            return;
        }
        self.draw(ctx, self.window_width, self.window_height);
    }
    fn gui(&mut self, ctx: &quarkstrom::egui::Context) {
        self.show_gui(ctx);
        // After GUI update, write changes to global config
        *crate::config::LJ_CONFIG.lock() = self.sim_config.clone();
    }
}

impl Renderer {
    fn available_scenarios() -> Vec<String> {
        let mut list = vec!["Default".to_string()];
        if let Ok(entries) = fs::read_dir("saved_state") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        list.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }
        list
    }

    fn random_color() -> Color32 {
        let mut rng = fastrand::Rng::new();
        Color32::from_rgb(
            (rng.f32() * 255.0) as u8,
            (rng.f32() * 255.0) as u8,
            (rng.f32() * 255.0) as u8,
        )
    }

    pub fn current_measurement_distance(&self) -> Option<f32> {
        if let (Some(start), Some(cursor)) = (self.measurement_start, self.measurement_cursor) {
            Some((cursor - start).mag())
        } else {
            None
        }
    }

    fn update_splash_particles(
        &mut self,
        width: f32,
        height: f32,
        _rects: &[egui::Rect],
        mouse_pos: Option<EVec2>,
    ) {
        let mut rng = fastrand::Rng::new();

        // Initialize particles on first update when we have actual window dimensions
        if self.splash_particles.is_empty() {
            for _ in 0..200 {
                let x = rng.f32() * width; // Use actual window width
                let y = rng.f32() * height; // Use actual window height

                let pos = Pos2::new(x, y);
                let vel = EVec2::new(
                    (rng.f32() - 0.5) * 2.0, // Gentle initial velocities
                    (rng.f32() - 0.5) * 2.0,
                );

                self.splash_particles.push(SplashParticle {
                    pos,
                    vel,
                    radius: 3.0,
                    stuck_timer: 0.0,
                    last_pos: pos,
                    charge: 1.0, // Positive charge
                });
            }
            println!(
                "Created {} particles distributed across window {}x{}",
                self.splash_particles.len(),
                width as u32,
                height as u32
            );
        }

        let dt = 0.0004; // Even smaller time step for ultra-smooth physics

        // Update mouse velocity tracking
        if let Some(current_mouse_pos) = mouse_pos {
            if let Some(last_pos) = self.last_mouse_pos {
                // Calculate mouse movement velocity
                self.mouse_velocity = (current_mouse_pos - last_pos) / dt;
                // Apply some smoothing to reduce jitter
                self.mouse_velocity *= 0.5;
            }
            self.last_mouse_pos = Some(current_mouse_pos);
        } else {
            // No mouse position available, decay the velocity
            self.mouse_velocity *= 0.9;
        }

        // Update pop effects
        self.pop_effects.retain_mut(|effect| {
            effect.pos.x += effect.vel.x;
            effect.pos.y += effect.vel.y;
            effect.vel.x *= 0.95; // Slow down
            effect.vel.y *= 0.95;
            effect.life -= dt;
            effect.life > 0.0
        });

        // Update character colors - gradual fade back to white
        for ch in &mut self.splash_chars {
            if ch.color_timer > 0.0 {
                ch.color_timer -= dt;
                let fade_progress = ch.color_timer / 2.0; // 2 seconds total

                // Interpolate between current color and original white
                let r = (ch.color.r() as f32 * fade_progress
                    + ch.original_color.r() as f32 * (1.0 - fade_progress))
                    as u8;
                let g = (ch.color.g() as f32 * fade_progress
                    + ch.original_color.g() as f32 * (1.0 - fade_progress))
                    as u8;
                let b = (ch.color.b() as f32 * fade_progress
                    + ch.original_color.b() as f32 * (1.0 - fade_progress))
                    as u8;
                ch.color = Color32::from_rgb(r, g, b);

                if ch.color_timer <= 0.0 {
                    ch.color = ch.original_color;
                    ch.color_timer = 0.0;
                }
            }
        }

        // Track particles to remove (stuck ones)
        let mut particles_to_remove = Vec::new();

        // Calculate electrostatic forces for all particles first
        let mut forces: Vec<(f32, f32)> = vec![(0.0, 0.0); self.splash_particles.len()];

        // Particle-to-particle repulsion
        for i in 0..self.splash_particles.len() {
            for j in i + 1..self.splash_particles.len() {
                let dx = self.splash_particles[i].pos.x - self.splash_particles[j].pos.x;
                let dy = self.splash_particles[i].pos.y - self.splash_particles[j].pos.y;
                let distance_sq = dx * dx + dy * dy + 1.0; // Add 1 to prevent division by zero
                let distance = distance_sq.sqrt();

                // Coulomb force: F = k * q1 * q2 / r^2 (positive charges repel)
                let force_magnitude =
                    500.0 * self.splash_particles[i].charge * self.splash_particles[j].charge
                        / distance_sq;
                let force_x = force_magnitude * dx / distance;
                let force_y = force_magnitude * dy / distance;

                // Apply equal and opposite forces
                forces[i].0 += force_x;
                forces[i].1 += force_y;
                forces[j].0 -= force_x;
                forces[j].1 -= force_y;
            }

            // Attraction to ASCII letters
            let art_width_px = self.splash_art_width as f32 * self.char_size;
            let art_height_px = self.splash_art_height as f32 * self.char_size;
            let art_center_x = width / 2.0;
            let art_center_y = height / 2.0 - 40.0;
            let art_left = art_center_x - art_width_px / 2.0;
            let art_top = art_center_y - art_height_px / 2.0;

            for splash_char in &self.splash_chars {
                let letter_x =
                    art_left + splash_char.col as f32 * self.char_size + self.char_size / 2.0;
                let letter_y =
                    art_top + splash_char.row as f32 * self.char_size + self.char_size / 2.0;

                let dx = letter_x - self.splash_particles[i].pos.x;
                let dy = letter_y - self.splash_particles[i].pos.y;
                let distance_sq = dx * dx + dy * dy + 100.0; // Minimum distance to prevent too strong attraction
                let distance = distance_sq.sqrt();

                // Only attract if reasonably close (within 150 pixels)
                if distance < 150.0 {
                    let letter_charge = -0.3; // Negative charge on letters
                    let force_magnitude =
                        300.0 * self.splash_particles[i].charge * letter_charge / distance_sq;
                    forces[i].0 += force_magnitude * dx / distance;
                    forces[i].1 += force_magnitude * dy / distance;
                }
            }
        }

        // Update particles
        for (i, p) in self.splash_particles.iter_mut().enumerate() {
            // Check if particle is stuck (hasn't moved much)
            let distance_moved =
                ((p.pos.x - p.last_pos.x).powi(2) + (p.pos.y - p.last_pos.y).powi(2)).sqrt();
            if distance_moved < 50.0 {
                // 50 pixels movement window
                p.stuck_timer += dt;
            } else {
                p.stuck_timer = 0.0;
                p.last_pos = p.pos;
            }

            // If stuck too long, mark for popping
            if p.stuck_timer > 3.0 {
                // 3 seconds stuck
                particles_to_remove.push(i);

                // Create pop effect
                let pop_chars = [',', '.', '·', '°', 'o', '*'];
                self.pop_effects.push(PopEffect {
                    pos: p.pos,
                    vel: EVec2::new((rng.f32() - 0.5) * 4.0, (rng.f32() - 0.5) * 4.0),
                    char: pop_chars[rng.usize(..pop_chars.len())],
                    life: 1.0,
                });
                continue;
            }

            // Apply electrostatic forces to velocity (adjusted for smaller timestep)
            p.vel.x += forces[i].0 * dt * 1.0; // Increased multiplier to compensate for smaller dt
            p.vel.y += forces[i].1 * dt * 1.0;

            // Add gentle mouse movement influence to ALL particles
            // All particles get a small velocity boost in the direction the mouse is moving
            p.vel.x += self.mouse_velocity.x * 0.000002; // Almost imperceptible influence (0.000002)
            p.vel.y += self.mouse_velocity.y * 0.000002;

            // Simple physics - move particle
            p.pos.x += p.vel.x;
            p.pos.y += p.vel.y;

            // Bounce off screen edges
            if p.pos.x <= 0.0 || p.pos.x >= width {
                p.vel.x = -p.vel.x;
                p.pos.x = p.pos.x.clamp(0.0, width);
            }
            if p.pos.y <= 0.0 || p.pos.y >= height {
                p.vel.y = -p.vel.y;
                p.pos.y = p.pos.y.clamp(0.0, height);
            }

            // Check collision with ASCII art letters
            let art_width_px = self.splash_art_width as f32 * self.char_size;
            let art_height_px = self.splash_art_height as f32 * self.char_size;
            let art_center_x = width / 2.0;
            let art_center_y = height / 2.0 - 40.0;
            let art_left = art_center_x - art_width_px / 2.0;
            let art_top = art_center_y - art_height_px / 2.0;

            // Check if particle is in the art area
            if p.pos.x >= art_left
                && p.pos.x <= art_left + art_width_px
                && p.pos.y >= art_top
                && p.pos.y <= art_top + art_height_px
            {
                let char_x = ((p.pos.x - art_left) / self.char_size) as usize;
                let char_y = ((p.pos.y - art_top) / self.char_size) as usize;

                if char_y < SPLASH_ART.len() && char_x < SPLASH_ART[char_y].chars().count() {
                    let ch = SPLASH_ART[char_y].chars().nth(char_x).unwrap_or(' ');

                    // If we hit a letter (non-space), apply gentle damping
                    if ch != ' ' {
                        // Simple collision with gentle damping - no bouncing energy
                        let cell_center_x =
                            art_left + char_x as f32 * self.char_size + self.char_size / 2.0;
                        let cell_center_y =
                            art_top + char_y as f32 * self.char_size + self.char_size / 2.0;

                        let dx = p.pos.x - cell_center_x;
                        let dy = p.pos.y - cell_center_y;

                        // Normal reflection without energy boost, just damping
                        if dx.abs() > dy.abs() {
                            p.vel.x = -p.vel.x;
                        } else {
                            p.vel.y = -p.vel.y;
                        }

                        // Apply gentle damping - 10x stronger
                        p.vel.x *= 0.99; // Stronger damping (0.99 instead of 0.999)
                        p.vel.y *= 0.99;

                        // Push particle away from the letter center to prevent overlap
                        let push_distance = 2.0;
                        if dx != 0.0 || dy != 0.0 {
                            let distance = (dx * dx + dy * dy).sqrt();
                            if distance > 0.0 {
                                p.pos.x += (dx / distance) * push_distance;
                                p.pos.y += (dy / distance) * push_distance;
                            }
                        }

                        // Find the corresponding SplashChar and change its color temporarily
                        for splash_char in &mut self.splash_chars {
                            if splash_char.row == char_y && splash_char.col == char_x {
                                splash_char.color = Self::random_color();
                                splash_char.color_timer = 2.0; // 2 seconds
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Remove stuck particles and spawn new ones
        for &i in particles_to_remove.iter().rev() {
            self.splash_particles.remove(i);

            // Spawn replacement particle at random location
            let x = rng.f32() * width;
            let y = rng.f32() * height;
            let pos = Pos2::new(x, y);
            let vel = EVec2::new(
                (rng.f32() - 0.5) * 2.0, // Slower initial velocity for respawned particles too
                (rng.f32() - 0.5) * 2.0,
            );

            self.splash_particles.push(SplashParticle {
                pos,
                vel,
                radius: 3.0,
                stuck_timer: 0.0,
                last_pos: pos,
                charge: 1.0, // Positive charge
            });
        }

        // Particle-to-particle collision detection
        let len = self.splash_particles.len();
        for i in 0..len {
            for j in i + 1..len {
                let dx = self.splash_particles[i].pos.x - self.splash_particles[j].pos.x;
                let dy = self.splash_particles[i].pos.y - self.splash_particles[j].pos.y;
                let distance_sq = dx * dx + dy * dy;
                let min_distance =
                    self.splash_particles[i].radius + self.splash_particles[j].radius;

                // If particles are colliding
                if distance_sq < min_distance * min_distance && distance_sq > 0.0 {
                    let distance = distance_sq.sqrt();

                    // Normalize collision vector
                    let nx = dx / distance;
                    let ny = dy / distance;

                    // Separate particles to prevent overlap
                    let overlap = min_distance - distance;
                    let separation = overlap * 0.5;

                    self.splash_particles[i].pos.x += nx * separation;
                    self.splash_particles[i].pos.y += ny * separation;
                    self.splash_particles[j].pos.x -= nx * separation;
                    self.splash_particles[j].pos.y -= ny * separation;

                    // Calculate relative velocity
                    let rel_vel_x = self.splash_particles[i].vel.x - self.splash_particles[j].vel.x;
                    let rel_vel_y = self.splash_particles[i].vel.y - self.splash_particles[j].vel.y;

                    // Calculate relative velocity along collision normal
                    let vel_along_normal = rel_vel_x * nx + rel_vel_y * ny;

                    // Don't resolve if velocities are separating
                    if vel_along_normal > 0.0 {
                        continue;
                    }

                    // Restitution (bounciness) - reduced for gentler collisions
                    let restitution = 0.9; // Reduced from 1.2 to 0.9
                    let impulse_scalar = -(1.0 + restitution) * vel_along_normal;

                    // Apply impulse (assuming equal mass)
                    let impulse_x = impulse_scalar * nx * 0.5;
                    let impulse_y = impulse_scalar * ny * 0.5;

                    self.splash_particles[i].vel.x += impulse_x;
                    self.splash_particles[i].vel.y += impulse_y;
                    self.splash_particles[j].vel.x -= impulse_x;
                    self.splash_particles[j].vel.y -= impulse_y;
                }
            }
        }
    }

    fn start_selected_scenario(&mut self) {
        if self.selected_scenario == 0 {
            if crate::scenario::load_and_apply_scenario().is_err() {
                let _ = crate::scenario::load_hardcoded_scenario();
            }
        } else {
            let name = &self.scenarios[self.selected_scenario];
            let path = format!("saved_state/{}.json", name);
            if let Some(tx) = SIM_COMMAND_SENDER.lock().as_ref() {
                let _ = tx.send(SimCommand::LoadState { path });
            }
        }
        self.show_splash = false;
    }
}

#[cfg(test)]
mod tests;
