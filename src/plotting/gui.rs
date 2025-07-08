// plotting/gui.rs
// GUI controls for the plotting system

use crate::plotting::{PlotType, Quantity, SamplingMode, PlotConfig, PlottingSystem, ExportFormat};
use crate::body::Species;
use quarkstrom::egui;

pub fn show_plotting_controls(
    ui: &mut egui::Ui,
    plotting_system: &mut PlottingSystem,
    show_plotting_window: &mut bool,
    _new_plot_type: &mut PlotType,
    _new_plot_quantity: &mut Quantity,
    _new_plot_sampling_mode: &mut SamplingMode,
    _new_plot_title: &mut String,
    _new_plot_spatial_bins: &mut usize,
    _new_plot_time_window: &mut f32,
    _new_plot_update_frequency: &mut f32,
) {
    egui::CollapsingHeader::new("Data Analysis & Plotting")
        .default_open(false)
        .show(ui, |ui| {
            if ui.button("Open Plotting Window").clicked() {
                *show_plotting_window = true;
            }
            
            ui.separator();
            
            // Quick plot buttons
            ui.label("Quick Plots:");
            ui.horizontal(|ui| {
                if ui.button("Li+ Population vs Time").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::TimeSeries,
                        quantity: Quantity::TotalSpeciesCount(Species::LithiumIon),
                        title: "Lithium Ion Population".to_string(),
                        sampling_mode: SamplingMode::Continuous,
                        spatial_bins: 50,
                        time_window: 20.0,
                        update_frequency: 2.0,
                    };
                    plotting_system.create_plot_window(config);
                }
                
                if ui.button("Charge Distribution vs X").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::SpatialProfileX,
                        quantity: Quantity::Charge,
                        title: "Charge Distribution (X-axis)".to_string(),
                        sampling_mode: SamplingMode::SingleTimestep,
                        spatial_bins: 100,
                        time_window: 10.0,
                        update_frequency: 1.0,
                    };
                    plotting_system.create_plot_window(config);
                }
            });
            
            ui.horizontal(|ui| {
                if ui.button("Velocity Profile vs X").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::SpatialProfileX,
                        quantity: Quantity::Velocity,
                        title: "X-Velocity Profile".to_string(),
                        sampling_mode: SamplingMode::SingleTimestep,
                        spatial_bins: 100,
                        time_window: 10.0,
                        update_frequency: 1.0,
                    };
                    plotting_system.create_plot_window(config);
                }
                
                if ui.button("All Species vs Time").clicked() {
                    // Create plots for all species
                    for species in [Species::LithiumIon, Species::LithiumMetal, Species::ElectrolyteAnion] {
                        let species_name = match species {
                            Species::LithiumIon => "Li+ Ions",
                            Species::LithiumMetal => "Li Metal",
                            Species::ElectrolyteAnion => "Anions",
                            _ => "Unknown",
                        };
                        let config = PlotConfig {
                            plot_type: PlotType::TimeSeries,
                            quantity: Quantity::TotalSpeciesCount(species),
                            title: format!("{} Population", species_name),
                            sampling_mode: SamplingMode::Continuous,
                            spatial_bins: 50,
                            time_window: 30.0,
                            update_frequency: 2.0,
                        };
                        plotting_system.create_plot_window(config);
                    }
                }
            });
            
            ui.horizontal(|ui| {
                if ui.button("Foil Current Analysis").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::TimeSeries,
                        quantity: Quantity::FoilCurrent(1), // Use foil ID 1 by default
                        title: "Foil Current vs Time".to_string(),
                        sampling_mode: SamplingMode::Continuous,
                        spatial_bins: 50,
                        time_window: 15.0,
                        update_frequency: 5.0,
                    };
                    plotting_system.create_plot_window(config);
                }
                
                if ui.button("Electron Count vs X").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::SpatialProfileX,
                        quantity: Quantity::ElectronCount,
                        title: "Average Electron Count vs X".to_string(),
                        sampling_mode: SamplingMode::SingleTimestep,
                        spatial_bins: 80,
                        time_window: 10.0,
                        update_frequency: 1.0,
                    };
                    plotting_system.create_plot_window(config);
                }
            });
            
            ui.separator();
            
            // Show active plots
            if !plotting_system.windows.is_empty() {
                ui.label(format!("Active Plots: {}", plotting_system.windows.len()));
                ui.horizontal_wrapped(|ui| {
                    let window_ids: Vec<String> = plotting_system.windows.keys().cloned().collect();
                    for window_id in window_ids {
                        if let Some(window) = plotting_system.windows.get(&window_id) {
                            let button_text = format!("{} ({})", window.config.title, format!("{:?}", window.config.plot_type));
                            if ui.button(button_text).clicked() {
                                // Toggle window visibility
                                if let Some(window) = plotting_system.windows.get_mut(&window_id) {
                                    window.is_open = !window.is_open;
                                }
                            }
                        }
                    }
                });
            }
        });
}

