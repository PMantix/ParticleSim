use super::*;

impl super::super::Renderer {
    pub fn show_charging_tab(&mut self, ui: &mut egui::Ui) {
        // Status block is first (see below). Keep a subtle title below it.

        // Always show status list at the top
        ui.group(|ui| {
            ui.heading("Foil Status");
            let foils = crate::renderer::state::FOILS.lock();
            if foils.is_empty() {
                ui.small("No foils available.");
            } else {
                egui::Grid::new("foil_status_grid_top")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Foil");
                        ui.label("Ratio");
                        ui.label("Mode");
                        ui.label("Setpoint");
                        ui.end_row();
                        for foil in foils.iter() {
                            ui.label(format!("{}", foil.id));
                            if let Some(diag) = &self.foil_electron_fraction_diagnostic {
                                if let Some(ratio) = diag.fractions.get(&foil.id) {
                                    let ratio_color = if *ratio > 1.05 {
                                        egui::Color32::LIGHT_BLUE
                                    } else if *ratio < 0.95 {
                                        egui::Color32::LIGHT_RED
                                    } else {
                                        egui::Color32::WHITE
                                    };
                                    ui.colored_label(ratio_color, format!("{:.3}", ratio));
                                } else {
                                    ui.label("N/A");
                                }
                            } else {
                                ui.label("N/A");
                            }
                            let (mode_text, setpoint_text) = match foil.charging_mode {
                                crate::body::foil::ChargingMode::Current => (
                                    "Current",
                                    format!(
                                        "DC:{:.3} AC:{:.3} Hz:{:.2}",
                                        foil.dc_current, foil.ac_current, foil.switch_hz
                                    ),
                                ),
                                crate::body::foil::ChargingMode::Overpotential => {
                                    let target = foil
                                        .overpotential_controller
                                        .as_ref()
                                        .map(|c| c.target_ratio)
                                        .unwrap_or(1.0);
                                    ("Overpotential", format!("Target:{:.3}", target))
                                }
                            };
                            ui.label(mode_text);
                            ui.label(setpoint_text);
                            ui.end_row();
                        }
                    });
            }
        });
        ui.add_space(6.0);
        ui.separator();
        ui.heading("⚡ Unified Charging");
        ui.small("Select a charging mode and configure settings below.");
        ui.separator();

        // Mode selector
        ui.horizontal(|ui| {
            ui.label("Mode:");
            let mut mode = self.charging_ui_mode;
            ui.radio_value(
                &mut mode,
                super::super::ChargingUiMode::Conventional,
                "Conventional",
            );
            ui.radio_value(
                &mut mode,
                super::super::ChargingUiMode::SwitchCharging,
                "Switch Charging",
            );
            ui.radio_value(
                &mut mode,
                super::super::ChargingUiMode::Advanced,
                "Advanced",
            );
            if mode != self.charging_ui_mode {
                match mode {
                    super::super::ChargingUiMode::Conventional => {
                        // Stop switch charging if it is not idle
                        if self.switch_ui_state.run_state != crate::switch_charging::RunState::Idle
                        {
                            self.switch_ui_state.stop();
                        }
                    }
                    super::super::ChargingUiMode::SwitchCharging => {
                        // Clear conventional groups to avoid conflicts
                        if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref()
                        {
                            let _ = tx.send(crate::renderer::state::SimCommand::ClearFoilGroups);
                        }
                    }
                    super::super::ChargingUiMode::Advanced => {
                        // Advanced operates on per-foil; ensure switch-run stops
                        if self.switch_ui_state.run_state != crate::switch_charging::RunState::Idle
                        {
                            self.switch_ui_state.stop();
                        }
                    }
                }
                self.charging_ui_mode = mode;
                // Persist selection for save
                *crate::renderer::state::PERSIST_UI_CHARGING_MODE.lock() =
                    Some(match self.charging_ui_mode {
                        super::super::ChargingUiMode::Conventional => "Conventional".to_string(),
                        super::super::ChargingUiMode::SwitchCharging => {
                            "SwitchCharging".to_string()
                        }
                        super::super::ChargingUiMode::Advanced => "Advanced".to_string(),
                    });
            }
        });

        ui.separator();

        match self.charging_ui_mode {
            super::super::ChargingUiMode::Conventional => {
                ui.label("Conventional mode groups foils into anodes and cathodes with parallel linkage within groups and opposite behavior between them.");
                ui.add_space(8.0);

                ui.group(|ui| {
                    ui.label("Preset groups:");
                    ui.small("Anodes (Group A): 1, 3, 5 | Cathodes (Group B): 2, 4");
                    ui.horizontal(|ui| {
                        if ui.button("Apply Conventional Preset").clicked() {
                            // Determine which preset IDs actually exist in the current sim
                            let foils = crate::renderer::state::FOILS.lock();
                            let have: std::collections::HashSet<u64> =
                                foils.iter().map(|f| f.id).collect();
                            let mut group_a: Vec<u64> = [1_u64, 3, 5]
                                .into_iter()
                                .filter(|id| have.contains(id))
                                .collect();
                            let mut group_b: Vec<u64> = [2_u64, 4]
                                .into_iter()
                                .filter(|id| have.contains(id))
                                .collect();
                            group_a.sort_unstable();
                            group_b.sort_unstable();
                            if let Some(tx) =
                                crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref()
                            {
                                let _ =
                                    tx.send(crate::renderer::state::SimCommand::SetFoilGroups {
                                        group_a,
                                        group_b,
                                    });
                            }
                        }
                        if ui.button("Clear Groups").clicked() {
                            if let Some(tx) =
                                crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref()
                            {
                                let _ =
                                    tx.send(crate::renderer::state::SimCommand::ClearFoilGroups);
                            }
                        }
                    });
                });

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.label("Grouped controls:");
                    let mut mode_over = self.conventional_is_overpotential;
                    ui.horizontal(|ui| {
                        ui.label("Control mode:");
                        ui.radio_value(&mut mode_over, false, "Current");
                        ui.radio_value(&mut mode_over, true, "Overpotential");
                    });
                    self.conventional_is_overpotential = mode_over;
                    *crate::renderer::state::PERSIST_UI_CONV_IS_OVER.lock() = Some(mode_over);

                    if mode_over {
                        let mut target = self.conventional_target_ratio;
                        ui.horizontal(|ui| {
                            ui.label("Target ratio (A side):");
                            ui.add(egui::DragValue::new(&mut target).speed(0.01).clamp_range(0.0..=2.0));
                            if ui.button("Apply to groups").clicked() {
                                if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                    let _ = tx.send(crate::renderer::state::SimCommand::ConventionalSetOverpotential { target_ratio: target });
                                }
                            }
                        });
                        self.conventional_target_ratio = target;
                        *crate::renderer::state::PERSIST_UI_CONV_TARGET.lock() = Some(target);
                        ui.small("Group B receives complementary target (2 - target)");
                    } else {
                        let mut current = self.conventional_current_setpoint;
                        ui.horizontal(|ui| {
                            ui.label("DC current (A side):");
                            ui.add(egui::DragValue::new(&mut current).speed(0.001).clamp_range(-10_000.0..=10_000.0));
                            if ui.button("Apply to groups").clicked() {
                                if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                    let _ = tx.send(crate::renderer::state::SimCommand::ConventionalSetCurrent { current });
                                }
                            }
                        });
                        self.conventional_current_setpoint = current;
                        *crate::renderer::state::PERSIST_UI_CONV_CURRENT.lock() = Some(current);
                        ui.small("Group B receives opposite current (-current)");
                    }
                });
            }
            super::super::ChargingUiMode::SwitchCharging => {
                ui.label("Switch Charging mode uses step-based role assignments (Anode/ Cathode A/B) and run control.");
                ui.add_space(4.0);
                // Embed the existing switch charging UI right here for minimal duplication
                crate::switch_charging::ui_switch_charging(ui, &mut self.switch_ui_state);
            }
            super::super::ChargingUiMode::Advanced => {
                ui.label("Advanced mode: per-foil controls.");
                ui.add_space(6.0);
                ui.group(|ui| {
                    ui.label("Per-foil controls:");
                    let foils = crate::renderer::state::FOILS.lock();
                    for foil in foils.iter() {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(format!("🔋 Foil {}", foil.id));
                            let mode_text = match foil.charging_mode { crate::body::foil::ChargingMode::Current => "Current", crate::body::foil::ChargingMode::Overpotential => "Overpotential" };
                            ui.small(format!("({})", mode_text));
                        });

                        // DC current
                        let mut dc = foil.dc_current;
                        ui.horizontal(|ui| {
                            ui.label("DC Current:");
                            ui.add(egui::Slider::new(&mut dc, -500.0..=500.0).step_by(0.01));
                            if ui.button("Apply").clicked() {
                                if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                    let _ = tx.send(crate::renderer::state::SimCommand::SetFoilDCCurrent { foil_id: foil.id, dc_current: dc });
                                }
                            }
                        });

                        // AC current and frequency
                        let mut ac = foil.ac_current;
                        let mut hz = foil.switch_hz;
                        ui.horizontal(|ui| {
                            ui.label("AC Amp / Hz:");
                            ui.add(egui::DragValue::new(&mut ac).speed(0.05));
                            ui.add(egui::DragValue::new(&mut hz).speed(0.1));
                            if ui.button("Apply").clicked() {
                                if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                    let _ = tx.send(crate::renderer::state::SimCommand::SetFoilACCurrent { foil_id: foil.id, ac_current: ac });
                                    let _ = tx.send(crate::renderer::state::SimCommand::SetFoilFrequency { foil_id: foil.id, switch_hz: hz });
                                }
                            }
                        });

                        // Charging mode
                        ui.horizontal(|ui| {
                            ui.label("Mode:");
                            let mut is_over = matches!(foil.charging_mode, crate::body::foil::ChargingMode::Overpotential);
                            ui.radio_value(&mut is_over, false, "Current");
                            ui.radio_value(&mut is_over, true, "Overpotential");
                            if ui.button("Apply").clicked() {
                                if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                    if is_over {
                                        let target = foil.overpotential_controller.as_ref().map(|c| c.target_ratio).unwrap_or(1.2);
                                        let _ = tx.send(crate::renderer::state::SimCommand::EnableOverpotentialMode { foil_id: foil.id, target_ratio: target });
                                    } else {
                                        let _ = tx.send(crate::renderer::state::SimCommand::DisableOverpotentialMode { foil_id: foil.id });
                                    }
                                }
                            }
                        });

                        if matches!(foil.charging_mode, crate::body::foil::ChargingMode::Overpotential) {
                            if let Some(ctrl) = &foil.overpotential_controller {
                                let mut target = ctrl.target_ratio;
                                ui.horizontal(|ui| {
                                    ui.label("Overpotential target:");
                                    ui.add(egui::DragValue::new(&mut target).speed(0.01).clamp_range(0.0..=2.0));
                                    if ui.button("Apply").clicked() {
                                        if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                            let _ = tx.send(crate::renderer::state::SimCommand::SetFoilOverpotentialTarget { foil_id: foil.id, target_ratio: target });
                                        }
                                    }
                                });
                            } else {
                                ui.small("No controller present (will be created on enable).");
                            }
                        }
                    }
                });
            }
        }

        // Status is now at the top; no duplicate at bottom
    }
}
