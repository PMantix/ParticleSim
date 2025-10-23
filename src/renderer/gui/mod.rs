use super::state::*;
use crate::body::Electron;
use crate::body::Species;
use crate::config::IsolineFieldMode;
use crate::profile_scope;
use crate::renderer::Body;
//use crate::switch_charging; // legacy UI is now embedded in Charging tab
use quarkstrom::egui::{self, Vec2 as EVec2};
use ultraviolet::Vec2;

pub mod analysis_tab;
pub mod charging_tab;
pub mod debug_tab;
pub mod diagnostics_tab;
pub mod measurement_tab;
pub mod physics_tab;
pub mod pid_controller;
pub mod scenario_tab;
pub mod simulation_tab;
pub mod soft_dynamics_tab;
pub mod species_tab;
pub mod visualization_tab;

pub use scenario_tab::make_body_with_species;

impl super::Renderer {
    pub fn show_gui(&mut self, ctx: &quarkstrom::egui::Context) {
        profile_scope!("gui_update");
        // Apply any persisted UI updates (e.g., after loading a scenario)
        self.sync_persisted_ui();
        if self.show_splash {
            self.show_splash_screen(ctx);
            return;
        }
        // Sync domain size from shared state (updated by simulation)
        self.domain_width = *crate::renderer::state::DOMAIN_WIDTH.lock();
        self.domain_height = *crate::renderer::state::DOMAIN_HEIGHT.lock();

        let mut settings_open = self.settings_window_open;
        egui::Window::new("Particle Simulation Controls")
            .default_width(500.0)
            .default_height(650.0)
            .default_pos(egui::Pos2::new(10.0, 10.0))
            .resizable(true)
            .open(&mut settings_open)
            .default_open(true) // Start open
            .collapsible(true)
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
                            super::GuiTab::Charging,
                            "⚡ Charging",
                        );
                        // Foils tab removed (merged into Charging workflows)
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
                            super::GuiTab::Measurement,
                            "📏 Measurement",
                        );
                        // Switch Charging tab removed (embedded under Charging)
                        ui.selectable_value(
                            &mut self.current_tab,
                            super::GuiTab::SoftDynamics,
                            "🔧 Soft Dynamics",
                        );
                    });
                });

                ui.separator();

                if self.current_tab != super::GuiTab::Measurement {
                    self.last_non_measurement_tab = self.current_tab;
                }

                self.switch_ui_state
                    .sync_sim_dt(*crate::renderer::state::TIMESTEP.lock());
                self.switch_ui_state.update_available_foils(&self.foils);
                let selected_for_assignment = self.selected_foil_ids.last().copied();
                self.switch_ui_state
                    .consume_selected_foil(selected_for_assignment);

                // Show content based on selected tab
                egui::ScrollArea::vertical()
                    .auto_shrink([true, false])
                    .show(ui, |ui| match self.current_tab {
                        super::GuiTab::Simulation => self.show_simulation_tab(ui),
                        super::GuiTab::Visualization => self.show_visualization_tab(ui),
                        super::GuiTab::Species => self.show_species_tab(ui),
                        super::GuiTab::Physics => self.show_physics_tab(ui),
                        super::GuiTab::Scenario => self.show_scenario_tab(ui),
                        super::GuiTab::Charging => self.show_charging_tab(ui),
                        // Removed tabs routed here previously are no longer used
                        super::GuiTab::Measurement => self.show_measurement_tab(ui),
                        super::GuiTab::Analysis => self.show_analysis_tab(ui),
                        super::GuiTab::Debug => self.show_debug_tab(ui),
                        super::GuiTab::Diagnostics => self.show_diagnostics_tab(ui),
                        super::GuiTab::SoftDynamics => self.show_soft_dynamics_tab(ui),
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

        // Show PID graph window if enabled
        self.show_pid_graph(ctx);
    }

    fn show_splash_screen(&mut self, ctx: &egui::Context) {
        use egui::{vec2, Align2, Color32, FontId, Pos2};
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            let width = rect.width();
            let height = rect.height();
            let char_w = self.char_size;
            let char_h = self.char_size;
            let start_x = rect.center().x - self.splash_art_width as f32 * char_w / 2.0;
            let start_y = rect.center().y - self.splash_art_height as f32 * char_h / 2.0 - 40.0;
            let mut rects = Vec::with_capacity(self.splash_chars.len());
            for ch in &self.splash_chars {
                let x = start_x + ch.col as f32 * char_w;
                let y = start_y + ch.row as f32 * char_h;
                let r = egui::Rect::from_min_size(Pos2::new(x, y), vec2(char_w, char_h));
                rects.push(r);
                ui.painter().text(
                    Pos2::new(x, y),
                    Align2::LEFT_TOP,
                    ch.ch,
                    FontId::monospace(char_h),
                    ch.color,
                );
            }

            // Get mouse position for particle interaction
            let mouse_pos = ui.input(|i| {
                if let Some(pos) = i.pointer.hover_pos() {
                    Some(EVec2::new(pos.x, pos.y))
                } else {
                    None
                }
            });

            self.update_splash_particles(width, height, &rects, mouse_pos);

            // Draw simple asterisk particles in muted grey
            for p in &self.splash_particles {
                ui.painter().text(
                    p.pos,
                    Align2::CENTER_CENTER,
                    "*",
                    FontId::monospace(char_h),
                    Color32::from_gray(120), // Muted grey color
                );
            }

            // Draw pop effects
            for effect in &self.pop_effects {
                let alpha = (effect.life * 255.0) as u8;
                let color = Color32::from_rgba_unmultiplied(200, 200, 200, alpha);
                ui.painter().text(
                    effect.pos,
                    Align2::CENTER_CENTER,
                    effect.char.to_string(),
                    FontId::monospace(char_h * 0.8),
                    color,
                );
            }
            let y = start_y + self.splash_art_height as f32 * char_h + 10.0;
            let center_x = rect.center().x;

            // Add contact info in lower right corner
            ui.allocate_ui_at_rect(rect, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
                    ui.add_space(10.0);
                    // GitHub link - right aligned
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.link("https://github.com/PMantix/ParticleSim").clicked() {
                                if let Err(_) = std::process::Command::new("cmd")
                                    .args(&[
                                        "/c",
                                        "start",
                                        "https://github.com/PMantix/ParticleSim",
                                    ])
                                    .spawn()
                                {
                                    // Fallback for other systems
                                    let _ = std::process::Command::new("xdg-open")
                                        .arg("https://github.com/PMantix/ParticleSim")
                                        .spawn();
                                }
                            }
                        });
                    });
                    // Email - right aligned
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("paquino@honda-ri.com");
                        });
                    });
                });
            });

            // Use vertical spacing to position center elements
            ui.add_space(y - ui.cursor().top());
            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.add_space(center_x - 150.0 - ui.cursor().left()); // Adjusted for wider content
                ui.label("Scenario Selection:");
                ui.add_space(10.0);
                egui::ComboBox::from_id_source("scenario_select")
                    .selected_text(&self.scenarios[self.selected_scenario])
                    .show_ui(ui, |ui| {
                        for (i, s) in self.scenarios.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_scenario, i, s);
                        }
                    });
            });

            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.add_space(center_x - 140.0 - ui.cursor().left());
                ui.label("Right-click or press any key to continue");
            });

            // Check for RIGHT mouse clicks to exit splash screen
            // Left clicks are reserved for UI interaction
            if ui.input(|i| i.pointer.secondary_clicked()) {
                self.start_selected_scenario();
            }
        });
    }

    fn show_status_header(&mut self, ui: &mut egui::Ui) {
        let status = PLAYBACK_STATUS.lock().clone();
        let is_paused = PAUSED.load(std::sync::atomic::Ordering::Relaxed);
        let time_label = format!("Time: {:.2} fs", status.sim_time);
        let frame_label = format!("Frame {}", status.frame);
        let history_label = format!("History {}/{}", status.cursor, status.latest_index);

        let (mode_text, color) = match status.mode {
            PlaybackModeStatus::Live => {
                if is_paused {
                    ("Live (Paused)", egui::Color32::YELLOW)
                } else {
                    ("Live", egui::Color32::GREEN)
                }
            }
            PlaybackModeStatus::HistoryPaused => ("History (Paused)", egui::Color32::YELLOW),
            PlaybackModeStatus::HistoryPlaying => ("History (Playing)", egui::Color32::LIGHT_BLUE),
        };

        ui.horizontal(|ui| {
            ui.label(time_label);
            ui.separator();
            ui.label(frame_label);
            ui.separator();
            ui.label(history_label);
            ui.separator();
            ui.colored_label(color, mode_text);
            if status.is_playing {
                ui.separator();
                ui.label(format!("Speed: {:.1}×", status.speed));
            }
        });

        // Charging mode indicator row
        ui.horizontal(|ui| {
            ui.label("Charging:");
            // If in playback, prefer historical context
            match status.mode {
                PlaybackModeStatus::HistoryPaused | PlaybackModeStatus::HistoryPlaying => {
                    let step_opt = *crate::renderer::state::SWITCH_STEP.lock();
                    if let Some(s) = step_opt {
                        ui.colored_label(
                            egui::Color32::LIGHT_BLUE,
                            format!("Switch Charging (History) • Step {}", s + 1),
                        );
                    } else {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "Conventional (History)");
                    }
                }
                PlaybackModeStatus::Live => match self.charging_ui_mode {
                    super::ChargingUiMode::Conventional | super::ChargingUiMode::Advanced => {
                        let label =
                            if matches!(self.charging_ui_mode, super::ChargingUiMode::Advanced) {
                                "Advanced"
                            } else {
                                "Conventional"
                            };
                        ui.colored_label(egui::Color32::LIGHT_GREEN, label);
                        let mode_str = if self.conventional_is_overpotential {
                            "Overpotential"
                        } else {
                            "Current"
                        };
                        ui.separator();
                        ui.small(format!("Group control: {}", mode_str));
                    }
                    super::ChargingUiMode::SwitchCharging => {
                        let (label, color) = match self.switch_ui_state.run_state {
                            crate::switch_charging::RunState::Idle => {
                                ("Switch Charging (Idle)", egui::Color32::LIGHT_GRAY)
                            }
                            crate::switch_charging::RunState::Running => {
                                ("Switch Charging (Running)", egui::Color32::LIGHT_GREEN)
                            }
                            crate::switch_charging::RunState::Paused => {
                                ("Switch Charging (Paused)", egui::Color32::YELLOW)
                            }
                        };
                        ui.colored_label(color, label);
                    }
                },
            }
        });
    }
}
