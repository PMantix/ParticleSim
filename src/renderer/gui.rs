use super::state::*;
use quarkstrom::egui;
use crate::renderer::Species;

impl super::Renderer {
    pub fn show_gui(&mut self, ctx: &quarkstrom::egui::Context) {
        egui::Window::new("")
            .open(&mut self.settings_window_open)
            .show(ctx, |ui| {
                // --- Field Controls ---
                ui.label("Field Controls:");
                let mut mag = *FIELD_MAGNITUDE.lock();
                ui.add(
                    egui::Slider::new(&mut mag, 0.0..=1000.0)
                        .text("Field |E|")
                        .clamp_to_range(true),
                );
                *FIELD_MAGNITUDE.lock() = mag;

                let mut dir = *FIELD_DIRECTION.lock();
                ui.add(
                    egui::Slider::new(&mut dir, 0.0..=360.0)
                        .text("Field θ (deg)")
                        .clamp_to_range(true),
                );
                *FIELD_DIRECTION.lock() = dir;

                ui.separator();

                // --- Display Options ---
                ui.label("Display Options:");
                ui.checkbox(&mut self.show_bodies, "Show Bodies");
                ui.checkbox(&mut self.show_quadtree, "Show Quadtree");

                ui.separator();

                // --- Simulation Controls ---
                ui.label("Simulation Controls:");
                ui.add(
                    egui::Slider::new(&mut *TIMESTEP.lock(), 0.001..=0.2)
                        .text("Timestep (dt)"),
                );

                let mut passes = COLLISION_PASSES.lock();
                ui.add(
                    egui::Slider::new(&mut *passes, 1..=10)
                        .text("Collision Passes")
                        .clamp_to_range(true),
                );

                if self.show_quadtree {
                    let range = &mut self.depth_range;
                    ui.horizontal(|ui| {
                        ui.label("Depth Range:");
                        ui.add(egui::DragValue::new(&mut range.0).speed(0.05));
                        ui.label("to");
                        ui.add(egui::DragValue::new(&mut range.1).speed(0.05));
                    });
                }

                ui.separator();

                // --- Visualization Overlays ---
                ui.label("Visualization Overlays:");
                ui.checkbox(&mut self.sim_config.show_field_isolines, "Show Field Isolines");
                ui.checkbox(&mut self.sim_config.show_velocity_vectors, "Show Velocity Vectors");
                ui.checkbox(&mut self.sim_config.show_electron_density, "Show Electron Density");

                ui.separator();

                // --- Scenario Controls ---
                ui.label("Scenario:");
                if ui.button("Delete All Particles").clicked() {
                    SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::DeleteAll).unwrap();
                }

                // Add Ring
                ui.horizontal(|ui| {
                    ui.label("Add Ring:");
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut self.scenario_radius).speed(0.1));
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.scenario_x).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.scenario_y).speed(0.1));
                    egui::ComboBox::from_label("Species")
                        .selected_text(format!("{:?}", self.scenario_species))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.scenario_species, Species::LithiumMetal, "Metal");
                            ui.selectable_value(&mut self.scenario_species, Species::LithiumIon, "Ion");
                        });
                    ui.label("Particle Radius:");
                    ui.add(egui::DragValue::new(&mut self.scenario_particle_radius).speed(0.05));
                    if ui.button("Add").clicked() {
                        let body = crate::body::Body::new(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            1.0,
                            self.scenario_particle_radius,
                            0.0,
                            self.scenario_species,
                        );
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddRing {
                            body,
                            x: self.scenario_x,
                            y: self.scenario_y,
                            radius: self.scenario_radius, // from GUI
                        }).unwrap();
                    }
                });

                // Add Filled Circle
                ui.horizontal(|ui| {
                    ui.label("Add Filled Circle:");
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut self.scenario_radius).speed(0.1));
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.scenario_x).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.scenario_y).speed(0.1));
                    egui::ComboBox::from_label("Species")
                        .selected_text(format!("{:?}", self.scenario_species))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.scenario_species, Species::LithiumMetal, "Metal");
                            ui.selectable_value(&mut self.scenario_species, Species::LithiumIon, "Ion");
                        });
                    ui.label("Particle Radius:");
                    ui.add(egui::DragValue::new(&mut self.scenario_particle_radius).speed(0.05));
                    if ui.button("Add").clicked() {
                        let body = crate::body::Body::new(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            1.0,
                            self.scenario_particle_radius,
                            0.0,
                            self.scenario_species,
                        );
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddCircle {
                            body,
                            x: self.scenario_x,
                            y: self.scenario_y,
                            radius: self.scenario_radius, // from GUI
                        }).unwrap();
                    }
                });

                // Add Rectangle
                ui.horizontal(|ui| {
                    ui.label("Add Rectangle:");
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut self.scenario_width).speed(0.1));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut self.scenario_height).speed(0.1));
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.scenario_x).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.scenario_y).speed(0.1));
                    egui::ComboBox::from_label("Species")
                        .selected_text(format!("{:?}", self.scenario_species))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.scenario_species, Species::LithiumMetal, "Metal");
                            ui.selectable_value(&mut self.scenario_species, Species::LithiumIon, "Ion");
                        });
                    ui.label("Particle Radius:");
                    ui.add(egui::DragValue::new(&mut self.scenario_particle_radius).speed(0.05));
                    if ui.button("Add").clicked() {
                        let body = crate::body::Body::new(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            1.0,
                            self.scenario_particle_radius,
                            0.0,
                            self.scenario_species,
                        );
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddRectangle {
                            body,
                            x: self.scenario_x,
                            y: self.scenario_y,
                            width: self.scenario_width,
                            height: self.scenario_height,
                        }).unwrap();
                    }
                });
            });
    }
}