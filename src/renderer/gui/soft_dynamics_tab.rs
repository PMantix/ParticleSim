use super::*;
use quarkstrom::egui::{RichText, Color32};

impl super::super::Renderer {
    pub fn show_soft_dynamics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔧 Soft Dynamics Configuration");
        ui.separator();

        // Status display (simplified - no live data for now)
        ui.group(|ui| {
            ui.label(RichText::new("📊 Status").strong());
            ui.separator();
            
            if self.sim_config.frustration_enabled {
                ui.horizontal(|ui| {
                    ui.label("System Status:");
                    ui.colored_label(Color32::GREEN, "Enabled");
                });
                
                let reduction = (1.0 - self.sim_config.frustration_soft_repulsion_factor) * 100.0;
                ui.horizontal(|ui| {
                    ui.label("Current Effect:");
                    if reduction > 0.1 {
                        ui.colored_label(Color32::LIGHT_BLUE, 
                            format!("{:.0}% force reduction when frustrated", reduction));
                    } else {
                        ui.label("Normal collision behavior");
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("System Status:");
                    ui.colored_label(Color32::GRAY, "Disabled");
                });
            }
        });

        ui.separator();

        // Main enable/disable control
        ui.group(|ui| {
            ui.label(RichText::new("⚙️ Enable/Disable").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.sim_config.frustration_enabled, "Enable Soft Dynamics").changed() {
                    self.update_frustration_config();
                }
                ui.label("📘").on_hover_text(
                    "When enabled, particles experiencing high forces but unable to move \n\
                     will have their collision repulsion softened, allowing them to \n\
                     'squeeze past' blocking particles."
                );
            });
        });

        ui.separator();

        // Parameter tuning controls
        ui.group(|ui| {
            ui.label(RichText::new("🎛️ Parameter Tuning").strong());
            ui.separator();

            let mut config_changed = false;

            // Force detection parameters
            ui.collapsing("Force Detection", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Min Force Threshold:");
                    if ui.add(egui::Slider::new(&mut self.sim_config.frustration_min_force_threshold, 0.1..=5.0)
                        .step_by(0.1)
                        .suffix(" force units")).changed() {
                        config_changed = true;
                    }
                });
                ui.small("Minimum force magnitude to consider a particle for frustration detection");

                ui.horizontal(|ui| {
                    ui.label("Movement Threshold:");
                    if ui.add(egui::Slider::new(&mut self.sim_config.frustration_stuck_movement_threshold, 0.01..=1.0)
                        .step_by(0.01)
                        .suffix(" distance units")).changed() {
                        config_changed = true;
                    }
                });
                ui.small("Maximum movement while still considered 'stuck'");
            });

            // Timing parameters
            ui.collapsing("Timing Controls", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Confirmation Steps:");
                    if ui.add(egui::Slider::new(&mut self.sim_config.frustration_confirmation_steps, 1..=50)
                        .suffix(" timesteps")).changed() {
                        config_changed = true;
                    }
                });
                ui.small("Number of consecutive steps needed to confirm frustration");

                ui.horizontal(|ui| {
                    ui.label("Max Duration:");
                    if ui.add(egui::Slider::new(&mut self.sim_config.frustration_max_duration, 10..=200)
                        .suffix(" timesteps")).changed() {
                        config_changed = true;
                    }
                });
                ui.small("Maximum time a particle can remain in frustrated state");
            });

            // Soft repulsion strength
            ui.collapsing("Collision Softening", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Soft Repulsion Factor:");
                    if ui.add(egui::Slider::new(&mut self.sim_config.frustration_soft_repulsion_factor, 0.1..=1.0)
                        .step_by(0.05)).changed() {
                        config_changed = true;
                    }
                });
                ui.small("Fraction of normal collision force (0.1 = 10% force, very soft)");

                // Show the effect visually
                let reduction = (1.0 - self.sim_config.frustration_soft_repulsion_factor) * 100.0;
                if reduction > 0.1 {
                    ui.horizontal(|ui| {
                        ui.label("Effect:");
                        ui.colored_label(Color32::LIGHT_BLUE, 
                            format!("{:.0}% force reduction", reduction));
                    });
                }
            });

            // Advanced parameters
            ui.collapsing("Advanced Settings", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Position History Size:");
                    if ui.add(egui::Slider::new(&mut self.sim_config.frustration_position_history_size, 3..=20)
                        .suffix(" positions")).changed() {
                        config_changed = true;
                    }
                });
                ui.small("Number of recent positions tracked for movement analysis");
            });

            if config_changed {
                self.update_frustration_config();
            }
        });

        ui.separator();

        // Lithium-ion overlap relaxation controls
        ui.group(|ui| {
            ui.label(RichText::new("🧲 Li⁺ Overlap Relaxation").strong());
            ui.separator();
            ui.small("Allow lithium ions to overlap slightly under strong electric forces.");

            let mut overlap_changed = false;

            ui.horizontal(|ui| {
                ui.label("Force Threshold:");
                if ui
                    .add(
                        egui::Slider::new(
                            &mut self.sim_config.li_overlap_force_threshold,
                            0.0..=1000.0,
                        )
                        .suffix(" force units"),
                    )
                    .changed()
                {
                    overlap_changed = true;
                }
            });
            ui.small("Electric force magnitude required before any Li⁺ overlap is permitted.");

            ui.horizontal(|ui| {
                ui.label("Overlap at Threshold:");
                if ui
                    .add(
                        egui::Slider::new(
                            &mut self.sim_config.li_overlap_at_threshold,
                            0.0..=2.0,
                        )
                        .step_by(0.01)
                        .suffix(" distance units"),
                    )
                    .changed()
                {
                    overlap_changed = true;
                }
            });
            ui.small("Allowed Li⁺ overlap when the electric force equals the threshold.");

            ui.horizontal(|ui| {
                ui.label("Force for Max Overlap:");
                if ui
                    .add(
                        egui::Slider::new(
                            &mut self.sim_config.li_overlap_force_max,
                            0.0..=1500.0,
                        )
                        .suffix(" force units"),
                    )
                    .changed()
                {
                    overlap_changed = true;
                }
            });
            ui.small("Electric force magnitude that grants the maximum Li⁺ overlap allowance.");

            ui.horizontal(|ui| {
                ui.label("Maximum Overlap:");
                if ui
                    .add(
                        egui::Slider::new(&mut self.sim_config.li_overlap_max, 0.0..=2.0)
                            .step_by(0.01)
                            .suffix(" distance units"),
                    )
                    .changed()
                {
                    overlap_changed = true;
                }
            });
            ui.small("Largest Li⁺ overlap permitted once forces exceed the max threshold.");

            if self.sim_config.li_overlap_force_max
                < self.sim_config.li_overlap_force_threshold
            {
                ui.colored_label(
                    Color32::YELLOW,
                    "Max overlap force should be ≥ the force threshold.",
                );
            }
            if self.sim_config.li_overlap_max < self.sim_config.li_overlap_at_threshold {
                ui.colored_label(
                    Color32::YELLOW,
                    "Maximum overlap should be ≥ the threshold overlap.",
                );
            }

            if overlap_changed {
                self.update_frustration_config();
            }
        });

        ui.separator();

        // Preset configurations
        ui.group(|ui| {
            ui.label(RichText::new("⚡ Quick Presets").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.button("🔧 Conservative").clicked() {
                    self.sim_config.frustration_enabled = true;
                    self.sim_config.frustration_min_force_threshold = 1.0;
                    self.sim_config.frustration_stuck_movement_threshold = 0.05;
                    self.sim_config.frustration_confirmation_steps = 15;
                    self.sim_config.frustration_soft_repulsion_factor = 0.5;
                    self.sim_config.frustration_max_duration = 50;
                    self.sim_config.frustration_position_history_size = 5;
                    self.update_frustration_config();
                }
                ui.small("Conservative settings - harder to trigger, moderate softening");
            });

            ui.horizontal(|ui| {
                if ui.button("⚖️ Balanced").clicked() {
                    self.sim_config.frustration_enabled = true;
                    self.sim_config.frustration_min_force_threshold = 0.5;
                    self.sim_config.frustration_stuck_movement_threshold = 0.1;
                    self.sim_config.frustration_confirmation_steps = 8;
                    self.sim_config.frustration_soft_repulsion_factor = 0.3;
                    self.sim_config.frustration_max_duration = 50;
                    self.sim_config.frustration_position_history_size = 5;
                    self.update_frustration_config();
                }
                ui.small("Default balanced settings");
            });

            ui.horizontal(|ui| {
                if ui.button("🚀 Aggressive").clicked() {
                    self.sim_config.frustration_enabled = true;
                    self.sim_config.frustration_min_force_threshold = 0.3;
                    self.sim_config.frustration_stuck_movement_threshold = 0.15;
                    self.sim_config.frustration_confirmation_steps = 5;
                    self.sim_config.frustration_soft_repulsion_factor = 0.2;
                    self.sim_config.frustration_max_duration = 100;
                    self.sim_config.frustration_position_history_size = 8;
                    self.update_frustration_config();
                }
                ui.small("Aggressive settings - triggers easily, strong softening");
            });
        });

        ui.separator();

        // Performance monitoring
        ui.group(|ui| {
            ui.label(RichText::new("📈 Performance Impact").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Overhead:");
                ui.colored_label(Color32::GREEN, "Minimal");
                ui.label("(Uses existing spatial structures)");
            });
            
            ui.horizontal(|ui| {
                ui.label("Memory Usage:");
                ui.label("~1-5 KB per frustrated particle"); 
            });
        });

        ui.separator();

        // Help and documentation
        ui.collapsing("📖 How It Works", |ui| {
            ui.label(RichText::new("The Soft Dynamics system solves ion desolvation blocking:").strong());
            ui.separator();
            
            ui.label("1. 🔍 Monitors particles with high external forces (like electric fields)");
            ui.label("2. 📊 Detects when these particles can't move despite the forces");
            ui.label("3. ⏱️ Confirms frustration over multiple timesteps to avoid false positives");
            ui.label("4. 🔧 Temporarily softens collision repulsion for confirmed frustrated particles");
            ui.label("5. 🎯 Allows Li+ ions to 'squeeze past' blocking EC/DMC particles");
            ui.label("6. 🔄 Returns to normal physics once particles can move freely");
            
            ui.separator();
            ui.small("This approach maintains physical realism while solving the 2D blocking problem \nwithout requiring complex 3D dynamics.");
        });
    }

    /// Update the simulation's frustration tracker configuration from GUI settings
    fn update_frustration_config(&mut self) {
        // Sanitize Li⁺ overlap parameters before syncing them
        if self.sim_config.li_overlap_force_threshold < 0.0 {
            self.sim_config.li_overlap_force_threshold = 0.0;
        }
        if self.sim_config.li_overlap_force_max < self.sim_config.li_overlap_force_threshold {
            self.sim_config.li_overlap_force_max = self.sim_config.li_overlap_force_threshold;
        }
        if (self.sim_config.li_overlap_force_max - self.sim_config.li_overlap_force_threshold).abs() < 1.0e-6 {
            self.sim_config.li_overlap_force_max =
                self.sim_config.li_overlap_force_threshold + 1.0e-6;
        }
        if self.sim_config.li_overlap_at_threshold < 0.0 {
            self.sim_config.li_overlap_at_threshold = 0.0;
        }
        if self.sim_config.li_overlap_max < self.sim_config.li_overlap_at_threshold {
            self.sim_config.li_overlap_max = self.sim_config.li_overlap_at_threshold;
        }

        // For now, just ensure that the config is synced with the LJ_CONFIG
        // The actual updating will be done through the existing config sync mechanism
        // in simulation.rs where self.config = crate::config::LJ_CONFIG.lock().clone()

        // Update the global config so it gets picked up by the simulation
        let mut global_config = crate::config::LJ_CONFIG.lock();
        global_config.frustration_enabled = self.sim_config.frustration_enabled;
        global_config.frustration_min_force_threshold = self.sim_config.frustration_min_force_threshold;
        global_config.frustration_stuck_movement_threshold = self.sim_config.frustration_stuck_movement_threshold;
        global_config.frustration_confirmation_steps = self.sim_config.frustration_confirmation_steps;
        global_config.frustration_soft_repulsion_factor = self.sim_config.frustration_soft_repulsion_factor;
        global_config.frustration_max_duration = self.sim_config.frustration_max_duration;
        global_config.frustration_position_history_size = self.sim_config.frustration_position_history_size;
        global_config.li_overlap_force_threshold = self.sim_config.li_overlap_force_threshold;
        global_config.li_overlap_at_threshold = self.sim_config.li_overlap_at_threshold;
        global_config.li_overlap_force_max = self.sim_config.li_overlap_force_max;
        global_config.li_overlap_max = self.sim_config.li_overlap_max;
        drop(global_config);
    }
}
