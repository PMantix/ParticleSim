use super::*;

impl super::super::Renderer {
    pub fn show_diagnostics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("� Diagnostics");

        // Transference Number Diagnostic
        ui.group(|ui| {
            ui.label("📊 Transient Transference Number");
            if let Some(diagnostic) = &self.transference_number_diagnostic {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Drift Direction:");
                    ui.label(format!(
                        "({:.3}, {:.3})",
                        diagnostic.drift_direction.x, diagnostic.drift_direction.y
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Li⁺ Drift Velocity:");
                    ui.label(format!("{:.6} units/s", diagnostic.lithium_drift_velocity));
                });
                ui.horizontal(|ui| {
                    ui.label("Anion Drift Velocity:");
                    ui.label(format!("{:.6} units/s", diagnostic.anion_drift_velocity));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Li⁺ Current Contribution:");
                    ui.label(format!("{:.6}", diagnostic.li_current_contribution));
                });
                ui.horizontal(|ui| {
                    ui.label("Anion Current Contribution:");
                    ui.label(format!("{:.6}", diagnostic.anion_current_contribution));
                });
                ui.horizontal(|ui| {
                    ui.label("Total Current:");
                    ui.label(format!("{:.6}", diagnostic.total_current));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Transference Number:");
                    ui.strong(format!("{:.3}", diagnostic.transference_number));
                });

                // Progress bar visualization
                ui.add(
                    egui::ProgressBar::new(diagnostic.transference_number)
                        .text(format!("t⁺ = {:.3}", diagnostic.transference_number))
                        .show_percentage(),
                );

                ui.separator();
                ui.label("ℹ️ Theory: t⁺ = 1 means only Li⁺ carries current");
                ui.label("   t⁺ = 0 means only anions carry current");
                ui.label("   Current ∝ charge × concentration × velocity");
            } else {
                ui.label("❌ No diagnostic data available.");
            }
        });

        ui.separator();

        // Foil electron fraction diagnostic
        ui.group(|ui| {
            ui.label("🔋 Foil Electron Ratio");
            
            // Update diagnostic periodically using quadtree for efficiency
            if let Some(diag) = &mut self.foil_electron_fraction_diagnostic {
                // Reconstruct quadtree from current node data for diagnostic calculation
                let mut temp_quadtree = crate::quadtree::Quadtree::new(1.0, 2.0, 1, 1024);
                temp_quadtree.nodes = self.quadtree.clone();
                
                // Only recalculate every 0.5 seconds to avoid performance issues
                let current_time = *crate::renderer::state::SIM_TIME.lock();
                diag.calculate_if_needed(&self.bodies, &self.foils, &temp_quadtree, current_time, 0.5);
                
                for foil in &self.foils {
                    if let Some(frac) = diag.fractions.get(&foil.id) {
                        ui.horizontal(|ui| {
                            ui.label(format!("Foil {}:", foil.id));
                            ui.label(format!("{:.3}", frac));
                        });
                    }
                }
            } else {
                ui.label("❌ No diagnostic data available.");
            }
        });

        ui.separator();

        // Additional diagnostic information
        ui.group(|ui| {
            ui.label("📈 Simulation Statistics");
            let lithium_count = self
                .bodies
                .iter()
                .filter(|b| b.species == crate::body::Species::LithiumIon)
                .count();
            let anion_count = self
                .bodies
                .iter()
                .filter(|b| b.species == crate::body::Species::ElectrolyteAnion)
                .count();
            let foil_count = self
                .bodies
                .iter()
                .filter(|b| b.species == crate::body::Species::FoilMetal)
                .count();

            ui.label(format!("Li⁺ particles: {}", lithium_count));
            ui.label(format!("Anion particles: {}", anion_count));
            ui.label(format!("Foil particles: {}", foil_count));
            ui.label(format!("Total particles: {}", self.bodies.len()));
        });
    }
}
