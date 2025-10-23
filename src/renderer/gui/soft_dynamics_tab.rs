use super::*;
use quarkstrom::egui::{Color32, RichText};

impl super::super::Renderer {
    pub fn show_soft_dynamics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(" Soft Collisions");
        ui.separator();

        // Status display
        ui.group(|ui| {
            ui.label(RichText::new(" Status").strong());
            ui.separator();

            let s = self.sim_config.li_collision_softness.clamp(0.0, 1.0);
            ui.horizontal(|ui| {
                ui.label("Softness:");
                let color = if s == 0.0 {
                    Color32::LIGHT_GRAY
                } else {
                    Color32::GREEN
                };
                ui.colored_label(color, format!("{:.3}", s));
            });
            ui.horizontal(|ui| {
                ui.label("0.0 = hard collisions, 1.0 = very soft (reduced correction)");
            });
        });

        ui.separator();

        // Main controls
        ui.group(|ui| {
            ui.label(RichText::new(" Controls").strong());
            ui.separator();

            // Species toggles
            ui.horizontal(|ui| {
                let mut li = self.sim_config.soft_collision_lithium_ion;
                if ui.checkbox(&mut li, "Li+ (cations)").changed() {
                    self.sim_config.soft_collision_lithium_ion = li;
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.soft_collision_lithium_ion = li;
                }
                let mut an = self.sim_config.soft_collision_anion;
                if ui.checkbox(&mut an, "Anions").changed() {
                    self.sim_config.soft_collision_anion = an;
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.soft_collision_anion = an;
                }
            });

            // Single softness slider
            if ui
                .add(
                    egui::Slider::new(&mut self.sim_config.li_collision_softness, 0.0..=1.0)
                        .text("Collision Softness Factor")
                        .step_by(0.01),
                )
                .changed()
            {
                let mut global_config = crate::config::LJ_CONFIG.lock();
                global_config.li_collision_softness = self.sim_config.li_collision_softness;
            }

            if self.sim_config.li_collision_softness == 0.0 {
                ui.add_space(6.0);
                ui.label("Softness at 0.0 reproduces baseline hard collisions (no change).");
            }
        });

        ui.separator();

        // Description/help
        ui.group(|ui| {
            ui.label(RichText::new(" ℹ How it Works").strong());
            ui.separator();

            ui.label("Applies a simple multiplicative reduction to collision corrections for selected species pairs (currently Li+ and Anions). No dependence on electric force magnitude.");
        });
    }
}
