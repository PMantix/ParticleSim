use super::state::*;
use quarkstrom::egui;
use crate::renderer::Species;
use ultraviolet::Vec2;
use crate::renderer::Body;
use crate::Electron;
use crate::config::IsolineFieldMode;

impl super::Renderer {
    pub fn show_gui(&mut self, ctx: &quarkstrom::egui::Context) {
        egui::Window::new("")
            .open(&mut self.settings_window_open)
            .show(ctx, |ui| {
                // --- Field Controls ---
                ui.label("Field Controls:");
                let mut mag = *FIELD_MAGNITUDE.lock();
                ui.add(
                    egui::Slider::new(&mut mag, 0.0..=200.0)
                        .text("Field |E|")
                        .clamp_to_range(true)
                        .step_by(1.0), // Set increment to 1
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
                    egui::Slider::new(&mut *TIMESTEP.lock(), 0.0001..=0.01)
                        .text("Timestep (dt)")
                        .step_by(0.005),
                );
                ui.add(
                    egui::Slider::new(&mut self.sim_config.damping_base, 0.95..=1.0)
                        .text("Damping Base")
                        .step_by(0.0001),
                );

                let mut passes = COLLISION_PASSES.lock();
                ui.add(
                    egui::Slider::new(&mut *passes, 2..=20)
                        .text("Collision Passes")
                        .clamp_to_range(true),
                );

                if ui.button("Step Simulation").clicked() {
                    SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::StepOnce).unwrap();
                }

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
                ui.checkbox(&mut self.sim_config.show_charge_density, "Show Charge Density");
                ui.checkbox(&mut self.sim_config.show_field_vectors, "Show Field Vectors"); // NEW
                egui::ComboBox::from_label("Isoline Field Mode")
                    .selected_text(format!("{:?}", self.sim_config.isoline_field_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.sim_config.isoline_field_mode,
                            IsolineFieldMode::Total,
                            "Total",
                        );
                        ui.selectable_value(
                            &mut self.sim_config.isoline_field_mode,
                            IsolineFieldMode::ExternalOnly,
                            "External Only",
                        );
                        ui.selectable_value(
                            &mut self.sim_config.isoline_field_mode,
                            IsolineFieldMode::BodyOnly,
                            "Body Only",
                        );
                    });
                ui.add(
                    egui::Slider::new(&mut self.velocity_vector_scale, 0.01..=1.0)
                        .text("Velocity Vector Scale")
                        .step_by(0.01),
                );

                ui.separator();

                // --- Lennard-Jones Parameters ---
                ui.label("Lennard-Jones Parameters:");
                ui.add(egui::Slider::new(&mut self.sim_config.lj_force_epsilon, 0.0..=5000.0)
                    .text("LJ Epsilon (attraction strength)")
                    .step_by(1.0));
                ui.add(egui::Slider::new(&mut self.sim_config.lj_force_sigma, 0.1..=5.0)
                    .text("LJ Sigma (particle size)")
                    .step_by(0.01));
                ui.add(egui::Slider::new(&mut self.sim_config.lj_force_cutoff, 0.5..=10.0)
                    .text("LJ Cutoff (range factor)")
                    .step_by(0.01));

                ui.separator();

                // --- Scenario Controls ---
                ui.label("Scenario:");

                if ui.button("Delete All Particles").clicked() {
                    SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::DeleteAll).unwrap();
                }

                // Common controls for all Add scenarios
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.scenario_x).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.scenario_y).speed(0.1));
                    ui.label("Particle Radius:");
                    ui.add(egui::DragValue::new(&mut self.scenario_particle_radius).speed(0.05));
                    egui::ComboBox::from_label("Species")
                        .selected_text(format!("{:?}", self.scenario_species))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.scenario_species, Species::LithiumMetal, "Metal");
                            ui.selectable_value(&mut self.scenario_species, Species::LithiumIon, "Ion");
                            ui.selectable_value(&mut self.scenario_species, Species::ElectrolyteAnion, "Anion");
                        });
                });

                // Add Ring / Filled Circle
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut self.scenario_radius).speed(0.1));
                    if ui.button("Add Ring").clicked() {
                        let body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            1.0,
                            self.scenario_particle_radius,
                            self.scenario_species,
                        );
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddRing {
                            body,
                            x: self.scenario_x,
                            y: self.scenario_y,
                            radius: self.scenario_radius,
                        }).unwrap();
                    }
                    if ui.button("Add Filled Circle").clicked() {
                        let body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            1.0,
                            self.scenario_particle_radius,
                            self.scenario_species,
                        );
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddCircle {
                            body,
                            x: self.scenario_x,
                            y: self.scenario_y,
                            radius: self.scenario_radius,
                        }).unwrap();
                    }
                });

                // Add Rectangle
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut self.scenario_width).speed(0.1));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut self.scenario_height).speed(0.1));
                    if ui.button("Add Rectangle").clicked() {
                        let body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            1.0,
                            self.scenario_particle_radius,
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

                // Add Foil
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut self.scenario_width).speed(0.1));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut self.scenario_height).speed(0.1));
                    ui.label("Current:");
                    ui.add(egui::DragValue::new(&mut self.scenario_current).speed(0.1));
                    if ui.button("Add Foil").clicked() {
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddFoil {
                            width: self.scenario_width,
                            height: self.scenario_height,
                            x: self.scenario_x,
                            y: self.scenario_y,
                            particle_radius: self.scenario_particle_radius,
                            current: self.scenario_current,
                        }).unwrap();
                    }
                });

                // --- Debug/Diagnostics ---
                ui.separator();
                ui.label("Debug/Diagnostics:");
                ui.checkbox(&mut self.sim_config.show_lj_vs_coulomb_ratio, "Show LJ/Coulomb Force Ratio");
            });
    }
}

pub fn make_body_with_species(pos: Vec2, vel: Vec2, mass: f32, radius: f32, species: Species) -> Body {
    use crate::config::{LITHIUM_METAL_NEUTRAL_ELECTRONS, FOIL_NEUTRAL_ELECTRONS};
    let mut body = Body::new(pos, vel, mass, radius, 0.0, species);
    body.electrons.clear();
    match species {
        Species::LithiumMetal => {
            for _ in 0..LITHIUM_METAL_NEUTRAL_ELECTRONS {
                body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
        }
        Species::FoilMetal => {
            for _ in 0..FOIL_NEUTRAL_ELECTRONS {
                body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
        }
        Species::LithiumIon => {
            // Ions: one less electron than neutral metal, positive charge
            if LITHIUM_METAL_NEUTRAL_ELECTRONS > 0 {
                for _ in 0..(LITHIUM_METAL_NEUTRAL_ELECTRONS - 1) {
                    body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                }
            }
        }
        Species::ElectrolyteAnion => {
            // Anions: one more electron than neutral metal, negative charge
            if LITHIUM_METAL_NEUTRAL_ELECTRONS > 0 {
                for _ in 0..(LITHIUM_METAL_NEUTRAL_ELECTRONS + 1) {
                    body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                }
            }
        }
    }
    body.update_charge_from_electrons();
    body.update_species();
    body
}

// In your rendering/drawing code, use:
// let color = match body.species {
//     Species::LithiumMetal => /* existing color */,
//     Species::LithiumIon => /* existing color */,
//     Species::FoilMetal => egui::Color32::from_rgb(255, 128, 0), // Orange or any distinct color
// };