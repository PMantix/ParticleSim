use super::*;
use std::sync::atomic::Ordering;

impl super::super::Renderer {
    pub fn show_visualization_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("👁️ Visualization Controls");

        // Display Options
        ui.group(|ui| {
            ui.label("🖼️ Display Options");
            ui.checkbox(&mut self.show_bodies, "Show Bodies");
            ui.checkbox(&mut self.show_quadtree, "Show Quadtree");
            if ui.checkbox(&mut self.side_view_mode, "📐 Side View (X-Z)")
                .on_hover_text("Toggle between top-down view (X-Y) and side view (X-Z) to visualize particle motion in the Z dimension").clicked() {
                // Optional: Add any side effects when toggling view mode
            }

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
                &mut self.sim_config.show_2d_domain_density,
                "Show 2D Domain Density",
            );
            ui.checkbox(
                &mut self.sim_config.show_field_vectors,
                "Show Field Vectors",
            );

            let mut depth = SHOW_Z_VISUALIZATION.load(Ordering::Relaxed);
            if ui.checkbox(&mut depth, "Show Depth Cue").changed() {
                SHOW_Z_VISUALIZATION.store(depth, Ordering::Relaxed);
                if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = sender.send(SimCommand::ToggleZVisualization { enabled: depth });
                }
            }

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

        ui.separator();

        // Species Dark Mode
        ui.group(|ui| {
            ui.label("🌙 Species Dark Mode");
            ui.checkbox(
                &mut self.species_dark_mode_enabled,
                "Enable Dark Mode"
            )
            .on_hover_text("Enable dark background mode for better visibility of particles");
            
            if self.species_dark_mode_enabled {
                ui.add(
                    egui::Slider::new(&mut self.species_dark_mode_strength, 0.0..=1.0)
                        .text("Dark Mode Strength")
                        .step_by(0.01)
                )
                .on_hover_text("Controls how dark the background becomes (0.0 = light, 1.0 = full dark)");
            }
        });
    }
}
