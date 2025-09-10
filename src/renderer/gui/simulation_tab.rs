use super::*;

impl super::super::Renderer {
    pub fn show_simulation_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙️ Simulation Controls");

        // Field Controls
        ui.group(|ui| {
            ui.label("🔋 Electric Field");
            let mut mag = *FIELD_MAGNITUDE.lock();
            ui.add(
                egui::Slider::new(&mut mag, 0.0..=200.0)
                    .text("Field |E|")
                    .clamp_to_range(true)
                    .step_by(1.0),
            );
            *FIELD_MAGNITUDE.lock() = mag;

            let mut dir = *FIELD_DIRECTION.lock();
            ui.add(
                egui::Slider::new(&mut dir, 0.0..=360.0)
                    .text("Field θ (deg)")
                    .clamp_to_range(true),
            );
            *FIELD_DIRECTION.lock() = dir;
        });

        ui.separator();

        // Core simulation parameters
        ui.group(|ui| {
            ui.label("⏱️ Simulation Parameters");
            ui.add(
                egui::Slider::new(&mut *TIMESTEP.lock(), 0.1..=5.0)
                    .text("Timestep (fs)")
                    .step_by(0.05)
                    .logarithmic(false),
            );
            ui.label("💡 Typical MD timesteps: 0.5-2.0 fs");
            ui.add(
                egui::Slider::new(&mut self.sim_config.damping_base, 0.95..=1.0)
                    .text("Damping Base")
                    .step_by(0.0001),
            );

            let mut passes = COLLISION_PASSES.lock();
            ui.add(
                egui::Slider::new(&mut *passes, 2..=50)
                    .text("Collision Passes")
                    .clamp_to_range(true),
            );

            if ui.button("Step Simulation").clicked() {
                SIM_COMMAND_SENDER
                    .lock()
                    .as_ref()
                    .unwrap()
                    .send(SimCommand::StepOnce)
                    .unwrap();
            }
        });
    }
}
