use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;

use crate::body::foil::{Foil, LinkMode};
use crate::body::Body;
use crate::config;
use crate::manual_measurement::{ManualMeasurementConfig, MeasurementResult};
use crate::quadtree::Node;

pub static TIMESTEP: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(config::DEFAULT_DT_FS));
pub static FIELD_MAGNITUDE: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.0));
pub static FIELD_DIRECTION: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(180.0));
pub static PAUSED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
pub static UPDATE_LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
pub static BODIES: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static QUADTREE: Lazy<Mutex<Vec<Node>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static FOILS: Lazy<Mutex<Vec<Foil>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static SPAWN: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static COLLISION_PASSES: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(config::COLLISION_PASSES));
pub static SIM_TIME: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.0));
// Current switching step index (0..3) for history playback highlighting
pub static SWITCH_STEP: Lazy<Mutex<Option<u8>>> = Lazy::new(|| Mutex::new(None));
pub static SHOW_Z_VISUALIZATION: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
pub static Z_VISUALIZATION_STRENGTH: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(1.0));
pub static DOMAIN_WIDTH: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(300.0)); // Default domain width
pub static DOMAIN_HEIGHT: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(300.0)); // Default domain height
                                                                              // Whether to compress saved scenarios (gzip). Default: true (compression on)
pub static SAVE_COMPRESS: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));
// Whether to include playback history when saving. Large size impact. Default: true for backward compatibility.
pub static SAVE_INCLUDE_HISTORY: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));
// Last applied thermostat scale factor for diagnostics
pub static LAST_THERMOSTAT_SCALE: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(1.0));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveFormat {
    Json,
    Binary,
}

impl SaveFormat {
    pub fn extension(self, compressed: bool) -> &'static str {
        match (self, compressed) {
            (SaveFormat::Json, true) => "json.gz",
            (SaveFormat::Json, false) => "json",
            (SaveFormat::Binary, true) => "bin.gz",
            (SaveFormat::Binary, false) => "bin",
        }
    }
}

pub static SAVE_FORMAT: Lazy<Mutex<SaveFormat>> = Lazy::new(|| Mutex::new(SaveFormat::Binary));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackModeStatus {
    Live,
    HistoryPaused,
    HistoryPlaying,
}

#[derive(Clone, Debug)]
pub struct PlaybackStatus {
    pub history_len: usize,
    pub latest_index: usize,
    pub cursor: usize,
    pub is_playing: bool,
    pub mode: PlaybackModeStatus,
    pub speed: f32,
    pub sim_time: f32,
    pub frame: usize,
    pub dt: f32,
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        Self {
            history_len: 0,
            latest_index: 0,
            cursor: 0,
            is_playing: false,
            mode: PlaybackModeStatus::Live,
            speed: 1.0,
            sim_time: 0.0,
            frame: 0,
            dt: crate::config::DEFAULT_DT_FS,
        }
    }
}

pub static PLAYBACK_STATUS: Lazy<Mutex<PlaybackStatus>> =
    Lazy::new(|| Mutex::new(PlaybackStatus::default()));

// Persisted UI controls (optional) so saves/loads can restore GUI selections
pub static PERSIST_UI_CHARGING_MODE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None)); // "Conventional" | "SwitchCharging"
pub static PERSIST_UI_CONV_IS_OVER: Lazy<Mutex<Option<bool>>> = Lazy::new(|| Mutex::new(None));
pub static PERSIST_UI_CONV_CURRENT: Lazy<Mutex<Option<f32>>> = Lazy::new(|| Mutex::new(None));
pub static PERSIST_UI_CONV_TARGET: Lazy<Mutex<Option<f32>>> = Lazy::new(|| Mutex::new(None));
// When true, Renderer should sync persisted UI values once (typically after load)
pub static PERSIST_UI_DIRTY: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

// PID Graph state
pub static SHOW_PID_GRAPH: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
//pub static PID_GRAPH_HISTORY_SIZE: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(1000));

