use super::*;

impl super::super::Renderer {
    pub fn show_foils_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔋 Foil Controls");

        // Advanced toggle
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.foils_advanced_controls, "Show advanced per-foil current controls");
            if self.foils_advanced_controls {
                ui.small("(DC/AC controls per foil and quick multi-foil controls)");
            }
        });

        ui.separator();

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

        // Main current control for selected foil moved near top
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
                    ui.label("⚡ Current Controls (Selected Foil)");
                    ui.label(format!("Configuring Foil {} (selected in simulation)", foil.id));

                    // DC Current control
                    let mut dc_current = foil.dc_current;
                    ui.horizontal(|ui| {
                        ui.label("DC Current:");
                        if ui.button("-").clicked() { dc_current -= 1.0; }
                        if ui.button("+").clicked() { dc_current += 1.0; }
                        if ui.button("0").clicked() { dc_current = 0.0; }
                        ui.add(egui::Slider::new(&mut dc_current, -500.0..=500.00).step_by(0.01));
                    });
                    if (dc_current - foil.dc_current).abs() > f32::EPSILON {
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::SetFoilDCCurrent { foil_id: foil.id, dc_current })
                            .unwrap();
                    }

                    // AC Current control
                    let mut ac_current = foil.ac_current;
                    ui.horizontal(|ui| {
                        ui.label("AC Amplitude:");
                        if ui.button("-").clicked() { ac_current -= 1.0; }
                        if ui.button("+").clicked() { ac_current += 1.0; }
                        if ui.button("0").clicked() { ac_current = 0.0; }
                        ui.add(egui::Slider::new(&mut ac_current, 0.0..=500.00).step_by(0.01));
                    });
                    if (ac_current - foil.ac_current).abs() > f32::EPSILON {
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::SetFoilACCurrent { foil_id: foil.id, ac_current })
                            .unwrap();
                    }

                    let mut hz = foil.switch_hz;
                    ui.horizontal(|ui| {
                        ui.label("Switch Hz:");
                        ui.add(egui::DragValue::new(&mut hz).speed(0.1));
                    });
                    if (hz - foil.switch_hz).abs() > f32::EPSILON {
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::SetFoilFrequency { foil_id: foil.id, switch_hz: hz })
                            .unwrap();
                    }

                    ui.separator();

                    // Charging Mode Control
                    ui.group(|ui| {
                        ui.label("⚡ Charging Mode");
                        let current_mode = foil.charging_mode;
                        let mut new_mode = current_mode;
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut new_mode, crate::body::foil::ChargingMode::Current, "Direct Current");
                            ui.radio_value(&mut new_mode, crate::body::foil::ChargingMode::Overpotential, "Overpotential");
                        });
                        if new_mode != current_mode {
                            if new_mode == crate::body::foil::ChargingMode::Overpotential {
                                SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::EnableOverpotentialMode { foil_id: foil.id, target_ratio: 1.2 }).unwrap();
                            } else {
                                SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::DisableOverpotentialMode { foil_id: foil.id }).unwrap();
                            }
                        }
                        if foil.charging_mode == crate::body::foil::ChargingMode::Overpotential {
                            if let Some(ref controller) = foil.overpotential_controller {
                                ui.separator();
                                ui.label("🎯 Overpotential Settings");
                                let mut target_ratio = controller.target_ratio;
                                ui.horizontal(|ui| {
                                    ui.label("Target Electron Ratio:");
                                    if ui.button("Cathodic").clicked() { target_ratio = 1.5; }
                                    if ui.button("Neutral").clicked() { target_ratio = 1.0; }
                                    if ui.button("Anodic").clicked() { target_ratio = 0.5; }
                                    ui.add(egui::Slider::new(&mut target_ratio, 0.1..=2.0).step_by(0.01));
                                });
                                if (target_ratio - controller.target_ratio).abs() > f32::EPSILON {
                                    SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::SetFoilOverpotentialTarget { foil_id: foil.id, target_ratio }).unwrap();
                                }
                                ui.horizontal(|ui| {
                                    ui.label("PID Gains:");
                                    ui.label(format!("P:{:.1} I:{:.1} D:{:.1}", controller.kp, controller.ki, controller.kd));
                                });
                            }
                        }
                    });
                });

                ui.separator();
            }
        }

        // Group Linking UI
        ui.group(|ui| {
            ui.label("👥 Foil Group Linking (A/B)");
            let foils = FOILS.lock();
            if foils.is_empty() {
                ui.label("No foils available to group.");
            } else {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Group A:");
                        for foil in foils.iter() {
                            let mut in_a = self.group_a_selected.contains(&foil.id);
                            if ui.checkbox(&mut in_a, format!("Foil {}", foil.id)).clicked() {
                                if in_a {
                                    if !self.group_a_selected.contains(&foil.id) {
                                        self.group_a_selected.push(foil.id);
                                    }
                                    // ensure not in B
                                    self.group_b_selected.retain(|&id| id != foil.id);
                                } else {
                                    self.group_a_selected.retain(|&id| id != foil.id);
                                }
                            }
                        }
                        if ui.button("Clear A").clicked() { self.group_a_selected.clear(); }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("Group B:");
                        for foil in foils.iter() {
                            let mut in_b = self.group_b_selected.contains(&foil.id);
                            if ui.checkbox(&mut in_b, format!("Foil {}", foil.id)).clicked() {
                                if in_b {
                                    if !self.group_b_selected.contains(&foil.id) {
                                        self.group_b_selected.push(foil.id);
                                    }
                                    // ensure not in A
                                    self.group_a_selected.retain(|&id| id != foil.id);
                                } else {
                                    self.group_b_selected.retain(|&id| id != foil.id);
                                }
                            }
                        }
                        if ui.button("Clear B").clicked() { self.group_b_selected.clear(); }
                    });
                });
                ui.horizontal(|ui| {
                    if ui.button("Apply Groups").clicked() {
                        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = sender.send(SimCommand::SetFoilGroups {
                                group_a: self.group_a_selected.clone(),
                                group_b: self.group_b_selected.clone(),
                            });
                        }
                    }
                    if ui.button("Clear Groups").clicked() {
                        self.group_a_selected.clear();
                        self.group_b_selected.clear();
                        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = sender.send(SimCommand::ClearFoilGroups);
                        }
                    }
                });
                ui.label("Within-group: parallel; Between-group: opposite.");
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
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::UnlinkFoils { a, b })
                            .unwrap();
                    }
                } else {
                    ui.label("❌ These foils are not linked");
                    ui.horizontal(|ui| {
                        if ui.button("🔗 Link Parallel").clicked() {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::LinkFoils {
                                    a,
                                    b,
                                    mode: LinkMode::Parallel,
                                })
                                .unwrap();
                        }
                        if ui.button("🔗 Link Opposite").clicked() {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::LinkFoils {
                                    a,
                                    b,
                                    mode: LinkMode::Opposite,
                                })
                                .unwrap();
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

        // All Foils Electron Ratio Overview 
        ui.group(|ui| {
            ui.label("🔋 All Foils Electron Ratios");
            
            if let Some(diag) = &mut self.foil_electron_fraction_diagnostic {
                let foils = FOILS.lock();
                for foil in foils.iter() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Foil {}:", foil.id));
                        if let Some(ratio) = diag.fractions.get(&foil.id) {
                            let ratio_color = if *ratio > 1.05 {
                                egui::Color32::LIGHT_BLUE  // Cathodic (electron-rich)
                            } else if *ratio < 0.95 {
                                egui::Color32::LIGHT_RED   // Anodic (electron-poor)
                            } else {
                                egui::Color32::WHITE       // Near neutral
                            };
                            ui.colored_label(ratio_color, format!("{:.3}", ratio));
                            
                            // Show charging mode
                            let mode_text = match foil.charging_mode {
                                crate::body::foil::ChargingMode::Current => "Current",
                                crate::body::foil::ChargingMode::Overpotential => "Overpotential",
                            };
                            ui.label(format!("({})", mode_text));
                        } else {
                            ui.label("N/A");
                        }
                    });
                }
                
                if foils.is_empty() {
                    ui.label("No foils available");
                }
            } else {
                ui.label("❌ Diagnostic not available");
            }
        });

        ui.separator();

        // Individual Foil Controls (Advanced)
        ui.group(|ui| {
            ui.label("⚡ Individual Foil Controls");
            if !self.foils_advanced_controls {
                ui.small("Hidden (enable 'advanced' above to edit per-foil DC/AC)");
                return;
            }

            let foils = FOILS.lock();
            if !foils.is_empty() {
                egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    for foil in foils.iter() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("🔋 Foil {}", foil.id));
                                
                                // Show linking status
                                if let Some(link_id) = foil.link_id {
                                    let link_text = match foil.mode {
                                        crate::body::foil::LinkMode::Parallel => format!("🔗 Parallel with Foil {}", link_id),
                                        crate::body::foil::LinkMode::Opposite => format!("🔗 Opposite to Foil {}", link_id),
                                    };
                                    ui.label(link_text);
                                }
                                
                                // Show current charging mode
                                let mode_text = match foil.charging_mode {
                                    crate::body::foil::ChargingMode::Current => "Current",
                                    crate::body::foil::ChargingMode::Overpotential => "Overpotential",
                                };
                                ui.label(format!("({})", mode_text));
                                
                                // Show electron ratio if available
                                if let Some(diagnostic) = &self.foil_electron_fraction_diagnostic {
                                    if let Some(ratio) = diagnostic.fractions.get(&foil.id) {
                                        let ratio_color = if *ratio > 1.05 {
                                            egui::Color32::LIGHT_BLUE  // Cathodic
                                        } else if *ratio < 0.95 {
                                            egui::Color32::LIGHT_RED   // Anodic
                                        } else {
                                            egui::Color32::WHITE       // Neutral
                                        };
                                        ui.colored_label(ratio_color, format!("R:{:.2}", ratio));
                                    }
                                }
                                
                                // Show PID output current for overpotential mode
                                if foil.charging_mode == crate::body::foil::ChargingMode::Overpotential {
                                    if let Some(ref controller) = foil.overpotential_controller {
                                        let current_color = if controller.last_output_current.abs() < 0.1 {
                                            egui::Color32::GRAY
                                        } else if controller.last_output_current > 0.0 {
                                            egui::Color32::GREEN
                                        } else {
                                            egui::Color32::RED
                                        };
                                        ui.colored_label(current_color, format!("I:{:.1}A", controller.last_output_current));
                                    }
                                }
                            });
                            
                            // Only show controls for "master" foils (lowest ID in linked pair)
                            let is_master_foil = foil.link_id.map_or(true, |link_id| foil.id < link_id);
                            
                            if is_master_foil {
                                // Charging Mode Selection
                                ui.horizontal(|ui| {
                                    let current_mode = foil.charging_mode;
                                    let mut new_mode = current_mode;
                                    
                                    ui.radio_value(&mut new_mode, crate::body::foil::ChargingMode::Current, "Current");
                                    ui.radio_value(&mut new_mode, crate::body::foil::ChargingMode::Overpotential, "Overpotential");
                                    
                                    if new_mode != current_mode {
                                        if new_mode == crate::body::foil::ChargingMode::Overpotential {
                                            SIM_COMMAND_SENDER
                                                .lock()
                                                .as_ref()
                                                .unwrap()
                                                .send(SimCommand::EnableOverpotentialMode {
                                                    foil_id: foil.id,
                                                target_ratio: 1.2, // Default to slightly cathodic to create initial error
                                            })
                                            .unwrap();
                                    } else {
                                        SIM_COMMAND_SENDER
                                            .lock()
                                            .as_ref()
                                            .unwrap()
                                            .send(SimCommand::DisableOverpotentialMode {
                                                foil_id: foil.id,
                                            })
                                            .unwrap();
                                    }
                                }
                            });
                            
                            // Current Mode Controls
                            if foil.charging_mode == crate::body::foil::ChargingMode::Current {
                                ui.horizontal(|ui| {
                                    ui.label("DC:");
                                    let mut dc_current = foil.dc_current;
                                    if ui.button("-").clicked() { dc_current -= 1.0; }
                                    if ui.button("+").clicked() { dc_current += 1.0; }
                                    if ui.button("0").clicked() { dc_current = 0.0; }
                                    ui.add(egui::Slider::new(&mut dc_current, -100.0..=100.0).step_by(0.01));
                                    
                                    if (dc_current - foil.dc_current).abs() > f32::EPSILON {
                                        SIM_COMMAND_SENDER
                                            .lock()
                                            .as_ref()
                                            .unwrap()
                                            .send(SimCommand::SetFoilDCCurrent {
                                                foil_id: foil.id,
                                                dc_current,
                                            })
                                            .unwrap();
                                    }
                                });
                                
                                ui.horizontal(|ui| {
                                    ui.label("AC:");
                                    let mut ac_current = foil.ac_current;
                                    if ui.button("-").clicked() { ac_current = (ac_current - 1.0).max(0.0); }
                                    if ui.button("+").clicked() { ac_current += 1.0; }
                                    if ui.button("0").clicked() { ac_current = 0.0; }
                                    ui.add(egui::Slider::new(&mut ac_current, 0.0..=100.0).step_by(0.01));
                                    
                                    if (ac_current - foil.ac_current).abs() > f32::EPSILON {
                                        SIM_COMMAND_SENDER
                                            .lock()
                                            .as_ref()
                                            .unwrap()
                                            .send(SimCommand::SetFoilACCurrent {
                                                foil_id: foil.id,
                                                ac_current,
                                            })
                                            .unwrap();
                                    }
                                });
                            }
                            
                            // Overpotential Mode Controls
                            if foil.charging_mode == crate::body::foil::ChargingMode::Overpotential {
                                if let Some(ref controller) = foil.overpotential_controller {
                                    ui.horizontal(|ui| {
                                        ui.label("Target:");
                                        let mut target_ratio = controller.target_ratio;
                                        if ui.button("Anodic").clicked() { target_ratio = 0.5; }
                                        if ui.button("Neutral").clicked() { target_ratio = 1.0; }
                                        if ui.button("Cathodic").clicked() { target_ratio = 1.5; }
                                        ui.add(egui::Slider::new(&mut target_ratio, 0.1..=2.0).step_by(0.01));
                                        
                                        if (target_ratio - controller.target_ratio).abs() > f32::EPSILON {
                                            SIM_COMMAND_SENDER
                                                .lock()
                                                .as_ref()
                                                .unwrap()
                                                .send(SimCommand::SetFoilOverpotentialTarget {
                                                    foil_id: foil.id,
                                                    target_ratio,
                                                })
                                                .unwrap();
                                        }
                                    });
                                    
                                    // PID Tuning Controls
                                    ui.horizontal(|ui| {
                                        ui.label("PID:");
                                        let mut kp = controller.kp;
                                        let mut ki = controller.ki;
                                        let mut kd = controller.kd;
                                        
                                        ui.label("P:");
                                        ui.add(egui::DragValue::new(&mut kp).speed(0.1));
                                        
                                        ui.label("I:");
                                        ui.add(egui::DragValue::new(&mut ki).speed(0.01));
                                        
                                        ui.label("D:");
                                        ui.add(egui::DragValue::new(&mut kd).speed(0.01));
                                        
                                        // Apply changes if any parameter changed
                                        if (kp - controller.kp).abs() > f32::EPSILON ||
                                           (ki - controller.ki).abs() > f32::EPSILON ||
                                           (kd - controller.kd).abs() > f32::EPSILON {
                                            SIM_COMMAND_SENDER
                                                .lock()
                                                .as_ref()
                                                .unwrap()
                                                .send(SimCommand::SetFoilPIDGains {
                                                    foil_id: foil.id,
                                                    kp,
                                                    ki,
                                                    kd,
                                                })
                                                .unwrap();
                                        }
                                    });
                                    
                                    // PID Preset Buttons
                                    ui.horizontal(|ui| {
                                        ui.label("Presets:");
                                        if ui.button("Conservative").clicked() {
                                            SIM_COMMAND_SENDER
                                                .lock()
                                                .as_ref()
                                                .unwrap()
                                                .send(SimCommand::SetFoilPIDGains {
                                                    foil_id: foil.id,
                                                    kp: 5.0,
                                                    ki: 0.05,
                                                    kd: 0.2,
                                                })
                                                .unwrap();
                                        }
                                        if ui.button("Balanced").clicked() {
                                            SIM_COMMAND_SENDER
                                                .lock()
                                                .as_ref()
                                                .unwrap()
                                                .send(SimCommand::SetFoilPIDGains {
                                                    foil_id: foil.id,
                                                    kp: 10.0,
                                                    ki: 0.1,
                                                    kd: 0.5,
                                                })
                                                .unwrap();
                                        }
                                        if ui.button("Aggressive").clicked() {
                                            SIM_COMMAND_SENDER
                                                .lock()
                                                .as_ref()
                                                .unwrap()
                                                .send(SimCommand::SetFoilPIDGains {
                                                    foil_id: foil.id,
                                                    kp: 20.0,
                                                    ki: 0.2,
                                                    kd: 1.0,
                                                })
                                                .unwrap();
                                        }
                                    });
                                    
                                    // PID Status Display
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Error: {:.3}", controller.integral_error));
                                        ui.label(format!("Prev: {:.3}", controller.previous_error));
                                        ui.label(format!("Max I: {:.1}", controller.max_current));
                                    });
                                    
                                    // PID Output Display
                                    ui.horizontal(|ui| {
                                        let current_color = if controller.last_output_current.abs() < 0.1 {
                                            egui::Color32::GRAY
                                        } else if controller.last_output_current > 0.0 {
                                            egui::Color32::LIGHT_GREEN // Positive current (cathodic)
                                        } else {
                                            egui::Color32::LIGHT_RED   // Negative current (anodic)
                                        };
                                        
                                        ui.colored_label(current_color, format!("Output: {:.2} e/step", controller.last_output_current));
                                        
                                        // Current direction indicator
                                        if controller.last_output_current > 0.1 {
                                            ui.label("⬇ (Cathodic)");
                                        } else if controller.last_output_current < -0.1 {
                                            ui.label("⬆ (Anodic)");
                                        } else {
                                            ui.label("⏸ (Neutral)");
                                        }
                                    });
                                    
                                    // PID Graph Controls
                                    ui.horizontal(|ui| {
                                        ui.label("📈 Graph:");
                                        
                                        let mut show_graph = SHOW_PID_GRAPH.load(std::sync::atomic::Ordering::Relaxed);
                                        if ui.checkbox(&mut show_graph, "Show PID Graph").clicked() {
                                            SHOW_PID_GRAPH.store(show_graph, std::sync::atomic::Ordering::Relaxed);
                                        }
                                        
                                        if show_graph {
                                            ui.label("History:");
                                            let mut history_size = *PID_GRAPH_HISTORY_SIZE.lock();
                                            ui.add(egui::Slider::new(&mut history_size, 100..=5000)
                                                .suffix(" steps"));
                                            
                                            if history_size != *PID_GRAPH_HISTORY_SIZE.lock() {
                                                *PID_GRAPH_HISTORY_SIZE.lock() = history_size;
                                                SIM_COMMAND_SENDER
                                                    .lock()
                                                    .as_ref()
                                                    .unwrap()
                                                    .send(SimCommand::SetPIDHistorySize {
                                                        foil_id: foil.id,
                                                        history_size,
                                                    })
                                                    .unwrap();
                                            }
                                        }
                                    });
                                }
                            }
                            
                            // Switch Hz (common to both modes)
                            ui.horizontal(|ui| {
                                ui.label("Switch Hz:");
                                let mut hz = foil.switch_hz;
                                ui.add(egui::DragValue::new(&mut hz).speed(0.1));
                                if (hz - foil.switch_hz).abs() > f32::EPSILON {
                                    SIM_COMMAND_SENDER
                                        .lock()
                                        .as_ref()
                                        .unwrap()
                                        .send(SimCommand::SetFoilFrequency {
                                            foil_id: foil.id,
                                            switch_hz: hz,
                                        })
                                        .unwrap();
                                }
                            });
                            } else {
                                // Show read-only status for linked foils
                                ui.label(format!("🔗 Controlled by linked foil (Master: Foil {})", 
                                    foil.link_id.unwrap().min(foil.id)));
                            }
                        });
                        ui.separator();
                    }
                });
            } else {
                ui.label("No foils available. Add foils in the Scenario tab first.");
            }
        });

        ui.separator();

        // If no foil selected (top controls), show a small hint
        if self.selected_particle_id.is_none() {
            ui.group(|ui| {
                ui.label("� How to control foil currents:");
                ui.label("• Select a foil particle in the simulation (Shift+Click)");
                ui.label("• Or create foils in the Scenario tab first");
                ui.label("• Current controls will appear here when a foil is selected");
            });
        }

        // Direct Current Controls for All Foils (Advanced)
        ui.separator();
        let foils = FOILS.lock();
        if !foils.is_empty() {
            ui.group(|ui| {
                ui.label("⚡ Quick Current Controls");
                if !self.foils_advanced_controls {
                    ui.small("Hidden (enable 'advanced' above to edit many foils at once)");
                    return;
                }
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
                                .speed(0.01),
                        );

                        // AC Current
                        let mut ac_current = foil.ac_current;
                        ui.add(
                            egui::DragValue::new(&mut ac_current)
                                .prefix("AC: ")
                                .speed(0.01)
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
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::SetFoilDCCurrent {
                                    foil_id: foil.id,
                                    dc_current,
                                })
                                .unwrap();
                        }
                        if (ac_current - foil.ac_current).abs() > f32::EPSILON {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::SetFoilACCurrent {
                                    foil_id: foil.id,
                                    ac_current,
                                })
                                .unwrap();
                        }
                        if (hz - foil.switch_hz).abs() > f32::EPSILON {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::SetFoilFrequency {
                                    foil_id: foil.id,
                                    switch_hz: hz,
                                })
                                .unwrap();
                        }
                    });
                }
            });
        }
    }
}

