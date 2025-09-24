use crate::profile_scope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::body::{foil::Foil, Body};
use crate::config::SimConfig;
use crate::simulation::Simulation;

#[derive(Clone, Serialize, Deserialize)]
pub struct SimulationState {
    pub bodies: Vec<Body>,
    pub foils: Vec<Foil>,
    pub body_to_foil: HashMap<u64, u64>,
    pub config: SimConfig,
    #[serde(default = "default_domain_width")]
    pub domain_width: f32,
    #[serde(default = "default_domain_height")]
    pub domain_height: f32,
    #[serde(default = "default_domain_depth")]
    pub domain_depth: f32,
    #[serde(default = "default_frame")]
    pub frame: usize,
    #[serde(default = "default_sim_time")]
    pub sim_time: f32,
    #[serde(default = "default_dt")]
    pub dt: f32,
    #[serde(default = "default_last_thermostat_time")]
    pub last_thermostat_time: f32,
    /// Switching charging step at the time of capture (0..3). None if not running.
    #[serde(default)]
    pub switch_step: Option<u8>,
}

fn default_domain_width() -> f32 {
    600.0 // Default domain width
}

fn default_domain_height() -> f32 {
    400.0 // Default domain height
}

fn default_domain_depth() -> f32 {
    crate::config::DOMAIN_DEPTH
}

fn default_frame() -> usize {
    0
}

fn default_sim_time() -> f32 {
    0.0
}

fn default_dt() -> f32 {
    crate::config::DEFAULT_DT_FS
}

fn default_last_thermostat_time() -> f32 {
    0.0
}

impl SimulationState {
    pub fn from_simulation(sim: &Simulation) -> Self {
        Self {
            bodies: sim.bodies.clone(),
            foils: sim.foils.clone(),
            body_to_foil: sim.body_to_foil.clone(),
            config: sim.config.clone(),
            domain_width: sim.domain_width,
            domain_height: sim.domain_height,
            domain_depth: sim.domain_depth,
            frame: sim.frame,
            sim_time: sim.frame as f32 * sim.dt,
            dt: sim.dt,
            last_thermostat_time: sim.last_thermostat_time,
            switch_step: Some(sim.switch_scheduler.current_step()),
        }
    }

    pub fn apply_to(self, sim: &mut Simulation) {
        sim.bodies = self.bodies;
        sim.foils = self.foils;
        sim.body_to_foil = self.body_to_foil;
        sim.config = self.config;
        sim.domain_width = self.domain_width;
        sim.domain_height = self.domain_height;
        sim.domain_depth = self.domain_depth;
        sim.frame = self.frame;
        sim.dt = self.dt;
        sim.last_thermostat_time = self.last_thermostat_time;
    // Update current switching step for playback visualization
    *crate::renderer::state::SWITCH_STEP.lock() = self.switch_step;

        // Update the shared state for the GUI (convert half-width/height to full width/height)
        *crate::renderer::state::DOMAIN_WIDTH.lock() = self.domain_width * 2.0;
        *crate::renderer::state::DOMAIN_HEIGHT.lock() = self.domain_height * 2.0;
        *crate::renderer::state::TIMESTEP.lock() = self.dt;
        *crate::renderer::state::SIM_TIME.lock() = self.sim_time;

        sim.quadtree.build(&mut sim.bodies);
        sim.cell_list.rebuild(&sim.bodies);
        sim.rewound_flags.resize(sim.bodies.len(), false);
    }
}

pub fn save_state<P: AsRef<Path>>(path: P, sim: &Simulation) -> std::io::Result<()> {
    profile_scope!("save_state");
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let state = SimulationState::from_simulation(sim);
    let json = serde_json::to_string_pretty(&state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

pub fn load_state<P: AsRef<Path>>(path: P) -> std::io::Result<SimulationState> {
    profile_scope!("load_state");
    let data = std::fs::read_to_string(path)?;
    let state: SimulationState = serde_json::from_str(&data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(state)
}
