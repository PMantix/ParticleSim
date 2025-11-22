use super::*;
use crate::body::Species;

impl super::super::Renderer {
    pub fn show_legend_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("📖 Particle Legend");
        ui.label("Visual guide to particles and symbols in the simulation.");
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Helper to draw a row
            let draw_row = |ui: &mut egui::Ui, name: &str, scientific: &str, color: [u8; 4], radius: f32, description: Option<&str>| {
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
                        ui.strong(name);
                        ui.label(egui::RichText::new(scientific).italics());
                        if let Some(desc) = description {
                            ui.small(desc);
                        }
                    });
                });
                ui.separator();
            };

            // 1. Li ion
            let props = crate::species::get_species_props(Species::LithiumIon);
            draw_row(ui, "Li Ion", "Lithium Ion (Li+)", props.color, props.radius * 5.0, None);

            // 2. Electrolyte anion
            let props = crate::species::get_species_props(Species::ElectrolyteAnion);
            draw_row(ui, "Anion", "Electrolyte Anion (PF6-)", props.color, props.radius * 3.0, None);

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
            draw_row(ui, "EC", "Ethylene Carbonate", props.color, props.radius * 3.0, None);

            // 5. EMC
            let props = crate::species::get_species_props(Species::EMC);
            draw_row(ui, "EMC", "Ethyl Methyl Carbonate", props.color, props.radius * 3.0, None);

            // 6. DMC
            let props = crate::species::get_species_props(Species::DMC);
            draw_row(ui, "DMC", "Dimethyl Carbonate", props.color, props.radius * 3.0, None);

            // 7. FEC
            let props = crate::species::get_species_props(Species::FEC);
            draw_row(ui, "FEC", "Fluoroethylene Carbonate", props.color, props.radius * 3.0, None);

            // 8. VC
            let props = crate::species::get_species_props(Species::VC);
            draw_row(ui, "VC", "Vinylene Carbonate", props.color, props.radius * 3.0, None);

            // 9. LLZO
            let props = crate::species::get_species_props(Species::LLZO);
            draw_row(ui, "LLZO", "Lithium Lanthanum Zirconium Oxide", props.color, props.radius * 2.0, Some("Solid Electrolyte"));

            // 10. LLZT
            let props = crate::species::get_species_props(Species::LLZT);
            draw_row(ui, "LLZT", "Lithium Lanthanum Zirconium Tantalum Oxide", props.color, props.radius * 2.0, Some("Solid Electrolyte"));

            // 11. S40B
            let props = crate::species::get_species_props(Species::S40B);
            draw_row(ui, "S40B", "Sulfide Solid Electrolyte", props.color, props.radius * 2.0, None);

            // 12. Abstract foil metal
            let props = crate::species::get_species_props(Species::FoilMetal);
            draw_row(ui, "Foil Metal", "Current Collector / Electrode", props.color, props.radius * 4.0, Some("Stationary metal structure"));
        });
    }
}
