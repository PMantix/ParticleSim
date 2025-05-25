use super::state::*;
use quarkstrom::egui;

impl super::Renderer {
    pub fn show_gui(&mut self, ctx: &quarkstrom::egui::Context) {
        egui::Window::new("")
            .open(&mut self.settings_window_open)
            .show(ctx, |ui| {
                // read‐modify‐write magnitude
                let mut mag = *FIELD_MAGNITUDE.lock();
                ui.add(
                    egui::Slider::new(&mut mag, 0.0..=1000.0)
                        .text("Field |E|")
                        .clamp_to_range(true),
                );
                *FIELD_MAGNITUDE.lock() = mag;

                // read‐modify‐write direction
                let mut dir = *FIELD_DIRECTION.lock();
                ui.add(
                    egui::Slider::new(&mut dir, 0.0..=360.0)
                        .text("Field θ (deg)")
                        .clamp_to_range(true),
                );
                *FIELD_DIRECTION.lock() = dir;

                //Show bodies GUI
                ui.checkbox(&mut self.show_bodies, "Show Bodies");

                //Show quadtree GUI
                ui.checkbox(&mut self.show_quadtree, "Show Quadtree");

                //Timestep GUI
                ui.add(
                    egui::Slider::new(&mut *TIMESTEP.lock(), 0.001..=0.2)
                        .text("Timestep (dt)"),
                );

                //collision passes GUI
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
                ui.label("Visualization Overlays:");
                ui.checkbox(&mut self.sim_config.show_field_isolines, "Show Field Isolines");
                ui.checkbox(&mut self.sim_config.show_velocity_vectors, "Show Velocity Vectors");
                ui.checkbox(&mut self.sim_config.show_electron_density, "Show Electron Density");
            });
    }
}