use super::*;
use quarkstrom::egui::{Color32, RichText};

impl super::super::Renderer {
    pub fn show_electrodes_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚡ Electrode Configuration");
        ui.separator();

        // Info section
        ui.group(|ui| {
            ui.label(RichText::new(" ℹ Info").strong());
            ui.separator();
            ui.label("Configure the geometry and spacing of electrodes in the simulation.");
            ui.label("Each electrode consists of a LithiumMetal rectangle with a thin FoilMetal current collector overlay.");
            ui.add_space(4.0);
            ui.label(RichText::new("⚠ Note:").color(Color32::YELLOW));
            ui.label("Changing electrode configuration will require regenerating measurement points.");
        });

        ui.separator();

        // Electrode count control
        ui.group(|ui| {
            ui.label(RichText::new(" 🔢 Electrode Count").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Number of electrodes:");
                ui.add(egui::Slider::new(&mut self.electrode_count, 1..=10).text("count"));
            });
            
            ui.label(format!("Current: {} electrodes", self.electrode_count));
        });

        ui.separator();

        // Metal dimensions
        ui.group(|ui| {
            ui.label(RichText::new(" 📏 LithiumMetal Dimensions").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(egui::DragValue::new(&mut self.electrode_metal_width)
                    .speed(1.0)
                    .clamp_range(10.0..=200.0)
                    .suffix(" Å"));
            });
            
            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.add(egui::DragValue::new(&mut self.electrode_metal_height)
                    .speed(5.0)
                    .clamp_range(50.0..=500.0)
                    .suffix(" Å"));
            });
        });

        ui.separator();

        // Foil dimensions
        ui.group(|ui| {
            ui.label(RichText::new(" 🔌 FoilMetal (Current Collector) Dimensions").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(egui::DragValue::new(&mut self.electrode_foil_width)
                    .speed(0.5)
                    .clamp_range(1.0..=50.0)
                    .suffix(" Å"));
            });
            
            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.add(egui::DragValue::new(&mut self.electrode_foil_height)
                    .speed(5.0)
                    .clamp_range(50.0..=500.0)
                    .suffix(" Å"));
            });
            
            ui.add_space(4.0);
            ui.label("💡 Tip: Foil width is typically much smaller than metal width (e.g., 7Å vs 50Å)");
        });

        ui.separator();

        // Spacing and positioning
        ui.group(|ui| {
            ui.label(RichText::new(" 📐 Spacing & Position").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Electrode spacing:");
                ui.add(egui::DragValue::new(&mut self.electrode_spacing)
                    .speed(5.0)
                    .clamp_range(50.0..=300.0)
                    .suffix(" Å"));
            });
            ui.label("(center-to-center distance between adjacent electrodes)");
            
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.label("Y-offset:");
                ui.add(egui::DragValue::new(&mut self.electrode_y_offset)
                    .speed(5.0)
                    .clamp_range(-200.0..=200.0)
                    .suffix(" Å"));
            });
            ui.label("(vertical displacement from center, 0 = centered)");
        });

        ui.separator();

        // Preview calculations
        ui.group(|ui| {
            ui.label(RichText::new(" 📊 Preview").strong());
            ui.separator();
            
            let total_width = if self.electrode_count > 1 {
                (self.electrode_count - 1) as f32 * self.electrode_spacing + self.electrode_metal_width
            } else {
                self.electrode_metal_width
            };
            
            let domain_width = *crate::renderer::state::DOMAIN_WIDTH.lock();
            let domain_height = *crate::renderer::state::DOMAIN_HEIGHT.lock();
            
            ui.label(format!("Total electrode assembly width: {:.1} Å", total_width));
            ui.label(format!("Current domain: {:.1} × {:.1} Å", domain_width, domain_height));
            
            if total_width > domain_width * 0.9 {
                ui.colored_label(Color32::RED, "⚠ Warning: Electrodes may exceed domain width!");
            }
            
            if self.electrode_metal_height > domain_height * 0.9 {
                ui.colored_label(Color32::RED, "⚠ Warning: Electrode height may exceed domain height!");
            }
        });

        ui.separator();

        // Apply button
        ui.group(|ui| {
            ui.label(RichText::new(" 🔧 Actions").strong());
            ui.separator();
            
            if ui.button(RichText::new("✓ Apply Electrode Configuration").size(16.0)).clicked() {
                self.apply_electrode_configuration();
            }
            
            ui.add_space(4.0);
            
            if ui.button("↺ Reset to Defaults").clicked() {
                self.electrode_metal_width = 50.0;
                self.electrode_metal_height = 350.0;
                self.electrode_foil_width = 7.0;
                self.electrode_foil_height = 350.0;
                self.electrode_spacing = 100.0;
                self.electrode_count = 5;
                self.electrode_y_offset = 0.0;
            }
        });

        ui.separator();

        // Warning section
        ui.group(|ui| {
            ui.label(RichText::new(" ⚠ Important Notes").strong().color(Color32::YELLOW));
            ui.separator();
            
            ui.label("• Applying a new electrode configuration will clear existing particles and foils");
            ui.label("• Measurement points that reference old foil positions will need to be regenerated");
            ui.label("• Switch to the Measurement tab after applying to regenerate measurement points");
            ui.label("• The simulation will be paused automatically when applying changes");
        });
    }

    fn apply_electrode_configuration(&mut self) {
        use crate::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
        use crate::body::Species;
        
        // Pause simulation first
        crate::renderer::state::PAUSED.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // Clear existing simulation (particles, foils, etc.)
        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
            let _ = sender.send(SimCommand::DeleteAll);
        }
        
        // Reset foil ID counter so new foils start at ID 1
        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
            let _ = sender.send(SimCommand::ResetFoilIds);
        }
        
        // Small delay to let DeleteAll and ResetFoilIds complete
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // Calculate electrode positions
        let start_x = if self.electrode_count > 1 {
            -((self.electrode_count - 1) as f32 * self.electrode_spacing) / 2.0
        } else {
            0.0
        };
        
        // Generate electrodes
        for i in 0..self.electrode_count {
            let x_center = start_x + (i as f32 * self.electrode_spacing);
            let y_center = self.electrode_y_offset;
            
            // Add LithiumMetal rectangle
            if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                let metal_body = crate::body::Body::new(
                    ultraviolet::Vec2::zero(),
                    ultraviolet::Vec2::zero(),
                    Species::LithiumMetal.mass(),
                    Species::LithiumMetal.radius(),
                    0.0,
                    Species::LithiumMetal,
                );
                
                let _ = sender.send(SimCommand::AddRectangle {
                    body: metal_body,
                    x: x_center - self.electrode_metal_width / 2.0,
                    y: y_center - self.electrode_metal_height / 2.0,
                    width: self.electrode_metal_width,
                    height: self.electrode_metal_height,
                });
            }
            
            // Add FoilMetal current collector
            if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                let _ = sender.send(SimCommand::AddFoil {
                    width: self.electrode_foil_width,
                    height: self.electrode_foil_height,
                    x: x_center - self.electrode_foil_width / 2.0,
                    y: y_center - self.electrode_foil_height / 2.0,
                    particle_radius: Species::FoilMetal.radius(),
                    current: 0.0,
                });
            }
        }
        
        // Re-add default electrolyte
        self.add_default_electrolyte();
        
        // Give a moment for foils to be created and IDs assigned
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        println!("✓ Applied electrode configuration: {} electrodes", self.electrode_count);
        println!("  Metal: {:.1}×{:.1} Å, Foil: {:.1}×{:.1} Å, Spacing: {:.1} Å",
            self.electrode_metal_width, self.electrode_metal_height,
            self.electrode_foil_width, self.electrode_foil_height,
            self.electrode_spacing);
        println!("⚠ Foil IDs reset - new foils start at ID 1");
        println!("⚠ Measurement points will need to be regenerated.");
        println!("  Go to Measurement tab and click 'Generate' to create new points.");
    }
    
    /// Add default electrolyte (1M LiPF6 in EC:DMC 1:1)
    /// Public so other tabs can reuse this functionality
    pub fn add_default_electrolyte(&self) {
        use crate::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
        use crate::body::Species;
        
        let domain_width = *crate::renderer::state::DOMAIN_WIDTH.lock();
        let domain_height = *crate::renderer::state::DOMAIN_HEIGHT.lock();
        
        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
            let molarity = 1.0;
            let total: usize = 5471;
            
            let solvent_to_salt_ratio = 15.0;
            let salt_fraction = 1.0 / (1.0 + solvent_to_salt_ratio);
            let lipf6_count = (total as f32 * salt_fraction * molarity / 1.0).round() as usize;
            let li_count = lipf6_count;
            let pf6_count = lipf6_count;
            let remaining = total.saturating_sub(li_count + pf6_count);
            
            // Calculate EC and DMC counts based on 1:1 volume ratio
            // This accounts for different densities and molar masses
            let solvent_parts = vec![
                (Species::EC, 1.0),   // 1 part by volume
                (Species::DMC, 1.0),  // 1 part by volume
            ];
            let solvent_counts = crate::species::calculate_solvent_particle_counts(&solvent_parts, remaining);
            
            let ec_count = solvent_counts.iter()
                .find(|(s, _)| *s == Species::EC)
                .map(|(_, c)| *c)
                .unwrap_or(0);
            let dmc_count = solvent_counts.iter()
                .find(|(s, _)| *s == Species::DMC)
                .map(|(_, c)| *c)
                .unwrap_or(0);
            
            // Add Li+ ions
            if li_count > 0 {
                let li_body = crate::body::Body::new(
                    ultraviolet::Vec2::zero(),
                    ultraviolet::Vec2::zero(),
                    Species::LithiumIon.mass(),
                    Species::LithiumIon.radius(),
                    1.0,
                    Species::LithiumIon,
                );
                let _ = sender.send(SimCommand::AddRandom {
                    body: li_body,
                    count: li_count,
                    domain_width,
                    domain_height,
                });
            }
            
            // Add PF6- anions
            if pf6_count > 0 {
                let pf6_body = crate::body::Body::new(
                    ultraviolet::Vec2::zero(),
                    ultraviolet::Vec2::zero(),
                    Species::ElectrolyteAnion.mass(),
                    Species::ElectrolyteAnion.radius(),
                    -1.0,
                    Species::ElectrolyteAnion,
                );
                let _ = sender.send(SimCommand::AddRandom {
                    body: pf6_body,
                    count: pf6_count,
                    domain_width,
                    domain_height,
                });
            }
            
            // Add EC solvent
            if ec_count > 0 {
                let ec_body = crate::body::Body::new(
                    ultraviolet::Vec2::zero(),
                    ultraviolet::Vec2::zero(),
                    Species::EC.mass(),
                    Species::EC.radius(),
                    0.0,
                    Species::EC,
                );
                let _ = sender.send(SimCommand::AddRandom {
                    body: ec_body,
                    count: ec_count,
                    domain_width,
                    domain_height,
                });
            }
            
            // Add DMC solvent
            if dmc_count > 0 {
                let dmc_body = crate::body::Body::new(
                    ultraviolet::Vec2::zero(),
                    ultraviolet::Vec2::zero(),
                    Species::DMC.mass(),
                    Species::DMC.radius(),
                    0.0,
                    Species::DMC,
                );
                let _ = sender.send(SimCommand::AddRandom {
                    body: dmc_body,
                    count: dmc_count,
                    domain_width,
                    domain_height,
                });
            }
            
            println!("✓ Added default electrolyte: {} Li+, {} PF6-, {} EC, {} DMC",
                li_count, pf6_count, ec_count, dmc_count);
        }
    }
}
