// plotting/gui.rs
// GUI controls for the plotting system

use crate::plotting::{PlotType, Quantity, SamplingMode, PlotConfig, PlottingSystem, ExportFormat};
use crate::body::Species;
use quarkstrom::egui;

pub fn show_plotting_controls(
    ui: &mut egui::Ui,
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
                if ui.button("Li+ Population").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::TimeSeries,
                        quantity: Quantity::TotalSpeciesCount(Species::LithiumIon),
                        title: "Lithium Ion Population".to_string(),
                        sampling_mode: SamplingMode::Continuous,
                        spatial_bins: 50,
                        time_window: 10.0,
                        update_frequency: 5.0,
                    };
                    plotting_system.create_plot_window(config);
                }
                
                if ui.button("Charge vs X").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::SpatialProfileX,
                        quantity: Quantity::Charge,
                        title: "Charge Distribution (X-axis)".to_string(),
                        sampling_mode: SamplingMode::SingleTimestep,
                        spatial_bins: 50,
                        time_window: 10.0,
                        update_frequency: 5.0,
                    };
                    plotting_system.create_plot_window(config);
                }
            });
            
            ui.horizontal(|ui| {
                if ui.button("Velocity Profile").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::SpatialProfileX,
                        quantity: Quantity::Velocity,
                        title: "Velocity Profile (X-axis)".to_string(),
                        sampling_mode: SamplingMode::SingleTimestep,
                        spatial_bins: 50,
                        time_window: 10.0,
                        update_frequency: 5.0,
                    };
                    plotting_system.create_plot_window(config);
                }
                
                if ui.button("Foil Current").clicked() {
                    let config = PlotConfig {
                        plot_type: PlotType::TimeSeries,
                        quantity: Quantity::FoilCurrent(1), // Use foil ID 1 by default
                        title: "Foil Current Analysis".to_string(),
                        sampling_mode: SamplingMode::Continuous,
                        spatial_bins: 50,
                        time_window: 10.0,
                        update_frequency: 10.0,
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
                        ui.selectable_value(new_plot_type, PlotType::ConcentrationMap, "Concentration Map");
                        ui.selectable_value(new_plot_type, PlotType::ChargeDistribution, "Charge Distribution");
                        ui.selectable_value(new_plot_type, PlotType::SpeciesPopulation, "Species Population");
                        ui.selectable_value(new_plot_type, PlotType::CurrentAnalysis, "Current Analysis");
                    });
            });
            
            ui.horizontal(|ui| {
                ui.label("Quantity:");
                show_quantity_selector(ui, new_plot_quantity);
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

fn show_quantity_selector(ui: &mut egui::Ui, quantity: &mut Quantity) {
    egui::ComboBox::from_id_source("quantity")
        .selected_text(format!("{:?}", quantity))
        .show_ui(ui, |ui| {
            ui.selectable_value(quantity, Quantity::Charge, "Charge");
            ui.selectable_value(quantity, Quantity::ElectronCount, "Electron Count");
            ui.selectable_value(quantity, Quantity::Velocity, "Velocity");
            ui.selectable_value(quantity, Quantity::TotalSpeciesCount(Species::LithiumIon), "Li+ Count");
            ui.selectable_value(quantity, Quantity::TotalSpeciesCount(Species::LithiumMetal), "Li Metal Count");
            ui.selectable_value(quantity, Quantity::TotalSpeciesCount(Species::ElectrolyteAnion), "Anion Count");
            ui.selectable_value(quantity, Quantity::SpeciesConcentration(Species::LithiumIon), "Li+ Concentration");
            ui.selectable_value(quantity, Quantity::FoilCurrent(1), "Foil Current (ID 1)");
            ui.selectable_value(quantity, Quantity::ElectronHopRate, "Electron Hop Rate");
            ui.selectable_value(quantity, Quantity::LocalFieldStrength, "Local Field Strength");
        });
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
    
    ui.separator();
    
    if window.data.x_data.is_empty() {
        ui.label("No data available yet...");
    } else {
        // Simple text-based plot display for now
        ui.label(format!("Data Range: X=[{:.2}, {:.2}], Y=[{:.2}, {:.2}]", 
            window.data.x_data.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            window.data.x_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            window.data.y_data.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            window.data.y_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        ));
        
        // Show recent data points in a scrollable area
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                let recent_count = window.data.x_data.len().min(10);
                let start_idx = window.data.x_data.len().saturating_sub(recent_count);
                
                ui.label("Recent Data Points:");
                for i in start_idx..window.data.x_data.len() {
                    ui.label(format!("  {:.3}, {:.3}", window.data.x_data[i], window.data.y_data[i]));
                }
            });
    }
    
    ui.separator();
    
    ui.horizontal(|ui| {
        ui.label(format!("Data points: {}", window.data.x_data.len()));
        if let Some(&last_timestamp) = window.data.timestamps.last() {
            ui.label(format!("Last update: {:.2}s", last_timestamp));
        }
        
        if ui.button("Clear Data").clicked() {
            window.data.x_data.clear();
            window.data.y_data.clear();
            window.data.timestamps.clear();
        }
        
        if ui.button("Manual Update").clicked() && matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
            // Manual update will be triggered by the main update loop
        }
    });
}
