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
                ui.add_space(4.0);
                // Placeholder: We will embed preset + compact controls in the next steps.
                ui.monospace("Coming next: Apply Conventional Preset (Anodes: 1,3,5 | Cathodes: 2,4) and group-level controls.");
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
