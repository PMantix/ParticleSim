// Helper to deduce measurement filename from experiment settings
use crate::switch_charging::{Mode, SwitchChargingConfig};

/// Compute a standard measurement filename based on experiment context:
/// Format (CONV mode): "Measurement_CONV_[B]_[C].csv"
/// Format (SWITCH mode): "Measurement_SWITCH_[B]_[C]_[D].csv"
/// where:
/// - A is charging_mode: "CONV" (conventional / no switching) or "SWITCH" (switching controls active)
/// - B is control_mode: "CC" (current control) or "OP" (overpotential control)
/// - C is current magnitude: "0p04" or "1p3" (replace '.' with 'p')
/// - D is steps per half cycle (only for SWITCH mode): "750" or "1000"
pub fn build_measurement_filename(
    switch_running: bool,
    switch_config: &SwitchChargingConfig,
) -> String {
    // A: Determine charging mode
    let charging_mode_str = if switch_running { "SWITCH" } else { "CONV" };

    // B & C: Determine control mode and value from global active setpoint
    let (control_mode_str, value) = if switch_config.use_global_active_inactive {
        let setpoint = &switch_config.global_active;
        let mode_str = match setpoint.mode {
            Mode::Current => "CC",
            Mode::Overpotential => "OP",
        };
        (mode_str, setpoint.value.abs())
    } else {
        // Fallback: use step 0 active setpoint if per-step mode is used
        if let Some(sai) = switch_config.step_active_inactive.get(&0) {
            let mode_str = match sai.active.mode {
                Mode::Current => "CC",
                Mode::Overpotential => "OP",
            };
            (mode_str, sai.active.value.abs())
        } else {
            // Ultimate fallback: use legacy step setpoint
            if let Some(sp) = switch_config.step_setpoints.get(&0) {
                let mode_str = match sp.mode {
                    Mode::Current => "CC",
                    Mode::Overpotential => "OP",
                };
                (mode_str, sp.value.abs())
            } else {
                ("CC", 0.02)
            }
        }
    };

    // Format the value: replace '.' with 'p'
    let value_str = format!("{:.2}", value).replace('.', "p");

    // Build filename: include delta_steps only for SWITCH mode
    if switch_running {
        format!(
            "Measurement_{}_{}_{}_{}.csv",
            charging_mode_str, control_mode_str, value_str, switch_config.delta_steps
        )
    } else {
        format!(
            "Measurement_{}_{}_{}.csv",
            charging_mode_str, control_mode_str, value_str
        )
    }
}
