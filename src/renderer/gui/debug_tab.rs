use super::*;

impl super::super::Renderer {
    pub fn show_debug_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🐛 Debug & Diagnostics");

        ui.group(|ui| {
            ui.label("🔍 Debug Visualizations");
            ui.checkbox(
                &mut self.sim_config.show_lj_vs_coulomb_ratio,
                "Show LJ/Coulomb Force Ratio",
            );
            ui.checkbox(
                &mut self.show_electron_deficiency,
                "Show Electron Deficiency/Excess",
            );
        });
    }
}