pub fn show_plotting_window(
    ctx: &egui::Context,
    plotting_system: &mut PlottingSystem,
    show_plotting_window: &mut bool,
    new_plot_type: &mut PlotType,
    new_plot_quantity: &mut Quantity,
    new_plot_sampling_mode: &mut SamplingMode,
    new_plot_title: &mut String,
    new_plot_spatial_bins: &mut usize,
    new_plot_time_window: &mut f32,
    new_plot_update_frequency: &mut f32,
) {
    egui::Window::new("Plotting Control Panel")
        .open(show_plotting_window)
        .default_size([400.0, 600.0])
        .show(ctx, |ui| {
            ui.heading("Create New Plot");
            
            ui.horizontal(|ui| {
                ui.label("Title:");
                ui.text_edit_singleline(new_plot_title);
            });
            
            ui.horizontal(|ui| {
                ui.label("Plot Type:");
                egui::ComboBox::from_id_source("plot_type")
                    .selected_text(format!("{:?}", new_plot_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(new_plot_type, PlotType::TimeSeries, "Time Series");
                        ui.selectable_value(new_plot_type, PlotType::SpatialProfileX, "Spatial Profile (X)");
                        ui.selectable_value(new_plot_type, PlotType::SpatialProfileY, "Spatial Profile (Y)");
                    });
            });
            
            ui.horizontal(|ui| {
                ui.label("Quantity:");
                show_quantity_selector(ui, new_plot_quantity, new_plot_type);
            });
            
            ui.horizontal(|ui| {
                ui.label("Sampling:");
                egui::ComboBox::from_id_source("sampling_mode")
                    .selected_text(format!("{:?}", new_plot_sampling_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(new_plot_sampling_mode, SamplingMode::SingleTimestep, "Single Timestep");
                        ui.selectable_value(new_plot_sampling_mode, SamplingMode::Continuous, "Continuous");
                        ui.selectable_value(new_plot_sampling_mode, SamplingMode::TimeAveraged { window: 1.0 }, "Time Averaged");
                    });
            });
            
            // Configuration parameters
            ui.separator();
            ui.heading("Parameters");
            
            ui.horizontal(|ui| {
                ui.label("Spatial Bins:");
                ui.add(egui::Slider::new(new_plot_spatial_bins, 10..=200).clamp_to_range(true));
            });
            
            ui.horizontal(|ui| {
                ui.label("Time Window (s):");
                ui.add(egui::Slider::new(new_plot_time_window, 1.0..=60.0).step_by(1.0));
            });
            
            ui.horizontal(|ui| {
                ui.label("Update Frequency (Hz):");
                ui.add(egui::Slider::new(new_plot_update_frequency, 0.1..=30.0).step_by(0.1));
            });
            
            if ui.button("Create Plot").clicked() {
                let config = PlotConfig {
                    plot_type: new_plot_type.clone(),
                    quantity: new_plot_quantity.clone(),
                    title: new_plot_title.clone(),
                    sampling_mode: new_plot_sampling_mode.clone(),
                    spatial_bins: *new_plot_spatial_bins,
                    time_window: *new_plot_time_window,
                    update_frequency: *new_plot_update_frequency,
                };
                plotting_system.create_plot_window(config);
            }
            
            ui.separator();
            ui.heading("Active Plots");
            
            show_active_plots(ui, plotting_system);
        });
}

