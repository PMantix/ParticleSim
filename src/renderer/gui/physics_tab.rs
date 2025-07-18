use super::*;

impl super::super::Renderer {
    pub fn show_physics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚛️ Physics Models");

        // Butler-Volmer Parameters
        ui.group(|ui| {
            ui.label("🔋 Butler-Volmer Parameters");
            ui.checkbox(&mut self.sim_config.use_butler_volmer, "Use Butler-Volmer");
            ui.add(
                egui::Slider::new(&mut self.sim_config.bv_exchange_current, 0.0..=1.0e6)
                    .text("Exchange Current i0")
                    .step_by(1.0),
            );
            ui.add(
                egui::Slider::new(&mut self.sim_config.bv_transfer_coeff, 0.0..=1.0)
                    .text("Transfer Coeff α")
                    .step_by(0.01),
            );
            ui.add(
                egui::Slider::new(&mut self.sim_config.bv_overpotential_scale, 0.0..=1.0)
                    .text("Overpotential Scale")
                    .step_by(0.0001),
            );
        });

        ui.separator();

        // External Electric Field Controls
        ui.group(|ui| {
            ui.label("⚡ External Electric Field");

            // Basic field magnitude and direction (duplicate of Simulation tab for convenience)
            let mut mag = *FIELD_MAGNITUDE.lock();
            ui.add(
                egui::Slider::new(&mut mag, 0.0..=1000.0)
                    .text("Field Magnitude |E|")
                    .clamp_to_range(true)
                    .step_by(1.0),
            );
            *FIELD_MAGNITUDE.lock() = mag;

            let mut dir = *FIELD_DIRECTION.lock();
            ui.add(
                egui::Slider::new(&mut dir, 0.0..=360.0)
                    .text("Field Direction θ (deg)")
                    .clamp_to_range(true),
            );
            *FIELD_DIRECTION.lock() = dir;

            ui.separator();

            // Field visualization controls
            ui.label("Field Visualization:");
            ui.checkbox(
                &mut self.sim_config.show_field_isolines,
                "Show Field Isolines",
            );
            ui.checkbox(
                &mut self.sim_config.show_field_vectors,
                "Show Field Vectors",
            );

            ui.horizontal(|ui| {
                ui.label("Isoline Mode:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", self.sim_config.isoline_field_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.sim_config.isoline_field_mode,
                            IsolineFieldMode::Total,
                            "Total Field",
                        );
                        ui.selectable_value(
                            &mut self.sim_config.isoline_field_mode,
                            IsolineFieldMode::ExternalOnly,
                            "External Only",
                        );
                        ui.selectable_value(
                            &mut self.sim_config.isoline_field_mode,
                            IsolineFieldMode::BodyOnly,
                            "Body Only",
                        );
                    });
            });

            ui.separator();

            // Additional field information
            ui.label("💡 Field Info:");
            ui.label("• Total field = External + Particle charges");
            ui.label("• External field affects all particles uniformly");
            ui.label("• Adjust in Simulation tab for basic controls");
        });
    }
}
