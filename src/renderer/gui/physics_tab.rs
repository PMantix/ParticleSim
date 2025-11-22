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
                    .step_by(0.01),
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

        ui.group(|ui| {
            ui.label("🪙 Electron Hopping");
            ui.add(
                egui::Slider::new(&mut self.sim_config.hop_rate_k0, 0.0..=20.0)
                    .text("Base Hop Rate k₀")
                    .step_by(0.1),
            );
            ui.small("Sets the baseline probability for hops—raise to speed up electron motion everywhere.");
            ui.add(
                egui::Slider::new(&mut self.sim_config.hop_transfer_coeff, 0.0..=1.0)
                    .text("Field Response α")
                    .step_by(0.01),
            );
            ui.small(
                "Controls how strongly potential differences amplify hopping; higher values react more to fields.",
            );
            ui.add(
                egui::Slider::new(&mut self.sim_config.hop_activation_energy, 0.0..=0.2)
                    .text("Activation Barrier (eV)")
                    .step_by(0.001),
            );
            ui.small("Lower the barrier to make hops exponentially easier even without strong fields.");
            ui.add(
                egui::Slider::new(&mut self.sim_config.hop_radius_factor, 0.5..=5.0)
                    .text("Neighbor Radius ×R")
                    .step_by(0.1),
            );
            ui.small("Expands the search distance for partner particles—grow it to enable longer-range hops.");
            ui.add(
                egui::Slider::new(&mut self.sim_config.hop_alignment_bias, 0.0..=1000.0)
                    .text("Field Alignment Bias")
                    .step_by(0.01),
            )
            .on_hover_text(
                "Scales how strongly hops prefer moving with the electric field; >1 boosts surface-directed hops.",
            );
            ui.small(
                "Set above 1 to favor down-field transfers, or below 1 to relax the surface-alignment preference.",
            );

            // Vacancy polarization bias slider
            ui.add(
                egui::Slider::new(&mut self.sim_config.hop_vacancy_polarization_gain, 0.0..=1000.0)
                    .text("Vacancy Polarization Bias")
                    .step_by(0.01),
            ).on_hover_text("Bias vacancy hops to move along the local valence-electron offset direction in metals. 0 = off.");
        });

        ui.separator();

        // SEI Formation Controls
        ui.group(|ui| {
            ui.label("🛡️ SEI Formation");
            ui.checkbox(&mut self.sim_config.sei_formation_enabled, "Enable SEI Formation");
            ui.add(
                egui::Slider::new(&mut self.sim_config.sei_formation_probability, 0.0..=1.0)
                    .text("Formation Probability")
                    .step_by(0.001)
                    .logarithmic(true),
            );
            ui.add(
                egui::Slider::new(&mut self.sim_config.sei_formation_bias, 0.0..=100.0)
                    .text("Charge Bias")
                    .step_by(0.1),
            );
            ui.small("Probability scales with negative charge magnitude on metal surface.");
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
                egui::Slider::new(&mut self.sim_config.coulomb_constant, 0.01..=1000.0)
                    .text("k_e")
                    .step_by(0.1)
                    .logarithmic(true),
            );
            ui.label(format!(
                "💡 Theoretical value: {:.3}",
                crate::units::COULOMB_CONSTANT
            ));
            ui.label("Scale up for stronger interactions");
        });

        ui.separator();

        // Dipole model selection
        ui.group(|ui| {
            ui.label("🧲 Polar Solvent Dipole Model (EC/DMC)");
            ui.horizontal(|ui| {
                ui.label("Model:");
                let mut model = self.sim_config.dipole_model;
                egui::ComboBox::from_id_source("dipole_model_combo")
                    .selected_text(match model {
                        crate::config::DipoleModel::SingleOffset => "Single offset (original)",
                        crate::config::DipoleModel::ConjugatePair => "Conjugate pair (±q)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut model,
                            crate::config::DipoleModel::SingleOffset,
                            "Single offset (original)",
                        );
                        ui.selectable_value(
                            &mut model,
                            crate::config::DipoleModel::ConjugatePair,
                            "Conjugate pair (±q)",
                        );
                    });
                if model != self.sim_config.dipole_model {
                    self.sim_config.dipole_model = model;
                    // Persist to global config so the sim thread picks it up next step
                    *crate::config::LJ_CONFIG.lock() = self.sim_config.clone();
                }
            });
            ui.small("Single offset: field difference nucleus vs electron (original). Conjugate pair: explicit ±q dipoles enabling dipole–dipole interactions.");
        });

        ui.separator();

        // Induced field from foil charging
        ui.group(|ui| {
            ui.label("📡 Induced External Field");
            ui.small("Automatically add an external field based on foil charging setpoints.");
            ui.add(
                egui::Slider::new(&mut self.sim_config.induced_field_gain, 0.0..=5_000_000.0)
                    .text("Induced Field Gain")
                    .step_by(0.1)
                    .logarithmic(true),
            );
            ui.add(
                egui::Slider::new(&mut self.sim_config.induced_field_smoothing, 0.0..=0.999)
                    .text("Induced Field Smoothing α")
                    .step_by(0.001),
            );
            ui.checkbox(
                &mut self.sim_config.induced_field_use_direction,
                "Use foil-based direction (neg→pos)",
            );
            ui.add(
                egui::Slider::new(
                    &mut self.sim_config.induced_field_overpot_scale,
                    0.0..=10_000_000.0,
                )
                .text("Overpotential→Drive Scale")
                .step_by(1.0)
                .logarithmic(true),
            );
            ui.small("Drive magnitude: |current| or |target_ratio−1|×scale.");
        });

        ui.group(|ui| {
            ui.label("🌡️ Simulation Temperature");
            let mut temp = self.sim_config.temperature;
            if ui
                .add(
                    egui::Slider::new(&mut temp, 0.01..=450.0)
                        .text("T")
                        .step_by(0.01),
                )
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

            ui.label("⏱️ Thermostat Interval (fs)");
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.sim_config.thermostat_interval_fs, 0.1..=10.0)
                        .text("Period")
                        .step_by(0.1),
                );
                ui.label("fs");
            });
            ui.small("How often to enforce temperature constraint");
            ui.small("Lower = more frequent, higher = more natural dynamics");
        });

        ui.separator();

        ui.group(|ui| {
            ui.label("🪐 Out-of-Plane");
            let mut enabled = self.sim_config.enable_out_of_plane;
            if ui.checkbox(&mut enabled, "Enable").changed() {
                self.sim_config.enable_out_of_plane = enabled;
                if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = sender.send(SimCommand::SetOutOfPlane {
                        enabled,
                        z_stiffness: self.sim_config.z_stiffness,
                        z_damping: self.sim_config.z_damping,
                        max_z: self.sim_config.max_z,
                    });
                }
            }
            ui.add(
                egui::Slider::new(&mut self.sim_config.z_stiffness, 0.0..=10.0).text("Z Stiffness"),
            );
            ui.add(egui::Slider::new(&mut self.sim_config.z_damping, 0.0..=10.0).text("Z Damping"));
            ui.add(egui::Slider::new(&mut self.sim_config.max_z, 0.01..=50.0).text("Max Z"));

            ui.separator();

            // Z-Visualization Controls
            ui.label("🎨 Z-Visualization");
            let mut z_viz_enabled = crate::renderer::state::SHOW_Z_VISUALIZATION
                .load(std::sync::atomic::Ordering::Relaxed);
            if ui.checkbox(&mut z_viz_enabled, "Show Z-depth").changed() {
                crate::renderer::state::SHOW_Z_VISUALIZATION
                    .store(z_viz_enabled, std::sync::atomic::Ordering::Relaxed);
                if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = sender.send(SimCommand::ToggleZVisualization {
                        enabled: z_viz_enabled,
                    });
                }
            }

            let mut z_viz_strength = *crate::renderer::state::Z_VISUALIZATION_STRENGTH.lock();
            if ui
                .add(
                    egui::Slider::new(&mut z_viz_strength, 0.1..=5.0)
                        .text("Z-viz Strength")
                        .step_by(0.1),
                )
                .changed()
            {
                *crate::renderer::state::Z_VISUALIZATION_STRENGTH.lock() = z_viz_strength;
                if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = sender.send(SimCommand::SetZVisualizationStrength {
                        strength: z_viz_strength,
                    });
                }
            }
            ui.small("Higher values = more dramatic Z-depth effect");

            if ui.button("Apply Z Settings").clicked() {
                if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = sender.send(SimCommand::SetOutOfPlane {
                        enabled: self.sim_config.enable_out_of_plane,
                        z_stiffness: self.sim_config.z_stiffness,
                        z_damping: self.sim_config.z_damping,
                        max_z: self.sim_config.max_z,
                    });
                }
            }
        });
    }
}
