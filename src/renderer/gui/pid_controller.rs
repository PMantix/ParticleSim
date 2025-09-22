// pid_controller.rs
// PID controller visualization and plotting functionality

use super::*;
use std::sync::atomic::Ordering;
use crate::renderer::state::SHOW_PID_GRAPH;

impl super::super::Renderer {
    pub fn show_pid_graph(&mut self, ctx: &egui::Context) {
        if !SHOW_PID_GRAPH.load(Ordering::Relaxed) {
            return;
        }

        egui::Window::new("📈 PID Controller Graph")
            .default_size(egui::vec2(800.0, 600.0))
            .show(ctx, |ui| {
                let foils = FOILS.lock();
                
                // Find master foils in overpotential mode with history
                let master_foils: Vec<_> = foils
                    .iter()
                    .filter(|foil| {
                        matches!(foil.charging_mode, crate::body::foil::ChargingMode::Overpotential)
                            && foil.overpotential_controller.as_ref()
                                .map(|c| c.master_foil_id.is_none() && !c.history.is_empty())
                                .unwrap_or(false)
                    })
                    .collect();

                if master_foils.is_empty() {
                    ui.label("No master foils with overpotential control and history data available.");
                    ui.label("Enable overpotential mode on a foil to see PID data.");
                    return;
                }

                // Foil selector
                ui.horizontal(|ui| {
                    ui.label("Foil:");
                    egui::ComboBox::from_id_source("pid_graph_foil_selector")
                        .selected_text(format!("Foil {}", self.selected_pid_foil_id.unwrap_or(master_foils[0].id)))
                        .show_ui(ui, |ui| {
                            for foil in &master_foils {
                                ui.selectable_value(&mut self.selected_pid_foil_id, Some(foil.id), 
                                    format!("Foil {}", foil.id));
                            }
                        });
                    
                    // Auto-select first foil if none selected or selected foil no longer exists
                    if self.selected_pid_foil_id.is_none() || 
                       !master_foils.iter().any(|f| Some(f.id) == self.selected_pid_foil_id) {
                        self.selected_pid_foil_id = Some(master_foils[0].id);
                    }
                });

                // Find the selected foil
                if let Some(selected_id) = self.selected_pid_foil_id {
                    if let Some(foil) = master_foils.iter().find(|f| f.id == selected_id) {
                        if let Some(controller) = &foil.overpotential_controller {
                            self.draw_pid_plot(ui, &controller.history);
                        }
                    }
                }
            });
    }

