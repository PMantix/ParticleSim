use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Sender};

use crate::body::Body;
use crate::body::foil::{Foil, LinkMode};
use crate::config;
use crate::quadtree::Node;

pub static TIMESTEP: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(config::DEFAULT_DT));
pub static FIELD_MAGNITUDE: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.0));
pub static FIELD_DIRECTION: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(180.0));
pub static PAUSED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
pub static UPDATE_LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
pub static BODIES: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static QUADTREE: Lazy<Mutex<Vec<Node>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static FOILS: Lazy<Mutex<Vec<Foil>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static SPAWN: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static COLLISION_PASSES: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(3));
pub static SIM_TIME: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.0));

//Simulation commands
// These are used to send commands to the simulation thread from the GUI thread
pub enum SimCommand {
    ChangeCharge {id: u64, delta: f32},
    AddBody { body: Body },
    DeleteAll,
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
    },
    AddFoil {
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        particle_radius: f32,
        current: f32,
    },
    LinkFoils { a: u64, b: u64, mode: LinkMode },
    UnlinkFoils { a: u64, b: u64 },
    SetFoilCurrent {
        foil_id: u64,
        current: f32,
    },
    SetFoilFrequency {
        foil_id: u64,
        switch_hz: f32,
    },
    SaveState { path: String },
    LoadState { path: String },
    StepOnce
}

pub static SIM_COMMAND_SENDER: Lazy<Mutex<Option<Sender<SimCommand>>>> = Lazy::new(|| Mutex::new(None));