use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::Path;
use crate::profile_scope;

use crate::simulation::Simulation;
use crate::body::{Body, foil::Foil};
use crate::config::SimConfig;

#[derive(Serialize, Deserialize)]
pub struct SimulationState {
    pub bodies: Vec<Body>,
    pub foils: Vec<Foil>,
    pub body_to_foil: HashMap<u64, u64>,
    pub config: SimConfig,
}

impl SimulationState {
    pub fn from_simulation(sim: &Simulation) -> Self {
        Self {
            bodies: sim.bodies.clone(),
            foils: sim.foils.clone(),
            body_to_foil: sim.body_to_foil.clone(),
            config: sim.config.clone(),
        }
    }

    pub fn apply_to(self, sim: &mut Simulation) {
        sim.bodies = self.bodies;
        sim.foils = self.foils;
        sim.body_to_foil = self.body_to_foil;
        sim.config = self.config;
        sim.quadtree.build(&mut sim.bodies);
        sim.cell_list.rebuild(&sim.bodies);
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
    let state: SimulationState = serde_json::from_str(&data).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(state)
}
