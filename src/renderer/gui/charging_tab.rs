use super::*;

impl super::super::Renderer {
    pub fn show_charging_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚡ Unified Charging");
        ui.small("Select the charging mode for this experiment. You can use either Conventional or Switch Charging.");
        ui.separator();

        // Mode selector
        ui.horizontal(|ui| {
            ui.label("Mode:");
            let mut mode = self.charging_ui_mode;
            ui.radio_value(&mut mode, super::super::ChargingUiMode::Conventional, "Conventional");
            ui.radio_value(&mut mode, super::super::ChargingUiMode::SwitchCharging, "Switch Charging");
            if mode != self.charging_ui_mode {
                // For now, just set it. Guardrails/transitions will come in later steps.
                self.charging_ui_mode = mode;
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
                            let have: std::collections::HashSet<u64> = foils.iter().map(|f| f.id).collect();
                            let mut group_a: Vec<u64> = [1_u64, 3, 5].into_iter().filter(|id| have.contains(id)).collect();
                            let mut group_b: Vec<u64> = [2_u64, 4].into_iter().filter(|id| have.contains(id)).collect();
                            group_a.sort_unstable();
                            group_b.sort_unstable();
                            if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                let _ = tx.send(crate::renderer::state::SimCommand::SetFoilGroups { group_a, group_b });
                            }
                        }
                        if ui.button("Clear Groups").clicked() {
                            if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                let _ = tx.send(crate::renderer::state::SimCommand::ClearFoilGroups);
                            }
                        }
                    });
                });

                ui.add_space(8.0);
                ui.label("Group controls (coming next): unified setpoint/target with mirroring.");
            }
            super::super::ChargingUiMode::SwitchCharging => {
                ui.label("Switch Charging mode uses step-based role assignments (Anode/ Cathode A/B) and run control.");
                ui.add_space(4.0);
                // Embed the existing switch charging UI right here for minimal duplication
                crate::switch_charging::ui_switch_charging(ui, &mut self.switch_ui_state);
            }
        }
    }
}
