use std::collections::{HashMap, BTreeSet};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Serialize;
use crate::body::Species;

#[derive(Clone, Serialize)]
pub struct FrameStats {
    pub frame: usize,
    pub time: f32,
    pub species_counts: HashMap<Species, usize>,
    pub total_charge: f32,
    pub ion_concentration: f32,
    pub foil_command_current: HashMap<u64, f32>,
    pub foil_response_current: HashMap<u64, f32>,
    pub charge_bins_x: Vec<f32>,
}

pub struct StatsCollector {
    pub history: Vec<FrameStats>,
    pub bins: usize,
    pub bounds: f32,
}

impl StatsCollector {
    pub fn new(bounds: f32, bins: usize) -> Self {
        Self { history: Vec::new(), bins, bounds }
    }

    pub fn record(&mut self, sim: &crate::simulation::Simulation, foil_response: HashMap<u64, f32>) {
        let frame = sim.frame;
        let time = frame as f32 * sim.dt;
        let mut species_counts: HashMap<Species, usize> = HashMap::new();
        let mut total_charge = 0.0f32;
        let mut ion_count = 0usize;
        let mut metal_ion_total = 0usize;
        let mut charge_bins_x = vec![0.0f32; self.bins];

        for b in &sim.bodies {
            *species_counts.entry(b.species).or_insert(0) += 1;
            total_charge += b.charge;
            match b.species {
                Species::LithiumIon => { ion_count += 1; metal_ion_total += 1; }
                Species::LithiumMetal | Species::FoilMetal => { metal_ion_total += 1; }
                _ => {}
            }
            let norm = ((b.pos.x + self.bounds) / (2.0 * self.bounds)).clamp(0.0, 1.0);
            let idx = (norm * self.bins as f32).floor() as usize;
            if idx < self.bins { charge_bins_x[idx] += b.charge; }
        }
        let ion_concentration = if metal_ion_total > 0 {
            ion_count as f32 / metal_ion_total as f32
        } else { 0.0 };

        let mut foil_command_current = HashMap::new();
        for foil in &sim.foils {
            let current = if foil.switch_hz > 0.0 {
                let ac = if (time * foil.switch_hz) % 1.0 < 0.5 { foil.ac_current } else { -foil.ac_current };
                foil.dc_current + ac
            } else {
                foil.current
            };
            foil_command_current.insert(foil.id, current);
        }

        let stats = FrameStats {
            frame,
            time,
            species_counts,
            total_charge,
            ion_concentration,
            foil_command_current,
            foil_response_current: foil_response,
            charge_bins_x,
        };
        self.history.push(stats);
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.history).unwrap_or_default()
    }

    pub fn to_csv(&self) -> String {
        use std::fmt::Write;
        let mut foil_ids: BTreeSet<u64> = BTreeSet::new();
        for s in &self.history { for (&id, _) in &s.foil_command_current { foil_ids.insert(id); } }
        for s in &self.history { for (&id, _) in &s.foil_response_current { foil_ids.insert(id); } }
        let species_list = [
            Species::LithiumIon,
            Species::LithiumMetal,
            Species::FoilMetal,
            Species::ElectrolyteAnion,
        ];
        let mut out = String::new();
        write!(out, "frame,time,total_charge,ion_concentration").unwrap();
        for sp in &species_list { write!(out, ",count_{:?}", sp).unwrap(); }
        for id in &foil_ids { write!(out, ",cmd_{id},resp_{id}").unwrap(); }
        writeln!(out).unwrap();
        for s in &self.history {
            write!(out, "{},{}", s.frame, s.time).unwrap();
            write!(out, ",{}", s.total_charge).unwrap();
            write!(out, ",{}", s.ion_concentration).unwrap();
            for sp in &species_list {
                let c = s.species_counts.get(sp).copied().unwrap_or(0);
                write!(out, ",{}", c).unwrap();
            }
            for id in &foil_ids {
                let cmd = s.foil_command_current.get(id).copied().unwrap_or(0.0);
                let resp = s.foil_response_current.get(id).copied().unwrap_or(0.0);
                write!(out, ",{},{}", cmd, resp).unwrap();
            }
            writeln!(out).unwrap();
        }
        out
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new(crate::config::DOMAIN_BOUNDS, 50)
    }
}

pub static ANALYSIS: Lazy<Mutex<StatsCollector>> = Lazy::new(|| Mutex::new(StatsCollector::default()));
