use super::*;
use quarkstrom::egui::{RichText, Color32};

impl super::super::Renderer {
    pub fn show_soft_dynamics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(" Li+ Collision Softness");
        ui.separator();

        // Status display
        ui.group(|ui| {
            ui.label(RichText::new(" Status").strong());
            ui.separator();

            let s = self.sim_config.li_collision_softness.clamp(0.0, 1.0);
            ui.horizontal(|ui| {
                ui.label("Softness:");
                let color = if s == 0.0 { Color32::LIGHT_GRAY } else { Color32::GREEN };
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

            // Single softness slider
            if ui
                .add(
                    egui::Slider::new(&mut self.sim_config.li_collision_softness, 0.0..=1.0)
                        .text("Li+ Collision Softness")
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

            ui.label("Applies a simple multiplicative reduction to collision corrections for pairs involving Li+ ions.");
            ui.label("Other species are unaffected. No dependence on electric force magnitude.");
        });
    }
}
