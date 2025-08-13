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

        ui.separator();

        // Coulomb constant control
        ui.group(|ui| {
            ui.label("🔌 Coulomb Constant");
            ui.add(
                egui::Slider::new(&mut self.sim_config.coulomb_constant, 1000.0..=20000.0)
                    .text("k_e")
                    .step_by(1.0)
                    .logarithmic(true),
            );
        });

        ui.separator();

        ui.group(|ui| {
            ui.label("🌡️ Simulation Temperature");
            let mut temp = self.sim_config.temperature;
            if ui
                .add(egui::Slider::new(&mut temp, 0.01..=300.0).text("T").step_by(0.01))
                .changed()
            {
                self.sim_config.temperature = temp;
                SIM_COMMAND_SENDER
                    .lock()
                    .as_ref()
                    .unwrap()
                    .send(SimCommand::SetTemperature { temperature: temp })
                    .unwrap();
            }
            
            ui.separator();
            
            ui.label("⏱️ Thermostat Frequency");
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.sim_config.thermostat_frequency, 0.1..=10.0)
                        .text("Period")
                        .step_by(0.1)
                );
                ui.label("time units");
            });
            ui.small("How often to enforce temperature constraint");
            ui.small("Lower = more frequent, higher = more natural dynamics");
        });

        ui.separator();

        // Pseudo Out-of-Plane Controls
        ui.group(|ui| {
            ui.label("🧭 Pseudo Out-of-Plane (Z) Motion");

            let mut enabled = self.sim_config.enable_out_of_plane;
            let mut max_z = self.sim_config.max_z;
            let mut z_stiffness = self.sim_config.z_stiffness;
            let mut z_damping = self.sim_config.z_damping;

            let mut changed = false;
            let mut enable_solvent = self.sim_config.enable_solvent_out_of_plane;
            let mut vis_min = self.sim_config.z_vis_min_frac;
            let mut vis_max = self.sim_config.z_vis_max_frac;

            if ui.checkbox(&mut enabled, "Enable").changed() { changed = true; }
            ui.add_enabled_ui(enabled, |ui| {
                if ui.add(egui::Slider::new(&mut max_z, 0.05..=5.0).text("Max |z|").step_by(0.01)).changed() { changed = true; }
                if ui.add(egui::Slider::new(&mut z_stiffness, 0.1..=100.0).text("Spring k").logarithmic(true)).changed() { changed = true; }
                if ui.add(egui::Slider::new(&mut z_damping, 0.01..=5.0).text("Damping γ").step_by(0.01)).changed() { changed = true; }
                ui.separator();
                if ui.checkbox(&mut enable_solvent, "Include Solvents (EC/DMC)").changed() { changed = true; }
                ui.collapsing("Visualization", |ui| {
                    ui.small("Adjust how strongly z displacement alters color/intensity.");
                    if ui.add(egui::Slider::new(&mut vis_min, 0.0..=1.0).text("Vis Min").step_by(0.01)).changed() { changed = true; }
                    if ui.add(egui::Slider::new(&mut vis_max, 0.0..=1.0).text("Vis Max").step_by(0.01)).changed() { changed = true; }
                    if vis_max < vis_min { vis_max = vis_min; }
                });
            });

            if changed {
                self.sim_config.enable_out_of_plane = enabled;
                self.sim_config.max_z = max_z;
                self.sim_config.z_stiffness = z_stiffness;
                self.sim_config.z_damping = z_damping;
                self.sim_config.enable_solvent_out_of_plane = enable_solvent;
                self.sim_config.z_vis_min_frac = vis_min;
                self.sim_config.z_vis_max_frac = vis_max;
                if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                    // Send command to simulation thread
                    sender.send(SimCommand::SetOutOfPlane { 
                        enabled,
                        max_z,
                        z_stiffness,
                        z_damping,
                        enable_solvent,
                        vis_min_frac: vis_min,
                        vis_max_frac: vis_max,
                    }).ok();
                }
            }

            ui.small("Physics-based pseudo-Z motion: lighter particles (Li⁺) naturally get more z-acceleration.");
            ui.small("Helps Li⁺ escape crowded solvent shells to reach electrodes via mass-based dynamics.");
        });
    }
}
