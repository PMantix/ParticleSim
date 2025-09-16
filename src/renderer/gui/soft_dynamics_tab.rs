use super::*;
use quarkstrom::egui::{RichText, Color32};

impl super::super::Renderer {
    pub fn show_soft_dynamics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(" Li+ Soft Collisions");
        ui.separator();

        // Status display
        ui.group(|ui| {
            ui.label(RichText::new(" Status").strong());
            ui.separator();
            
            if self.sim_config.li_soft_collisions_enabled {
                ui.horizontal(|ui| {
                    ui.label("Li+ Soft Collisions:");
                    ui.colored_label(Color32::GREEN, "Enabled");
                });
                
                ui.horizontal(|ui| {
                    ui.label("Scale Factor:");
                    ui.label(format!("{:.3}", self.sim_config.li_soft_collision_scale));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Collision Range:");
                    ui.label(format!("{:.2}x - {:.2}x", 
                        self.sim_config.li_min_collision_factor,
                        self.sim_config.li_max_collision_factor));
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Li+ Soft Collisions:");
                    ui.colored_label(Color32::LIGHT_GRAY, "Disabled");
                });
            }
        });

        ui.separator();

        // Main controls
        ui.group(|ui| {
            ui.label(RichText::new(" Controls").strong());
            ui.separator();
            
            // Enable/disable checkbox
            if ui.checkbox(&mut self.sim_config.li_soft_collisions_enabled, "Enable Li+ Soft Collisions").changed() {
                let mut global_config = crate::config::LJ_CONFIG.lock();
                global_config.li_soft_collisions_enabled = self.sim_config.li_soft_collisions_enabled;
            }

            if self.sim_config.li_soft_collisions_enabled {
                ui.add_space(10.0);
                
                // Scale factor - how much the electric force affects collision softening
                ui.label("Collision Scale Factor:");
                ui.add_space(5.0);
                if ui.add(egui::Slider::new(&mut self.sim_config.li_soft_collision_scale, 0.01..=2.0)
                    .text("Scale")
                    .step_by(0.01)
                ).changed() {
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.li_soft_collision_scale = self.sim_config.li_soft_collision_scale;
                }
                
                ui.add_space(5.0);
                ui.label("Higher values = more collision softening for high electric forces");
                
                ui.add_space(15.0);
                
                // Minimum collision factor
                ui.label("Minimum Collision Factor:");
                ui.add_space(5.0);
                if ui.add(egui::Slider::new(&mut self.sim_config.li_min_collision_factor, 0.1..=1.0)
                    .text("Min Factor")
                    .step_by(0.01)
                ).changed() {
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.li_min_collision_factor = self.sim_config.li_min_collision_factor;
                }
                
                ui.add_space(5.0);
                ui.label("Minimum softening even with low electric forces");
                
                ui.add_space(15.0);
                
                // Maximum collision factor
                ui.label("Maximum Collision Factor:");
                ui.add_space(5.0);
                if ui.add(egui::Slider::new(&mut self.sim_config.li_max_collision_factor, 1.0..=5.0)
                    .text("Max Factor")
                    .step_by(0.01)
                ).changed() {
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.li_max_collision_factor = self.sim_config.li_max_collision_factor;
                }
                
                ui.add_space(5.0);
                ui.label("Maximum softening even with very high electric forces");
            }
        });

        ui.separator();

        // Preset buttons
        ui.group(|ui| {
            ui.label(RichText::new(" Presets").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.button("Conservative").clicked() {
                    self.sim_config.li_soft_collisions_enabled = true;
                    self.sim_config.li_soft_collision_scale = 0.1;
                    self.sim_config.li_min_collision_factor = 0.8;
                    self.sim_config.li_max_collision_factor = 2.0;
                    
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.li_soft_collisions_enabled = self.sim_config.li_soft_collisions_enabled;
                    global_config.li_soft_collision_scale = self.sim_config.li_soft_collision_scale;
                    global_config.li_min_collision_factor = self.sim_config.li_min_collision_factor;
                    global_config.li_max_collision_factor = self.sim_config.li_max_collision_factor;
                }
                
                if ui.button("Moderate").clicked() {
                    self.sim_config.li_soft_collisions_enabled = true;
                    self.sim_config.li_soft_collision_scale = 0.5;
                    self.sim_config.li_min_collision_factor = 0.5;
                    self.sim_config.li_max_collision_factor = 3.0;
                    
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.li_soft_collisions_enabled = self.sim_config.li_soft_collisions_enabled;
                    global_config.li_soft_collision_scale = self.sim_config.li_soft_collision_scale;
                    global_config.li_min_collision_factor = self.sim_config.li_min_collision_factor;
                    global_config.li_max_collision_factor = self.sim_config.li_max_collision_factor;
                }
                
                if ui.button("Aggressive").clicked() {
                    self.sim_config.li_soft_collisions_enabled = true;
                    self.sim_config.li_soft_collision_scale = 1.0;
                    self.sim_config.li_min_collision_factor = 0.2;
                    self.sim_config.li_max_collision_factor = 4.0;
                    
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.li_soft_collisions_enabled = self.sim_config.li_soft_collisions_enabled;
                    global_config.li_soft_collision_scale = self.sim_config.li_soft_collision_scale;
                    global_config.li_min_collision_factor = self.sim_config.li_min_collision_factor;
                    global_config.li_max_collision_factor = self.sim_config.li_max_collision_factor;
                }
            });
            
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label("Conservative: Minimal collision softening");
            });
            ui.horizontal(|ui| {
                ui.label("Moderate: Balanced approach");
            });
            ui.horizontal(|ui| {
                ui.label("Aggressive: Strong collision softening");
            });
        });

        ui.separator();

        // Description/help
        ui.group(|ui| {
            ui.label(RichText::new("ℹ How it Works").strong());
            ui.separator();
            
            ui.label("Li+ Soft Collisions scale collision softening based on electric force magnitude:");
            ui.add_space(5.0);
            ui.label(" Higher electric forces  more collision softening");
            ui.label(" Lower electric forces  normal hard collisions");
            ui.label(" Only affects Li+ ions (other particles unaffected)");
            ui.label(" Scale Factor controls how sensitive to electric forces");
            ui.label(" Min/Max factors set the softening limits");
        });

    }
}