fn show_quantity_selector(ui: &mut egui::Ui, quantity: &mut Quantity, plot_type: &PlotType) {
    // First validate that the current quantity is compatible with the plot type
    if !is_quantity_compatible_with_plot_type(quantity, plot_type) {
        // Reset to a safe default
        *quantity = match plot_type {
            PlotType::TimeSeries => {
                Quantity::TotalSpeciesCount(Species::LithiumIon)
            }
            _ => Quantity::Charge,
        };
    }
    
    egui::ComboBox::from_id_source("quantity")
        .selected_text(format!("{:?}", quantity))
        .show_ui(ui, |ui| {
            // Always available quantities
            ui.selectable_value(quantity, Quantity::Charge, "Charge");
            ui.selectable_value(quantity, Quantity::ElectronCount, "Electron Count");
            ui.selectable_value(quantity, Quantity::Velocity, "Velocity");
            
            // Species-related quantities
            ui.selectable_value(quantity, Quantity::TotalSpeciesCount(Species::LithiumIon), "Li+ Count");
            ui.selectable_value(quantity, Quantity::TotalSpeciesCount(Species::LithiumMetal), "Li Metal Count");
            ui.selectable_value(quantity, Quantity::TotalSpeciesCount(Species::ElectrolyteAnion), "Anion Count");
            
            // Time series only quantities
            if matches!(plot_type, PlotType::TimeSeries) {
                ui.selectable_value(quantity, Quantity::FoilCurrent(1), "Foil Current (ID 1)");
                ui.selectable_value(quantity, Quantity::ElectronHopRate, "Electron Hop Rate");
                ui.selectable_value(quantity, Quantity::DendriteFormationRate, "Dendrite Formation Rate");
            }
            
            // Spatial quantities only
            if matches!(plot_type, PlotType::SpatialProfileX | PlotType::SpatialProfileY) {
                ui.selectable_value(quantity, Quantity::LocalFieldStrength, "Local Field Strength");
            }
        });
}

fn is_quantity_compatible_with_plot_type(quantity: &Quantity, plot_type: &PlotType) -> bool {
    match quantity {
        // These quantities only make sense for time series
        Quantity::FoilCurrent(_) | Quantity::ElectronHopRate | Quantity::DendriteFormationRate => {
            matches!(plot_type, PlotType::TimeSeries)
        }
        // Local field strength is only meaningful for spatial plots
        Quantity::LocalFieldStrength => {
            matches!(plot_type, PlotType::SpatialProfileX | PlotType::SpatialProfileY)
        }
        // Other quantities work with most plot types
        _ => true,
    }
}

fn show_active_plots(ui: &mut egui::Ui, plotting_system: &mut PlottingSystem) {
    let window_ids: Vec<String> = plotting_system.windows.keys().cloned().collect();
    
    for window_id in window_ids {
        ui.horizontal(|ui| {
            if let Some(window) = plotting_system.windows.get(&window_id) {
                ui.label(&window.config.title);
                
                let status = if window.is_open { "Open" } else { "Closed" };
                ui.label(format!("({})", status));
                
                if ui.button("Toggle").clicked() {
                    if let Some(window) = plotting_system.windows.get_mut(&window_id) {
                        window.is_open = !window.is_open;
                    }
                }
                
                if ui.button("Export CSV").clicked() {
                    if let Ok(path) = plotting_system.export_data(&window_id, ExportFormat::CSV) {
                        println!("Exported plot data to: {}", path);
                    }
                }
                
                if ui.button("Export JSON").clicked() {
                    if let Ok(path) = plotting_system.export_data(&window_id, ExportFormat::JSON) {
                        println!("Exported plot data to: {}", path);
                    }
                }
                
                if ui.button("Export TSV").clicked() {
                    if let Ok(path) = plotting_system.export_data(&window_id, ExportFormat::TSV) {
                        println!("Exported plot data to: {}", path);
                    }
                }
                
                if ui.button("Remove").clicked() {
                    plotting_system.remove_window(&window_id);
                }
            }
        });
    }
}

pub fn show_plot_windows(ctx: &egui::Context, plotting_system: &mut PlottingSystem) {
    let window_ids: Vec<String> = plotting_system.windows.keys().cloned().collect();
    
    for window_id in window_ids {
        if let Some(window) = plotting_system.windows.get_mut(&window_id) {
            if window.is_open {
                let mut is_open = true;
                egui::Window::new(&window.config.title)
                    .id(egui::Id::new(&window_id))
                    .open(&mut is_open)
                    .default_size([400.0, 300.0])
                    .show(ctx, |ui| {
                        show_plot_content(ui, window);
                    });
                
                window.is_open = is_open;
            }
        }
    }
}

