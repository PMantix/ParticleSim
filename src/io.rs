use crate::profile_scope;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufWriter, Cursor, Read, Write};
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

fn default_history_capacity() -> usize {
    crate::config::PLAYBACK_HISTORY_FRAMES
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedScenario {
    pub current: SimulationState,
    #[serde(default)]
    pub history: Vec<SimulationState>,
    #[serde(default)]
    pub history_cursor: usize,
    #[serde(default = "default_history_capacity")]
    pub history_capacity: usize,
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
    let state = SavedScenario {
        current: SimulationState::from_simulation(sim),
        history: sim.simple_history.iter().cloned().collect(),
        history_cursor: sim.history_cursor,
        history_capacity: sim.history_capacity,
    };
    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);
    let mut encoder = GzEncoder::new(writer, Compression::default());
    serde_json::to_writer(&mut encoder, &state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let mut writer = encoder.finish()?;
    writer.flush()
}

pub fn load_state<P: AsRef<Path>>(path: P) -> std::io::Result<SavedScenario> {
    profile_scope!("load_state");
    let data = std::fs::read(path)?;
    if let Some(decoded) = maybe_decompress_gzip(&data)? {
        parse_saved_scenario_bytes(&decoded)
    } else {
        parse_saved_scenario_bytes(&data)
    }
}

fn parse_saved_scenario_bytes(bytes: &[u8]) -> std::io::Result<SavedScenario> {
    match serde_json::from_slice::<SavedScenario>(bytes) {
        Ok(scenario) => Ok(scenario),
        Err(primary_err) => match serde_json::from_slice::<SimulationState>(bytes) {
            Ok(state) => Ok(SavedScenario {
                current: state,
                history: Vec::new(),
                history_cursor: 0,
                history_capacity: default_history_capacity(),
            }),
            Err(legacy_err) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "failed to parse saved scenario: {}; legacy format error: {}",
                    primary_err, legacy_err
                ),
            )),
        },
    }
}

fn maybe_decompress_gzip(data: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
    if data.len() < 2 || data[0] != 0x1f || data[1] != 0x8b {
        return Ok(None);
    }

    let mut decoder = GzDecoder::new(Cursor::new(data));
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded)?;
    Ok(Some(decoded))
}
