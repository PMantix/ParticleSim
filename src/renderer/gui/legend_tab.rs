use super::*;
use crate::body::Species;

impl super::super::Renderer {
    pub fn show_legend_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("📖 Particle Legend");
        ui.label("Visual guide to particles and symbols in the simulation.");
        ui.add_space(10.0);

        let hovered_species = self.hovered_species;

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Helper to draw a row
            let draw_row = |ui: &mut egui::Ui, species: Option<Species>, name: &str, scientific: &str, color: [u8; 4], radius: f32, description: Option<&str>| {
                let is_highlighted = species.is_some() && species == hovered_species;
                
                let mut rect_color = egui::Color32::TRANSPARENT;
                let mut text_color = ui.visuals().text_color();

                if is_highlighted {
                    // Stronger yellow background
                    rect_color = egui::Color32::from_rgba_premultiplied(255, 255, 0, 128);
                    // Dark text for contrast
                    text_color = egui::Color32::BLACK;
                }

                egui::Frame::none().fill(rect_color).inner_margin(2.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Draw particle circle
                        let (rect, _response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                        let center = rect.center();
                        // Scale radius for visibility in legend, but keep relative sizes somewhat
                        // Clamp to reasonable display size
                        let draw_radius = radius.clamp(3.0, 12.0); 
                        
                        let fill_color = egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        ui.painter().circle_filled(center, draw_radius, fill_color);

                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(name).strong().color(text_color));
                            ui.label(egui::RichText::new(scientific).italics().color(text_color));
                            if let Some(desc) = description {
                                ui.label(egui::RichText::new(desc).small().color(text_color));
                            }
                        });
                    });
                });
                ui.separator();
            };

            // 1. Li ion
            let props = crate::species::get_species_props(Species::LithiumIon);
            draw_row(ui, Some(Species::LithiumIon), "Li Ion", "Lithium Ion (Li+)", props.color, props.radius * 5.0, None);

            // 2. Lithium Metal
            let props = crate::species::get_species_props(Species::LithiumMetal);
            draw_row(ui, Some(Species::LithiumMetal), "Li Metal", "Lithium Metal (Li)", props.color, props.radius * 4.0, Some("Plated metal"));

            // 3. Electrolyte anion
            let props = crate::species::get_species_props(Species::ElectrolyteAnion);
            draw_row(ui, Some(Species::ElectrolyteAnion), "PF6", "Hexafluorophosphate (PF6-)", props.color, props.radius * 3.0, None);

            // 3. Electron
            // Individual electron
            ui.horizontal(|ui| {
                let (rect, _response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                let center = rect.center();
                ui.painter().circle_filled(center, 3.0, egui::Color32::from_rgba_unmultiplied(0, 128, 255, 255));
                
                ui.vertical(|ui| {
                    ui.strong("Electron");
                    ui.label(egui::RichText::new("e-").italics());
                });
            });
            ui.separator();

            // Excess electrons (Green halo/circle)
             ui.horizontal(|ui| {
                let (rect, _response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                let center = rect.center();
                ui.painter().circle_filled(center, 8.0, egui::Color32::from_rgba_unmultiplied(0, 255, 0, 255));
                
                ui.vertical(|ui| {
                    ui.strong("Excess Electrons");
                    ui.small("Indicates negative charge accumulation (surplus e-)");
                });
            });
            ui.separator();

            // Void space / Deficit (Red halo/circle)
             ui.horizontal(|ui| {
                let (rect, _response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                let center = rect.center();
                ui.painter().circle_filled(center, 8.0, egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255));
                
                ui.vertical(|ui| {
                    ui.strong("Electron Deficit");
                    ui.small("Indicates positive charge accumulation (lack of e-)");
                });
            });
            ui.separator();

            // 4. EC
            let props = crate::species::get_species_props(Species::EC);
            draw_row(ui, Some(Species::EC), "EC", "Ethylene Carbonate", props.color, props.radius * 3.0, None);

            // 5. EMC
            let props = crate::species::get_species_props(Species::EMC);
            draw_row(ui, Some(Species::EMC), "EMC", "Ethyl Methyl Carbonate", props.color, props.radius * 3.0, None);

            // 6. DMC
            let props = crate::species::get_species_props(Species::DMC);
            draw_row(ui, Some(Species::DMC), "DMC", "Dimethyl Carbonate", props.color, props.radius * 3.0, None);

            // 7. FEC
            let props = crate::species::get_species_props(Species::FEC);
            draw_row(ui, Some(Species::FEC), "FEC", "Fluoroethylene Carbonate", props.color, props.radius * 3.0, None);

            // 8. VC
            let props = crate::species::get_species_props(Species::VC);
            draw_row(ui, Some(Species::VC), "VC", "Vinylene Carbonate", props.color, props.radius * 3.0, None);

            // 9. LLZO
            let props = crate::species::get_species_props(Species::LLZO);
            draw_row(ui, Some(Species::LLZO), "LLZO", "Lithium Lanthanum Zirconium Oxide", props.color, props.radius * 2.0, Some("Solid Electrolyte"));

            // 10. LLZT
            let props = crate::species::get_species_props(Species::LLZT);
            draw_row(ui, Some(Species::LLZT), "LLZT", "Lithium Lanthanum Zirconium Tantalum Oxide", props.color, props.radius * 2.0, Some("Solid Electrolyte"));

            // 11. S40B
            let props = crate::species::get_species_props(Species::S40B);
            draw_row(ui, Some(Species::S40B), "S40B", "Sulfide Solid Electrolyte", props.color, props.radius * 2.0, None);

            // 12. Abstract foil metal
            let props = crate::species::get_species_props(Species::FoilMetal);
            draw_row(ui, Some(Species::FoilMetal), "Foil Metal", "Current Collector / Electrode", props.color, props.radius * 4.0, Some("Stationary metal structure"));
        });

        // Draw floating tooltip if hovering a species in the simulation view
        if let Some(species) = hovered_species {
            let mouse_pos = ui.input(|i| i.pointer.hover_pos());
            if let Some(pos) = mouse_pos {
                // Only show if mouse is NOT over the window (approximate check)
                // Actually, we want to show it regardless if we are hovering a particle
                // But if we are hovering the legend itself, we might not want to show it?
                // The hovered_species is only set if we are hovering a body in the simulation view
                // (checked in input.rs).
                // However, input.rs check might be naive and check even if mouse is over UI.
                // But usually UI blocks mouse input to simulation if we handle it right.
                // For now, just show it.
                
                // Use a fixed offset to not cover the mouse
                let tooltip_pos = pos + egui::vec2(15.0, 15.0);
                
                // Create a small area for the tooltip
                egui::Area::new("particle_tooltip")
                    .fixed_pos(tooltip_pos)
                    .order(egui::Order::Tooltip)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            let name = match species {
                                Species::LithiumIon => "Li+",
                                Species::LithiumMetal => "Li Metal",
                                Species::ElectrolyteAnion => "PF6-",
                                Species::EC => "EC",
                                Species::EMC => "EMC",
                                Species::DMC => "DMC",
                                Species::FEC => "FEC",
                                Species::VC => "VC",
                                Species::LLZO => "LLZO",
                                Species::LLZT => "LLZT",
                                Species::S40B => "S40B",
                                Species::FoilMetal => "Foil Metal",
                            };
                            ui.label(name);
                        });
                    });
            }
        }
    }
}