fn show_plot_content(ui: &mut egui::Ui, window: &mut crate::plotting::PlotWindow) {
    ui.horizontal(|ui| {
        ui.label(format!("Type: {:?}", window.config.plot_type));
        ui.label(format!("Quantity: {:?}", window.config.quantity));
        ui.label(format!("Sampling: {:?}", window.config.sampling_mode));
    });
    
    ui.horizontal(|ui| {
        ui.label(format!("Bins: {}", window.config.spatial_bins));
        ui.label(format!("Update freq: {:.1} Hz", window.config.update_frequency));
        if let Some(&last_timestamp) = window.data.timestamps.last() {
            ui.label(format!("Last data: {:.2}s", last_timestamp));
        }
    });
    
    ui.separator();
    
    if window.data.x_data.is_empty() {
        ui.label("No data available yet...");
        match window.config.sampling_mode {
            crate::plotting::SamplingMode::SingleTimestep => {
                ui.label("Click 'Manual Update' below to capture current state");
            }
            crate::plotting::SamplingMode::Continuous => {
                ui.label("Start or resume the simulation to see live data");
            }
            crate::plotting::SamplingMode::TimeAveraged { .. } => {
                ui.label("Start or resume the simulation to see time-averaged data");
            }
        }
    } else {
        // Create custom plot visualization using egui's drawing primitives
        let (x_label, y_label) = get_axis_labels(&window.config);
        
        // Calculate plot area
        let available = ui.available_size();
        let plot_size = egui::Vec2::new(available.x - 20.0, (250.0_f32).min(available.y - 100.0));
        
        let (rect, _response) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
        
        if ui.is_rect_visible(rect) {
            // Draw plot background
            ui.painter().rect_filled(rect, 2.0, egui::Color32::from_gray(240));
            ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
            
            // Calculate plot ranges - use domain bounds for spatial axes
            let (plot_x_min, plot_x_max, plot_y_min, plot_y_max) = 
                calculate_plot_ranges(&window.config, &window.data);
            
            // Convert data points to screen coordinates
            let mut screen_points = Vec::new();
            for i in 0..window.data.x_data.len() {
                let x_norm = (window.data.x_data[i] - plot_x_min) / (plot_x_max - plot_x_min);
                let y_norm = 1.0 - (window.data.y_data[i] - plot_y_min) / (plot_y_max - plot_y_min); // Flip Y
                
                let screen_x = rect.min.x + (x_norm as f32) * rect.width();
                let screen_y = rect.min.y + (y_norm as f32) * rect.height();
                
                screen_points.push(egui::Pos2::new(screen_x, screen_y));
            }
            
            // Draw data line
            if screen_points.len() > 1 {
                for i in 0..screen_points.len() - 1 {
                    ui.painter().line_segment(
                        [screen_points[i], screen_points[i + 1]], 
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 100, 255))
                    );
                }
            }
            
            // Draw data points
            for point in &screen_points {
                ui.painter().circle_filled(*point, 2.0, egui::Color32::from_rgb(0, 100, 255));
            }
            
            // Draw axes labels
            let label_color = egui::Color32::BLACK;
            let font = egui::FontId::proportional(12.0);
            
            // X-axis label (bottom center)
            ui.painter().text(
                egui::Pos2::new(rect.center().x, rect.max.y + 15.0),
                egui::Align2::CENTER_TOP,
                x_label,
                font.clone(),
                label_color,
            );
            
            // Y-axis label (left center, rotated)
            ui.painter().text(
                egui::Pos2::new(rect.min.x - 30.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                y_label,
                font.clone(),
                label_color,
            );
            
            // Draw axis tick labels
            // X-axis ticks
            for i in 0..=4 {
                let x_norm = i as f32 / 4.0;
                let x_val = plot_x_min + (plot_x_max - plot_x_min) * x_norm as f64;
                let screen_x = rect.min.x + x_norm * rect.width();
                
                ui.painter().text(
                    egui::Pos2::new(screen_x, rect.max.y + 5.0),
                    egui::Align2::CENTER_TOP,
                    format!("{:.2}", x_val),
                    egui::FontId::proportional(10.0),
                    label_color,
                );
            }
            
            // Y-axis ticks
            for i in 0..=4 {
                let y_norm = i as f32 / 4.0;
                let y_val = plot_y_min + (plot_y_max - plot_y_min) * (1.0 - y_norm as f64);
                let screen_y = rect.min.y + y_norm * rect.height();
                
                ui.painter().text(
                    egui::Pos2::new(rect.min.x - 5.0, screen_y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.2}", y_val),
                    egui::FontId::proportional(10.0),
                    label_color,
                );
            }
        }
        
        // Show data statistics
        ui.separator();
        ui.horizontal(|ui| {
            let x_min = window.data.x_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let x_max = window.data.x_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let y_min = window.data.y_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let y_max = window.data.y_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            ui.label(format!("X: [{:.3}, {:.3}]", x_min, x_max));
            ui.label(format!("Y: [{:.3}, {:.3}]", y_min, y_max));
            ui.label(format!("Points: {}", window.data.x_data.len()));
        });
    }
    
    ui.separator();
    
    ui.horizontal(|ui| {
        if let Some(&last_timestamp) = window.data.timestamps.last() {
            ui.label(format!("Last update: {:.2}s", last_timestamp));
        }
        
        if ui.button("Clear Data").clicked() {
            window.data.x_data.clear();
            window.data.y_data.clear();
            window.data.timestamps.clear();
        }
        
        if ui.button("Manual Update").clicked() {
            // This will trigger an update on the next frame by resetting last_update
            window.last_update = 0.0; // Force update regardless of sampling mode
        }
        
        if ui.button("Export CSV").clicked() {
            if let Ok(path) = crate::plotting::export::export_plot_data(&window.data, crate::plotting::ExportFormat::CSV) {
                ui.label(format!("Exported to: {}", path));
            }
        }
        
        if ui.button("Export JSON").clicked() {
            if let Ok(path) = crate::plotting::export::export_plot_data(&window.data, crate::plotting::ExportFormat::JSON) {
                ui.label(format!("Exported to: {}", path));
            }
        }
        
        if ui.button("Export TSV").clicked() {
            if let Ok(path) = crate::plotting::export::export_plot_data(&window.data, crate::plotting::ExportFormat::TSV) {
                ui.label(format!("Exported to: {}", path));
            }
        }
    });
}