//Simulation commands
// These are used to send commands to the simulation thread from the GUI thread
#[allow(dead_code)]
pub enum SimCommand {
    ChangeCharge {
        id: u64,
        delta: f32,
    },
    AddBody {
        body: Body,
    },
    DeleteAll,
    ResetFoilIds,
    DeleteSpecies {
        species: crate::body::Species,
    },
    AddCircle {
        body: crate::body::Body,
        x: f32,
        y: f32,
        radius: f32,
    },
    AddRing {
        body: crate::body::Body,
        x: f32,
        y: f32,
        radius: f32,
    },
    AddRectangle {
        body: crate::body::Body,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
    },
    AddRandom {
        body: crate::body::Body,
        count: usize,
        domain_width: f32,
        domain_height: f32,
    },
    AddFoil {
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        particle_radius: f32,
        current: f32,
    },
    LinkFoils {
        a: u64,
        b: u64,
        mode: LinkMode,
    },
    UnlinkFoils {
        a: u64,
        b: u64,
    },
    SetFoilCurrent {
        foil_id: u64,
        current: f32,
    },
    SetFoilDCCurrent {
        foil_id: u64,
        dc_current: f32,
    },
    SetFoilACCurrent {
        foil_id: u64,
        ac_current: f32,
    },
    SetFoilFrequency {
        foil_id: u64,
        switch_hz: f32,
    },
    SetFoilChargingMode {
        foil_id: u64,
        mode: crate::body::foil::ChargingMode,
    },
    SetFoilOverpotentialTarget {
        foil_id: u64,
        target_ratio: f32,
    },
    SetFoilPIDGains {
        foil_id: u64,
        kp: f32,
        ki: f32,
        kd: f32,
    },
    SetPIDHistorySize {
        foil_id: u64,
        history_size: usize,
    },
    EnableOverpotentialMode {
        foil_id: u64,
        target_ratio: f32,
    },
    DisableOverpotentialMode {
        foil_id: u64,
    },
    // Group linking controls
    SetFoilGroups {
        group_a: Vec<u64>,
        group_b: Vec<u64>,
    },
    ClearFoilGroups,
    // Conventional grouped controls
    ConventionalSetCurrent {
        current: f32,
    },
    ConventionalSetOverpotential {
        target_ratio: f32,
    },
    SaveState {
        path: String,
    },
    LoadState {
        path: String,
    },
    StepOnce,
    SetDomainSize {
        width: f32,
        height: f32,
    },
    SetTemperature {
        temperature: f32,
    },
    SetOutOfPlane {
        enabled: bool,
        max_z: f32,
        z_stiffness: f32,
        z_damping: f32,
    },
    ToggleZVisualization {
        enabled: bool,
    },
    SetZVisualizationStrength {
        strength: f32,
    },
    PlaybackSeek {
        index: usize,
    },
    PlaybackPlay {
        auto_resume: bool,
    },
    PlaybackPause,
    PlaybackSetSpeed {
        speed: f32,
    },
    PlaybackResumeLive,
    PlaybackResumeFromCurrent,
    ResetTime,
    // Manual measurement commands
    StartManualMeasurement {
        config: ManualMeasurementConfig,
    },
    StopManualMeasurement,
    // Foil mass update command
    UpdateFoilMasses {
        mass: f32,
    },
}

pub static SIM_COMMAND_SENDER: Lazy<Mutex<Option<Sender<SimCommand>>>> =
    Lazy::new(|| Mutex::new(None));

// Manual measurement recorder shared state - stores latest measurements
pub static MANUAL_MEASUREMENT_RESULTS: Lazy<Mutex<Vec<MeasurementResult>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

// Foil metrics logging global controls (GUI -> Simulation bridge)
pub static FOIL_METRICS_ENABLED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(true));
pub static FOIL_METRICS_FILENAME_OVERRIDE: Lazy<Mutex<Option<String>>> =
    Lazy::new(|| Mutex::new(None));
pub static FOIL_METRICS_USE_SEPARATE_INTERVAL: Lazy<AtomicBool> =
    Lazy::new(|| AtomicBool::new(false));
pub static FOIL_METRICS_INTERVAL_FS: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(1000.0));

// Foil metrics field selection
pub static FOIL_METRICS_INCLUDE_SETPOINT: Lazy<std::sync::atomic::AtomicBool> =
    Lazy::new(|| std::sync::atomic::AtomicBool::new(true));
pub static FOIL_METRICS_INCLUDE_ACTUAL_RATIO: Lazy<std::sync::atomic::AtomicBool> =
    Lazy::new(|| std::sync::atomic::AtomicBool::new(true));
pub static FOIL_METRICS_INCLUDE_DELTA_ELECTRONS: Lazy<std::sync::atomic::AtomicBool> =
    Lazy::new(|| std::sync::atomic::AtomicBool::new(true));
pub static FOIL_METRICS_INCLUDE_LI_METAL: Lazy<std::sync::atomic::AtomicBool> =
    Lazy::new(|| std::sync::atomic::AtomicBool::new(true));
