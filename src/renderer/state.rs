use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Sender, Receiver, channel};

use crate::body::Body;
use crate::quadtree::Node;

pub static TIMESTEP: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.015));
pub static FIELD_MAGNITUDE: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.0));
pub static FIELD_DIRECTION: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.0));
pub static PAUSED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
pub static UPDATE_LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
pub static BODIES: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static QUADTREE: Lazy<Mutex<Vec<Node>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static SPAWN: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static COLLISION_PASSES: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(4));

//Simulation commands
// These are used to send commands to the simulation thread from the GUI thread
pub enum SimCommand {
    ChangeCharge {id: u64, delta: f32},
}

pub static SIM_COMMAND_SENDER: Lazy<Mutex<Option<Sender<SimCommand>>>> = Lazy::new(|| Mutex::new(None));