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
use std::path::PathBuf;
use image;
use wgpu;
use pollster;

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
    // Screen capture fields
    recording: bool,
    record_rect: Option<(Vec2, Vec2)>,
    capture_interval: f32,
    next_capture_time: f32,
    output_dir: std::path::PathBuf,
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
            recording: false,
            record_rect: None,
            capture_interval: 0.1,
            next_capture_time: 0.0,
            output_dir: std::path::PathBuf::from("captures"),
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

    fn post_render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, width: u32, height: u32) {
        self.capture_frame(device, queue, texture, width, height);
    }
}

impl Renderer {
    pub fn start_recording(&mut self) {
        self.recording = true;
        self.record_rect = None;
        self.next_capture_time = *crate::renderer::state::SIM_TIME.lock() + self.capture_interval;
    }

    fn save_rgba_png(&self, data: &[u8], width: u32, height: u32, sim_time: f32) {
        std::fs::create_dir_all(&self.output_dir).ok();
        let fname = format!("frame_{:.2}.png", sim_time).replace('.', "_");
        let path = self.output_dir.join(fname);
        let img = image::RgbaImage::from_raw(width, height, data.to_vec()).unwrap();
        img.save(path).unwrap();
    }

    fn world_to_pixel(&self, world: Vec2, width: u32, height: u32) -> (u32, u32) {
        let width_pixels = width as f32 * self.scale_factor;
        let height_pixels = height as f32 * self.scale_factor;
        let rel = (world - self.pos) / self.scale;
        let mx = (rel.x + width_pixels / height_pixels) * height_pixels / 2.0;
        let my = (1.0 - rel.y) * height_pixels / 2.0;
        let x = mx.clamp(0.0, width_pixels - 1.0).round() as u32;
        let y = my.clamp(0.0, height_pixels - 1.0).round() as u32;
        (x, y)
    }

    pub fn capture_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, width: u32, height: u32) {
        let sim_time = *crate::renderer::state::SIM_TIME.lock();
        if !self.recording || sim_time < self.next_capture_time {
            return;
        }
        self.next_capture_time += self.capture_interval;

        let buffer_size = (width * height * 4) as wgpu::BufferAddress;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("capture_encoder") });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(std::num::NonZeroU32::new(4 * width).unwrap()),
                    rows_per_image: Some(std::num::NonZeroU32::new(height).unwrap()),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        queue.submit(std::iter::once(encoder.finish()));

        device.poll(wgpu::Maintain::Wait);
        let slice = buffer.slice(..);
        pollster::block_on(slice.map_async(wgpu::MapMode::Read)).unwrap();
        let data = slice.get_mapped_range().to_vec();
        buffer.unmap();

        let mut rgba = vec![0u8; data.len()];
        for i in 0..(width * height) as usize {
            rgba[i * 4] = data[i * 4 + 2];
            rgba[i * 4 + 1] = data[i * 4 + 1];
            rgba[i * 4 + 2] = data[i * 4];
            rgba[i * 4 + 3] = data[i * 4 + 3];
        }

        let (mut final_w, mut final_h) = (width, height);
        let mut final_data = rgba;
        if let Some((a, b)) = self.record_rect {
            let (x0, y0) = self.world_to_pixel(a, width, height);
            let (x1, y1) = self.world_to_pixel(b, width, height);
            let min_x = x0.min(x1);
            let min_y = y0.min(y1);
            let max_x = x0.max(x1);
            let max_y = y0.max(y1);
            final_w = max_x - min_x;
            final_h = max_y - min_y;
            let mut cropped = vec![0u8; (final_w * final_h * 4) as usize];
            for y in 0..final_h {
                let src = ((min_y + y) * width + min_x) as usize * 4;
                let dst = (y * final_w) as usize * 4;
                let len = (final_w * 4) as usize;
                cropped[dst..dst + len].copy_from_slice(&final_data[src..src + len]);
            }
            final_data = cropped;
        }

        self.save_rgba_png(&final_data, final_w, final_h, sim_time);
    }
}

#[cfg(test)]
mod tests;

