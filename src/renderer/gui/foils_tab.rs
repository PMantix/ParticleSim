use super::*;

impl super::super::Renderer {
    pub fn show_foils_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔋 Foil Controls");

        // Foil Selection for Linking
        ui.group(|ui| {
            ui.label("🎯 Foil Selection for Linking");
            ui.label("Select foils by clicking on them in the simulation, or use the list below:");

            let foils = FOILS.lock();
            if !foils.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Available foils:");
                    for foil in foils.iter() {
                        let is_selected = self.selected_foil_ids.contains(&foil.id);
                        let button_text = if is_selected {
                            format!("✓ Foil {}", foil.id)
                        } else {
                            format!("Foil {}", foil.id)
                        };

                        if ui.button(button_text).clicked() {
                            if is_selected {
                                // Remove from selection
                                self.selected_foil_ids.retain(|&id| id != foil.id);
                            } else {
                                // Add to selection (limit to 2 for linking)
                                if self.selected_foil_ids.len() < 2 {
                                    self.selected_foil_ids.push(foil.id);
                                } else {
                                    // Replace oldest selection
                                    self.selected_foil_ids.remove(0);
                                    self.selected_foil_ids.push(foil.id);
                                }
                            }
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Selected: {}/2 foils",
                        self.selected_foil_ids.len()
                    ));
                    if ui.button("Clear Selection").clicked() {
                        self.selected_foil_ids.clear();
                    }
                });
            } else {
                ui.label("No foils available. Add foils in the Scenario tab first.");
            }
        });

        ui.separator();

        // Foil Linking Controls
        ui.group(|ui| {
            ui.label("🔗 Foil Link Controls");
            if self.selected_foil_ids.len() == 2 {
                let a = self.selected_foil_ids[0];
                let b = self.selected_foil_ids[1];
                let foils = FOILS.lock();
                let linked = foils
                    .iter()
                    .find(|f| f.id == a)
                    .and_then(|f| f.link_id)
                    .map(|id| id == b)
                    .unwrap_or(false);

                ui.label(format!("Selected foils: {} and {}", a, b));

                if linked {
                    ui.label("✅ These foils are currently linked");
                    if ui.button("🔓 Unlink Foils").clicked() {
                        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = sender.send(SimCommand::UnlinkFoils { a, b });
                        }
                    }
                } else {
                    ui.label("❌ These foils are not linked");
                    ui.horizontal(|ui| {
                        if ui.button("🔗 Link Parallel").clicked() {
                            if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                                let _ = sender.send(SimCommand::LinkFoils {
                                    a,
                                    b,
                                    mode: LinkMode::Parallel,
                                });
                            }
                        }
                        if ui.button("🔗 Link Opposite").clicked() {
                            if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                                let _ = sender.send(SimCommand::LinkFoils {
                                    a,
                                    b,
                                    mode: LinkMode::Opposite,
                                });
                            }
                        }
                    });
                    ui.label("Parallel: same current | Opposite: inverted current");
                }
            } else {
                ui.label("Select exactly 2 foils above to link them together.");
                ui.label("Linked foils share current settings - one controls both.");
            }
        });

        ui.separator();

        // Foil Current Controls for Selected Foil
        if let Some(selected_id) = self.selected_particle_id {
            let maybe_foil = {
                let foils = FOILS.lock();
                foils
                    .iter()
                    .find(|f| f.body_ids.contains(&selected_id))
                    .cloned()
            };
            if let Some(foil) = maybe_foil {
                ui.group(|ui| {
                    ui.label("⚡ Current Controls");
                    ui.label(format!(
                        "Configuring Foil {} (selected in simulation)",
                        foil.id
                    ));

                    // DC Current control
                    let mut dc_current = foil.dc_current;
                    ui.horizontal(|ui| {
                        ui.label("DC Current:");
                        if ui.button("-").clicked() {
                            dc_current -= 1.0;
                        }
                        if ui.button("+").clicked() {
                            dc_current += 1.0;
                        }
                        if ui.button("0").clicked() {
                            dc_current = 0.0;
                        }
                        ui.add(egui::Slider::new(&mut dc_current, -500.0..=500.00).step_by(0.1));
                    });
                    if (dc_current - foil.dc_current).abs() > f32::EPSILON {
                        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = sender.send(SimCommand::SetFoilDCCurrent {
                                foil_id: foil.id,
                                dc_current,
                            });
                        }
                    }

                    // AC Current control
                    let mut ac_current = foil.ac_current;
                    ui.horizontal(|ui| {
                        ui.label("AC Amplitude:");
                        if ui.button("-").clicked() {
                            ac_current -= 1.0;
                        }
                        if ui.button("+").clicked() {
                            ac_current += 1.0;
                        }
                        if ui.button("0").clicked() {
                            ac_current = 0.0;
                        }
                        ui.add(egui::Slider::new(&mut ac_current, 0.0..=500.00).step_by(0.1));
                    });
                    if (ac_current - foil.ac_current).abs() > f32::EPSILON {
                        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = sender.send(SimCommand::SetFoilACCurrent {
                                foil_id: foil.id,
                                ac_current,
                            });
                        }
                    }

                    let mut hz = foil.switch_hz;
                    ui.horizontal(|ui| {
                        ui.label("Switch Hz:");
                        ui.add(egui::DragValue::new(&mut hz).speed(0.1));
                    });
                    if (hz - foil.switch_hz).abs() > f32::EPSILON {
                        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = sender.send(SimCommand::SetFoilFrequency {
                                foil_id: foil.id,
                                switch_hz: hz,
                            });
                        }
                    }
                });

                ui.separator();

                // Current Waveform Plot
                ui.group(|ui| {
                    ui.label("📈 Current Waveform");
                    use egui::plot::{Line, Plot, PlotPoints};
                    let seconds = 5.0;
                    let steps = 200;
                    // Use actual simulation time and respect pause state
                    let sim_time = *SIM_TIME.lock();
                    let is_paused = PAUSED.load(std::sync::atomic::Ordering::Relaxed);
                    let current_time = if is_paused {
                        // When paused, freeze the time display
                        sim_time
                    } else {
                        sim_time
                    };
                    let selected_ids = self.selected_foil_ids.clone();
                    Plot::new("foil_wave_plot")
                        .height(100.0)
                        .allow_scroll(false)
                        .allow_zoom(false)
                        .show(ui, |plot_ui| {
                            let colors = [
                                egui::Color32::LIGHT_BLUE,
                                egui::Color32::LIGHT_RED,
                                egui::Color32::LIGHT_GREEN,
                                egui::Color32::YELLOW,
                            ];
                            let foils = FOILS.lock();
                            for (idx, fid) in selected_ids.iter().enumerate() {
                                if let Some(f) = foils.iter().find(|f| f.id == *fid) {
                                    let dt = seconds / steps as f32;
                                    let mut points_vec: Vec<[f64; 2]> =
                                        Vec::with_capacity(steps + 1);
                                    for i in 0..=steps {
                                        let t = i as f32 * dt;
                                        let effective_current = if let Some(link_id) = f.link_id {
                                            // For linked foils, determine if this is master or slave
                                            let is_master = f.id < link_id;
                                            if is_master {
                                                // Master calculates from its own DC + AC components
                                                let mut current = f.dc_current;
                                                if f.switch_hz > 0.0 {
                                                    let plot_time = current_time + t;
                                                    let ac_component =
                                                        if (plot_time * f.switch_hz) % 1.0 < 0.5 {
                                                            f.ac_current
                                                        } else {
                                                            -f.ac_current
                                                        };
                                                    current += ac_component;
                                                }
                                                current
                                            } else {
                                                // Slave uses the propagated current value (but for plot, we need to calculate what it would be)
                                                // Find the master foil to calculate its effective current
                                                if let Some(master_foil) =
                                                    foils.iter().find(|mf| mf.id == link_id)
                                                {
                                                    let mut master_current = master_foil.dc_current;
                                                    if master_foil.switch_hz > 0.0 {
                                                        let plot_time = current_time + t;
                                                        let ac_component = if (plot_time
                                                            * master_foil.switch_hz)
                                                            % 1.0
                                                            < 0.5
                                                        {
                                                            master_foil.ac_current
                                                        } else {
                                                            -master_foil.ac_current
                                                        };
                                                        master_current += ac_component;
                                                    }
                                                    // Apply link mode
                                                    match master_foil.mode {
                                                        crate::body::foil::LinkMode::Parallel => {
                                                            master_current
                                                        }
                                                        crate::body::foil::LinkMode::Opposite => {
                                                            -master_current
                                                        }
                                                    }
                                                } else {
                                                    f.dc_current // Fallback to DC current
                                                }
                                            }
                                        } else {
                                            // Non-linked foil
                                            let mut current = f.dc_current;
                                            if f.switch_hz > 0.0 {
                                                let plot_time = current_time + t;
                                                let ac_component =
                                                    if (plot_time * f.switch_hz) % 1.0 < 0.5 {
                                                        f.ac_current
                                                    } else {
                                                        -f.ac_current
                                                    };
                                                current += ac_component;
                                            }
                                            current
                                        };
                                        points_vec.push([t as f64, effective_current as f64]);
                                    }
                                    let points = PlotPoints::from(points_vec);
                                    plot_ui
                                        .line(Line::new(points).color(colors[idx % colors.len()]));
                                }
                            }
                        });
                });
            }
        } else {
            ui.group(|ui| {
                ui.label("� How to control foil currents:");
                ui.label("• Select a foil particle in the simulation (Shift+Click)");
                ui.label("• Or create foils in the Scenario tab first");
                ui.label("• Current controls will appear here when a foil is selected");
            });
        }

        // Direct Current Controls for All Foils
        ui.separator();
        let foils = FOILS.lock();
        if !foils.is_empty() {
            ui.group(|ui| {
                ui.label("⚡ Quick Current Controls");
                ui.label("Adjust any foil's current directly:");

                for foil in foils.iter() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Foil {}", foil.id));
                        if foil.link_id.is_some() {
                            ui.label("🔗");
                        }

                        // DC Current
                        let mut dc_current = foil.dc_current;
                        if ui.small_button("-1").clicked() {
                            dc_current -= 1.0;
                        }
                        if ui.small_button("+1").clicked() {
                            dc_current += 1.0;
                        }
                        ui.add(
                            egui::DragValue::new(&mut dc_current)
                                .prefix("DC: ")
                                .speed(0.1),
                        );

                        // AC Current
                        let mut ac_current = foil.ac_current;
                        ui.add(
                            egui::DragValue::new(&mut ac_current)
                                .prefix("AC: ")
                                .speed(0.1)
                                .clamp_range(0.0..=500.0),
                        );

                        // Frequency
                        let mut hz = foil.switch_hz;
                        ui.add(
                            egui::DragValue::new(&mut hz)
                                .prefix("Hz: ")
                                .speed(0.1)
                                .clamp_range(0.0..=100.0),
                        );

                        // Apply changes
                        if (dc_current - foil.dc_current).abs() > f32::EPSILON {
                            if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                                let _ = sender.send(SimCommand::SetFoilDCCurrent {
                                    foil_id: foil.id,
                                    dc_current,
                                });
                            }
                        }
                        if (ac_current - foil.ac_current).abs() > f32::EPSILON {
                            if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                                let _ = sender.send(SimCommand::SetFoilACCurrent {
                                    foil_id: foil.id,
                                    ac_current,
                                });
                            }
                        }
                        if (hz - foil.switch_hz).abs() > f32::EPSILON {
                            if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                                let _ = sender.send(SimCommand::SetFoilFrequency {
                                    foil_id: foil.id,
                                    switch_hz: hz,
                                });
                            }
                        }
                    });
                }
            });
        }
    }
}
