use ultraviolet::Vec2;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

#[allow(dead_code)] // Allow unused methods for potential future use

/// History point for PID controller debugging and analysis
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PidHistoryPoint {
    pub timestamp: f64,
    pub step: u64,
    pub target_ratio: f32,
    pub actual_ratio: f32,
    pub setpoint: f32,  // Same as target_ratio for compatibility
    pub actual: f32,    // Same as actual_ratio for compatibility
    pub error: f32,
    pub output_current: f32,
    pub output: f32,    // Same as output_current for compatibility
    pub integral_error: f32,
    pub p_term: f32,
    pub i_term: f32,
    pub d_term: f32,
}

//static NEXT_FOIL_ID: AtomicU64 = AtomicU64::new(1);

/// Mode describing how currents are linked between foils.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkMode {
    /// Currents have the same sign and magnitude.
    Parallel,
    /// Currents have opposite sign but equal magnitude.
    Opposite,
}

/// Charging control mode for foils.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargingMode {
    /// Direct current control - specify electrons/second
    Current,
    /// Overpotential control - specify target electron ratio, auto-adjust current
    Overpotential,
}

/// Overpotential controller parameters for voltage-controlled charging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverpotentialController {
    /// Target electron ratio (1.0 = neutral, >1.0 = cathodic, <1.0 = anodic)
    pub target_ratio: f32,
    /// Proportional gain for PID controller
    pub kp: f32,
    /// Integral gain for PID controller  
    pub ki: f32,
    /// Derivative gain for PID controller
    pub kd: f32,
    /// Integral error accumulator
    pub integral_error: f32,
    /// Previous error for derivative calculation
    pub previous_error: f32,
    /// Maximum allowed current magnitude to prevent instability
    pub max_current: f32,
    /// Last computed output current for display
    pub last_output_current: f32,
    /// History of PID controller performance for plotting
    pub history: VecDeque<PidHistoryPoint>,
    /// Maximum number of history points to keep
    pub max_history_size: usize,
    /// Master foil ID for slave controllers (None for master)
    pub master_foil_id: Option<u64>,
}

/// Collection of fixed lithium metal particles representing a foil.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Foil {
    /// Unique identifier for this foil.
    pub id: u64,
    /// Unique IDs of bodies that belong to this foil within `Simulation::bodies`.
    pub body_ids: Vec<u64>,
    /// DC component of current (constant base current).
    pub dc_current: f32,
    /// AC component of current (amplitude of oscillating current).
    pub ac_current: f32,
    /// Internal accumulator used to emit/remove fractional electrons per step.
    pub accum: f32,
    /// Frequency in Hz for toggling the current on/off. `0.0` disables switching.
    pub switch_hz: f32,
    /// Identifier of a linked foil, if any.
    pub link_id: Option<u64>,
    /// Link mode describing how the currents are related.
    pub mode: LinkMode,
    /// Charging control mode (current vs overpotential)
    pub charging_mode: ChargingMode,
    /// Overpotential controller (only used when charging_mode = Overpotential)
    pub overpotential_controller: Option<OverpotentialController>,
    /// Slave overpotential current for linked foils
    pub slave_overpotential_current: f32,
}

impl Foil {
    pub fn new(
        body_ids: Vec<u64>,
        _origin: Vec2,
        _width: f32,
        _height: f32,
        current: f32,
        _switch_hz: f32,
    ) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            body_ids,
            dc_current: current, // Initialize DC current to the provided current
            ac_current: 0.0,     // No AC component by default
            accum: 0.0,
            switch_hz: 0.0,
            link_id: None,
            mode: LinkMode::Parallel,
            charging_mode: ChargingMode::Current, // Default to current control
            overpotential_controller: None,       // No overpotential controller by default
            slave_overpotential_current: 0.0,    // Initialize slave current to zero
        }
    }

    /// Enable overpotential control mode with specified target electron ratio
    pub fn enable_overpotential_mode(&mut self, target_ratio: f32) {
        self.charging_mode = ChargingMode::Overpotential;
        self.overpotential_controller = Some(OverpotentialController {
            target_ratio,
            // Crisper defaults to improve response near small errors
            kp: 20.0,           // Proportional gain - tunable
            ki: 0.4,            // Integral gain - tunable  
            kd: 0.2,            // Derivative gain - tunable
            integral_error: 0.0,
            previous_error: 0.0,
            max_current: 500.0, // Maximum current limit - tunable
            last_output_current: 0.0,
            history: VecDeque::new(),
            max_history_size: 1000,
            master_foil_id: None,
        });
    }

    /// Disable overpotential mode and return to current control
    pub fn disable_overpotential_mode(&mut self) {
        self.charging_mode = ChargingMode::Current;
        self.overpotential_controller = None;
    }

    /// Update overpotential controller and return computed current
    pub fn compute_overpotential_current(&mut self, actual_ratio: f32, dt: f32) -> f32 {
        if let Some(controller) = &mut self.overpotential_controller {
            let error = controller.target_ratio - actual_ratio;
            
            // PID calculation
            controller.integral_error += error * dt;
            let derivative_error = (error - controller.previous_error) / dt;
            
            let pid_output = controller.kp * error 
                           + controller.ki * controller.integral_error
                           + controller.kd * derivative_error;
            
            controller.previous_error = error;
            
            // Clamp to maximum current
            let output = pid_output.clamp(-controller.max_current, controller.max_current);
            controller.last_output_current = output;
            output
        } else {
            0.0 // No controller available
        }
    }

    /// Enable slave mode for overpotential control (linked to master foil)
    #[allow(dead_code)]
    pub fn enable_overpotential_slave_mode(&mut self, _master_id: u64) {
        self.charging_mode = ChargingMode::Overpotential;
        self.overpotential_controller = None; // Slaves don't have their own controller
        self.slave_overpotential_current = 0.0;
    }
}