    fn draw_pid_plot(&self, ui: &mut egui::Ui, history: &std::collections::VecDeque<crate::body::foil::PidHistoryPoint>) {
        if history.is_empty() {
            ui.label("No history data available yet. Enable overpotential mode and let the simulation run to collect data.");
            return;
        }

        ui.label(format!("PID History: {} data points", history.len()));

        // Plot dimensions
        let plot_height = 200.0;
        let plot_width = 600.0;
        let margin = 40.0;

        // Find data ranges
        let min_step = history.iter().map(|p| p.step).min().unwrap_or(0) as f64;
        let max_step = history.iter().map(|p| p.step).max().unwrap_or(1) as f64;
        
        // Main tracking plot (setpoint vs actual)
        ui.label("📈 Setpoint vs Actual");
        let setpoint_min = history.iter().map(|p| p.setpoint).fold(f32::INFINITY, f32::min) as f64;
        let setpoint_max = history.iter().map(|p| p.setpoint).fold(f32::NEG_INFINITY, f32::max) as f64;
        let actual_min = history.iter().map(|p| p.actual).fold(f32::INFINITY, f32::min) as f64;
        let actual_max = history.iter().map(|p| p.actual).fold(f32::NEG_INFINITY, f32::max) as f64;
        
        let y_min = (setpoint_min.min(actual_min) - 0.1).max(0.0);
        let y_max = setpoint_max.max(actual_max) + 0.1;
        
        self.draw_simple_plot(ui, history, plot_width, plot_height, margin, 
            min_step, max_step, y_min, y_max,
            &[("Setpoint", egui::Color32::BLACK, |p| p.setpoint as f64),
              ("Actual", egui::Color32::BLUE, |p| p.actual as f64)]);
        
        ui.separator();

        // Error and output plot
        ui.label("📉 Error & PID Output");
        let error_min = history.iter().map(|p| p.error).fold(f32::INFINITY, f32::min) as f64;
        let error_max = history.iter().map(|p| p.error).fold(f32::NEG_INFINITY, f32::max) as f64;
        let output_min = history.iter().map(|p| p.output).fold(f32::INFINITY, f32::min) as f64;
        let output_max = history.iter().map(|p| p.output).fold(f32::NEG_INFINITY, f32::max) as f64;
        
        let y_min2 = error_min.min(output_min) - 0.1;
        let y_max2 = error_max.max(output_max) + 0.1;
        
        self.draw_simple_plot(ui, history, plot_width, plot_height, margin,
            min_step, max_step, y_min2, y_max2,
            &[("Error", egui::Color32::RED, |p| p.error as f64),
              ("Output", egui::Color32::DARK_GREEN, |p| p.output as f64)]);
              
        ui.separator();

        // PID terms plot
        ui.label("🔧 PID Terms");
        let p_min = history.iter().map(|p| p.p_term).fold(f32::INFINITY, f32::min) as f64;
        let p_max = history.iter().map(|p| p.p_term).fold(f32::NEG_INFINITY, f32::max) as f64;
        let i_min = history.iter().map(|p| p.i_term).fold(f32::INFINITY, f32::min) as f64;
        let i_max = history.iter().map(|p| p.i_term).fold(f32::NEG_INFINITY, f32::max) as f64;
        let d_min = history.iter().map(|p| p.d_term).fold(f32::INFINITY, f32::min) as f64;
        let d_max = history.iter().map(|p| p.d_term).fold(f32::NEG_INFINITY, f32::max) as f64;
        
        let y_min3 = p_min.min(i_min).min(d_min) - 0.1;
        let y_max3 = p_max.max(i_max).max(d_max) + 0.1;
        
        self.draw_simple_plot(ui, history, plot_width, plot_height, margin,
            min_step, max_step, y_min3, y_max3,
            &[("P Term", egui::Color32::from_rgb(255, 100, 100), |p| p.p_term as f64),
              ("I Term", egui::Color32::from_rgb(100, 100, 255), |p| p.i_term as f64),
              ("D Term", egui::Color32::from_rgb(100, 255, 100), |p| p.d_term as f64)]);
        
        // Display current statistics at the bottom
        if let Some(latest) = history.back() {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("📊 Latest Values:");
                ui.colored_label(egui::Color32::BLUE, format!("Actual: {:.3}", latest.actual));
                ui.colored_label(egui::Color32::BLACK, format!("Setpoint: {:.3}", latest.setpoint));
                ui.colored_label(
                    if latest.error.abs() < 0.01 { egui::Color32::GREEN } else { egui::Color32::RED },
                    format!("Error: {:.3}", latest.error)
                );
                ui.colored_label(egui::Color32::DARK_GREEN, format!("Output: {:.2}A", latest.output));
            });
        }
    }

    fn draw_simple_plot(&self, ui: &mut egui::Ui, 
                       history: &std::collections::VecDeque<crate::body::foil::PidHistoryPoint>,
                       width: f32, height: f32, margin: f32,
                       x_min: f64, x_max: f64, y_min: f64, y_max: f64,
                       series: &[(&str, egui::Color32, fn(&crate::body::foil::PidHistoryPoint) -> f64)]) {
        
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            
            // Draw background
            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(250, 250, 250));
            painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
            
            // Plot area
            let plot_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(margin, margin/2.0),
                egui::vec2(width - margin * 1.5, height - margin)
            );
            
            // Draw grid lines
            for i in 0..5 {
                let y = plot_rect.min.y + (i as f32) * plot_rect.height() / 4.0;
                painter.line_segment(
                    [egui::pos2(plot_rect.min.x, y), egui::pos2(plot_rect.max.x, y)],
                    egui::Stroke::new(0.5, egui::Color32::LIGHT_GRAY)
                );
            }
            
            // Transform point to screen coordinates
            let transform_point = |step: u64, value: f64| -> egui::Pos2 {
                let x_norm = if x_max > x_min { (step as f64 - x_min) / (x_max - x_min) } else { 0.0 };
                let y_norm = if y_max > y_min { (value - y_min) / (y_max - y_min) } else { 0.5 };
                egui::pos2(
                    plot_rect.min.x + x_norm as f32 * plot_rect.width(),
                    plot_rect.max.y - y_norm as f32 * plot_rect.height()
                )
            };
            
            // Draw each series
            for (_name, color, value_fn) in series {
                let points: Vec<egui::Pos2> = history.iter()
                    .map(|point| transform_point(point.step, value_fn(point)))
                    .collect();
                
                if points.len() > 1 {
                    for i in 0..points.len()-1 {
                        painter.line_segment(
                            [points[i], points[i+1]],
                            egui::Stroke::new(2.0, *color)
                        );
                    }
                }
            }
            
            // Draw y-axis labels
            for i in 0..5 {
                let y = plot_rect.min.y + (i as f32) * plot_rect.height() / 4.0;
                let value = y_max - (i as f64) * (y_max - y_min) / 4.0;
                painter.text(
                    egui::pos2(rect.min.x + 5.0, y - 8.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{:.2}", value),
                    egui::FontId::monospace(10.0),
                    egui::Color32::BLACK,
                );
            }
            
            // Draw legend
            let mut legend_y = rect.min.y + 10.0;
            for (name, color, _) in series {
                painter.line_segment(
                    [egui::pos2(rect.max.x - 80.0, legend_y), egui::pos2(rect.max.x - 60.0, legend_y)],
                    egui::Stroke::new(2.0, *color)
                );
                painter.text(
                    egui::pos2(rect.max.x - 55.0, legend_y),
                    egui::Align2::LEFT_CENTER,
                    *name,
                    egui::FontId::default(),
                    egui::Color32::BLACK,
                );
                legend_y += 15.0;
            }
        }
    }
}