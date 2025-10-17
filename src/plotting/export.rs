// plotting/export.rs
// Data export functionality for plots

use super::{PlotData, ExportFormat};
use std::fs::File;
use std::io::Write;

pub fn export_plot_data(data: &PlotData, format: ExportFormat) -> Result<String, String> {
    // Use current system time for timestamp
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let timestamp = duration.as_secs();
    
    let filename = match format {
        ExportFormat::CSV => format!("plot_{}_{}.csv", data.config.title.replace(" ", "_"), timestamp),
        ExportFormat::JSON => format!("plot_{}_{}.json", data.config.title.replace(" ", "_"), timestamp),
        ExportFormat::TSV => format!("plot_{}_{}.tsv", data.config.title.replace(" ", "_"), timestamp),
    };

    let content = match format {
        ExportFormat::CSV => export_csv(data)?,
        ExportFormat::JSON => export_json(data)?,
        ExportFormat::TSV => export_tsv(data)?,
    };

    let path = std::path::Path::new("plots").join(&filename);
    
    // Create plots directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let mut file = File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(content.as_bytes()).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

fn export_csv(data: &PlotData) -> Result<String, String> {
    let mut content = String::new();
    
    // Header
    content.push_str("# Plot Data Export\n");
    content.push_str(&format!("# Title: {}\n", data.config.title));
    content.push_str(&format!("# Plot Type: {:?}\n", data.config.plot_type));
    content.push_str(&format!("# Quantity: {:?}\n", data.config.quantity));
    content.push_str(&format!("# Sampling Mode: {:?}\n", data.config.sampling_mode));
    
    for (key, value) in &data.metadata {
        content.push_str(&format!("# {}: {}\n", key, value));
    }
    
    content.push('\n');
    
    // Data header
    match data.config.plot_type {
        super::PlotType::SpatialProfileX => content.push_str("X_Position,Value\n"),
        super::PlotType::SpatialProfileY => content.push_str("Y_Position,Value\n"),
        super::PlotType::TimeSeries => content.push_str("Time,Value\n"),
    }
    
    // Data rows
    for i in 0..data.x_data.len().min(data.y_data.len()) {
        content.push_str(&format!("{},{}\n", data.x_data[i], data.y_data[i]));
    }
    
    Ok(content)
}

fn export_json(data: &PlotData) -> Result<String, String> {
    serde_json::to_string_pretty(data).map_err(|e| format!("JSON serialization error: {}", e))
}

fn export_tsv(data: &PlotData) -> Result<String, String> {
    let csv_content = export_csv(data)?;
    Ok(csv_content.replace(",", "\t"))
}
