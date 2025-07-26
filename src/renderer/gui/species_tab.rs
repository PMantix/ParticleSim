use super::*;

impl super::super::Renderer {
    pub fn show_species_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔬 Species Configuration");

        ui.label("Configure all properties for each species:");

        // Species selection dropdown
        egui::ComboBox::from_label("Edit Species")
            .selected_text(format!("{:?}", self.selected_lj_species))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.selected_lj_species,
                    Species::LithiumMetal,
                    "Lithium Metal",
                );
                ui.selectable_value(
                    &mut self.selected_lj_species,
                    Species::LithiumIon,
                    "Lithium Ion",
                );
                ui.selectable_value(
                    &mut self.selected_lj_species,
                    Species::FoilMetal,
                    "Foil Metal",
                );
                ui.selectable_value(
                    &mut self.selected_lj_species,
                    Species::ElectrolyteAnion,
                    "Electrolyte Anion",
                );
                ui.selectable_value(
                    &mut self.selected_lj_species,
                    Species::EC,
                    "EC",
                );
                ui.selectable_value(
                    &mut self.selected_lj_species,
                    Species::DMC,
                    "DMC",
                );
            });

        // Get current properties for selected species
        let mut current_props = crate::species::get_species_props(self.selected_lj_species);
        let mut changed = false;

        ui.separator();

        // Basic Properties
        ui.group(|ui| {
            ui.label("📏 Basic Properties");

            // Mass control
            if ui
                .add(
                    egui::Slider::new(&mut current_props.mass, 0.1..=1e8)
                        .text("Mass")
                        .logarithmic(true)
                        .step_by(0.1),
                )
                .changed()
            {
                changed = true;
            }

            // Radius control
            if ui
                .add(
                    egui::Slider::new(&mut current_props.radius, 0.1..=10.0)
                        .text("Radius")
                        .step_by(0.01),
                )
                .changed()
            {
                changed = true;
            }

            // Damping control
            if ui
                .add(
                    egui::Slider::new(&mut current_props.damping, 0.1..=1.0)
                        .text("Damping")
                        .step_by(0.001),
                )
                .changed()
            {
                changed = true;
            }

            // Color picker
            let mut c = egui::Color32::from_rgba_unmultiplied(
                current_props.color[0],
                current_props.color[1],
                current_props.color[2],
                current_props.color[3],
            );
            if ui.color_edit_button_srgba(&mut c).changed() {
                current_props.color = c.to_array();
                changed = true;
            }
        });

        ui.separator();

        // Lennard-Jones Parameters
        ui.group(|ui| {
            ui.label("⚛️ Lennard-Jones Parameters");

            // LJ enabled checkbox
            if ui
                .checkbox(&mut current_props.lj_enabled, "Enable LJ interactions")
                .changed()
            {
                changed = true;
            }

            // Only show LJ parameter controls if LJ is enabled for this species
            if current_props.lj_enabled {
                if ui
                    .add(
                        egui::Slider::new(&mut current_props.lj_epsilon, 0.0..=15000.0)
                            .text("LJ Epsilon (depth)")
                            .step_by(1.0),
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut current_props.lj_sigma, 0.1..=5.0)
                            .text("LJ Sigma (diameter)")
                            .step_by(0.01),
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut current_props.lj_cutoff, 0.5..=10.0)
                            .text("LJ Cutoff multiplier")
                            .step_by(0.01),
                    )
                    .changed()
                {
                    changed = true;
                }
            } else {
                ui.colored_label(
                    egui::Color32::GRAY,
                    "LJ interactions disabled for this species",
                );
            }
        });

        ui.separator();

        // Short-range Repulsion
        ui.group(|ui| {
            ui.label("🛑 Repulsive Potential");
            if ui
                .checkbox(&mut current_props.enable_repulsion, "Enable repulsive potential")
                .changed()
            {
                changed = true;
            }
            if current_props.enable_repulsion {
                if ui
                    .add(
                        egui::Slider::new(&mut current_props.repulsion_strength, 0.0..=20.0)
                            .text("Strength k")
                            .step_by(0.1),
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut current_props.repulsion_cutoff, 0.1..=5.0)
                            .text("Cutoff r0")
                            .step_by(0.01),
                    )
                    .changed()
                {
                    changed = true;
                }
            }
        });

        ui.separator();

        // Electron Polarization
        ui.group(|ui| {
            ui.label("🌀 Electron Polarization");
            if ui
                .add(
                    egui::Slider::new(&mut current_props.polar_offset, 0.0..=1.5)
                        .text("Drift Radius Factor"),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut current_props.polar_charge, 0.0..=1.0)
                        .text("Effective Charge"),
                )
                .changed()
            {
                changed = true;
            }
        });

        // Update species properties if changed
        if changed {
            crate::species::update_species_props(self.selected_lj_species, current_props);
        }

        ui.separator();

        // Reset to defaults button
        if ui.button("Reset to Default Properties").clicked() {
            if let Some(default_props) =
                crate::species::SPECIES_PROPERTIES.get(&self.selected_lj_species)
            {
                crate::species::update_species_props(self.selected_lj_species, *default_props);
            }
        }

        // Show current effective values
        ui.group(|ui| {
            ui.label("📊 Current Effective Values");
            ui.horizontal(|ui| {
                ui.label(format!("Mass: {:.2}", current_props.mass));
                ui.label(format!("Radius: {:.2}", current_props.radius));
                ui.label(format!("Damping: {:.3}", current_props.damping));
            });
            ui.horizontal(|ui| {
                ui.label(format!("Polar offset: {:.2}", current_props.polar_offset));
                ui.label(format!("Polar charge: {:.2}", current_props.polar_charge));
            });
            if current_props.enable_repulsion {
                ui.horizontal(|ui| {
                    ui.label(format!("Repulsion k: {:.2}", current_props.repulsion_strength));
                    ui.label(format!("Repulsion r0: {:.2}", current_props.repulsion_cutoff));
                });
            }
            if current_props.lj_enabled {
                ui.horizontal(|ui| {
                    ui.label(format!("LJ ε: {:.1}", current_props.lj_epsilon));
                    ui.label(format!("LJ σ: {:.2}", current_props.lj_sigma));
                    ui.label(format!("LJ cutoff: {:.2}", current_props.lj_cutoff));
                });
                ui.label(format!(
                    "Effective LJ range: {:.2}",
                    current_props.lj_cutoff * current_props.lj_sigma
                ));
            }
        });
    }
}
