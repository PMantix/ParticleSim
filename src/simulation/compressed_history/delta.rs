// Delta computation and reconstruction logic
// Separated from types to reduce compilation overhead

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ultraviolet::Vec2;

use super::types::{LightBody, LightFoil};
// Species import removed - not needed in delta module

/// Error type for frame reconstruction
#[derive(Debug)]
pub enum ReconstructionError {
    MissingKeyframe(usize),
    InvalidDelta,
    InconsistentState,
}

/// Delta representing changes to a body between frames
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BodyDelta {
    /// Body ID this delta applies to
    pub id: u64,
    /// Position change (if significant)
    pub pos_delta: Option<Vec2>,
    /// Z position change (if significant)
    pub z_delta: Option<f32>,
    /// Velocity change (if significant)
    pub vel_delta: Option<Vec2>,
    /// Z velocity change (if significant)
    pub vz_delta: Option<f32>,
    /// Charge change (if significant)  
    pub charge_delta: Option<f32>,
    /// New electron positions (if electrons moved significantly)
    pub electrons: Option<Vec<crate::body::Electron>>,
}

/// Delta representing changes to a foil between frames
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoilDelta {
    /// Foil ID this delta applies to
    pub id: u64,
    /// DC current change
    pub dc_current_delta: Option<f32>,
    /// AC current change
    pub ac_current_delta: Option<f32>,
    /// Switch frequency change
    pub switch_hz_delta: Option<f32>,
    /// Link ID change
    pub link_id_delta: Option<Option<u64>>,
    /// Slave overpotential current change
    pub slave_overpotential_current_delta: Option<f32>,
    /// Body IDs change (if bodies were added/removed)
    pub body_ids_delta: Option<Vec<u64>>,
}

/// Complete delta frame containing all changes between two snapshots
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeltaSnapshot {
    /// Frame number this delta applies to
    pub frame: usize,
    /// Simulation time delta
    pub dt_delta: Option<f32>,
    /// Thermostat time delta
    pub thermostat_delta: Option<f32>,
    /// Body changes
    pub body_deltas: Vec<BodyDelta>,
    /// New bodies added
    pub new_bodies: Vec<LightBody>,
    /// Foil changes
    pub foil_deltas: Vec<FoilDelta>,
    /// New foils added  
    pub new_foils: Vec<LightFoil>,
    /// Body-to-foil mapping changes
    pub body_to_foil_changes: Option<HashMap<u64, Option<u64>>>, // None = removed mapping
    /// Config changes (rare but possible)
    pub config_delta: Option<crate::config::SimConfig>,
    /// Domain dimension changes
    pub domain_delta: Option<(f32, f32, f32)>, // (width, height, depth)
}

/// Compute delta for a body if it has changed significantly
pub fn compute_body_delta(current: &LightBody, previous: &LightBody) -> Option<BodyDelta> {
    let thresholds = super::config::ChangeThresholds::default();
    
    if !current.has_changed(previous, &thresholds) {
        return None;
    }

    // Compute individual deltas only for significant changes
    let pos_delta = if (current.pos - previous.pos).mag() > thresholds.position_epsilon {
        Some(current.pos - previous.pos)
    } else {
        None
    };
    
    let z_delta = if (current.z - previous.z).abs() > thresholds.z_epsilon {
        Some(current.z - previous.z)
    } else {
        None
    };

    let vel_delta = if (current.vel - previous.vel).mag() > thresholds.velocity_epsilon {
        Some(current.vel - previous.vel)
    } else {
        None
    };
    
    let vz_delta = if (current.vz - previous.vz).abs() > thresholds.velocity_epsilon {
        Some(current.vz - previous.vz)
    } else {
        None
    };

    let charge_delta = if (current.charge - previous.charge).abs() > thresholds.charge_epsilon {
        Some(current.charge - previous.charge)
    } else {
        None
    };

    // Check for electron changes
    let electrons = if current.electrons.len() != previous.electrons.len() ||
        current.electrons.iter().zip(previous.electrons.iter()).any(|(a, b)| {
            (a.rel_pos - b.rel_pos).mag() > thresholds.electron_epsilon
        }) {
        Some(current.electrons.clone())
    } else {
        None
    };

    Some(BodyDelta {
        id: 0, // Will be set by caller
        pos_delta,
        z_delta,
        vel_delta,
        vz_delta,
        charge_delta,
        electrons,
    })
}

/// Compute delta for a foil if it has changed
pub fn compute_foil_delta(current: &LightFoil, previous: &LightFoil) -> Option<FoilDelta> {
    if !current.has_changed(previous) {
        return None;
    }

    Some(FoilDelta {
        id: current.id,
        dc_current_delta: if current.dc_current != previous.dc_current {
            Some(current.dc_current - previous.dc_current)
        } else {
            None
        },
        ac_current_delta: if current.ac_current != previous.ac_current {
            Some(current.ac_current - previous.ac_current)
        } else {
            None
        },
        switch_hz_delta: if current.switch_hz != previous.switch_hz {
            Some(current.switch_hz - previous.switch_hz)
        } else {
            None
        },
        link_id_delta: if current.link_id != previous.link_id {
            Some(current.link_id)
        } else {
            None
        },
        slave_overpotential_current_delta: if current.slave_overpotential_current != previous.slave_overpotential_current {
            Some(current.slave_overpotential_current - previous.slave_overpotential_current)
        } else {
            None
        },
        body_ids_delta: if current.body_ids != previous.body_ids {
            Some(current.body_ids.clone())
        } else {
            None
        },
    })
}

/// Apply a body delta to reconstruct the current state
pub fn apply_body_delta(previous: &mut LightBody, delta: &BodyDelta) -> Result<(), ReconstructionError> {
    if let Some(pos_delta) = delta.pos_delta {
        previous.pos += pos_delta;
    }
    
    if let Some(z_delta) = delta.z_delta {
        previous.z += z_delta;
    }
    
    if let Some(vel_delta) = delta.vel_delta {
        previous.vel += vel_delta;
    }
    
    if let Some(vz_delta) = delta.vz_delta {
        previous.vz += vz_delta;
    }
    
    if let Some(charge_delta) = delta.charge_delta {
        previous.charge += charge_delta;
    }
    
    if let Some(ref electrons) = delta.electrons {
        previous.electrons = electrons.clone();
    }
    
    Ok(())
}

/// Apply a foil delta to reconstruct the current state
pub fn apply_foil_delta(previous: &mut LightFoil, delta: &FoilDelta) -> Result<(), ReconstructionError> {
    if let Some(dc_delta) = delta.dc_current_delta {
        previous.dc_current += dc_delta;
    }
    
    if let Some(ac_delta) = delta.ac_current_delta {
        previous.ac_current += ac_delta;
    }
    
    if let Some(hz_delta) = delta.switch_hz_delta {
        previous.switch_hz += hz_delta;
    }
    
    if let Some(link_delta) = delta.link_id_delta {
        previous.link_id = link_delta;
    }
    
    if let Some(slave_delta) = delta.slave_overpotential_current_delta {
        previous.slave_overpotential_current += slave_delta;
    }
    
    if let Some(ref body_ids) = delta.body_ids_delta {
        previous.body_ids = body_ids.clone();
    }
    
    Ok(())
}