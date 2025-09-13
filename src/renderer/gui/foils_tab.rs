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
                // Reconstruct quadtree from current node data for diagnostic calculation
                let mut temp_quadtree = crate::quadtree::Quadtree::new(1.0, 2.0, 1, 1024);
                temp_quadtree.nodes = self.quadtree.clone();
                
                // Recalculate every 0.5 fs to avoid performance issues
                let current_time = *SIM_TIME.lock();
                diag.calculate_if_needed(&self.bodies, &self.foils, &temp_quadtree, current_time, 0.5);
                
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

        // Individual Foil Controls (Always Visible)
        ui.group(|ui| {
            ui.label("⚡ Individual Foil Controls");
            
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
                                    ui.add(egui::Slider::new(&mut dc_current, -100.0..=100.0).step_by(0.1));
                                    
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
                                    ui.add(egui::Slider::new(&mut ac_current, 0.0..=100.0).step_by(0.1));
                                    
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
                                        
                                        ui.colored_label(current_color, format!("Output: {:.2}A", controller.last_output_current));
                                        
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
                            .send(SimCommand::SetFoilFrequency {
                                foil_id: foil.id,
                                switch_hz: hz,
                            })
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
                                // Enable overpotential mode with default target ratio
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
                                // Disable overpotential mode
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
                        
                        // Overpotential controls (only show when in overpotential mode)
                        if foil.charging_mode == crate::body::foil::ChargingMode::Overpotential {
                            if let Some(ref controller) = foil.overpotential_controller {
                                ui.separator();
                                ui.label("🎯 Overpotential Settings");
                                
                                let mut target_ratio = controller.target_ratio;
                                ui.horizontal(|ui| {
                                    ui.label("Target Electron Ratio:");
                                    if ui.button("Cathodic").clicked() {
                                        target_ratio = 1.5;
                                    }
                                    if ui.button("Neutral").clicked() {
                                        target_ratio = 1.0;
                                    }
                                    if ui.button("Anodic").clicked() {
                                        target_ratio = 0.5;
                                    }
                                    ui.add(egui::Slider::new(&mut target_ratio, 0.1..=2.0).step_by(0.01));
                                });
                                
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
                                
                                // Display controller status
                                ui.horizontal(|ui| {
                                    ui.label("PID Gains:");
                                    ui.label(format!("P:{:.1} I:{:.1} D:{:.1}", controller.kp, controller.ki, controller.kd));
                                });
                            }
                        }
                    });
                });

                ui.separator();

                // Foil Electron Ratio Display
                ui.group(|ui| {
                    ui.label("🔋 Foil Electron Ratio");
                    
                    // Update diagnostic periodically for real-time monitoring
                    if let Some(diag) = &mut self.foil_electron_fraction_diagnostic {
                        // Reconstruct quadtree from current node data for diagnostic calculation
                        let mut temp_quadtree = crate::quadtree::Quadtree::new(1.0, 2.0, 1, 1024);
                        temp_quadtree.nodes = self.quadtree.clone();
                        
                        // Recalculate every 0.5 fs to avoid performance issues
                        let current_time = *SIM_TIME.lock();
                        diag.calculate_if_needed(&self.bodies, &self.foils, &temp_quadtree, current_time, 0.5);
                        
                        if let Some(ratio) = diag.fractions.get(&foil.id) {
                            ui.horizontal(|ui| {
                                ui.label("Current ratio:");
                                let ratio_color = if *ratio > 1.05 {
                                    egui::Color32::LIGHT_BLUE  // Cathodic (electron-rich)
                                } else if *ratio < 0.95 {
                                    egui::Color32::LIGHT_RED   // Anodic (electron-poor)
                                } else {
                                    egui::Color32::WHITE       // Near neutral
                                };
                                ui.colored_label(ratio_color, format!("{:.3}", ratio));
                                ui.label(if *ratio > 1.05 {
                                    "(cathodic)"
                                } else if *ratio < 0.95 {
                                    "(anodic)"
                                } else {
                                    "(neutral)"
                                });
                            });
                        } else {
                            ui.label("❌ Ratio data not available");
                        }
                    } else {
                        ui.label("❌ No diagnostic available");
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

    pub fn show_pid_graph(&mut self, ctx: &egui::Context) {
        if !SHOW_PID_GRAPH.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        egui::Window::new("📈 PID Controller Graph")
            .default_size(egui::vec2(800.0, 600.0))
            .show(ctx, |ui| {
                let foils = FOILS.lock();
                
                // Find master foils in overpotential mode with history
                let master_foils: Vec<_> = foils
                    .iter()
                    .filter(|foil| {
                        matches!(foil.charging_mode, crate::body::foil::ChargingMode::Overpotential)
                            && foil.overpotential_controller.as_ref()
                                .map(|c| c.master_foil_id.is_none() && !c.history.is_empty())
                                .unwrap_or(false)
                    })
                    .collect();

                if master_foils.is_empty() {
                    ui.label("No master foils with overpotential control and history data available.");
                    ui.label("Enable overpotential mode on a foil to see PID data.");
                    return;
                }

                // Foil selector
                ui.horizontal(|ui| {
                    ui.label("Foil:");
                    egui::ComboBox::from_id_source("pid_graph_foil_selector")
                        .selected_text(format!("Foil {}", self.selected_pid_foil_id.unwrap_or(master_foils[0].id)))
                        .show_ui(ui, |ui| {
                            for foil in &master_foils {
                                ui.selectable_value(&mut self.selected_pid_foil_id, Some(foil.id), 
                                    format!("Foil {}", foil.id));
                            }
                        });
                    
                    // Auto-select first foil if none selected or selected foil no longer exists
                    if self.selected_pid_foil_id.is_none() || 
                       !master_foils.iter().any(|f| Some(f.id) == self.selected_pid_foil_id) {
                        self.selected_pid_foil_id = Some(master_foils[0].id);
                    }
                });

                // Find the selected foil
                if let Some(selected_id) = self.selected_pid_foil_id {
                    if let Some(foil) = master_foils.iter().find(|f| f.id == selected_id) {
                        if let Some(controller) = &foil.overpotential_controller {
                            self.draw_pid_plot(ui, &controller.history);
                        }
                    }
                }
            });
    }

    fn draw_pid_plot(&self, ui: &mut egui::Ui, history: &std::collections::VecDeque<crate::body::foil::PidHistoryPoint>) {
        if history.is_empty() {
            ui.label("No history data available yet. Enable overpotential mode and let the simulation run to collect data.");
            return;
        }

        ui.label(format!("PID History: {} data points", history.len()));

        // Plot dimensions
        let plot_height = 200.0;
        let plot_width = 600.0;
        let margin = 40.0;

        // Find data ranges
        let min_step = history.iter().map(|p| p.step).min().unwrap_or(0) as f64;
        let max_step = history.iter().map(|p| p.step).max().unwrap_or(1) as f64;
        
        // Main tracking plot (setpoint vs actual)
        ui.label("📈 Setpoint vs Actual");
        let setpoint_min = history.iter().map(|p| p.setpoint).fold(f32::INFINITY, f32::min) as f64;
        let setpoint_max = history.iter().map(|p| p.setpoint).fold(f32::NEG_INFINITY, f32::max) as f64;
        let actual_min = history.iter().map(|p| p.actual).fold(f32::INFINITY, f32::min) as f64;
        let actual_max = history.iter().map(|p| p.actual).fold(f32::NEG_INFINITY, f32::max) as f64;
        
        let y_min = (setpoint_min.min(actual_min) - 0.1).max(0.0);
        let y_max = setpoint_max.max(actual_max) + 0.1;
        
        self.draw_simple_plot(ui, history, plot_width, plot_height, margin, 
            min_step, max_step, y_min, y_max,
            &[("Setpoint", egui::Color32::BLACK, |p| p.setpoint as f64),
              ("Actual", egui::Color32::BLUE, |p| p.actual as f64)]);
        
        ui.separator();

        // Error and output plot
        ui.label("📉 Error & PID Output");
        let error_min = history.iter().map(|p| p.error).fold(f32::INFINITY, f32::min) as f64;
        let error_max = history.iter().map(|p| p.error).fold(f32::NEG_INFINITY, f32::max) as f64;
        let output_min = history.iter().map(|p| p.output).fold(f32::INFINITY, f32::min) as f64;
        let output_max = history.iter().map(|p| p.output).fold(f32::NEG_INFINITY, f32::max) as f64;
        
        let y_min2 = error_min.min(output_min) - 0.1;
        let y_max2 = error_max.max(output_max) + 0.1;
        
        self.draw_simple_plot(ui, history, plot_width, plot_height, margin,
            min_step, max_step, y_min2, y_max2,
            &[("Error", egui::Color32::RED, |p| p.error as f64),
              ("Output", egui::Color32::DARK_GREEN, |p| p.output as f64)]);
              
        ui.separator();

        // PID terms plot
        ui.label("🔧 PID Terms");
        let p_min = history.iter().map(|p| p.p_term).fold(f32::INFINITY, f32::min) as f64;
        let p_max = history.iter().map(|p| p.p_term).fold(f32::NEG_INFINITY, f32::max) as f64;
        let i_min = history.iter().map(|p| p.i_term).fold(f32::INFINITY, f32::min) as f64;
        let i_max = history.iter().map(|p| p.i_term).fold(f32::NEG_INFINITY, f32::max) as f64;
        let d_min = history.iter().map(|p| p.d_term).fold(f32::INFINITY, f32::min) as f64;
        let d_max = history.iter().map(|p| p.d_term).fold(f32::NEG_INFINITY, f32::max) as f64;
        
        let y_min3 = p_min.min(i_min).min(d_min) - 0.1;
        let y_max3 = p_max.max(i_max).max(d_max) + 0.1;
        
        self.draw_simple_plot(ui, history, plot_width, plot_height, margin,
            min_step, max_step, y_min3, y_max3,
            &[("P Term", egui::Color32::from_rgb(255, 100, 100), |p| p.p_term as f64),
              ("I Term", egui::Color32::from_rgb(100, 100, 255), |p| p.i_term as f64),
              ("D Term", egui::Color32::from_rgb(100, 255, 100), |p| p.d_term as f64)]);
        
        // Display current statistics at the bottom
        if let Some(latest) = history.back() {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("📊 Latest Values:");
                ui.colored_label(egui::Color32::BLUE, format!("Actual: {:.3}", latest.actual));
                ui.colored_label(egui::Color32::BLACK, format!("Setpoint: {:.3}", latest.setpoint));
                ui.colored_label(
                    if latest.error.abs() < 0.01 { egui::Color32::GREEN } else { egui::Color32::RED },
                    format!("Error: {:.3}", latest.error)
                );
                ui.colored_label(egui::Color32::DARK_GREEN, format!("Output: {:.2}A", latest.output));
            });
        }
    }

    fn draw_simple_plot(&self, ui: &mut egui::Ui, 
                       history: &std::collections::VecDeque<crate::body::foil::PidHistoryPoint>,
                       width: f32, height: f32, margin: f32,
                       x_min: f64, x_max: f64, y_min: f64, y_max: f64,
                       series: &[(&str, egui::Color32, fn(&crate::body::foil::PidHistoryPoint) -> f64)]) {
        
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            
            // Draw background
            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(250, 250, 250));
            painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
            
            // Plot area
            let plot_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(margin, margin/2.0),
                egui::vec2(width - margin * 1.5, height - margin)
            );
            
            // Draw grid lines
            for i in 0..5 {
                let y = plot_rect.min.y + (i as f32) * plot_rect.height() / 4.0;
                painter.line_segment(
                    [egui::pos2(plot_rect.min.x, y), egui::pos2(plot_rect.max.x, y)],
                    egui::Stroke::new(0.5, egui::Color32::LIGHT_GRAY)
                );
            }
            
            // Transform point to screen coordinates
            let transform_point = |step: u64, value: f64| -> egui::Pos2 {
                let x_norm = if x_max > x_min { (step as f64 - x_min) / (x_max - x_min) } else { 0.0 };
                let y_norm = if y_max > y_min { (value - y_min) / (y_max - y_min) } else { 0.5 };
                egui::pos2(
                    plot_rect.min.x + x_norm as f32 * plot_rect.width(),
                    plot_rect.max.y - y_norm as f32 * plot_rect.height()
                )
            };
            
            // Draw each series
            for (_name, color, value_fn) in series {
                let points: Vec<egui::Pos2> = history.iter()
                    .map(|point| transform_point(point.step, value_fn(point)))
                    .collect();
                
                if points.len() > 1 {
                    for i in 0..points.len()-1 {
                        painter.line_segment(
                            [points[i], points[i+1]],
                            egui::Stroke::new(2.0, *color)
                        );
                    }
                }
            }
            
            // Draw y-axis labels
            for i in 0..5 {
                let y = plot_rect.min.y + (i as f32) * plot_rect.height() / 4.0;
                let value = y_max - (i as f64) * (y_max - y_min) / 4.0;
                painter.text(
                    egui::pos2(rect.min.x + 5.0, y - 8.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{:.2}", value),
                    egui::FontId::monospace(10.0),
                    egui::Color32::BLACK,
                );
            }
            
            // Draw legend
            let mut legend_y = rect.min.y + 10.0;
            for (name, color, _) in series {
                painter.line_segment(
                    [egui::pos2(rect.max.x - 80.0, legend_y), egui::pos2(rect.max.x - 60.0, legend_y)],
                    egui::Stroke::new(2.0, *color)
                );
                painter.text(
                    egui::pos2(rect.max.x - 55.0, legend_y),
                    egui::Align2::LEFT_CENTER,
                    *name,
                    egui::FontId::default(),
                    egui::Color32::BLACK,
                );
                legend_y += 15.0;
            }
        }
    }
}