fn get_axis_labels(config: &crate::plotting::PlotConfig) -> (&'static str, &'static str) {
    use crate::plotting::{PlotType, Quantity};
    
    let x_label = match config.plot_type {
        PlotType::SpatialProfileX => "X Position",
        PlotType::SpatialProfileY => "Y Position", 
        PlotType::TimeSeries => "Time (s)",
    };
    
    let y_label = match config.quantity {
        Quantity::Charge => "Charge",
        Quantity::ElectronCount => "Electron Count",
        Quantity::Velocity => "Velocity",
        Quantity::TotalSpeciesCount(_) => "Count",
        Quantity::FoilCurrent(_) => "Current (A)",
        Quantity::ElectronHopRate => "Hop Rate (1/s)",
        Quantity::LocalFieldStrength => "Field Strength",
        Quantity::DendriteFormationRate => "Formation Rate (1/s)",
    };
    
    (x_label, y_label)
}

fn calculate_plot_ranges(config: &crate::plotting::PlotConfig, data: &crate::plotting::PlotData) -> (f64, f64, f64, f64) {
    use crate::plotting::PlotType;
    use crate::config::DOMAIN_BOUNDS;
    
    // For spatial plots, use domain bounds for the spatial axis
    match config.plot_type {
        PlotType::SpatialProfileX => {
            // X-axis should be domain bounds, Y-axis based on data
            let x_min = -(DOMAIN_BOUNDS as f64);
            let x_max = DOMAIN_BOUNDS as f64;
            
            let (y_min, y_max) = if data.y_data.is_empty() {
                (0.0, 1.0)
            } else {
                let data_y_min = data.y_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let data_y_max = data.y_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let y_range = (data_y_max - data_y_min).max(0.001);
                let y_padding = y_range * 0.05;
                (data_y_min - y_padding, data_y_max + y_padding)
            };
            
            (x_min, x_max, y_min, y_max)
        }
        PlotType::SpatialProfileY => {
            // X-axis should be domain bounds, Y-axis based on data
            let x_min = -(DOMAIN_BOUNDS as f64);
            let x_max = DOMAIN_BOUNDS as f64;
            
            let (y_min, y_max) = if data.y_data.is_empty() {
                (0.0, 1.0)
            } else {
                let data_y_min = data.y_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let data_y_max = data.y_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let y_range = (data_y_max - data_y_min).max(0.001);
                let y_padding = y_range * 0.05;
                (data_y_min - y_padding, data_y_max + y_padding)
            };
            
            (x_min, x_max, y_min, y_max)
        }
        _ => {
            // For all other plot types, use data bounds with padding
            if data.x_data.is_empty() || data.y_data.is_empty() {
                (0.0, 1.0, 0.0, 1.0)
            } else {
                let data_x_min = data.x_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let data_x_max = data.x_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let data_y_min = data.y_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let data_y_max = data.y_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                
                let x_range = (data_x_max - data_x_min).max(0.001);
                let y_range = (data_y_max - data_y_min).max(0.001);
                let x_padding = x_range * 0.05;
                let y_padding = y_range * 0.05;
                
                (data_x_min - x_padding, data_x_max + x_padding, 
                 data_y_min - y_padding, data_y_max + y_padding)
            }
        }
    }
}
