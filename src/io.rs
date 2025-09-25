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
    /// Full switch charging configuration (role assignments, setpoints, timing)
    #[serde(default = "default_switch_config")]
    pub switch_config: crate::switch_charging::SwitchChargingConfig,
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
            switch_config: sim.switch_config.clone(),
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
        // Apply persisted switch charging configuration (ensuring defaults for missing steps)
        sim.switch_config = self.switch_config;
        sim.switch_config.ensure_all_steps();
        // Notify UI of applied configuration if channel exists
        if let Some(tx) = &sim.switch_status_tx {
            let _ = tx.send(crate::switch_charging::SwitchStatus::ConfigApplied(sim.switch_config.clone()));
        }
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

fn default_switch_config() -> crate::switch_charging::SwitchChargingConfig {
    crate::switch_charging::SwitchChargingConfig::default()
}

pub fn save_state<P: AsRef<Path>>(path: P, sim: &Simulation) -> std::io::Result<()> {
    profile_scope!("save_state");
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let include_history = *crate::renderer::state::SAVE_INCLUDE_HISTORY.lock();
    let state = if include_history {
        SavedScenario {
            current: SimulationState::from_simulation(sim),
            history: sim.simple_history.iter().cloned().collect(),
            history_cursor: sim.history_cursor,
            history_capacity: sim.history_capacity,
        }
    } else {
        SavedScenario {
            current: SimulationState::from_simulation(sim),
            history: Vec::new(),
            history_cursor: 0,
            history_capacity: sim.history_capacity,
        }
    };
    // Write to a temporary file first to avoid truncation on crash/interruption
    // Compression preference primarily controlled by global SAVE_COMPRESS; fallback to extension for legacy compatibility
    let use_gzip = *crate::renderer::state::SAVE_COMPRESS.lock();
    let save_format = *crate::renderer::state::SAVE_FORMAT.lock();
    if !use_gzip {
        // If user disabled compression but filename explicitly ends with .gz, still honor preference OFF
        // (we will just write plain JSON even if extension is .gz)
    } else if !use_gzip {
        // unreachable duplicate branch, kept for clarity
    }
    // Legacy compatibility: if global says compress=false but extension is .gz we could force; we choose NOT to override.
    // If global says compress=true but extension not .gz we still compress; caller likely appended .gz already in UI.
    let tmp_path = path.with_extension({
        let mut os = path.extension().map(|e| e.to_os_string()).unwrap_or_default();
        os.push(".tmp");
        os
    });
    {
        let file = std::fs::File::create(&tmp_path)?;
        let writer = BufWriter::new(file);
        match (save_format, use_gzip) {
            (crate::renderer::state::SaveFormat::Json, false) => {
                serde_json::to_writer(writer, &state)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            }
            (crate::renderer::state::SaveFormat::Json, true) => {
                let mut encoder = GzEncoder::new(writer, Compression::fast());
                serde_json::to_writer(&mut encoder, &state)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                let mut writer = encoder.finish()?;
                writer.flush()?;
            }
            (crate::renderer::state::SaveFormat::Binary, false) => {
                // Binary bincode (little-endian, varint) default config
                bincode::serialize_into(writer, &state)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            }
            (crate::renderer::state::SaveFormat::Binary, true) => {
                let mut encoder = GzEncoder::new(writer, Compression::fast());
                bincode::serialize_into(&mut encoder, &state)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                let mut writer = encoder.finish()?;
                writer.flush()?;
            }
        }
    }
    // Atomically replace (on Windows rename over existing is atomic for files not in use)
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

pub fn load_state<P: AsRef<Path>>(path: P) -> std::io::Result<SavedScenario> {
    profile_scope!("load_state");
    let data = match std::fs::read(path.as_ref()) {
        Ok(d) => d,
        Err(e) => return Err(e),
    };
    if let Some(decoded) = maybe_decompress_gzip(&data)? {
        parse_saved_scenario_bytes(&decoded)
    } else {
        parse_saved_scenario_bytes(&data)
    }
}

fn parse_saved_scenario_bytes(bytes: &[u8]) -> std::io::Result<SavedScenario> {
    // Try JSON SavedScenario
    if let Ok(scenario) = serde_json::from_slice::<SavedScenario>(bytes) {
        return Ok(scenario);
    }
    // Try JSON SimulationState (legacy single-state saves)
    if let Ok(state) = serde_json::from_slice::<SimulationState>(bytes) {
        return Ok(SavedScenario {
            current: state,
            history: Vec::new(),
            history_cursor: 0,
            history_capacity: default_history_capacity(),
        });
    }
    // Try binary (bincode) SavedScenario
    if let Ok(scenario) = bincode::deserialize::<SavedScenario>(bytes) {
        return Ok(scenario);
    }
    // Try binary SimulationState
    if let Ok(state) = bincode::deserialize::<SimulationState>(bytes) {
        return Ok(SavedScenario {
            current: state,
            history: Vec::new(),
            history_cursor: 0,
            history_capacity: default_history_capacity(),
        });
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "failed to parse saved scenario: not valid JSON or binary format",
    ))
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
