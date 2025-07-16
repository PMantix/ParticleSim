use super::*;

impl super::super::Renderer {
    pub fn show_analysis_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("📊 Analysis & Plotting");

        // Plotting & Analysis
        crate::plotting::gui::show_plotting_controls(
            ui,
            &mut self.plotting_system,
            &mut self.show_plotting_window,
            &mut self.new_plot_type,
            &mut self.new_plot_quantity,
            &mut self.new_plot_sampling_mode,
            &mut self.new_plot_title,
            &mut self.new_plot_spatial_bins,
            &mut self.new_plot_time_window,
            &mut self.new_plot_update_frequency,
        );
    }
}
