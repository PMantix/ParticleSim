/// Export DOE results to CSV format for Excel analysis
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use super::measurement::MeasurementSample;
use super::config::TestCase;

pub fn export_results_to_csv(
    case: &TestCase,
    samples: &[MeasurementSample],
    output_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;
    
    // Create filename based on case ID
    let filename = format!("{}/{}.csv", output_dir, case.case_id);
    let mut file = File::create(&filename)?;
    
    // Group samples by timestamp
    let mut time_groups: HashMap<String, Vec<&MeasurementSample>> = HashMap::new();
    for sample in samples {
        let time_key = format!("{:.1}", sample.time_fs);
        time_groups.entry(time_key).or_insert_with(Vec::new).push(sample);
    }
    
    // Sort timestamps
    let mut timestamps: Vec<_> = time_groups.keys().collect();
    timestamps.sort_by(|a, b| {
        a.parse::<f32>().unwrap_or(0.0)
            .partial_cmp(&b.parse::<f32>().unwrap_or(0.0))
            .unwrap()
    });
    
    // Get unique position labels (sorted)
    let mut position_labels: Vec<String> = Vec::new();
    if let Some(first_group) = timestamps.first().and_then(|t| time_groups.get(*t)) {
        position_labels = first_group.iter().map(|s| s.position_label.clone()).collect();
        position_labels.sort();
    }
    
    // Write header with all position columns
    write!(file, "Time_fs")?;
    for label in &position_labels {
        write!(file, ",{}_Edge", label)?;
    }
    for label in &position_labels {
        write!(file, ",{}_LiMetal", label)?;
    }
    for label in &position_labels {
        write!(file, ",{}_LiIon", label)?;
    }
    writeln!(file)?;
    
    // Write data rows (one row per timestamp)
    for time_key in timestamps {
        let group = time_groups.get(time_key).unwrap();
        
        // Create a map for quick lookup by position label
        let sample_map: HashMap<_, _> = group.iter()
            .map(|s| (s.position_label.as_str(), *s))
            .collect();
        
        // Write timestamp
        write!(file, "{}", time_key)?;
        
        // Write edge positions for all positions
        for label in &position_labels {
            if let Some(sample) = sample_map.get(label.as_str()) {
                write!(file, ",{}", sample.lithium_metal_edge_position)?;
            } else {
                write!(file, ",0")?;
            }
        }
        
        // Write Li metal counts for all positions
        for label in &position_labels {
            if let Some(sample) = sample_map.get(label.as_str()) {
                write!(file, ",{}", sample.lithium_metal_count)?;
            } else {
                write!(file, ",0")?;
            }
        }
        
        // Write Li ion counts for all positions
        for label in &position_labels {
            if let Some(sample) = sample_map.get(label.as_str()) {
                write!(file, ",{}", sample.lithium_ion_count)?;
            } else {
                write!(file, ",0")?;
            }
        }
        
        writeln!(file)?;
    }
    
    println!("✓ Exported results for case {} to {}", case.case_id, filename);
    Ok(())
}

/// Export summary statistics for all cases
pub fn export_doe_summary(
    cases: &[TestCase],
    all_samples: &[Vec<MeasurementSample>],
    output_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = format!("{}/DOE_Summary.csv", output_dir);
    let mut file = File::create(&filename)?;
    
    // Write header
    writeln!(
        file,
        "Case_ID,Mode,Overpotential,Switching_Freq,Final_Li_Metal_Count,Avg_Edge_Position,Max_Edge_Position"
    )?;
    
    // Write summary row for each case
    for (case, samples) in cases.iter().zip(all_samples.iter()) {
        if samples.is_empty() {
            continue;
        }
        
        // Calculate summary statistics
        let final_samples: Vec<_> = samples.iter()
            .filter(|s| s.time_fs >= samples.last().map(|l| l.time_fs - 1000.0).unwrap_or(0.0))
            .collect();
        
        let final_li_metal_count: usize = final_samples.iter()
            .map(|s| s.lithium_metal_count)
            .sum::<usize>() / final_samples.len().max(1);
        
        let avg_edge_position: f32 = final_samples.iter()
            .map(|s| s.lithium_metal_edge_position)
            .sum::<f32>() / final_samples.len() as f32;
        
        let max_edge_position: f32 = samples.iter()
            .map(|s| s.lithium_metal_edge_position.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        
        let freq_str = case.switching_frequency_steps
            .map(|f| f.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        
        writeln!(
            file,
            "{},{:?},{},{},{},{},{}",
            case.case_id,
            case.mode,
            case.overpotential,
            freq_str,
            final_li_metal_count,
            avg_edge_position,
            max_edge_position
        )?;
    }
    
    println!("✓ Exported DOE summary to {}", filename);
    Ok(())
}
