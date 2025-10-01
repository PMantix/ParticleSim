/// Automatic measurement system for DOE
use crate::body::Body;
use crate::body::Species;
use super::config::MeasurementPoint;

#[derive(Debug, Clone)]
pub struct MeasurementSample {
    pub time_fs: f32,
    pub position_label: String,
    pub lithium_metal_edge_position: f32,
    pub lithium_metal_count: usize,
    pub lithium_ion_count: usize,
    pub total_charge: f32,
}

pub struct AutoMeasurement {
    measurement_points: Vec<MeasurementPoint>,
    samples: Vec<MeasurementSample>,
    last_measurement_time: f32,
    measurement_interval: f32,
}

impl AutoMeasurement {
    pub fn new(measurement_points: Vec<MeasurementPoint>, measurement_interval: f32) -> Self {
        Self {
            measurement_points,
            samples: Vec::new(),
            last_measurement_time: 0.0,
            measurement_interval,
        }
    }
    
    /// Check if it's time to take a measurement
    pub fn should_measure(&self, current_time_fs: f32) -> bool {
        current_time_fs - self.last_measurement_time >= self.measurement_interval
    }
    
    /// Perform measurements at all configured points
    pub fn measure(&mut self, bodies: &[Body], current_time_fs: f32) {
        if !self.should_measure(current_time_fs) {
            return;
        }
        
        self.last_measurement_time = current_time_fs;
        
        for point in &self.measurement_points {
            let sample = self.measure_at_point(bodies, point, current_time_fs);
            self.samples.push(sample);
        }
    }
    
    /// Measure lithium metal deposition at a specific point
    fn measure_at_point(
        &self,
        bodies: &[Body],
        point: &MeasurementPoint,
        time_fs: f32,
    ) -> MeasurementSample {
        // Define measurement region based on point and direction
        let (x_min, x_max, y_min, y_max) = match point.direction.as_str() {
            "left" => (
                point.x - point.width_ang,
                point.x,
                point.y - 5.0, // Small vertical tolerance
                point.y + 5.0,
            ),
            "right" => (
                point.x,
                point.x + point.width_ang,
                point.y - 5.0,
                point.y + 5.0,
            ),
            "up" => (
                point.x - 5.0,
                point.x + 5.0,
                point.y,
                point.y + point.width_ang,
            ),
            "down" => (
                point.x - 5.0,
                point.x + 5.0,
                point.y - point.width_ang,
                point.y,
            ),
            _ => (point.x - point.width_ang/2.0, point.x + point.width_ang/2.0, point.y - 5.0, point.y + 5.0),
        };
        
        // Find lithium metal particles in the measurement region
        let mut lithium_metal_bodies = Vec::new();
        let mut lithium_ion_count = 0;
        let mut total_charge = 0.0;
        
        for body in bodies {
            let pos = body.pos;
            
            // Check if body is in measurement region
            if pos.x >= x_min && pos.x <= x_max && pos.y >= y_min && pos.y <= y_max {
                match body.species {
                    Species::LithiumMetal => {
                        lithium_metal_bodies.push(body.clone());
                        total_charge += body.charge as f32;
                    }
                    Species::LithiumIon => {
                        lithium_ion_count += 1;
                        total_charge += body.charge as f32;
                    }
                    _ => {}
                }
            }
        }
        
        // Find leading edge position (furthest from foil in measurement direction)
        let edge_position = if !lithium_metal_bodies.is_empty() {
            match point.direction.as_str() {
                "left" => lithium_metal_bodies.iter()
                    .map(|b| b.pos.x)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(point.x),
                "right" => lithium_metal_bodies.iter()
                    .map(|b| b.pos.x)
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(point.x),
                "up" => lithium_metal_bodies.iter()
                    .map(|b| b.pos.y)
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(point.y),
                "down" => lithium_metal_bodies.iter()
                    .map(|b| b.pos.y)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(point.y),
                _ => point.x,
            }
        } else {
            point.x // No metal detected, return reference position
        };
        
        MeasurementSample {
            time_fs,
            position_label: point.label.clone(),
            lithium_metal_edge_position: edge_position,
            lithium_metal_count: lithium_metal_bodies.len(),
            lithium_ion_count,
            total_charge,
        }
    }
    
    /// Get all collected samples
    pub fn get_samples(&self) -> &[MeasurementSample] {
        &self.samples
    }
    
    /// Clear all samples (for next test case)
    pub fn clear(&mut self) {
        self.samples.clear();
        self.last_measurement_time = 0.0;
    }
}
