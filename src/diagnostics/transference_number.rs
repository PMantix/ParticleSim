// diagnostics/transference_number.rs
// Implementation of transient transference number diagnostic

use crate::body::{Body, Species};
use crate::profile_scope;
use ultraviolet::Vec2;

pub struct TransferenceNumberDiagnostic {
    pub drift_direction: Vec2,
    pub lithium_drift_velocity: f32,
    pub anion_drift_velocity: f32,
    pub transference_number: f32,
    pub li_current_contribution: f32,
    pub anion_current_contribution: f32,
    pub total_current: f32,
}

impl TransferenceNumberDiagnostic {
    pub fn new() -> Self {
        Self {
            drift_direction: Vec2::zero(),
            lithium_drift_velocity: 0.0,
            anion_drift_velocity: 0.0,
            transference_number: 0.0,
            li_current_contribution: 0.0,
            anion_current_contribution: 0.0,
            total_current: 0.0,
        }
    }

    pub fn calculate(&mut self, bodies: &[Body]) {
        profile_scope!("transference_calculation_internal");
        let mut lithium_velocities = Vec::new();
        let mut anion_velocities = Vec::new();

        // Calculate drift direction based on foil clusters
        let foil_positions: Vec<Vec2> = bodies
            .iter()
            .filter(|b| b.species == Species::FoilMetal)
            .map(|b| b.pos)
            .collect();

        if foil_positions.len() >= 2 {
            // Find the two most distant foil clusters by calculating mean positions
            // For simplicity, we'll use the first two foils for now
            // In a more sophisticated implementation, we'd cluster the foils first
            let mean_position_1 = foil_positions[0];
            let mean_position_2 = foil_positions[1];
            let direction_vec = mean_position_2 - mean_position_1;
            if direction_vec.mag() > 1e-6 {
                self.drift_direction = direction_vec.normalized();
            } else {
                // Fallback to x-axis if foils are at same position
                self.drift_direction = Vec2::new(1.0, 0.0);
            }
        } else {
            // Fallback to x-axis if insufficient foils
            self.drift_direction = Vec2::new(1.0, 0.0);
        }

        // Project velocities onto drift direction
        for body in bodies {
            let projection = body.vel.dot(self.drift_direction);
            match body.species {
                Species::LithiumIon => lithium_velocities.push(projection),
                Species::ElectrolyteAnion => anion_velocities.push(projection),
                _ => {}
            }
        }

        // Calculate mean drift velocities (handle empty cases)
        self.lithium_drift_velocity = if !lithium_velocities.is_empty() {
            lithium_velocities.iter().copied().sum::<f32>() / lithium_velocities.len() as f32
        } else {
            0.0
        };

        self.anion_drift_velocity = if !anion_velocities.is_empty() {
            anion_velocities.iter().copied().sum::<f32>() / anion_velocities.len() as f32
        } else {
            0.0
        };

        // Calculate transference number
        // Current contribution = charge × number_density × velocity
        // For Li+: charge = +1, for anions: charge = -1
        self.li_current_contribution =
            lithium_velocities.len() as f32 * self.lithium_drift_velocity * 1.0; // +1 charge
        self.anion_current_contribution =
            anion_velocities.len() as f32 * self.anion_drift_velocity * (-1.0); // -1 charge

        // Total current is the sum of both contributions (considering sign)
        self.total_current = self.li_current_contribution + self.anion_current_contribution;

        // Transference number is the fraction of current carried by Li+
        self.transference_number = if self.total_current.abs() > 1e-6 {
            self.li_current_contribution / self.total_current
        } else {
            0.0 // Default to 0 if no meaningful transport
        };
    }
}
