// diagnostics.rs
// Module providing advanced diagnostics for the simulation, including
// transient transference number calculations.

use crate::body::{Body, Species};
use crate::body::foil::Foil;
use ultraviolet::Vec2;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Write};

/// Record containing transference number information for a single timestep.
#[derive(Clone, Debug)]
pub struct TransferenceRecord {
    pub time: f32,
    pub t_plus: f32,
    pub li_drift: f32,
    pub anion_drift: f32,
    pub drift_direction: Vec2,
}

/// Diagnostic calculator that tracks the transient transference number.
pub struct TransferenceDiagnostics {
    window: usize,
    li_history: VecDeque<f32>,
    anion_history: VecDeque<f32>,
    pub records: Vec<TransferenceRecord>,
}

impl TransferenceDiagnostics {
    /// Create a new diagnostics object with the specified averaging window.
    /// A window of 1 disables time averaging.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            li_history: VecDeque::new(),
            anion_history: VecDeque::new(),
            records: Vec::new(),
        }
    }

    /// Update the diagnostic using the provided bodies and foils.
    /// If `region` is `Some((center, radius))`, only bodies within that
    /// region are considered.
    pub fn update(
        &mut self,
        bodies: &[Body],
        foils: &[Foil],
        time: f32,
        region: Option<(Vec2, f32)>,
    ) {
        if let Some((li_drift, an_drift, dir, li_count, an_count)) =
            compute_transference(bodies, foils, region)
        {
            self.li_history.push_back(li_drift);
            self.anion_history.push_back(an_drift);
            if self.li_history.len() > self.window {
                self.li_history.pop_front();
            }
            if self.anion_history.len() > self.window {
                self.anion_history.pop_front();
            }

            let li_avg = self.li_history.iter().copied().sum::<f32>()
                / self.li_history.len() as f32;
            let an_avg = self.anion_history.iter().copied().sum::<f32>()
                / self.anion_history.len() as f32;

            let num = li_count as f32 * li_avg;
            let den = num + an_count as f32 * an_avg.abs();
            if den > 0.0 {
                let record = TransferenceRecord {
                    time,
                    t_plus: num / den,
                    li_drift: li_avg,
                    anion_drift: an_avg,
                    drift_direction: dir,
                };
                self.records.push(record);
            }
        }
    }

    /// Write the recorded data to a CSV file.
    #[allow(dead_code)]
    pub fn export_csv(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "time,t_plus,li_drift,anion_drift,dir_x,dir_y")?;
        for r in &self.records {
            writeln!(
                writer,
                "{},{},{},{},{},{}",
                r.time,
                r.t_plus,
                r.li_drift,
                r.anion_drift,
                r.drift_direction.x,
                r.drift_direction.y
            )?;
        }
        Ok(())
    }
}

fn foil_center(foil: &Foil, bodies: &[Body]) -> Option<Vec2> {
    let mut sum = Vec2::zero();
    let mut count = 0;
    for id in &foil.body_ids {
        if let Some(b) = bodies.iter().find(|b| b.id == *id) {
            sum += b.pos;
            count += 1;
        }
    }
    if count > 0 {
        Some(sum / count as f32)
    } else {
        None
    }
}

fn drift_direction(foils: &[Foil], bodies: &[Body]) -> Option<Vec2> {
    let centers: Vec<Vec2> = foils.iter().filter_map(|f| foil_center(f, bodies)).collect();
    if centers.len() < 2 {
        return None;
    }
    let mut pair = (centers[0], centers[1]);
    let mut max_d = (centers[0] - centers[1]).mag_sq();
    for i in 0..centers.len() {
        for j in (i + 1)..centers.len() {
            let d = (centers[j] - centers[i]).mag_sq();
            if d > max_d {
                max_d = d;
                pair = (centers[i], centers[j]);
            }
        }
    }
    let dir = pair.1 - pair.0;
    let mag = dir.mag();
    if mag > 1e-6 { Some(dir / mag) } else { None }
}

fn compute_transference(
    bodies: &[Body],
    foils: &[Foil],
    region: Option<(Vec2, f32)>,
) -> Option<(f32, f32, Vec2, usize, usize)> {
    let dir = drift_direction(foils, bodies)?;
    let mut li_sum = 0.0;
    let mut an_sum = 0.0;
    let mut li_count = 0;
    let mut an_count = 0;

    for body in bodies {
        if let Some((center, radius)) = region {
            if (body.pos - center).mag() > radius {
                continue;
            }
        }
        match body.species {
            Species::LithiumIon => {
                li_sum += body.vel.dot(dir);
                li_count += 1;
            }
            Species::ElectrolyteAnion => {
                an_sum += body.vel.dot(dir);
                an_count += 1;
            }
            _ => {}
        }
    }
    if li_count + an_count == 0 {
        return None;
    }
    let li_avg = if li_count > 0 { li_sum / li_count as f32 } else { 0.0 };
    let an_avg = if an_count > 0 { an_sum / an_count as f32 } else { 0.0 };
    Some((li_avg, an_avg, dir, li_count, an_count))
}
