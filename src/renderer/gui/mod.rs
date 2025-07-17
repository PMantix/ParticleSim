use super::state::*;
use crate::body::foil::LinkMode;
use crate::body::Electron;
use crate::body::Species;
use crate::config::IsolineFieldMode;
use crate::renderer::Body;
use quarkstrom::egui;
use ultraviolet::Vec2;

pub mod analysis_tab;
pub mod debug_tab;
pub mod diagnostics_tab;
pub mod foils_tab;
pub mod physics_tab;
pub mod scenario_tab;
pub mod screen_capture_tab;
pub mod simulation_tab;
pub mod species_tab;
pub mod visualization_tab;

pub use scenario_tab::make_body_with_species;

impl super::Renderer {
    pub fn show_gui(&mut self, ctx: &quarkstrom::egui::Context) {
        let mut settings_open = self.settings_window_open;
        egui::Window::new("Particle Simulation Controls")
            .default_width(320.0)
            .resizable(true)
            .open(&mut settings_open)
            .show(ctx, |ui| {
                // Status header - always visible
                self.show_status_header(ui);

                ui.separator();

                // Tab bar - organized in two rows
                ui.vertical(|ui| {
                    // First row of tabs
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Simulation,
                            "⚙️ Simulation",
                        );
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Visualization,
                            "👁️ Visualization",
                        );
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Species,
                            "🔬 Species",
                        );
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Physics,
                            "⚛️ Physics",
                        );
                    });
                    // Second row of tabs
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Scenario,
                            "🌐 Scenario",
                        );
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Foils,
                            "🔋 Foils",
                        );
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Analysis,
                            "📊 Analysis",
                        );
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Diagnostics,
                            "🔬 Diagnostics",
                        );
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::Debug,
                            "🐛 Debug",
                        );
                    });
                    // Third row of tabs
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::ScreenCapture,
                            "📷 Screen Capture",
                        );
                    });
                });

                ui.separator();

                // Show content based on selected tab
                egui::ScrollArea::vertical()
                    .auto_shrink([true, false])
                    .show(ui, |ui| match self.current_tab {
                    super::GuiTab::Simulation => self.show_simulation_tab(ui),
                    super::GuiTab::Visualization => self.show_visualization_tab(ui),
                    super::GuiTab::Species => self.show_species_tab(ui),
                    super::GuiTab::Physics => self.show_physics_tab(ui),
                    super::GuiTab::Scenario => self.show_scenario_tab(ui),
                    super::GuiTab::Foils => self.show_foils_tab(ui),
                    super::GuiTab::Analysis => self.show_analysis_tab(ui),
                    super::GuiTab::Debug => self.show_debug_tab(ui),
                    super::GuiTab::Diagnostics => self.show_diagnostics_tab(ui),
                    super::GuiTab::ScreenCapture => self.show_screen_capture_tab(ui),
                });
            });

        self.settings_window_open = settings_open;

        // Show plotting control window if open
        if self.show_plotting_window {
            crate::plotting::gui::show_plotting_window(
                ctx,
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

        // Show individual plot windows
        crate::plotting::gui::show_plot_windows(ctx, &mut self.plotting_system);
    }

    fn show_status_header(&mut self, ui: &mut egui::Ui) {
        // Use actual simulation time, not renderer time
        let sim_time = *SIM_TIME.lock();
        ui.label(format!("Time: {:.2} s", sim_time));

        // Show pause status
        let is_paused = PAUSED.load(std::sync::atomic::Ordering::Relaxed);
        if is_paused {
            ui.colored_label(egui::Color32::YELLOW, "⏸ PAUSED");
        } else {
            ui.colored_label(egui::Color32::GREEN, "▶ RUNNING");
        }
    }
}
