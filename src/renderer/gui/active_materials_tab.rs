// GUI tab for Active Materials configuration
// Allows selection of electrode materials (Graphite, LFP, NMC, etc.) and cell setup

use super::*;
use crate::electrode::{
    ActiveMaterialRegion, MaterialType, ElectrodeRole,
    ANODE_MATERIALS, CATHODE_MATERIALS,
};
use quarkstrom::egui::{Color32, RichText};

impl super::super::Renderer {
    pub fn show_active_materials_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔋 Active Materials");
        ui.separator();

        // Info section
        ui.group(|ui| {
            ui.label(RichText::new("ℹ️ About Active Materials").strong());
            ui.separator();
            ui.label("Configure electrode active materials for intercalation-based cells.");
            ui.label("Unlike lithium metal (plating/stripping), these materials store Li⁺ via intercalation.");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Current mode:").strong());
                if self.use_intercalation_electrodes {
                    ui.colored_label(Color32::GREEN, "Intercalation Electrodes");
                } else {
                    ui.colored_label(Color32::YELLOW, "Lithium Metal Symmetric");
                }
            });
        });

        ui.separator();

        // Mode toggle
        ui.group(|ui| {
            ui.label(RichText::new("⚙️ Electrode Mode").strong());
            ui.separator();
            
            ui.checkbox(
                &mut self.use_intercalation_electrodes,
                "Use intercalation electrode materials"
            );
            
            if self.use_intercalation_electrodes {
                ui.add_space(4.0);
                ui.colored_label(Color32::LIGHT_BLUE, 
                    "📋 Li⁺ will be absorbed into electrodes, tracking state-of-charge");
            } else {
                ui.add_space(4.0);
                ui.label("📋 Using symmetric lithium metal electrodes (plating/stripping)");
            }
        });

        if !self.use_intercalation_electrodes {
            ui.separator();
            ui.colored_label(Color32::GRAY, 
                "Enable 'Use intercalation electrode materials' to configure active materials.");
            return;
        }

        ui.separator();

        // Anode material selection
        ui.group(|ui| {
            ui.label(RichText::new("⚡ Anode Material (Negative)").strong());
            ui.separator();
            
            egui::ComboBox::from_id_source("anode_material_combo")
                .selected_text(self.selected_anode_material.display_name())
                .show_ui(ui, |ui| {
                    for &material in ANODE_MATERIALS {
                        ui.selectable_value(
                            &mut self.selected_anode_material,
                            material,
                            material.display_name(),
                        );
                    }
                });
            
            ui.add_space(4.0);
            self.show_material_info(ui, self.selected_anode_material);
        });

        ui.separator();

        // Cathode material selection
        ui.group(|ui| {
            ui.label(RichText::new("⚡ Cathode Material (Positive)").strong());
            ui.separator();
            
            egui::ComboBox::from_id_source("cathode_material_combo")
                .selected_text(self.selected_cathode_material.display_name())
                .show_ui(ui, |ui| {
                    for &material in CATHODE_MATERIALS {
                        ui.selectable_value(
                            &mut self.selected_cathode_material,
                            material,
                            material.display_name(),
                        );
                    }
                });
            
            ui.add_space(4.0);
            self.show_material_info(ui, self.selected_cathode_material);
        });

        ui.separator();

        // Cell configuration presets
        ui.group(|ui| {
            ui.label(RichText::new("🏭 Cell Configuration Presets").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.button("Li || Graphite (Half-cell)").clicked() {
                    self.selected_anode_material = MaterialType::Graphite;
                    self.selected_cathode_material = MaterialType::LFP; // Counter electrode
                    self.use_intercalation_electrodes = true;
                    self.cell_preset = CellPreset::LiGraphiteHalfCell;
                }
                if ui.button("Graphite || LFP").clicked() {
                    self.selected_anode_material = MaterialType::Graphite;
                    self.selected_cathode_material = MaterialType::LFP;
                    self.use_intercalation_electrodes = true;
                    self.cell_preset = CellPreset::GraphiteLFP;
                }
            });
            
            ui.horizontal(|ui| {
                if ui.button("Graphite || NMC").clicked() {
                    self.selected_anode_material = MaterialType::Graphite;
                    self.selected_cathode_material = MaterialType::NMC;
                    self.use_intercalation_electrodes = true;
                    self.cell_preset = CellPreset::GraphiteNMC;
                }
                if ui.button("LTO || LFP").clicked() {
                    self.selected_anode_material = MaterialType::LTO;
                    self.selected_cathode_material = MaterialType::LFP;
                    self.use_intercalation_electrodes = true;
                    self.cell_preset = CellPreset::LTOLFP;
                }
            });
        });

        ui.separator();

        // Multi-layer cell setup
        ui.group(|ui| {
            ui.label(RichText::new("📐 Multi-Layer Cell Setup").strong());
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Number of electrode pairs:");
                ui.add(egui::Slider::new(&mut self.intercalation_layer_count, 1..=5));
            });
            
            ui.horizontal(|ui| {
                ui.label("Initial anode SOC:");
                ui.add(egui::Slider::new(&mut self.initial_anode_soc, 0.0..=1.0)
                    .text(""));
                ui.label(format!("{:.0}%", self.initial_anode_soc * 100.0));
            });
            
            ui.horizontal(|ui| {
                ui.label("Initial cathode SOC:");
                ui.add(egui::Slider::new(&mut self.initial_cathode_soc, 0.0..=1.0)
                    .text(""));
                ui.label(format!("{:.0}%", self.initial_cathode_soc * 100.0));
            });
            
            ui.add_space(8.0);
            
            // Preview
            let anode_color = self.selected_anode_material.color_at_soc(self.initial_anode_soc);
            let cathode_color = self.selected_cathode_material.color_at_soc(self.initial_cathode_soc);
            
            ui.horizontal(|ui| {
                ui.label("Preview colors:");
                let anode_c32 = Color32::from_rgba_unmultiplied(
                    anode_color[0], anode_color[1], anode_color[2], anode_color[3]
                );
                let cathode_c32 = Color32::from_rgba_unmultiplied(
                    cathode_color[0], cathode_color[1], cathode_color[2], cathode_color[3]
                );
                
                ui.colored_label(anode_c32, "■ Anode");
                ui.label("|");
                ui.colored_label(cathode_c32, "■ Cathode");
            });
            
            ui.add_space(8.0);
            
            if ui.button(RichText::new("🚀 Generate Cell").size(16.0)).clicked() {
                self.generate_intercalation_cell();
            }
        });

        ui.separator();

        // Active regions status
        if !self.active_material_regions.is_empty() {
            ui.group(|ui| {
                ui.label(RichText::new("📊 Active Regions Status").strong());
                ui.separator();
                
                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    for region in &self.active_material_regions {
                        ui.horizontal(|ui| {
                            let color = region.current_color();
                            let c32 = Color32::from_rgba_unmultiplied(
                                color[0], color[1], color[2], color[3]
                            );
                            
                            ui.colored_label(c32, "■");
                            ui.label(format!(
                                "{} (ID {}): SOC {:.1}% | {}/{} Li",
                                region.material.display_name(),
                                region.id,
                                region.state_of_charge * 100.0,
                                region.lithium_count,
                                region.lithium_capacity
                            ));
                        });
                        
                        // Mini progress bar for SOC
                        let progress = region.state_of_charge;
                        ui.add(egui::ProgressBar::new(progress)
                            .text(format!("{:.0}%", progress * 100.0)));
                        ui.add_space(4.0);
                    }
                });
            });
        }
    }
    
    fn show_material_info(&self, ui: &mut egui::Ui, material: MaterialType) {
        ui.horizontal(|ui| {
            ui.label("Formula:");
            ui.label(RichText::new(material.formula()).monospace());
            ui.label("|");
            ui.label("Role:");
            match material.role() {
                ElectrodeRole::Anode => ui.colored_label(Color32::LIGHT_RED, "Anode"),
                ElectrodeRole::Cathode => ui.colored_label(Color32::LIGHT_BLUE, "Cathode"),
            };
        });
        
        // OCV at 50% SOC
        let ocv = material.open_circuit_voltage(0.5);
        ui.horizontal(|ui| {
            ui.label("OCV at 50% SOC:");
            ui.label(format!("{:.2} V vs Li/Li⁺", ocv));
        });
        
        // Show color gradient
        ui.horizontal(|ui| {
            ui.label("Color gradient:");
            for soc in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let color = material.color_at_soc(soc);
                let c32 = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                ui.colored_label(c32, "■");
            }
            ui.label("(0% → 100% SOC)");
        });
    }
    
    fn generate_intercalation_cell(&mut self) {
        use crate::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
        use crate::body::Species;
        
        println!("🔋 Generating intercalation cell:");
        println!("  Anode: {} at {:.0}% SOC", 
            self.selected_anode_material.display_name(),
            self.initial_anode_soc * 100.0);
        println!("  Cathode: {} at {:.0}% SOC",
            self.selected_cathode_material.display_name(),
            self.initial_cathode_soc * 100.0);
        println!("  Layers: {}", self.intercalation_layer_count);
        
        // Pause simulation
        crate::renderer::state::PAUSED.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // Clear existing
        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
            let _ = sender.send(SimCommand::DeleteAll);
        }
        
        // Reset foil ID counter so new foils start at ID 1
        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
            let _ = sender.send(SimCommand::ResetFoilIds);
        }
        
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // Calculate positions for alternating anode/cathode layers
        let spacing = self.electrode_spacing;
        let total_electrodes = self.intercalation_layer_count * 2;
        let start_x = -((total_electrodes - 1) as f32 * spacing) / 2.0;
        
        // Track foil IDs for group assignment (anodes vs cathodes)
        let mut anode_foil_ids: Vec<u64> = Vec::new();
        let mut cathode_foil_ids: Vec<u64> = Vec::new();
        
        // Clear and regenerate active regions
        self.active_material_regions.clear();
        
        for i in 0..total_electrodes {
            let x_center = start_x + (i as f32 * spacing);
            let y_center = self.electrode_y_offset;
            
            let is_anode = (i % 2) == 0;
            let material = if is_anode {
                self.selected_anode_material
            } else {
                self.selected_cathode_material
            };
            let initial_soc = if is_anode {
                self.initial_anode_soc
            } else {
                self.initial_cathode_soc
            };
            
            // Convert MaterialType to Species and create electrode particles
            let electrode_species = material.to_species();
            if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                let electrode_body = crate::body::Body::new(
                    ultraviolet::Vec2::zero(),
                    ultraviolet::Vec2::zero(),
                    electrode_species.mass(),
                    electrode_species.radius(),
                    0.0,
                    electrode_species,
                );
                
                let _ = sender.send(SimCommand::AddRectangle {
                    body: electrode_body,
                    x: x_center - self.electrode_metal_width / 2.0,
                    y: y_center - self.electrode_metal_height / 2.0,
                    width: self.electrode_metal_width,
                    height: self.electrode_metal_height,
                });
                
                // Add foil current collector
                let _ = sender.send(SimCommand::AddFoil {
                    width: self.electrode_foil_width,
                    height: self.electrode_foil_height,
                    x: x_center - self.electrode_foil_width / 2.0,
                    y: y_center - self.electrode_foil_height / 2.0,
                    particle_radius: Species::FoilMetal.radius(),
                    current: 0.0,
                });
                
                // Track foil ID for group assignment (foil IDs start at 1, created in order)
                let foil_id = (i + 1) as u64;
                if is_anode {
                    anode_foil_ids.push(foil_id);
                } else {
                    cathode_foil_ids.push(foil_id);
                }
            }
            
            // Create active material region for tracking
            let surface_area = self.electrode_metal_width * self.electrode_metal_height;
            let mut region = ActiveMaterialRegion::new(material, surface_area)
                .with_initial_soc(initial_soc);
            region.center_x = x_center;
            region.center_y = y_center;
            
            self.active_material_regions.push(region);
        }
        
        // Assign foil groups (Group A = anodes, Group B = cathodes)
        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
            let _ = sender.send(SimCommand::SetFoilGroups {
                group_a: anode_foil_ids.clone(),
                group_b: cathode_foil_ids.clone(),
            });
        }
        
        // Sync active material regions to simulation for intercalation physics
        if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
            let _ = sender.send(SimCommand::SyncActiveMaterialRegions {
                regions: self.active_material_regions.clone(),
            });
        }
        
        // Add electrolyte
        self.add_default_electrolyte();
        
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        println!("✓ Generated {} electrode layers with {} active regions",
            total_electrodes,
            self.active_material_regions.len());
        println!("  Anode foils (Group A): {:?}", anode_foil_ids);
        println!("  Cathode foils (Group B): {:?}", cathode_foil_ids);
    }
}

/// Preset cell configurations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CellPreset {
    #[default]
    LithiumSymmetric,
    LiGraphiteHalfCell,
    GraphiteLFP,
    GraphiteNMC,
    LTOLFP,
    Custom,
}
