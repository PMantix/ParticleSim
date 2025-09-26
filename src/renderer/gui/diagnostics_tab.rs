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
                    ui.label(format!("{:.6} Å/fs", diagnostic.lithium_drift_velocity));
                });
                ui.horizontal(|ui| {
                    ui.label("PF6⁻ Drift Velocity:");
                    ui.label(format!("{:.6} Å/fs", diagnostic.anion_drift_velocity));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Li⁺ Current Contribution:");
                    ui.label(format!("{:.6} e/fs", diagnostic.li_current_contribution));
                });
                ui.horizontal(|ui| {
                    ui.label("PF6⁻ Current Contribution:");
                    ui.label(format!("{:.6} e/fs", diagnostic.anion_current_contribution));
                });
                ui.horizontal(|ui| {
                    ui.label("Total Current:");
                    ui.label(format!("{:.6} e/fs", diagnostic.total_current));
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
                ui.label("   t⁺ = 0 means only PF6⁻ anions carry current");
                ui.label("   Current ∝ charge × concentration × velocity");
            } else {
                ui.label("❌ No diagnostic data available.");
            }
        });

        ui.separator();

        // Foil electron fraction diagnostic
        ui.group(|ui| {
            ui.label("🔋 Foil Electron Ratio");
            
            if let Some(diag) = &mut self.foil_electron_fraction_diagnostic {
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

        // Solvation diagnostic
        ui.group(|ui| {
            ui.label("🧪 Solvation State");
            if let Some(diag) = &self.solvation_diagnostic {
                ui.horizontal(|ui| {
                    ui.label("Temperature:");
                    ui.label(format!("{:.3}", diag.temperature));
                });
                ui.horizontal(|ui| {
                    ui.label("Avg Li coordination:");
                    ui.label(format!("{:.2}", diag.avg_li_coordination));
                });
                ui.horizontal(|ui| {
                    ui.label("Avg anion coordination:");
                    ui.label(format!("{:.2}", diag.avg_anion_coordination));
                });
                ui.separator();
                ui.label("Solvation distribution:");
                ui.label(format!(
                    "CIP: {:.3}\nSIP: {:.3}\nS2IP: {:.3}\nFD: {:.3}",
                    diag.cip_fraction, diag.sip_fraction, diag.s2ip_fraction, diag.fd_fraction
                ));
                
                ui.separator();
                ui.label("🔍 Visual Overlays:");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_cip_ions, "Show CIP");
                    ui.checkbox(&mut self.show_sip_ions, "Show SIP");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_s2ip_ions, "Show S2IP");
                    ui.checkbox(&mut self.show_fd_ions, "Show FD");
                });
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
                .filter(|b| b.species == crate::body::Species::LithiumCation)
                .count();
            let anion_count = self
                .bodies
                .iter()
                .filter(|b| b.species == crate::body::Species::Pf6Anion)
                .count();
            let foil_count = self
                .bodies
                .iter()
                .filter(|b| b.species == crate::body::Species::FoilMetal)
                .count();

            ui.label(format!("Li⁺ particles: {}", lithium_count));
            ui.label(format!("PF6 anion particles: {}", anion_count));
            ui.label(format!("Foil particles: {}", foil_count));
            ui.label(format!("Total particles: {}", self.bodies.len()));
            let temp_global = crate::simulation::compute_temperature(&self.bodies);
            let temp_liquid = crate::simulation::utils::compute_liquid_temperature(&self.bodies);
            ui.label(format!("Global T: {:.3} K", temp_global));
            
            // Handle liquid temperature display
            if temp_liquid.is_nan() {
                ui.label("Liquid T: Initializing...");
            } else {
                ui.label(format!("Liquid T: {:.3} K", temp_liquid));
            }
            
            // Show thermostat scale with context
            let scale = *crate::renderer::state::LAST_THERMOSTAT_SCALE.lock();
            if temp_liquid.is_nan() {
                ui.label("Thermostat: Waiting for particles...");
            } else if (scale - 1.0).abs() < 0.001 {
                ui.label("Thermostat: At target temperature");
            } else {
                ui.label(format!("Thermostat scale: {:.4}", scale));
            }
        });

        ui.separator();

        // 2D Domain Density Calculation
        ui.group(|ui| {
            ui.label("🗺️ 2D Domain Density");
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.sim_config.show_2d_domain_density, "Show Density Heatmap").changed() {
                    // Update global config when toggle changes
                    let mut global_config = crate::config::LJ_CONFIG.lock();
                    global_config.show_2d_domain_density = self.sim_config.show_2d_domain_density;
                }
            });
            
            if self.sim_config.show_2d_domain_density {
                ui.separator();
                ui.label("📊 Species Selection:");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.density_calc_lithium_cation, "Li⁺");
                    ui.checkbox(&mut self.density_calc_lithium_metal, "Li⁰");
                    ui.checkbox(&mut self.density_calc_foil_metal, "Foil");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.density_calc_pf6_anion, "PF6⁻");
                    ui.checkbox(&mut self.density_calc_ec, "EC");
                    ui.checkbox(&mut self.density_calc_dmc, "DMC");
                });
                
                ui.separator();
                
                // Calculate and display numerical density
                let (avg_density, particle_count, effective_area) = self.calculate_numerical_density();
                ui.label("📈 Density Metrics:");
                ui.horizontal(|ui| {
                    ui.label("Selected Particles:");
                    ui.label(format!("{}", particle_count));
                });
                ui.horizontal(|ui| {
                    ui.label("Effective Area:");
                    ui.label(format!("{:.1} Ų", effective_area));
                });
                ui.horizontal(|ui| {
                    ui.label("Number Density:");
                    ui.strong(format!("{:.6} particles/Ų", avg_density));
                });
                
                if avg_density > 0.0 {
                    ui.horizontal(|ui| {
                        ui.label("Area per Particle:");
                        ui.label(format!("{:.1} Ų/particle", 1.0 / avg_density));
                    });
                }
                
                if particle_count > 0 {
                    ui.separator();
                    ui.label("ℹ️ Number Density = Selected Particles / Effective Area");
                    ui.label("   Area calculation adapts to particle distribution");
                    ui.label("   (bounding box for spread out, buffered area for dense)");
                } else {
                    ui.separator();
                    ui.label("⚠️ No particles selected - choose species above");
                }
            }
        });
    }
}
