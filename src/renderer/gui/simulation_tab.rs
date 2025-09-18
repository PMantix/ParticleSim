use super::*;

impl super::super::Renderer {
    pub fn show_simulation_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙️ Simulation Controls");

        let playback_status = PLAYBACK_STATUS.lock().clone();
        let sender_opt = SIM_COMMAND_SENDER.lock().clone();
        if self.playback_follow_live && playback_status.mode == PlaybackModeStatus::Live {
            self.playback_cursor = playback_status.latest_index;
        } else {
            self.playback_cursor = playback_status.cursor;
        }
        self.playback_speed = playback_status.speed;

        ui.group(|ui| {
            ui.label("🎞 Playback History");

            let latest_index = playback_status.latest_index;
            let slider_response = ui.add(
                egui::Slider::new(&mut self.playback_cursor, 0..=latest_index)
                    .text("History Frame"),
            );
            if slider_response.changed() {
                if self.playback_cursor != latest_index {
                    self.playback_follow_live = false;
                }
                if let Some(sender) = sender_opt.clone() {
                    let _ = sender.send(SimCommand::PlaybackSeek {
                        index: self.playback_cursor,
                    });
                }
            }

            ui.horizontal(|ui| {
                let history_available = playback_status.history_len > 1;
                if ui
                    .add_enabled(history_available, egui::Button::new("▶ Play"))
                    .clicked()
                {
                    if let Some(sender) = sender_opt.clone() {
                        let _ = sender.send(SimCommand::PlaybackPlay {
                            auto_resume: self.playback_auto_resume,
                        });
                    }
                }
                if ui.button("⏸ Pause").clicked() {
                    if let Some(sender) = sender_opt.clone() {
                        let _ = sender.send(SimCommand::PlaybackPause);
                    }
                }
                if ui.button("Go Live").clicked() {
                    self.playback_follow_live = true;
                    if let Some(sender) = sender_opt.clone() {
                        let _ = sender.send(SimCommand::PlaybackResumeLive);
                    }
                }
                if ui.button("Resume Here").clicked() {
                    self.playback_follow_live = true;
                    if let Some(sender) = sender_opt.clone() {
                        let _ = sender.send(SimCommand::PlaybackResumeFromCurrent);
                    }
                }
            });

            let speed_response = ui.add(
                egui::Slider::new(&mut self.playback_speed, 0.1..=4.0)
                    .text("Playback Speed (×)")
                    .logarithmic(true),
            );
            if speed_response.changed() {
                if let Some(sender) = sender_opt.clone() {
                    let _ = sender.send(SimCommand::PlaybackSetSpeed {
                        speed: self.playback_speed,
                    });
                }
            }

            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut self.playback_follow_live, "Follow live edge")
                    .changed()
                    && self.playback_follow_live
                {
                    self.playback_cursor = playback_status.latest_index;
                    if let Some(sender) = sender_opt.clone() {
                        let _ = sender.send(SimCommand::PlaybackSeek {
                            index: self.playback_cursor,
                        });
                    }
                }
                ui.checkbox(
                    &mut self.playback_auto_resume,
                    "Auto-resume when playback catches up",
                );
            });

            ui.label(format!(
                "Currently viewing frame {} ({:.2} fs, Δt {:.2} fs)",
                playback_status.frame, playback_status.sim_time, playback_status.dt
            ));
        });

        // Field Controls
        ui.group(|ui| {
            ui.label("🔋 Electric Field");
            let mut mag = *FIELD_MAGNITUDE.lock();
            ui.add(
                egui::Slider::new(&mut mag, 0.0..=1.0)
                    .text("Field |E|")
                    .clamp_to_range(true)
                    .step_by(0.0001),
            );
            *FIELD_MAGNITUDE.lock() = mag;

            let mut dir = *FIELD_DIRECTION.lock();
            ui.add(
                egui::Slider::new(&mut dir, 0.0..=360.0)
                    .text("Field θ (deg)")
                    .clamp_to_range(true),
            );
            *FIELD_DIRECTION.lock() = dir;
        });

        ui.separator();

        // Core simulation parameters
        ui.group(|ui| {
            ui.label("⏱️ Simulation Parameters");
            ui.add(
                egui::Slider::new(&mut *TIMESTEP.lock(), 0.1..=100.0)
                    .text("Timestep (fs)")
                    .step_by(0.05)
                    .logarithmic(false),
            );
            ui.label("💡 Typical MD timesteps: 0.5-2.0 fs");
            ui.add(
                egui::Slider::new(&mut self.sim_config.damping_base, 0.95..=1.0)
                    .text("Damping Base")
                    .step_by(0.0001),
            );

            let mut passes = COLLISION_PASSES.lock();
            ui.add(
                egui::Slider::new(&mut *passes, 2..=50)
                    .text("Collision Passes")
                    .clamp_to_range(true),
            );

            if ui.button("Step Simulation").clicked() {
                if let Some(sender) = sender_opt {
                    let _ = sender.send(SimCommand::StepOnce);
                }
            }
        });
    }
}
