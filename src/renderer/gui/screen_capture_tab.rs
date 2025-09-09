use super::*;

impl super::super::Renderer {
    pub fn show_screen_capture_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("📷 Screen Capture");
        
        ui.separator();
        
        // Recording status
        ui.horizontal(|ui| {
            let status_text = if self.screen_capture_enabled {
                format!("🔴 Recording - Frame {}", self.capture_counter)
            } else {
                "⏹️ Not Recording".to_string()
            };
            ui.label(status_text);
        });
        
        ui.separator();
        
        // Recording controls
        ui.group(|ui| {
            ui.label("Recording Controls:");
            
            ui.horizontal(|ui| {
                if ui.button(if self.screen_capture_enabled { "⏹️ Stop Recording" } else { "🔴 Start Recording" }).clicked() {
                    self.screen_capture_enabled = !self.screen_capture_enabled;
                    if self.screen_capture_enabled {
                        self.capture_counter = 0;
                        self.last_capture_time = *crate::renderer::state::SIM_TIME.lock();
                        // Ensure capture folder exists
                        if let Err(e) = std::fs::create_dir_all(&self.capture_folder) {
                            println!("Warning: Could not create capture folder '{}': {}", self.capture_folder, e);
                        } else {
                            println!("🔴 Screen capture recording started - will capture every {:.1} fs to folder: {}",
                                    self.capture_interval, self.capture_folder);
                        }
                    } else {
                        println!("⏹️ Screen capture recording stopped");
                    }
                }
                
                if ui.button("📷 Capture Now").clicked() {
                    // Trigger immediate capture
                    self.should_capture_next_frame = true;
                    let current_time = *crate::renderer::state::SIM_TIME.lock();
                    println!("📷 Manual capture triggered at simulation time {:.2} fs", current_time);
                }
            });
            
            ui.horizontal(|ui| {
                ui.label("Capture interval (fs):");
                ui.add(egui::DragValue::new(&mut self.capture_interval)
                    .clamp_range(0.1..=10.0)
                    .speed(0.1));
            });
            
            ui.horizontal(|ui| {
                ui.label("Save folder:");
                if ui.text_edit_singleline(&mut self.capture_folder).changed() {
                    // Ensure the folder path is valid and create if it doesn't exist
                    if let Err(e) = std::fs::create_dir_all(&self.capture_folder) {
                        println!("Warning: Could not create capture folder '{}': {}", self.capture_folder, e);
                    } else {
                        println!("Capture folder set to: {}", self.capture_folder);
                    }
                }
            });
        });
        
        ui.separator();
        
        // Capture region controls
        ui.group(|ui| {
            ui.label("Capture Region:");
            
            ui.horizontal(|ui| {
                if ui.button("Full Screen").clicked() {
                    self.capture_region = None;
                    self.is_selecting_region = false;
                    self.selection_start = None;
                    self.selection_end = None;
                    println!("📺 Capture region set to full screen");
                }
                
                let region_button_text = if self.is_selecting_region {
                    if self.selection_start.is_some() {
                        "👆 Drag to set region"
                    } else {
                        "👆 Click and drag to select"
                    }
                } else {
                    "📐 Set Region"
                };
                
                if ui.button(region_button_text).clicked() {
                    if self.is_selecting_region {
                        // Cancel region selection
                        self.cancel_region_selection();
                        println!("❌ Region selection cancelled");
                    } else {
                        // Start region selection
                        self.is_selecting_region = true;
                        self.selection_start = None;
                        self.selection_end = None;
                        println!("📐 Region selection started - click and drag in the simulation view");
                    }
                }
            });
            
            if let Some((top_left, bottom_right)) = self.capture_region {
                ui.label(format!("Region: World coords ({:.1}, {:.1}) to ({:.1}, {:.1})", 
                    top_left.x, top_left.y, bottom_right.x, bottom_right.y));
                ui.colored_label(egui::Color32::GREEN, "✅ Captures selected world region only");
                
                if ui.button("Clear Region").clicked() {
                    self.clear_capture_region();
                }
            } else {
                ui.label("Capturing full simulation window");
                ui.colored_label(egui::Color32::GREEN, "✅ Full window capture");
            }
            
            if self.is_selecting_region {
                ui.colored_label(egui::Color32::YELLOW, "⚠️ Region selection active - click and drag in simulation view");
                if let Some(start) = self.selection_start {
                    if let Some(end) = self.selection_end {
                        ui.label(format!("Selection: ({:.0}, {:.0}) to ({:.0}, {:.0})", start.x, start.y, end.x, end.y));
                    } else {
                        ui.label(format!("Start: ({:.0}, {:.0})", start.x, start.y));
                    }
                }
            }
        });
        
        ui.separator();
        
        // Statistics and info
        ui.group(|ui| {
            ui.label("Statistics:");
            
            ui.horizontal(|ui| {
                ui.label("Total captures this session:");
                ui.label(format!("{}", self.capture_counter));
            });
            
            ui.horizontal(|ui| {
                ui.label("Last capture time:");
                ui.label(format!("{:.2} fs", self.last_capture_time));
            });
            
            ui.horizontal(|ui| {
                ui.label("Current simulation time:");
                let current_time = *crate::renderer::state::SIM_TIME.lock();
                ui.label(format!("{:.2} fs", current_time));
            });
            
            if self.screen_capture_enabled {
                let current_time = *crate::renderer::state::SIM_TIME.lock();
                let time_until_next = self.capture_interval - (current_time - self.last_capture_time);
                if time_until_next > 0.0 {
                    ui.horizontal(|ui| {
                        ui.label("Next capture in:");
                        ui.label(format!("{:.1} fs", time_until_next));
                    });
                }
            }
        });
    }
}
