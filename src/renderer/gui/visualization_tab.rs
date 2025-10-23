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
            // Isoline controls
            ui.add(
                egui::Slider::new(&mut self.sim_config.isoline_count, 3..=51)
                    .text("Isoline levels")
                    .step_by(1.0),
            );
            ui.add(
                egui::Slider::new(&mut self.sim_config.isoline_target_samples, 1..=120)
                    .text("Isoline fidelity")
                    .step_by(1.0)
                    .logarithmic(true),
            );
            ui.add(
                egui::Slider::new(&mut self.sim_config.isoline_clip_margin, -0.2..=0.2)
                    .text("Isoline clip margin")
                    .step_by(0.005)
                    .clamp_to_range(true),
            )
            .on_hover_text(
                "Clips extreme low/high sampled values to avoid outliers (applied symmetrically)",
            );
            ui.add(
                egui::Slider::new(&mut self.sim_config.isoline_bias, -0.5..=0.5)
                    .text("Isoline bias (offset)")
                    .step_by(0.01),
            )
            .on_hover_text("Shifts percentile mapping of levels up/down the range after clipping");
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.sim_config.isoline_local_refine,
                    "Adaptive refinement",
                );
                if self.sim_config.isoline_local_refine {
                    ui.add(
                        egui::Slider::new(&mut self.sim_config.isoline_local_refine_factor, 1..=10)
                            .text("Refine factor")
                            .step_by(1.0),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.sim_config.isoline_local_refine_band,
                            0.1..=10.0,
                        )
                        .text("Refine band")
                        .step_by(0.05),
                    );
                }
            });

            ui.separator();
            ui.label("Isoline coloring and fill");
            ui.add(
                egui::Slider::new(&mut self.sim_config.isoline_color_strength, 0.0..=1.0)
                    .text("Color strength")
                    .step_by(0.01),
            )
            .on_hover_text("0 = white lines; 1 = full blue/red deviation");
            ui.add(
                egui::Slider::new(&mut self.sim_config.isoline_color_gamma, 0.05..=3.0)
                    .text("Color gamma")
                    .step_by(0.05),
            )
            .on_hover_text("Adjusts how quickly color saturates with |potential| (perceptual)");
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.sim_config.isoline_filled, "Filled isobands");
                if self.sim_config.isoline_filled {
                    let mut alpha_u8 = self.sim_config.isoline_fill_alpha as i32;
                    if ui
                        .add(egui::Slider::new(&mut alpha_u8, 10..=200).text("Fill alpha"))
                        .changed()
                    {
                        self.sim_config.isoline_fill_alpha = alpha_u8 as u8;
                    }
                }
            });

            ui.add(
                egui::Slider::new(&mut self.sim_config.isoline_distribution_gamma, 0.05..=50.0)
                    .text("Level distribution gamma")
                    .step_by(0.05),
            )
            .on_hover_text(
                "1.0 = linear; >1 concentrates levels near extremes; <1 concentrates near center",
            );
            // Dipole visualization controls
            ui.checkbox(&mut self.show_dipoles, "Show EC/DMC dipoles");
            if self.show_dipoles {
                ui.add(egui::Slider::new(&mut self.dipole_scale, 0.1..=5.0).text("Dipole scale"));
            }

            // Velocity vector scale control
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
            ui.checkbox(&mut self.species_dark_mode_enabled, "Enable Dark Mode")
                .on_hover_text("Enable dark background mode for better visibility of particles");

            if self.species_dark_mode_enabled {
                ui.add(
                    egui::Slider::new(&mut self.species_dark_mode_strength, 0.0..=1.0)
                        .text("Dark Mode Strength")
                        .step_by(0.01),
                )
                .on_hover_text(
                    "Controls how dark the background becomes (0.0 = light, 1.0 = full dark)",
                );
            }
        });

        ui.separator();
    }
}
