use super::*;

impl super::Renderer {
    fn show_visualization_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("👁️ Visualization Controls");

        // Display Options
        ui.group(|ui| {
            ui.label("🖼️ Display Options");
            ui.checkbox(&mut self.show_bodies, "Show Bodies");
            ui.checkbox(&mut self.show_quadtree, "Show Quadtree");

            if self.show_quadtree {
                let range = &mut self.depth_range;
                ui.horizontal(|ui| {
                    ui.label("Depth Range:");
                    ui.add(egui::DragValue::new(&mut range.0).speed(0.05));
                    ui.label("to");
                    ui.add(egui::DragValue::new(&mut range.1).speed(0.05));
                });
            }
        });

        ui.separator();

        // Visualization Overlays
        ui.group(|ui| {
            ui.label("🎨 Overlays");
            ui.checkbox(
                &mut self.sim_config.show_field_isolines,
                "Show Field Isolines",
            );
            ui.checkbox(
                &mut self.sim_config.show_velocity_vectors,
                "Show Velocity Vectors",
            );
            ui.checkbox(
                &mut self.sim_config.show_charge_density,
                "Show Charge Density",
            );
            ui.checkbox(
                &mut self.sim_config.show_field_vectors,
                "Show Field Vectors",
            );

            egui::ComboBox::from_label("Isoline Field Mode")
                .selected_text(format!("{:?}", self.sim_config.isoline_field_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.sim_config.isoline_field_mode,
                        IsolineFieldMode::Total,
                        "Total",
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
            ui.add(
                egui::Slider::new(&mut self.velocity_vector_scale, 0.01..=1.0)
                    .text("Velocity Vector Scale")
                    .step_by(0.01),
            );
        });
    }
}
